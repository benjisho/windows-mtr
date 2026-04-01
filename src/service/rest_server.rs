use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use anyhow::{Context, anyhow};
use axum::body::{Body, to_bytes};
use axum::extract::{ConnectInfo, Path, Request, State};
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::{Next, from_fn_with_state};
use axum::routing::{get, post};
use axum::{Json, Router};
use subtle::ConstantTimeEq;
use tokio::net::TcpListener;
use tokio::signal;
use tokio::time::timeout;

use crate::api_error::ApiError;
use crate::service::api_models::{
    ApiResponseMetaDto, CreateProbeDataDto, CreateProbeRequestDto, CreateProbeResponseDto,
    HealthDataDto, HealthResponseDto, ProbeResultResponseDto,
};
use crate::service::rest_api::{
    AuthStrategy, CreateProbeApiRequest, FixedWindowRateLimiter, NormalizedCreateProbeRequest,
    ProbeConcurrencyGate, ProbeProtocol, RestApiConfig, RestApiValidationError,
    validate_payload_size,
};
use crate::service::{
    EnhancedUiConfig, ProbeRequest, UiMode, build_probe_plan, run_embedded_trippy,
};

const API_KEY_HEADER: &str = "X-API-Key";
const EMBEDDED_TRIPPY_ENV: &str = "WINDOWS_MTR_EMBEDDED_TRIPPY";

type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug, Clone)]
enum RequestAuthError {
    MissingApiKeyHeader,
    InvalidApiKey,
    MissingMtlsIdentity,
    UntrustedMtlsIngress,
    NoneLocalOnlyRemoteAccessDenied,
}

