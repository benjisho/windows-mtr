use std::net::SocketAddr;
use std::time::{Duration, Instant};
use windows_mtr::service::rest_api::{
    AuthStrategy, CreateProbeApiRequest, FixedWindowRateLimiter, ProbeConcurrencyGate,
    ProbeProtocol, RestApiConfig, RestApiValidationError, validate_payload_size,
};

#[test]
fn default_config_binds_localhost_and_validates() {
    let config = RestApiConfig::default();
    assert_eq!(config.bind_addr, SocketAddr::from(([127, 0, 0, 1], 3000)));
    assert!(config.validate_security_defaults().is_ok());
}

#[test]
fn non_local_bind_requires_opt_in_and_auth() {
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

    config.api_key = Some("test-api-key".to_string());
    assert!(config.validate_security_defaults().is_ok());
}

#[test]
fn rejects_invalid_rate_limit_configuration() {
    let mut config = RestApiConfig {
        max_requests_per_window: 0,
        ..RestApiConfig::default()
    };

    assert!(matches!(
        config.validate_security_defaults(),
        Err(RestApiValidationError::InvalidRateLimit(_))
    ));

    config.max_requests_per_window = 1;
    config.rate_limit_window = Duration::from_secs(0);
    assert!(matches!(
        config.validate_security_defaults(),
        Err(RestApiValidationError::InvalidRateLimit(_))
    ));
}

#[test]
fn request_validation_normalizes_and_deduplicates_targets() {
    let request = CreateProbeApiRequest {
        targets: vec![
            " EXAMPLE.COM ".to_string(),
            "example.com".to_string(),
            "1.1.1.1".to_string(),
        ],
        protocol: ProbeProtocol::Tcp,
        port: Some(443),
        count: None,
        max_hops: None,
        resolve_dns: None,
        include_asn: None,
        interval_seconds: Some(0.5),
        timeout_seconds: Some(1.0),
    };

    let normalized = request
        .normalize_and_validate(&RestApiConfig::default())
        .expect("request should validate");

    assert_eq!(normalized.targets, vec!["example.com", "1.1.1.1"]);
}

#[test]
fn rejects_invalid_untrusted_inputs() {
    let request = CreateProbeApiRequest {
        targets: vec!["bad host".to_string()],
        protocol: ProbeProtocol::Udp,
        port: Some(53),
        count: None,
        max_hops: None,
        resolve_dns: None,
        include_asn: None,
        interval_seconds: Some(-1.0),
        timeout_seconds: Some(0.5),
    };

    assert!(matches!(
        request.normalize_and_validate(&RestApiConfig::default()),
        Err(RestApiValidationError::InvalidOption(_))
            | Err(RestApiValidationError::InvalidTarget(_))
    ));
}

#[test]
fn rejects_oversized_payloads() {
    let config = RestApiConfig::default();
    assert!(matches!(
        validate_payload_size(config.max_payload_bytes + 1, &config),
        Err(RestApiValidationError::OversizedPayload(_))
    ));
}

#[test]
fn enforces_max_concurrent_probes_limit() {
    let gate = ProbeConcurrencyGate::new(1).expect("valid gate");
    let first = gate.try_acquire().expect("first should pass");

    assert!(matches!(
        gate.try_acquire(),
        Err(RestApiValidationError::ConcurrencyLimitExceeded { .. })
    ));

    drop(first);
    assert!(gate.try_acquire().is_ok());
}

#[test]
fn fixed_window_rate_limit_rejects_burst() {
    let now = Instant::now();
    let mut limiter =
        FixedWindowRateLimiter::new(2, Duration::from_secs(1), now).expect("valid limiter");

    assert!(limiter.allow(now).is_ok());
    assert!(limiter.allow(now).is_ok());
    assert!(matches!(
        limiter.allow(now),
        Err(RestApiValidationError::RateLimitExceeded { .. })
    ));
}

#[test]
fn rejects_too_many_targets_per_request() {
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
fn normalizes_defaults_for_documented_optional_fields() {
    let request = CreateProbeApiRequest {
        targets: vec!["example.com".to_string()],
        protocol: ProbeProtocol::Icmp,
        port: None,
        count: None,
        max_hops: None,
        resolve_dns: None,
        include_asn: None,
        interval_seconds: None,
        timeout_seconds: None,
    };

    let normalized = request
        .normalize_and_validate(&RestApiConfig::default())
        .expect("request should validate");

    assert_eq!(normalized.count, None);
    assert_eq!(normalized.max_hops, None);
    assert!(normalized.resolve_dns);
    assert!(!normalized.include_asn);
}

#[test]
fn accepts_documented_optional_fields_within_bounds() {
    let request = CreateProbeApiRequest {
        targets: vec!["example.com".to_string()],
        protocol: ProbeProtocol::Tcp,
        port: Some(443),
        count: Some(5),
        max_hops: Some(255),
        resolve_dns: Some(false),
        include_asn: Some(true),
        interval_seconds: Some(0.25),
        timeout_seconds: Some(1.0),
    };

    let normalized = request
        .normalize_and_validate(&RestApiConfig::default())
        .expect("request should validate");

    assert_eq!(normalized.count, Some(5));
    assert_eq!(normalized.max_hops, Some(255));
    assert!(!normalized.resolve_dns);
    assert!(normalized.include_asn);
}

#[test]
fn rejects_out_of_bounds_count_and_max_hops() {
    let config = RestApiConfig::default();

    let zero_count = CreateProbeApiRequest {
        targets: vec!["example.com".to_string()],
        protocol: ProbeProtocol::Icmp,
        port: None,
        count: Some(0),
        max_hops: None,
        resolve_dns: None,
        include_asn: None,
        interval_seconds: None,
        timeout_seconds: None,
    };
    assert!(matches!(
        zero_count.normalize_and_validate(&config),
        Err(RestApiValidationError::InvalidOption(_))
    ));

    let invalid_max_hops = CreateProbeApiRequest {
        targets: vec!["example.com".to_string()],
        protocol: ProbeProtocol::Icmp,
        port: None,
        count: None,
        max_hops: Some(256),
        resolve_dns: None,
        include_asn: None,
        interval_seconds: None,
        timeout_seconds: None,
    };
    assert!(matches!(
        invalid_max_hops.normalize_and_validate(&config),
        Err(RestApiValidationError::InvalidOption(_))
    ));
}
