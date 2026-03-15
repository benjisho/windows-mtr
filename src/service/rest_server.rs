use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, anyhow};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use tokio::net::TcpListener;
use tokio::signal;
use tokio::time::timeout;

use crate::service::api_models::{
    CreateProbeRequestDto, CreateProbeResponseDto, HealthResponseDto, ProbeResultResponseDto,
};
use crate::service::rest_api::{
    CreateProbeApiRequest, NormalizedCreateProbeRequest, ProbeConcurrencyGate, ProbeProtocol,
    RestApiConfig, RestApiValidationError,
};

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

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("REST API server failed")?;

    Ok(())
}

async fn get_health() -> Json<HealthResponseDto> {
    Json(HealthResponseDto {
        status: "ok",
        service: "windows-mtr",
        version: env!("CARGO_PKG_VERSION"),
    })
}

async fn create_probe(
    State(state): State<RestServerState>,
    Json(payload): Json<CreateProbeRequestDto>,
) -> Result<(StatusCode, Json<CreateProbeResponseDto>), (StatusCode, String)> {
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
    State(state): State<RestServerState>,
    Path(id): Path<String>,
) -> Result<Json<ProbeResultResponseDto>, (StatusCode, String)> {
    run_with_timeout(state.config.request_timeout, async move {
        if id.trim().is_empty() {
            return Err((
                StatusCode::BAD_REQUEST,
                "probe id must not be empty".to_string(),
            ));
        }

        let store = state
            .store
            .lock()
            .map_err(|_| internal_error_response("failed to lock probe store"))?;
        let job = store
            .get(&id)
            .ok_or_else(|| (StatusCode::NOT_FOUND, format!("probe not found: {id}")))?;

        Ok(Json(ProbeResultResponseDto::from(&job)))
    })
    .await
}

async fn run_with_timeout<T>(
    duration: std::time::Duration,
    future: impl std::future::Future<Output = Result<T, (StatusCode, String)>>,
) -> Result<T, (StatusCode, String)> {
    match timeout(duration, future).await {
        Ok(result) => result,
        Err(_) => Err((
            StatusCode::REQUEST_TIMEOUT,
            format!("request processing exceeded timeout of {duration:?}"),
        )),
    }
}

fn validation_error_response(error: RestApiValidationError) -> (StatusCode, String) {
    match error {
        RestApiValidationError::InvalidPort(ref message)
            if message == "port is required for tcp/udp probes" =>
        {
            (StatusCode::UNPROCESSABLE_ENTITY, error.to_string())
        }
        RestApiValidationError::ConcurrencyLimitExceeded { .. }
        | RestApiValidationError::RateLimitExceeded { .. } => {
            (StatusCode::TOO_MANY_REQUESTS, error.to_string())
        }
        RestApiValidationError::OversizedPayload(_) => {
            (StatusCode::PAYLOAD_TOO_LARGE, error.to_string())
        }
        _ => (StatusCode::BAD_REQUEST, error.to_string()),
    }
}

fn internal_error_response(message: &str) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, message.to_string())
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
