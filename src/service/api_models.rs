use serde::{Deserialize, Serialize};

use crate::service::rest_api::{CreateProbeApiRequest, ProbeProtocol};
use crate::service::rest_server::{
    ProbeExecutionResult, ProbeJob, ProbeJobStatus, ProbeTargetExecutionResult,
};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreateProbeRequestDto {
    pub targets: Vec<String>,
    pub protocol: ApiProbeProtocol,
    pub port: Option<u16>,
    pub count: Option<usize>,
    pub max_hops: Option<u16>,
    pub resolve_dns: Option<bool>,
    pub include_asn: Option<bool>,
    pub interval_seconds: Option<f32>,
    pub timeout_seconds: Option<f32>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ApiProbeProtocol {
    Icmp,
    Tcp,
    Udp,
}

impl From<ApiProbeProtocol> for ProbeProtocol {
    fn from(value: ApiProbeProtocol) -> Self {
        match value {
            ApiProbeProtocol::Icmp => ProbeProtocol::Icmp,
            ApiProbeProtocol::Tcp => ProbeProtocol::Tcp,
            ApiProbeProtocol::Udp => ProbeProtocol::Udp,
        }
    }
}

impl From<CreateProbeRequestDto> for CreateProbeApiRequest {
    fn from(value: CreateProbeRequestDto) -> Self {
        Self {
            targets: value.targets,
            protocol: value.protocol.into(),
            port: value.port,
            count: value.count,
            max_hops: value.max_hops,
            resolve_dns: value.resolve_dns,
            include_asn: value.include_asn,
            interval_seconds: value.interval_seconds,
            timeout_seconds: value.timeout_seconds,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct HealthResponseDto {
    pub meta: ApiResponseMetaDto,
    pub data: HealthDataDto,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiResponseMetaDto {
    pub schema_version: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HealthDataDto {
    pub status: &'static str,
    pub service: &'static str,
    pub version: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateProbeResponseDto {
    pub meta: ApiResponseMetaDto,
    pub data: CreateProbeDataDto,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateProbeDataDto {
    pub id: String,
    pub status: ApiProbeStatusDto,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProbeResultResponseDto {
    pub meta: ApiResponseMetaDto,
    pub data: ProbeResultDataDto,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProbeResultDataDto {
    pub id: String,
    pub status: ApiProbeStatusDto,
    pub result: Option<ProbeExecutionResultDto>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProbeExecutionResultDto {
    pub targets: Vec<String>,
    pub protocol: &'static str,
    pub completed: bool,
    pub target_results: Vec<ProbeTargetExecutionResultDto>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProbeTargetExecutionResultDto {
    pub target: String,
    pub success: bool,
    pub error: Option<String>,
}

impl From<ProbeTargetExecutionResult> for ProbeTargetExecutionResultDto {
    fn from(value: ProbeTargetExecutionResult) -> Self {
        Self {
            target: value.target,
            success: value.success,
            error: value.error,
        }
    }
}

impl From<ProbeExecutionResult> for ProbeExecutionResultDto {
    fn from(value: ProbeExecutionResult) -> Self {
        Self {
            targets: value.targets,
            protocol: value.protocol,
            completed: value.completed,
            target_results: value.target_results.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiProbeStatusDto {
    Queued,
    Running,
    Completed,
    Failed,
}

impl From<ProbeJobStatus> for ApiProbeStatusDto {
    fn from(value: ProbeJobStatus) -> Self {
        match value {
            ProbeJobStatus::Queued => Self::Queued,
            ProbeJobStatus::Running => Self::Running,
            ProbeJobStatus::Completed => Self::Completed,
            ProbeJobStatus::Failed => Self::Failed,
        }
    }
}

impl From<&ProbeJob> for ProbeResultResponseDto {
    fn from(value: &ProbeJob) -> Self {
        Self {
            meta: ApiResponseMetaDto {
                schema_version: "v1",
                request_id: None,
            },
            data: ProbeResultDataDto {
                id: value.id.clone(),
                status: value.status.into(),
                result: value.result.clone().map(Into::into),
                error: value.error.clone(),
            },
        }
    }
}
