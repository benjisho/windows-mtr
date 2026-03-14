use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

use crate::error::MtrError;
use crate::service::ProbeError;

/// Machine-readable API error mapped from domain failures.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ApiError {
    pub status: StatusCode,
    pub code: &'static str,
    pub title: &'static str,
    pub detail: String,
}

/// JSON envelope returned for API failures.
#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct ApiErrorResponse {
    pub error: ApiProblemDetails,
}

/// RFC 9457 style problem details with a project-specific machine code.
#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct ApiProblemDetails {
    #[serde(rename = "type")]
    pub type_url: &'static str,
    pub title: &'static str,
    pub status: u16,
    pub detail: String,
    pub code: &'static str,
}

impl ApiError {
    fn new(status: StatusCode, code: &'static str, title: &'static str, detail: String) -> Self {
        Self {
            status,
            code,
            title,
            detail,
        }
    }

    fn problem_type(code: &'static str) -> &'static str {
        match code {
            "invalid_target" => "https://windows-mtr.dev/problems/invalid-target",
            "invalid_ip_address" => "https://windows-mtr.dev/problems/invalid-ip-address",
            "invalid_option" => "https://windows-mtr.dev/problems/invalid-option",
            "missing_port" => "https://windows-mtr.dev/problems/missing-port",
            "insufficient_privileges" => "https://windows-mtr.dev/problems/insufficient-privileges",
            "probe_backend_unavailable" => {
                "https://windows-mtr.dev/problems/probe-backend-unavailable"
            }
            "probe_backend_install_failed" => {
                "https://windows-mtr.dev/problems/probe-backend-install-failed"
            }
            "probe_execution_failed" => "https://windows-mtr.dev/problems/probe-execution-failed",
            "internal_io_error" => "https://windows-mtr.dev/problems/internal-io-error",
            _ => "https://windows-mtr.dev/problems/internal-error",
        }
    }

    pub fn response(&self) -> ApiErrorResponse {
        ApiErrorResponse {
            error: ApiProblemDetails {
                type_url: Self::problem_type(self.code),
                title: self.title,
                status: self.status.as_u16(),
                detail: self.detail.clone(),
                code: self.code,
            },
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self.response())).into_response()
    }
}

impl From<ProbeError> for ApiError {
    fn from(value: ProbeError) -> Self {
        match value {
            ProbeError::HostResolutionError(_) => Self::new(
                StatusCode::BAD_REQUEST,
                "invalid_target",
                "Invalid target",
                value.to_string(),
            ),
            ProbeError::PortRequired(_, _) => Self::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "missing_port",
                "Invalid probe configuration",
                value.to_string(),
            ),
            ProbeError::InvalidOption(_) => Self::new(
                StatusCode::BAD_REQUEST,
                "invalid_option",
                "Invalid request",
                value.to_string(),
            ),
        }
    }
}

impl From<MtrError> for ApiError {
    fn from(value: MtrError) -> Self {
        match value {
            MtrError::HostResolutionError(_) => Self::new(
                StatusCode::BAD_REQUEST,
                "invalid_target",
                "Invalid target",
                value.to_string(),
            ),
            MtrError::InvalidIpAddress(_) => Self::new(
                StatusCode::BAD_REQUEST,
                "invalid_ip_address",
                "Invalid request",
                value.to_string(),
            ),
            MtrError::InvalidOption(_) => Self::new(
                StatusCode::BAD_REQUEST,
                "invalid_option",
                "Invalid request",
                value.to_string(),
            ),
            MtrError::PortRequired(_, _) => Self::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "missing_port",
                "Invalid probe configuration",
                value.to_string(),
            ),
            MtrError::InsufficientPrivileges => Self::new(
                StatusCode::FORBIDDEN,
                "insufficient_privileges",
                "Insufficient privileges",
                value.to_string(),
            ),
            MtrError::TrippyNotFound => Self::new(
                StatusCode::BAD_GATEWAY,
                "probe_backend_unavailable",
                "Probe backend unavailable",
                value.to_string(),
            ),
            MtrError::TrippyInstallFailed(_) => Self::new(
                StatusCode::BAD_GATEWAY,
                "probe_backend_install_failed",
                "Probe backend failure",
                value.to_string(),
            ),
            MtrError::TrippyExecutionFailed(_) => Self::new(
                StatusCode::BAD_GATEWAY,
                "probe_execution_failed",
                "Probe execution failure",
                value.to_string(),
            ),
            MtrError::IoError(_) => Self::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_io_error",
                "Internal server error",
                value.to_string(),
            ),
            MtrError::Other(_) => Self::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                "Internal server error",
                value.to_string(),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::{EnhancedUiConfig, ProbeRequest, UiMode, build_probe_plan};

    fn base_request() -> ProbeRequest {
        ProbeRequest {
            host: "8.8.8.8".to_string(),
            tcp: false,
            udp: false,
            port: None,
            source_port: None,
            report: false,
            json_output: None,
            count: None,
            interval_seconds: None,
            timeout_seconds: None,
            report_wide: false,
            no_dns: false,
            max_hops: None,
            show_asn: false,
            dns_lookup_as_info: false,
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

    #[test]
    fn validation_error_maps_to_4xx() {
        let mut request = base_request();
        request.tcp = true;

        let probe_error = build_probe_plan(&request).expect_err("expected validation error");
        let api_error = ApiError::from(probe_error);

        assert_eq!(api_error.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(api_error.code, "missing_port");
        assert!(
            api_error
                .detail
                .contains("Port option required for TCP protocol")
        );
    }

    #[test]
    fn unsupported_mode_error_maps_to_400() {
        let mut request = base_request();
        request.ui_mode = UiMode::Enhanced;
        request.report = true;

        let probe_error = build_probe_plan(&request).expect_err("expected unsupported mode");
        let api_error = ApiError::from(probe_error);

        assert_eq!(api_error.status, StatusCode::BAD_REQUEST);
        assert_eq!(api_error.code, "invalid_option");
    }

    #[test]
    fn host_resolution_error_maps_to_bad_request() {
        let mut request = base_request();
        request.host = "invalid host with spaces".to_string();

        let probe_error = build_probe_plan(&request).expect_err("expected host resolution error");
        let api_error = ApiError::from(probe_error);

        assert_eq!(api_error.status, StatusCode::BAD_REQUEST);
        assert_eq!(api_error.code, "invalid_target");
    }

    #[test]
    fn runtime_probe_failure_maps_to_bad_gateway() {
        let api_error = ApiError::from(MtrError::TrippyExecutionFailed("spawn failed".to_string()));

        assert_eq!(api_error.status, StatusCode::BAD_GATEWAY);
        assert_eq!(api_error.code, "probe_execution_failed");

        let body = api_error.response();
        assert_eq!(body.error.status, 502);
        assert_eq!(body.error.code, "probe_execution_failed");
        assert_eq!(
            body.error.type_url,
            "https://windows-mtr.dev/problems/probe-execution-failed"
        );
    }
}
