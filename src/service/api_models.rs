use serde::{Deserialize, Serialize};

use crate::service::rest_api::{CreateProbeApiRequest, ProbeProtocol};
use crate::service::rest_server::{ProbeExecutionResult, ProbeJob, ProbeJobStatus};

#[derive(Debug, Clone, Deserialize)]
pub struct CreateProbeRequestDto {
    pub targets: Vec<String>,
    pub protocol: ApiProbeProtocol,
    pub port: Option<u16>,
    pub interval_seconds: Option<f32>,
    pub timeout_seconds: Option<f32>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
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
            interval_seconds: value.interval_seconds,
            timeout_seconds: value.timeout_seconds,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct HealthResponseDto {
    pub status: &'static str,
    pub service: &'static str,
    pub version: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateProbeResponseDto {
    pub id: String,
    pub status: ApiProbeStatusDto,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProbeResultResponseDto {
    pub id: String,
    pub status: ApiProbeStatusDto,
    pub result: Option<ProbeExecutionResultDto>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProbeExecutionResultDto {
    pub targets: Vec<String>,
    pub protocol: &'static str,
    pub completed: bool,
}

impl From<ProbeExecutionResult> for ProbeExecutionResultDto {
    fn from(value: ProbeExecutionResult) -> Self {
        Self {
            targets: value.targets,
            protocol: value.protocol,
            completed: value.completed,
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
            id: value.id.clone(),
            status: value.status.into(),
            result: value.result.clone().map(Into::into),
        }
    }
}