impl RequestAuthError {
    fn into_api_error(self) -> ApiError {
        match self {
            Self::MissingApiKeyHeader => ApiError {
                status: StatusCode::UNAUTHORIZED,
                code: "missing_api_key",
                title: "Authentication required",
                detail: format!("missing required authentication header: {API_KEY_HEADER}"),
            },
            Self::InvalidApiKey => ApiError {
                status: StatusCode::FORBIDDEN,
                code: "invalid_api_key",
                title: "Forbidden",
                detail: "provided API key is invalid".to_string(),
            },
            Self::MissingMtlsIdentity => ApiError {
                status: StatusCode::UNAUTHORIZED,
                code: "missing_mtls_identity",
                title: "Authentication required",
                detail: "mTLS is configured but request identity was not provided by upstream"
                    .to_string(),
            },
            Self::UntrustedMtlsIngress => ApiError {
                status: StatusCode::FORBIDDEN,
                code: "untrusted_mtls_ingress",
                title: "Forbidden",
                detail: "mTLS identity headers are accepted only from trusted ingress sources"
                    .to_string(),
            },
            Self::NoneLocalOnlyRemoteAccessDenied => ApiError {
                status: StatusCode::FORBIDDEN,
                code: "auth_strategy_violation",
                title: "Remote access not allowed for none-local-only",
                detail:
                    "auth strategy none-local-only permits requests only from loopback addresses"
                        .to_string(),
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProbeExecutionResult {
    pub targets: Vec<String>,
    pub protocol: &'static str,
    pub completed: bool,
    pub target_results: Vec<ProbeTargetExecutionResult>,
}

#[derive(Debug, Clone)]
pub struct ProbeTargetExecutionResult {
    pub target: String,
    pub success: bool,
    pub error: Option<String>,
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
    pub finished_at: Option<Instant>,
}

#[derive(Debug)]
struct ProbeStore {
    jobs: HashMap<String, ProbeJob>,
    max_completed_jobs: usize,
    completed_job_ttl: std::time::Duration,
}

impl ProbeStore {
    fn prune(&mut self, now: Instant) {
        self.jobs.retain(|_, job| match job.finished_at {
            Some(finished_at) => now.duration_since(finished_at) < self.completed_job_ttl,
            None => true,
        });

        let terminal_count = self
            .jobs
            .values()
            .filter(|job| job.finished_at.is_some())
            .count();

        if terminal_count <= self.max_completed_jobs {
            return;
        }

        let mut completed = self
            .jobs
            .iter()
            .filter_map(|(id, job)| job.finished_at.map(|finished_at| (id.clone(), finished_at)))
            .collect::<Vec<_>>();
        completed.sort_by_key(|(_, finished_at)| *finished_at);

        let to_remove = terminal_count.saturating_sub(self.max_completed_jobs);
        for (id, _) in completed.into_iter().take(to_remove) {
            self.jobs.remove(&id);
        }
    }

    fn upsert(&mut self, job: ProbeJob) {
        self.prune(Instant::now());
        self.jobs.insert(job.id.clone(), job);
        self.prune(Instant::now());
    }

    fn get(&mut self, id: &str) -> Option<ProbeJob> {
        self.prune(Instant::now());
        self.jobs.get(id).cloned()
    }
}

#[derive(Debug, Clone)]
pub struct RestServerState {
    pub config: RestApiConfig,
    pub concurrency_gate: Arc<ProbeConcurrencyGate>,
    probe_rate_limiter: Arc<Mutex<FixedWindowRateLimiter>>,
    store: Arc<Mutex<ProbeStore>>,
    next_id: Arc<AtomicU64>,
    probe_runner_path: Arc<PathBuf>,
}

impl RestServerState {
    pub fn new(config: RestApiConfig) -> Result<Self, RestApiValidationError> {
        Self::new_with_probe_runner(config, probe_runner_path_from_env())
    }

    pub fn new_with_probe_runner(
        config: RestApiConfig,
        probe_runner_path: PathBuf,
    ) -> Result<Self, RestApiValidationError> {
        let max_completed_jobs = config.max_completed_jobs;
        let completed_job_ttl = config.completed_job_ttl;
        let gate = Arc::new(ProbeConcurrencyGate::new(config.max_concurrent_probes)?);
        let limiter = Arc::new(Mutex::new(FixedWindowRateLimiter::new(
            config.max_requests_per_window,
            config.rate_limit_window,
            Instant::now(),
        )?));

        Ok(Self {
            config,
            concurrency_gate: gate,
            probe_rate_limiter: limiter,
            store: Arc::new(Mutex::new(ProbeStore {
                jobs: HashMap::new(),
                max_completed_jobs,
                completed_job_ttl,
            })),
            next_id: Arc::new(AtomicU64::new(1)),
            probe_runner_path: Arc::new(probe_runner_path),
        })
    }

    fn next_job_id(&self) -> String {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        format!("probe-{id}")
    }
}

pub fn build_router(state: RestServerState) -> Router {
    let probe_guard_state = state.clone();

    Router::new()
        .route("/api/v1/health", get(get_health))
        .route(
            "/api/v1/probes",
            post(create_probe).route_layer(from_fn_with_state(
                probe_guard_state,
                enforce_probe_request_guards,
            )),
        )
        .route("/api/v1/probes/{id}", get(get_probe))
        .with_state(state)
}

async fn enforce_probe_request_guards(
    State(state): State<RestServerState>,
    request: Request,
    next: Next,
) -> ApiResult<axum::response::Response> {
    {
        let mut limiter = state
            .probe_rate_limiter
            .lock()
            .map_err(|_| internal_error_response("failed to lock probe rate limiter"))?;
        limiter
            .allow(Instant::now())
            .map_err(validation_error_response)?;
    }

    let (parts, body) = request.into_parts();
    let payload = to_bytes(body, state.config.max_payload_bytes + 1)
        .await
        .map_err(|_| {
            validation_error_response(RestApiValidationError::OversizedPayload(
                "request body exceeds configured payload limit".to_string(),
            ))
        })?;

    validate_payload_size(payload.len(), &state.config).map_err(validation_error_response)?;

    let request = Request::from_parts(parts, Body::from(payload));
    Ok(next.run(request).await)
}

pub async fn run_rest_api_server(config: RestApiConfig) -> anyhow::Result<()> {
    config
        .validate_security_defaults()
        .map_err(|e| anyhow!("REST API configuration error: {e}"))
        .context("failed to validate REST API security defaults")?;

    let state = RestServerState::new_with_probe_runner(
        config.clone(),
        probe_runner_path_from_current_exe()
            .map_err(|e| anyhow!("failed to resolve probe runner path: {e}"))?,
    )
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
) -> ApiResult<Json<HealthResponseDto>> {
    enforce_request_auth(&state.config, remote_addr, &headers)?;
    Ok(Json(HealthResponseDto {
        meta: ApiResponseMetaDto {
            schema_version: "v1",
            request_id: None,
        },
        data: HealthDataDto {
            status: "ok",
            service: "windows-mtr",
            version: env!("CARGO_PKG_VERSION"),
        },
    }))
}

async fn create_probe(
    ConnectInfo(remote_addr): ConnectInfo<std::net::SocketAddr>,
    State(state): State<RestServerState>,
    headers: HeaderMap,
    Json(payload): Json<CreateProbeRequestDto>,
) -> ApiResult<(StatusCode, Json<CreateProbeResponseDto>)> {
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
            finished_at: None,
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
                meta: ApiResponseMetaDto {
                    schema_version: "v1",
                    request_id: None,
                },
                data: CreateProbeDataDto {
                    id,
                    status: ProbeJobStatus::Queued.into(),
                },
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

    match execute_probe(normalized, state.probe_runner_path.clone()).await {
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
    probe_runner_path: Arc<PathBuf>,
) -> Result<ProbeExecutionResult, String> {
    if normalized.targets.is_empty() {
        return Err("at least one target is required".to_string());
    }

    let protocol = match normalized.protocol {
        ProbeProtocol::Tcp => "tcp",
        ProbeProtocol::Udp => "udp",
        ProbeProtocol::Icmp => "icmp",
    };

    let mut targets = Vec::with_capacity(normalized.targets.len());
    let mut target_results = Vec::with_capacity(normalized.targets.len());

    for host in &normalized.targets {
        let request = normalized_to_probe_request(&normalized, host.clone());
        let plan = match build_probe_plan(&request) {
            Ok(plan) => plan,
            Err(error) => {
                target_results.push(ProbeTargetExecutionResult {
                    target: host.clone(),
                    success: false,
                    error: Some(format!("failed to build probe plan: {error}")),
                });
                targets.push(host.clone());
                continue;
            }
        };

        let validated_target = plan.validated_host.clone();
        let trippy_args = plan.trippy_args;
        let json_output = plan.json_output;
        let runner_path = probe_runner_path.clone();

        let probe_result = tokio::task::spawn_blocking(move || {
            run_embedded_trippy(
                runner_path.as_ref(),
                &trippy_args,
                json_output,
                EMBEDDED_TRIPPY_ENV,
            )
        })
        .await;

        match probe_result {
            Ok(Ok(result)) if result.exit_code == 0 => {
                targets.push(validated_target.clone());
                target_results.push(ProbeTargetExecutionResult {
                    target: validated_target,
                    success: true,
                    error: None,
                });
            }
            Ok(Ok(result)) => {
                targets.push(validated_target.clone());
                target_results.push(ProbeTargetExecutionResult {
                    target: validated_target,
                    success: false,
                    error: Some(format!(
                        "probe execution failed with exit code {}",
                        result.exit_code
                    )),
                });
            }
            Ok(Err(error)) => {
                eprintln!("probe execution failed for {validated_target}: {error}");
                targets.push(validated_target.clone());
                target_results.push(ProbeTargetExecutionResult {
                    target: validated_target,
                    success: false,
                    error: Some("probe execution failed".to_string()),
                });
            }
            Err(error) => {
                eprintln!("probe task panicked for {validated_target}: {error}");
                targets.push(validated_target.clone());
                target_results.push(ProbeTargetExecutionResult {
                    target: validated_target,
                    success: false,
                    error: Some("probe execution failed".to_string()),
                });
            }
        }
    }

    let completed = target_results.iter().all(|result| result.success);
    if !target_results.is_empty() && target_results.iter().all(|result| !result.success) {
        let error_details = target_results
            .iter()
            .map(|result| format!("{}: probe execution failed", result.target))
            .collect::<Vec<_>>()
            .join("; ");
        return Err(format!("all target probes failed: {error_details}"));
    }

    Ok(ProbeExecutionResult {
        targets,
        protocol,
        completed,
        target_results,
    })
}

fn probe_runner_path_from_env() -> PathBuf {
    for key in [
        "CARGO_BIN_EXE_mtr",
        "CARGO_BIN_EXE_windows-mtr",
        "CARGO_BIN_EXE_windows_mtr",
    ] {
        if let Some(path) = env::var_os(key)
            && !path.is_empty()
        {
            return PathBuf::from(path);
        }
    }

    PathBuf::from("windows-mtr")
}

fn probe_runner_path_from_current_exe() -> Result<PathBuf, &'static str> {
    // SAFETY: this path is used only to re-exec ourselves for local probe execution,
    // not for trust, auth, or authorization decisions.
    let path =
        // nosemgrep: rust.lang.security.current-exe.current-exe
        env::current_exe().map_err(|_| "failed to resolve current executable path")?;
    Ok(path)
}

fn normalized_to_probe_request(
    normalized: &NormalizedCreateProbeRequest,
    host: String,
) -> ProbeRequest {
    ProbeRequest {
        host,
        tcp: matches!(normalized.protocol, ProbeProtocol::Tcp),
        udp: matches!(normalized.protocol, ProbeProtocol::Udp),
        port: normalized.port,
        source_port: None,
        report: true,
        json_output: None,
        count: normalized.count.or(Some(1)),
        interval_seconds: normalized.interval_seconds,
        timeout_seconds: normalized.timeout_seconds,
        report_wide: false,
        no_dns: !normalized.resolve_dns,
        max_hops: normalized.max_hops,
        show_asn: normalized.include_asn,
        dns_lookup_as_info: normalized.include_asn,
        packet_size: None,
        src: None,
        interface: None,
        ecmp: None,
        dns_cache_ttl_seconds: None,
        trippy_flags: None,
        ui_mode: UiMode::Default,
        enhanced_ui: EnhancedUiConfig {
            latency_warn_ms: 100.0,
            latency_bad_ms: 250.0,
            loss_warn_pct: 2.0,
            loss_bad_pct: 5.0,
            row_coloring: true,
            sparklines: true,
            summary: true,
        },
        has_enhanced_overrides: false,
    }
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
        finished_at: match status {
            ProbeJobStatus::Completed | ProbeJobStatus::Failed => Some(Instant::now()),
            ProbeJobStatus::Queued | ProbeJobStatus::Running => None,
        },
    });

    Ok(())
}

async fn get_probe(
    ConnectInfo(remote_addr): ConnectInfo<std::net::SocketAddr>,
    State(state): State<RestServerState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> ApiResult<Json<ProbeResultResponseDto>> {
    enforce_request_auth(&state.config, remote_addr, &headers)?;

    run_with_timeout(state.config.request_timeout, async move {
        if id.trim().is_empty() || id.chars().any(char::is_whitespace) {
            return Err(error_response(
                StatusCode::BAD_REQUEST,
                "invalid_probe_id",
                "Invalid probe id",
                "probe id must not be empty or contain whitespace".to_string(),
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
                "Probe not found",
                format!("probe not found: {id}"),
            )
        })?;

        Ok(Json(ProbeResultResponseDto::from(&job)))
    })
    .await
}

async fn run_with_timeout<T>(
    duration: std::time::Duration,
    future: impl std::future::Future<Output = ApiResult<T>>,
) -> ApiResult<T> {
    match timeout(duration, future).await {
        Ok(result) => result,
        Err(_) => Err(error_response(
            StatusCode::REQUEST_TIMEOUT,
            "request_timeout",
            "Request timed out",
            "request processing timed out".to_string(),
        )),
    }
}

fn validation_error_response(error: RestApiValidationError) -> ApiError {
    match error {
        RestApiValidationError::InvalidPort(ref message)
            if message == "port is required for tcp/udp probes" =>
        {
            error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "invalid_request",
                "Invalid request",
                error.to_string(),
            )
        }
        RestApiValidationError::ConcurrencyLimitExceeded { .. }
        | RestApiValidationError::RateLimitExceeded { .. } => error_response(
            StatusCode::TOO_MANY_REQUESTS,
            "rate_limited",
            "Rate limited",
            error.to_string(),
        ),
        RestApiValidationError::OversizedPayload(_) => error_response(
            StatusCode::PAYLOAD_TOO_LARGE,
            "payload_too_large",
            "Payload too large",
            error.to_string(),
        ),
        _ => error_response(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "Invalid request",
            error.to_string(),
        ),
    }
}

fn internal_error_response(message: &str) -> ApiError {
    error_response(
        StatusCode::INTERNAL_SERVER_ERROR,
        "internal_error",
        "Internal server error",
        message.to_string(),
    )
}

fn error_response(
    status: StatusCode,
    code: &'static str,
    title: &'static str,
    detail: String,
) -> ApiError {
    ApiError {
        status,
        code,
        title,
        detail,
    }
}

fn enforce_request_auth(
    config: &RestApiConfig,
    remote_addr: std::net::SocketAddr,
    headers: &HeaderMap,
) -> ApiResult<()> {
    let request_is_loopback = remote_addr.ip().is_loopback();

    match config.auth_strategy {
        AuthStrategy::NoneLocalOnly if request_is_loopback => Ok(()),
        AuthStrategy::NoneLocalOnly => {
            Err(RequestAuthError::NoneLocalOnlyRemoteAccessDenied.into_api_error())
        }
        AuthStrategy::ApiKey => {
            let provided = headers
                .get(API_KEY_HEADER)
                .and_then(|value| value.to_str().ok())
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| RequestAuthError::MissingApiKeyHeader.into_api_error())?;

            let expected = config
                .api_key
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| {
                    error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "auth_configuration_error",
                        "Internal server error",
                        "auth_strategy=api-key requires configured api_key".to_string(),
                    )
                })?;

            if constant_time_equals(provided.as_bytes(), expected.as_bytes()) {
                Ok(())
            } else {
                Err(RequestAuthError::InvalidApiKey.into_api_error())
            }
        }
        AuthStrategy::Mtls => {
            let ingress_is_trusted = config
                .trusted_mtls_ingress_ips
                .iter()
                .any(|ip| *ip == remote_addr.ip());

            if !ingress_is_trusted {
                return Err(RequestAuthError::UntrustedMtlsIngress.into_api_error());
            }

            headers
                .get("X-Client-Cert")
                .or_else(|| headers.get("X-SSL-Client-Verify"))
                .map(|_| ())
                .ok_or_else(|| RequestAuthError::MissingMtlsIdentity.into_api_error())
        }
    }
}

