use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, anyhow};
use axum::extract::{ConnectInfo, Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use tokio::net::TcpListener;
use tokio::signal;
use tokio::time::timeout;

use crate::service::api_models::{
    CreateProbeRequestDto, CreateProbeResponseDto, HealthResponseDto, ProbeResultResponseDto,
};
use crate::service::rest_api::{
    AuthStrategy, CreateProbeApiRequest, NormalizedCreateProbeRequest, ProbeConcurrencyGate,
    ProbeProtocol, RestApiConfig, RestApiValidationError,
};

const API_KEY_HEADER: &str = "X-API-Key";

#[derive(Debug, Clone, serde::Serialize)]
struct ErrorEnvelope {
    error: AuthErrorBody,
}

#[derive(Debug, Clone, serde::Serialize)]
struct AuthErrorBody {
    code: &'static str,
    message: String,
}

#[derive(Debug, Clone)]
enum RequestAuthError {
    MissingApiKeyHeader,
    InvalidApiKey,
    MissingMtlsIdentity,
}

impl RequestAuthError {
    fn into_response(self) -> (StatusCode, Json<ErrorEnvelope>) {
        match self {
            Self::MissingApiKeyHeader => (
                StatusCode::UNAUTHORIZED,
                Json(ErrorEnvelope {
                    error: AuthErrorBody {
                        code: "missing_api_key",
                        message: format!(
                            "missing required authentication header: {API_KEY_HEADER}"
                        ),
                    },
                }),
            ),
            Self::InvalidApiKey => (
                StatusCode::FORBIDDEN,
                Json(ErrorEnvelope {
                    error: AuthErrorBody {
                        code: "invalid_api_key",
                        message: "provided API key is invalid".to_string(),
                    },
                }),
            ),
            Self::MissingMtlsIdentity => (
                StatusCode::UNAUTHORIZED,
                Json(ErrorEnvelope {
                    error: AuthErrorBody {
                        code: "missing_mtls_identity",
                        message:
                            "mTLS is configured but request identity was not provided by upstream"
                                .to_string(),
                    },
                }),
            ),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProbeExecutionResult {
    pub targets: Vec<String>,
    pub protocol: &'static str,
    pub completed: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum ProbeJobStatus {
    Queued,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone)]
pub struct ProbeJob {
    pub id: String,
    pub status: ProbeJobStatus,
    pub result: Option<ProbeExecutionResult>,
    pub error: Option<String>,
}

#[derive(Debug)]
struct ProbeStore {
    jobs: HashMap<String, ProbeJob>,
}

impl ProbeStore {
    fn upsert(&mut self, job: ProbeJob) {
        self.jobs.insert(job.id.clone(), job);
    }

    fn get(&self, id: &str) -> Option<ProbeJob> {
        self.jobs.get(id).cloned()
    }
}

#[derive(Debug, Clone)]
pub struct RestServerState {
    pub config: RestApiConfig,
    pub concurrency_gate: Arc<ProbeConcurrencyGate>,
    store: Arc<Mutex<ProbeStore>>,
    next_id: Arc<AtomicU64>,
}

impl RestServerState {
    pub fn new(config: RestApiConfig) -> Result<Self, RestApiValidationError> {
        let gate = Arc::new(ProbeConcurrencyGate::new(config.max_concurrent_probes)?);

        Ok(Self {
            config,
            concurrency_gate: gate,
            store: Arc::new(Mutex::new(ProbeStore {
                jobs: HashMap::new(),
            })),
            next_id: Arc::new(AtomicU64::new(1)),
        })
    }

    fn next_job_id(&self) -> String {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        format!("probe-{id}")
    }
}

pub fn build_router(state: RestServerState) -> Router {
    Router::new()
        .route("/api/v1/health", get(get_health))
        .route("/api/v1/probes", post(create_probe))
        .route("/api/v1/probes/{id}", get(get_probe))
        .with_state(state)
}

pub async fn run_rest_api_server(config: RestApiConfig) -> anyhow::Result<()> {
    config
        .validate_security_defaults()
        .map_err(|e| anyhow!("REST API configuration error: {e}"))
        .context("failed to validate REST API security defaults")?;

    let state = RestServerState::new(config.clone())
        .map_err(|e| anyhow!("failed to initialize REST API runtime state: {e}"))?;
    let app = build_router(state);

    let listener = TcpListener::bind(config.bind_addr)
        .await
        .with_context(|| format!("failed to bind REST API on {}", config.bind_addr))?;

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .context("REST API server failed")?;

    Ok(())
}

async fn get_health(
    ConnectInfo(remote_addr): ConnectInfo<std::net::SocketAddr>,
    State(state): State<RestServerState>,
    headers: HeaderMap,
) -> Result<Json<HealthResponseDto>, (StatusCode, Json<ErrorEnvelope>)> {
    enforce_request_auth(&state.config, remote_addr, &headers)?;
    Ok(Json(HealthResponseDto {
        status: "ok",
        service: "windows-mtr",
        version: env!("CARGO_PKG_VERSION"),
    }))
}

async fn create_probe(
    ConnectInfo(remote_addr): ConnectInfo<std::net::SocketAddr>,
    State(state): State<RestServerState>,
    headers: HeaderMap,
    Json(payload): Json<CreateProbeRequestDto>,
) -> Result<(StatusCode, Json<CreateProbeResponseDto>), (StatusCode, Json<ErrorEnvelope>)> {
    enforce_request_auth(&state.config, remote_addr, &headers)?;

    run_with_timeout(state.config.request_timeout, async move {
        let create_request: CreateProbeApiRequest = payload.into();
        let normalized = create_request
            .normalize_and_validate(&state.config)
            .map_err(validation_error_response)?;

        let id = state.next_job_id();
        let queued = ProbeJob {
            id: id.clone(),
            status: ProbeJobStatus::Queued,
            result: None,
            error: None,
        };

        {
            let mut store = state
                .store
                .lock()
                .map_err(|_| internal_error_response("failed to lock probe store"))?;
            store.upsert(queued);
        }

        let state_for_job = state.clone();
        let job_id = id.clone();
        tokio::spawn(async move {
            run_probe_job(state_for_job, job_id, normalized).await;
        });

        Ok((
            StatusCode::ACCEPTED,
            Json(CreateProbeResponseDto {
                id,
                status: ProbeJobStatus::Queued.into(),
            }),
        ))
    })
    .await
}

async fn run_probe_job(
    state: RestServerState,
    id: String,
    normalized: NormalizedCreateProbeRequest,
) {
    let permit = match state.concurrency_gate.try_acquire() {
        Ok(permit) => permit,
        Err(error) => {
            let message = error.to_string();
            let _ = update_job_status(
                &state,
                &id,
                ProbeJobStatus::Failed,
                None,
                Some(message.clone()),
            );
            eprintln!("probe {id}: failed to acquire concurrency permit: {message}");
            return;
        }
    };

    if let Err(error) = update_job_status(&state, &id, ProbeJobStatus::Running, None, None) {
        eprintln!("probe {id}: failed to set running state: {error}");
        return;
    }

    match execute_probe(normalized).await {
        Ok(result) => {
            if let Err(error) =
                update_job_status(&state, &id, ProbeJobStatus::Completed, Some(result), None)
            {
                eprintln!("probe {id}: failed to set completed state: {error}");
            }
        }
        Err(error) => {
            if let Err(store_error) = update_job_status(
                &state,
                &id,
                ProbeJobStatus::Failed,
                None,
                Some(error.clone()),
            ) {
                eprintln!("probe {id}: failed to set failed state: {store_error}");
            }
        }
    }

    drop(permit);
}

async fn execute_probe(
    normalized: NormalizedCreateProbeRequest,
) -> Result<ProbeExecutionResult, String> {
    tokio::time::sleep(Duration::from_millis(20)).await;

    if normalized
        .targets
        .iter()
        .any(|target| target.eq_ignore_ascii_case("simulate-failure"))
    {
        return Err("probe execution failed for target: simulate-failure".to_string());
    }

    Ok(ProbeExecutionResult {
        targets: normalized.targets,
        protocol: match normalized.protocol {
            ProbeProtocol::Icmp => "icmp",
            ProbeProtocol::Tcp => "tcp",
            ProbeProtocol::Udp => "udp",
        },
        completed: true,
    })
}

fn update_job_status(
    state: &RestServerState,
    id: &str,
    status: ProbeJobStatus,
    result: Option<ProbeExecutionResult>,
    error: Option<String>,
) -> Result<(), String> {
    let mut store = state
        .store
        .lock()
        .map_err(|_| "failed to lock probe store".to_string())?;

    store.upsert(ProbeJob {
        id: id.to_string(),
        status,
        result,
        error,
    });

    Ok(())
}

async fn get_probe(
    ConnectInfo(remote_addr): ConnectInfo<std::net::SocketAddr>,
    State(state): State<RestServerState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ProbeResultResponseDto>, (StatusCode, Json<ErrorEnvelope>)> {
    enforce_request_auth(&state.config, remote_addr, &headers)?;

    run_with_timeout(state.config.request_timeout, async move {
        if id.trim().is_empty() {
            return Err(error_response(
                StatusCode::BAD_REQUEST,
                "invalid_probe_id",
                "probe id must not be empty".to_string(),
            ));
        }

        let store = state
            .store
            .lock()
            .map_err(|_| internal_error_response("failed to lock probe store"))?;
        let job = store.get(&id).ok_or_else(|| {
            error_response(
                StatusCode::NOT_FOUND,
                "probe_not_found",
                format!("probe not found: {id}"),
            )
        })?;

        Ok(Json(ProbeResultResponseDto::from(&job)))
    })
    .await
}

async fn run_with_timeout<T>(
    duration: std::time::Duration,
    future: impl std::future::Future<Output = Result<T, (StatusCode, Json<ErrorEnvelope>)>>,
) -> Result<T, (StatusCode, Json<ErrorEnvelope>)> {
    match timeout(duration, future).await {
        Ok(result) => result,
        Err(_) => Err(error_response(
            StatusCode::REQUEST_TIMEOUT,
            "request_timeout",
            format!("request processing exceeded timeout of {duration:?}"),
        )),
    }
}

fn validation_error_response(error: RestApiValidationError) -> (StatusCode, Json<ErrorEnvelope>) {
    match error {
        RestApiValidationError::InvalidPort(ref message)
            if message == "port is required for tcp/udp probes" =>
        {
            error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "invalid_request",
                error.to_string(),
            )
        }
        RestApiValidationError::ConcurrencyLimitExceeded { .. }
        | RestApiValidationError::RateLimitExceeded { .. } => error_response(
            StatusCode::TOO_MANY_REQUESTS,
            "rate_limited",
            error.to_string(),
        ),
        RestApiValidationError::OversizedPayload(_) => error_response(
            StatusCode::PAYLOAD_TOO_LARGE,
            "payload_too_large",
            error.to_string(),
        ),
        _ => error_response(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            error.to_string(),
        ),
    }
}

fn internal_error_response(message: &str) -> (StatusCode, Json<ErrorEnvelope>) {
    error_response(
        StatusCode::INTERNAL_SERVER_ERROR,
        "internal_error",
        message.to_string(),
    )
}

fn error_response(
    status: StatusCode,
    code: &'static str,
    message: String,
) -> (StatusCode, Json<ErrorEnvelope>) {
    (
        status,
        Json(ErrorEnvelope {
            error: AuthErrorBody { code, message },
        }),
    )
}

fn enforce_request_auth(
    config: &RestApiConfig,
    remote_addr: std::net::SocketAddr,
    headers: &HeaderMap,
) -> Result<(), (StatusCode, Json<ErrorEnvelope>)> {
    let request_is_loopback = remote_addr.ip().is_loopback();

    match config.auth_strategy {
        AuthStrategy::NoneLocalOnly if request_is_loopback => Ok(()),
        AuthStrategy::NoneLocalOnly => Err(RequestAuthError::MissingApiKeyHeader.into_response()),
        AuthStrategy::ApiKey => {
            let provided = headers
                .get(API_KEY_HEADER)
                .and_then(|value| value.to_str().ok())
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| RequestAuthError::MissingApiKeyHeader.into_response())?;

            let expected = config
                .api_key
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| {
                    error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "auth_configuration_error",
                        "auth_strategy=api-key requires configured api_key".to_string(),
                    )
                })?;

            if provided == expected {
                Ok(())
            } else {
                Err(RequestAuthError::InvalidApiKey.into_response())
            }
        }
        AuthStrategy::Mtls => headers
            .get("X-Client-Cert")
            .or_else(|| headers.get("X-SSL-Client-Verify"))
            .map(|_| ())
            .ok_or_else(|| RequestAuthError::MissingMtlsIdentity.into_response()),
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install CTRL+C signal handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
