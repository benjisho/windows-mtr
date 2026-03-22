use std::collections::HashSet;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

const MAX_HOSTNAME_LEN: usize = 253;
const MAX_LABEL_LEN: usize = 63;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum AuthStrategy {
    ApiKey,
    Mtls,
    NoneLocalOnly,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ProbeProtocol {
    Icmp,
    Tcp,
    Udp,
}

#[derive(Debug, Clone)]
pub struct RestApiConfig {
    pub bind_addr: SocketAddr,
    pub allow_non_local_bind: bool,
    pub auth_strategy: AuthStrategy,
    pub api_key: Option<String>,
    pub request_timeout: Duration,
    pub max_concurrent_probes: usize,
    pub max_requests_per_window: usize,
    pub rate_limit_window: Duration,
    pub max_targets_per_request: usize,
    pub max_payload_bytes: usize,
    pub max_completed_jobs: usize,
    pub completed_job_ttl: Duration,
    pub trusted_mtls_ingress_ips: Vec<IpAddr>,
}

impl Default for RestApiConfig {
    fn default() -> Self {
        Self {
            bind_addr: SocketAddr::from(([127, 0, 0, 1], 3000)),
            allow_non_local_bind: false,
            auth_strategy: AuthStrategy::NoneLocalOnly,
            api_key: None,
            request_timeout: Duration::from_secs(10),
            max_concurrent_probes: 8,
            max_requests_per_window: 8,
            rate_limit_window: Duration::from_secs(10),
            max_targets_per_request: 8,
            max_payload_bytes: 16 * 1024,
            max_completed_jobs: 1024,
            completed_job_ttl: Duration::from_secs(15 * 60),
            trusted_mtls_ingress_ips: vec![
                IpAddr::from([127, 0, 0, 1]),
                "::1".parse().expect("valid localhost ipv6 literal"),
            ],
        }
    }
}

impl RestApiConfig {
    pub fn validate_security_defaults(&self) -> Result<(), RestApiValidationError> {
        if !self.allow_non_local_bind && !self.bind_addr.ip().is_loopback() {
            return Err(RestApiValidationError::NonLocalBindRequiresOptIn(
                self.bind_addr,
            ));
        }

        if self.request_timeout.is_zero() {
            return Err(RestApiValidationError::InvalidTimeout(
                "request_timeout must be greater than zero".to_string(),
            ));
        }

        if self.max_concurrent_probes == 0 {
            return Err(RestApiValidationError::InvalidConcurrencyLimit(
                "max_concurrent_probes must be at least 1".to_string(),
            ));
        }

        if self.max_requests_per_window == 0 {
            return Err(RestApiValidationError::InvalidRateLimit(
                "max_requests_per_window must be at least 1".to_string(),
            ));
        }

        if self.rate_limit_window.is_zero() {
            return Err(RestApiValidationError::InvalidRateLimit(
                "rate_limit_window must be greater than zero".to_string(),
            ));
        }

        if self.max_targets_per_request == 0 {
            return Err(RestApiValidationError::InvalidTargetLimit(
                "max_targets_per_request must be at least 1".to_string(),
            ));
        }

        if self.max_payload_bytes == 0 {
            return Err(RestApiValidationError::OversizedPayload(
                "max_payload_bytes must be at least 1".to_string(),
            ));
        }
        if self.max_completed_jobs == 0 {
            return Err(RestApiValidationError::InvalidOption(
                "max_completed_jobs must be at least 1".to_string(),
            ));
        }
        if self.completed_job_ttl.is_zero() {
            return Err(RestApiValidationError::InvalidOption(
                "completed_job_ttl must be greater than zero".to_string(),
            ));
        }

        if self.auth_strategy == AuthStrategy::NoneLocalOnly && !self.bind_addr.ip().is_loopback() {
            return Err(RestApiValidationError::AuthStrategyViolation(
                "auth_strategy=none-local-only is only valid for localhost binds".to_string(),
            ));
        }

        if self.auth_strategy == AuthStrategy::ApiKey
            && self
                .api_key
                .as_ref()
                .is_none_or(|key| key.trim().is_empty())
        {
            return Err(RestApiValidationError::AuthStrategyViolation(
                "auth_strategy=api-key requires a non-empty api_key".to_string(),
            ));
        }

        if self.auth_strategy == AuthStrategy::Mtls && self.trusted_mtls_ingress_ips.is_empty() {
            return Err(RestApiValidationError::AuthStrategyViolation(
                "auth_strategy=mtls requires at least one trusted ingress IP".to_string(),
            ));
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct CreateProbeApiRequest {
    pub targets: Vec<String>,
    pub protocol: ProbeProtocol,
    pub port: Option<u16>,
    pub count: Option<usize>,
    pub max_hops: Option<u16>,
    pub resolve_dns: Option<bool>,
    pub include_asn: Option<bool>,
    pub interval_seconds: Option<f32>,
    pub timeout_seconds: Option<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NormalizedCreateProbeRequest {
    pub targets: Vec<String>,
    pub protocol: ProbeProtocol,
    pub port: Option<u16>,
    pub count: Option<usize>,
    pub max_hops: Option<u8>,
    pub resolve_dns: bool,
    pub include_asn: bool,
    pub interval_seconds: Option<f32>,
    pub timeout_seconds: Option<f32>,
}

impl CreateProbeApiRequest {
    pub fn normalize_and_validate(
        self,
        config: &RestApiConfig,
    ) -> Result<NormalizedCreateProbeRequest, RestApiValidationError> {
        if self.targets.is_empty() {
            return Err(RestApiValidationError::InvalidTarget(
                "at least one target is required".to_string(),
            ));
        }

        if self.targets.len() > config.max_targets_per_request {
            return Err(RestApiValidationError::TooManyTargets {
                provided: self.targets.len(),
                limit: config.max_targets_per_request,
            });
        }

        if matches!(self.protocol, ProbeProtocol::Tcp | ProbeProtocol::Udp) && self.port.is_none() {
            return Err(RestApiValidationError::InvalidPort(
                "port is required for tcp/udp probes".to_string(),
            ));
        }

        if matches!(self.protocol, ProbeProtocol::Icmp) && self.port.is_some() {
            return Err(RestApiValidationError::InvalidPort(
                "port is not allowed for icmp probes".to_string(),
            ));
        }

        if self.port == Some(0) {
            return Err(RestApiValidationError::InvalidPort(
                "port must be between 1 and 65535".to_string(),
            ));
        }

        let count = validate_optional_count(self.count)?;
        let max_hops = validate_optional_max_hops(self.max_hops)?;
        let resolve_dns = self.resolve_dns.unwrap_or(true);
        let include_asn = self.include_asn.unwrap_or(false);

        let interval_seconds =
            validate_optional_positive("interval_seconds", self.interval_seconds)?;
        let timeout_seconds = validate_optional_positive("timeout_seconds", self.timeout_seconds)?;

        if let (Some(interval), Some(timeout)) = (interval_seconds, timeout_seconds)
            && timeout < interval
        {
            return Err(RestApiValidationError::InvalidTimeout(
                "timeout_seconds must be greater than or equal to interval_seconds".to_string(),
            ));
        }

        let mut normalized_targets = Vec::with_capacity(self.targets.len());
        let mut dedupe = HashSet::with_capacity(self.targets.len());

        for raw in self.targets {
            let normalized = normalize_target(raw)?;
            if dedupe.insert(normalized.clone()) {
                normalized_targets.push(normalized);
            }
        }

        Ok(NormalizedCreateProbeRequest {
            targets: normalized_targets,
            protocol: self.protocol,
            port: self.port,
            count,
            max_hops,
            resolve_dns,
            include_asn,
            interval_seconds,
            timeout_seconds,
        })
    }
}

fn validate_optional_count(value: Option<usize>) -> Result<Option<usize>, RestApiValidationError> {
    let Some(raw) = value else {
        return Ok(None);
    };

    if raw == 0 {
        return Err(RestApiValidationError::InvalidOption(
            "count must be greater than or equal to 1".to_string(),
        ));
    }

    Ok(Some(raw))
}

fn validate_optional_max_hops(value: Option<u16>) -> Result<Option<u8>, RestApiValidationError> {
    let Some(raw) = value else {
        return Ok(None);
    };

    if !(1..=255).contains(&raw) {
        return Err(RestApiValidationError::InvalidOption(
            "max_hops must be between 1 and 255".to_string(),
        ));
    }

    Ok(Some(raw as u8))
}

fn validate_optional_positive(
    field_name: &str,
    value: Option<f32>,
) -> Result<Option<f32>, RestApiValidationError> {
    let Some(raw) = value else {
        return Ok(None);
    };

    if !raw.is_finite() || raw <= 0.0 {
        return Err(RestApiValidationError::InvalidOption(format!(
            "{field_name} must be a positive finite number"
        )));
    }

    Ok(Some(raw))
}

fn normalize_target(raw: String) -> Result<String, RestApiValidationError> {
    let target = raw.trim();
    if target.is_empty() {
        return Err(RestApiValidationError::InvalidTarget(
            "target must not be empty".to_string(),
        ));
    }

    if let Ok(ip) = IpAddr::from_str(target) {
        return Ok(ip.to_string());
    }

    let lower = target.to_ascii_lowercase();
    if lower.len() > MAX_HOSTNAME_LEN {
        return Err(RestApiValidationError::InvalidTarget(format!(
            "hostname exceeds {MAX_HOSTNAME_LEN} characters"
        )));
    }

    if !is_valid_hostname(&lower) {
        return Err(RestApiValidationError::InvalidTarget(format!(
            "invalid hostname: {target}"
        )));
    }

    Ok(lower)
}

fn is_valid_hostname(hostname: &str) -> bool {
    let labels: Vec<&str> = hostname.split('.').collect();
    if labels.is_empty() {
        return false;
    }

    labels.iter().all(|label| {
        if label.is_empty() || label.len() > MAX_LABEL_LEN {
            return false;
        }

        let starts_or_ends_hyphen = label.starts_with('-') || label.ends_with('-');
        if starts_or_ends_hyphen {
            return false;
        }

        label
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
    })
}

pub fn validate_payload_size(
    payload_size_bytes: usize,
    config: &RestApiConfig,
) -> Result<(), RestApiValidationError> {
    if payload_size_bytes > config.max_payload_bytes {
        return Err(RestApiValidationError::OversizedPayload(format!(
            "payload size {} exceeds maximum {} bytes",
            payload_size_bytes, config.max_payload_bytes
        )));
    }

    Ok(())
}

#[derive(Debug)]
pub struct ProbeConcurrencyGate {
    in_flight: AtomicUsize,
    limit: usize,
}

impl ProbeConcurrencyGate {
    pub fn new(limit: usize) -> Result<Self, RestApiValidationError> {
        if limit == 0 {
            return Err(RestApiValidationError::InvalidConcurrencyLimit(
                "concurrency limit must be >= 1".to_string(),
            ));
        }

        Ok(Self {
            in_flight: AtomicUsize::new(0),
            limit,
        })
    }

    pub fn try_acquire(&self) -> Result<ProbeConcurrencyPermit<'_>, RestApiValidationError> {
        let updated = self.in_flight.fetch_add(1, Ordering::AcqRel) + 1;
        if updated > self.limit {
            self.in_flight.fetch_sub(1, Ordering::AcqRel);
            return Err(RestApiValidationError::ConcurrencyLimitExceeded { limit: self.limit });
        }

        Ok(ProbeConcurrencyPermit { gate: self })
    }

    #[cfg(test)]
    pub fn current_in_flight(&self) -> usize {
        self.in_flight.load(Ordering::Acquire)
    }
}

pub struct ProbeConcurrencyPermit<'a> {
    gate: &'a ProbeConcurrencyGate,
}

impl Drop for ProbeConcurrencyPermit<'_> {
    fn drop(&mut self) {
        self.gate.in_flight.fetch_sub(1, Ordering::AcqRel);
    }
}

#[derive(Debug)]
pub struct FixedWindowRateLimiter {
    max_requests: usize,
    window: Duration,
    window_started_at: Instant,
    count: usize,
}

impl FixedWindowRateLimiter {
    pub fn new(
        max_requests: usize,
        window: Duration,
        now: Instant,
    ) -> Result<Self, RestApiValidationError> {
        if max_requests == 0 {
            return Err(RestApiValidationError::InvalidRateLimit(
                "max_requests must be >= 1".to_string(),
            ));
        }

        if window.is_zero() {
            return Err(RestApiValidationError::InvalidRateLimit(
                "rate limit window must be greater than zero".to_string(),
            ));
        }

        Ok(Self {
            max_requests,
            window,
            window_started_at: now,
            count: 0,
        })
    }

    pub fn allow(&mut self, now: Instant) -> Result<(), RestApiValidationError> {
        if now.duration_since(self.window_started_at) >= self.window {
            self.window_started_at = now;
            self.count = 0;
        }

        if self.count >= self.max_requests {
            return Err(RestApiValidationError::RateLimitExceeded {
                max_requests: self.max_requests,
            });
        }

        self.count += 1;
        Ok(())
    }
}

#[derive(thiserror::Error, Debug, Clone, Eq, PartialEq)]
pub enum RestApiValidationError {
    #[error("non-local bind requires explicit opt-in: {0}")]
    NonLocalBindRequiresOptIn(SocketAddr),

    #[error("invalid target: {0}")]
    InvalidTarget(String),

    #[error("invalid port: {0}")]
    InvalidPort(String),

    #[error("invalid timeout: {0}")]
    InvalidTimeout(String),

    #[error("invalid option: {0}")]
    InvalidOption(String),

    #[error("too many targets in request: provided {provided}, maximum {limit}")]
    TooManyTargets { provided: usize, limit: usize },

    #[error("payload rejected: {0}")]
    OversizedPayload(String),

    #[error("concurrency limit exceeded: max {limit} in-flight probes")]
    ConcurrencyLimitExceeded { limit: usize },

    #[error("rate limit exceeded: max {max_requests} requests per window")]
    RateLimitExceeded { max_requests: usize },

    #[error("authentication strategy violation: {0}")]
    AuthStrategyViolation(String),

    #[error("invalid concurrency limit: {0}")]
    InvalidConcurrencyLimit(String),

    #[error("invalid target limit: {0}")]
    InvalidTargetLimit(String),

    #[error("invalid rate limit: {0}")]
    InvalidRateLimit(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secure_defaults_require_local_bind_for_no_auth() {
        let mut config = RestApiConfig {
            bind_addr: SocketAddr::from(([0, 0, 0, 0], 3000)),
            ..RestApiConfig::default()
        };
        assert!(matches!(
            config.validate_security_defaults(),
            Err(RestApiValidationError::NonLocalBindRequiresOptIn(_))
        ));

        config.allow_non_local_bind = true;
        assert!(matches!(
            config.validate_security_defaults(),
            Err(RestApiValidationError::AuthStrategyViolation(_))
        ));

        config.auth_strategy = AuthStrategy::ApiKey;
        assert!(matches!(
            config.validate_security_defaults(),
            Err(RestApiValidationError::AuthStrategyViolation(_))
        ));

        config.api_key = Some("secret".to_string());
        assert!(config.validate_security_defaults().is_ok());
    }

    #[test]
    fn normalize_and_validate_targets_and_intervals() {
        let request = CreateProbeApiRequest {
            targets: vec![" Example.COM ".to_string(), "1.1.1.1".to_string()],
            protocol: ProbeProtocol::Tcp,
            port: Some(443),
            count: None,
            max_hops: None,
            resolve_dns: None,
            include_asn: None,
            interval_seconds: Some(1.0),
            timeout_seconds: Some(2.0),
        };

        let normalized = request
            .normalize_and_validate(&RestApiConfig::default())
            .expect("request should be valid");

        assert_eq!(normalized.targets, vec!["example.com", "1.1.1.1"]);
    }

    #[test]
    fn normalize_rejects_invalid_hostname() {
        let request = CreateProbeApiRequest {
            targets: vec!["bad host".to_string()],
            protocol: ProbeProtocol::Icmp,
            port: None,
            count: None,
            max_hops: None,
            resolve_dns: None,
            include_asn: None,
            interval_seconds: None,
            timeout_seconds: None,
        };

        assert!(matches!(
            request.normalize_and_validate(&RestApiConfig::default()),
            Err(RestApiValidationError::InvalidTarget(_))
        ));
    }

    #[test]
    fn normalize_rejects_port_for_icmp() {
        let request = CreateProbeApiRequest {
            targets: vec!["1.1.1.1".to_string()],
            protocol: ProbeProtocol::Icmp,
            port: Some(443),
            count: None,
            max_hops: None,
            resolve_dns: None,
            include_asn: None,
            interval_seconds: None,
            timeout_seconds: None,
        };

        assert!(matches!(
            request.normalize_and_validate(&RestApiConfig::default()),
            Err(RestApiValidationError::InvalidPort(_))
        ));
    }

    #[test]
    fn oversized_payload_is_rejected() {
        let config = RestApiConfig::default();
        assert!(matches!(
            validate_payload_size(config.max_payload_bytes + 1, &config),
            Err(RestApiValidationError::OversizedPayload(_))
        ));
    }

    #[test]
    fn concurrency_gate_enforces_limit() {
        let gate = ProbeConcurrencyGate::new(1).expect("valid gate");
        let first = gate.try_acquire().expect("first acquisition should pass");

        assert!(matches!(
            gate.try_acquire(),
            Err(RestApiValidationError::ConcurrencyLimitExceeded { .. })
        ));

        drop(first);
        assert_eq!(gate.current_in_flight(), 0);
        assert!(gate.try_acquire().is_ok());
    }

    #[test]
    fn fixed_window_rate_limiter_rejects_abusive_burst() {
        let now = Instant::now();
        let mut limiter =
            FixedWindowRateLimiter::new(2, Duration::from_secs(1), now).expect("valid limiter");

        assert!(limiter.allow(now).is_ok());
        assert!(limiter.allow(now).is_ok());
        assert!(matches!(
            limiter.allow(now),
            Err(RestApiValidationError::RateLimitExceeded { .. })
        ));

        let next_window = now + Duration::from_secs(1);
        assert!(limiter.allow(next_window).is_ok());
    }

    #[test]
    fn target_limit_is_enforced() {
        let request = CreateProbeApiRequest {
            targets: vec![
                "a.example.com".to_string(),
                "b.example.com".to_string(),
                "c.example.com".to_string(),
                "d.example.com".to_string(),
                "e.example.com".to_string(),
                "f.example.com".to_string(),
                "g.example.com".to_string(),
                "h.example.com".to_string(),
                "i.example.com".to_string(),
            ],
            protocol: ProbeProtocol::Icmp,
            port: None,
            count: None,
            max_hops: None,
            resolve_dns: None,
            include_asn: None,
            interval_seconds: None,
            timeout_seconds: None,
        };

        assert!(matches!(
            request.normalize_and_validate(&RestApiConfig::default()),
            Err(RestApiValidationError::TooManyTargets { .. })
        ));
    }

    #[test]
    fn retention_defaults_require_positive_limits() {
        let mut config = RestApiConfig {
            max_completed_jobs: 0,
            ..RestApiConfig::default()
        };
        assert!(matches!(
            config.validate_security_defaults(),
            Err(RestApiValidationError::InvalidOption(_))
        ));

        config.max_completed_jobs = 1;
        config.completed_job_ttl = Duration::ZERO;
        assert!(matches!(
            config.validate_security_defaults(),
            Err(RestApiValidationError::InvalidOption(_))
        ));
    }

    #[test]
    fn mtls_requires_at_least_one_trusted_ingress_ip() {
        let config = RestApiConfig {
            auth_strategy: AuthStrategy::Mtls,
            trusted_mtls_ingress_ips: Vec::new(),
            ..RestApiConfig::default()
        };

        assert!(matches!(
            config.validate_security_defaults(),
            Err(RestApiValidationError::AuthStrategyViolation(_))
        ));
    }
}
