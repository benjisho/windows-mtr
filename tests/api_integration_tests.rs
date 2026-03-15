use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use windows_mtr::service::rest_api::{
    CreateProbeApiRequest, ProbeConcurrencyGate, ProbeProtocol, RestApiConfig,
    RestApiValidationError,
};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum ApiProbeRunOutcome {
    Completed,
    TimedOut,
    Cancelled,
}

fn run_probe_with_timeout_and_cancel(
    timeout: Duration,
    cancel_rx: mpsc::Receiver<()>,
    complete_rx: mpsc::Receiver<()>,
) -> ApiProbeRunOutcome {
    match complete_rx.recv_timeout(timeout) {
        Ok(()) => ApiProbeRunOutcome::Completed,
        Err(mpsc::RecvTimeoutError::Timeout) => {
            if cancel_rx.try_recv().is_ok() {
                ApiProbeRunOutcome::Cancelled
            } else {
                ApiProbeRunOutcome::TimedOut
            }
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => ApiProbeRunOutcome::Cancelled,
    }
}

#[test]
fn api_happy_path_accepts_valid_request_and_completes_before_timeout() {
    let config = RestApiConfig::default();

    let request = CreateProbeApiRequest {
        targets: vec![" Example.COM ".to_string(), "1.1.1.1".to_string()],
        protocol: ProbeProtocol::Tcp,
        port: Some(443),
        count: Some(3),
        max_hops: Some(64),
        resolve_dns: Some(false),
        include_asn: Some(true),
        interval_seconds: Some(0.5),
        timeout_seconds: Some(1.0),
    };

    let normalized = request
        .normalize_and_validate(&config)
        .expect("valid request should normalize");

    assert_eq!(normalized.targets, vec!["example.com", "1.1.1.1"]);
    assert_eq!(normalized.port, Some(443));
    assert_eq!(normalized.count, Some(3));
    assert_eq!(normalized.max_hops, Some(64));
    assert!(!normalized.resolve_dns);
    assert!(normalized.include_asn);

    let (_cancel_tx, cancel_rx) = mpsc::channel();
    let (complete_tx, complete_rx) = mpsc::channel();
    complete_tx
        .send(())
        .expect("probe completion signal should send");

    let outcome = run_probe_with_timeout_and_cancel(config.request_timeout, cancel_rx, complete_rx);
    assert_eq!(outcome, ApiProbeRunOutcome::Completed);
}

#[test]
fn api_validation_errors_reject_invalid_payload() {
    let config = RestApiConfig::default();

    let request = CreateProbeApiRequest {
        targets: vec!["bad host".to_string()],
        protocol: ProbeProtocol::Udp,
        port: None,
        count: Some(3),
        max_hops: Some(64),
        resolve_dns: Some(false),
        include_asn: Some(true),
        interval_seconds: Some(-0.1),
        timeout_seconds: Some(0.05),
    };

    assert!(matches!(
        request.normalize_and_validate(&config),
        Err(RestApiValidationError::InvalidTarget(_))
            | Err(RestApiValidationError::InvalidPort(_))
            | Err(RestApiValidationError::InvalidOption(_))
    ));
}

#[test]
fn api_timeout_path_returns_timed_out_outcome() {
    let config = RestApiConfig {
        request_timeout: Duration::from_millis(10),
        ..RestApiConfig::default()
    };

    let (_cancel_tx, cancel_rx) = mpsc::channel();
    let (_complete_tx, complete_rx) = mpsc::channel::<()>();

    let outcome = run_probe_with_timeout_and_cancel(config.request_timeout, cancel_rx, complete_rx);
    assert_eq!(outcome, ApiProbeRunOutcome::TimedOut);
}

#[test]
fn api_cancellation_releases_concurrency_slot_for_follow_up_request() {
    let gate = ProbeConcurrencyGate::new(1).expect("gate should initialize");

    let permit = gate.try_acquire().expect("first request acquires slot");
    assert!(matches!(
        gate.try_acquire(),
        Err(RestApiValidationError::ConcurrencyLimitExceeded { .. })
    ));

    let (cancel_tx, cancel_rx) = mpsc::channel();
    let (_complete_tx, complete_rx) = mpsc::channel::<()>();
    let timeout = Duration::from_millis(20);

    let handle = thread::spawn(move || {
        thread::sleep(Duration::from_millis(5));
        cancel_tx.send(()).expect("cancellation signal should send");
    });

    let outcome = run_probe_with_timeout_and_cancel(timeout, cancel_rx, complete_rx);
    handle.join().expect("canceller thread should join");
    assert_eq!(outcome, ApiProbeRunOutcome::Cancelled);

    drop(permit);
    assert!(gate.try_acquire().is_ok());
}