fn constant_time_equals(a: &[u8], b: &[u8]) -> bool {
    a.ct_eq(b).into()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_time_equals_handles_equal_and_mismatched_inputs() {
        assert!(constant_time_equals(b"secret-key", b"secret-key"));
        assert!(!constant_time_equals(b"secret-key", b"secret-kez"));
        assert!(!constant_time_equals(b"secret-key", b"secret-key-extended"));
    }

    #[test]
    fn probe_store_prunes_expired_and_old_completed_jobs() {
        let mut store = ProbeStore {
            jobs: HashMap::new(),
            max_completed_jobs: 1,
            completed_job_ttl: std::time::Duration::from_millis(50),
        };

        let old = Instant::now() - std::time::Duration::from_millis(100);
        store.jobs.insert(
            "old-completed".to_string(),
            ProbeJob {
                id: "old-completed".to_string(),
                status: ProbeJobStatus::Completed,
                result: None,
                error: None,
                finished_at: Some(old),
            },
        );

        store.upsert(ProbeJob {
            id: "new-completed".to_string(),
            status: ProbeJobStatus::Completed,
            result: None,
            error: None,
            finished_at: Some(Instant::now()),
        });

        assert!(!store.jobs.contains_key("old-completed"));
        assert!(store.jobs.contains_key("new-completed"));
    }

    #[test]
    fn probe_store_enforces_completed_job_cap_after_insert() {
        let mut store = ProbeStore {
            jobs: HashMap::new(),
            max_completed_jobs: 1,
            completed_job_ttl: std::time::Duration::from_secs(60),
        };

        store.upsert(ProbeJob {
            id: "completed-1".to_string(),
            status: ProbeJobStatus::Completed,
            result: None,
            error: None,
            finished_at: Some(Instant::now()),
        });

        store.upsert(ProbeJob {
            id: "completed-2".to_string(),
            status: ProbeJobStatus::Completed,
            result: None,
            error: None,
            finished_at: Some(Instant::now()),
        });

        assert_eq!(store.jobs.len(), 1);
        assert!(store.jobs.contains_key("completed-2"));
    }
}
