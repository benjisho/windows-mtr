use std::net::IpAddr;
use windows_mtr::service::{
    EnhancedUiConfig, JsonOutput, ProbeError, ProbeRequest, UiMode, build_embedded_trippy_args,
    build_json_snapshot_args, build_probe_plan, parse_passthrough_flags,
};

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
fn plan_requires_port_for_tcp_udp() {
    let mut tcp = base_request();
    tcp.tcp = true;
    assert!(matches!(
        build_probe_plan(&tcp),
        Err(ProbeError::PortRequired(_, 'T'))
    ));

    let mut udp = base_request();
    udp.udp = true;
    assert!(matches!(
        build_probe_plan(&udp),
        Err(ProbeError::PortRequired(_, 'U'))
    ));
}

#[test]
fn plan_rejects_report_wide_without_report_or_json() {
    let mut request = base_request();
    request.report_wide = true;

    assert!(matches!(
        build_probe_plan(&request),
        Err(ProbeError::InvalidOption(_))
    ));
}

#[test]
fn plan_rejects_invalid_host() {
    let mut request = base_request();
    request.host = "invalid host with spaces".to_string();

    assert!(matches!(
        build_probe_plan(&request),
        Err(ProbeError::HostResolutionError(_))
    ));
}

#[test]
fn parse_passthrough_flags_supports_wrapped_single_token() {
    let parsed = parse_passthrough_flags("\"--tui-refresh-rate 150ms\"").expect("should parse");
    assert_eq!(parsed, vec!["--tui-refresh-rate", "150ms"]);
}

#[test]
fn parse_passthrough_flags_rejects_invalid_quoting() {
    assert!(matches!(
        parse_passthrough_flags("--foo 'bar"),
        Err(ProbeError::InvalidOption(_))
    ));
}

#[test]
fn build_embedded_trippy_args_maps_core_fields() {
    let request = ProbeRequest {
        host: "example.com".to_string(),
        tcp: true,
        udp: false,
        port: Some(443),
        source_port: Some(50000),
        report: true,
        json_output: None,
        count: Some(10),
        interval_seconds: Some(0.5),
        timeout_seconds: Some(3.0),
        report_wide: true,
        no_dns: true,
        max_hops: Some(20),
        show_asn: true,
        dns_lookup_as_info: false,
        packet_size: Some(128),
        src: Some("192.0.2.2".parse::<IpAddr>().expect("valid test ip")),
        interface: Some("Ethernet".to_string()),
        ecmp: Some("paris".to_string()),
        dns_cache_ttl_seconds: Some(120),
        trippy_flags: Some("--log-format json --verbose".to_string()),
        ui_mode: UiMode::Default,
        enhanced_ui: base_request().enhanced_ui,
        has_enhanced_overrides: false,
    };

    let trippy_args = build_embedded_trippy_args(&request, "example.com").expect("should build");
    assert_eq!(
        trippy_args,
        vec![
            "mtr",
            "--mode",
            "pretty",
            "--tcp",
            "--target-port",
            "443",
            "--source-port",
            "50000",
            "--report-cycles",
            "10",
            "--min-round-duration",
            "0.5s",
            "--grace-duration",
            "3s",
            "--tui-address-mode",
            "ip",
            "--max-ttl",
            "20",
            "--dns-lookup-as-info",
            "--packet-size",
            "128",
            "--source-address",
            "192.0.2.2",
            "--interface",
            "Ethernet",
            "--multipath-strategy",
            "paris",
            "--dns-ttl",
            "120s",
            "--log-format",
            "json",
            "--verbose",
            "example.com"
        ]
    );
}

#[test]
fn build_embedded_trippy_args_supports_json_mode() {
    let mut request = base_request();
    request.json_output = Some(JsonOutput::Compact);

    let trippy_args = build_embedded_trippy_args(&request, "8.8.8.8").expect("should build");
    assert_eq!(trippy_args, vec!["mtr", "--mode", "json", "8.8.8.8"]);
}

#[test]
fn build_json_snapshot_args_uses_json_mode_without_tui_flags() {
    let mut request = base_request();
    request.udp = true;
    request.port = Some(53);
    request.count = Some(10);
    request.trippy_flags = Some("--dns-resolve-method system".to_string());

    let args = build_json_snapshot_args(&request, "8.8.8.8").expect("should build");

    assert!(args.windows(2).any(|w| w == ["--mode", "json"]));
    assert!(args.windows(2).any(|w| w == ["--report-cycles", "1"]));
    assert!(!args.iter().any(|t| t == "tui"));
    assert!(!args.iter().any(|t| t.starts_with("--tui-")));
}

#[test]
fn build_json_snapshot_args_no_dns_does_not_emit_tui_flags() {
    let mut request = base_request();
    request.no_dns = true;

    let args = build_json_snapshot_args(&request, "8.8.8.8").expect("should build");

    assert!(!args.iter().any(|t| t.starts_with("--tui-")));
}

#[test]
fn build_json_snapshot_args_rejects_conflicting_passthrough_flags() {
    let mut request = base_request();
    request.ui_mode = UiMode::Dashboard;
    request.trippy_flags = Some("--mode tui --tui-refresh-rate 150ms".to_string());

    assert!(matches!(
        build_probe_plan(&request),
        Err(ProbeError::InvalidOption(_))
    ));
}

#[test]
fn enhanced_ui_is_soft_disabled_with_actionable_error() {
    let mut request = base_request();
    request.ui_mode = UiMode::Enhanced;

    assert!(matches!(
        build_probe_plan(&request),
        Err(ProbeError::InvalidOption(msg)) if msg.contains("enhanced UI is not available with bundled Trippy 0.13.0")
    ));
}

#[test]
fn build_embedded_trippy_args_enhanced_has_no_unsupported_tui_flags() {
    let mut request = base_request();
    request.ui_mode = UiMode::Enhanced;

    let args = build_embedded_trippy_args(&request, "8.8.8.8").expect("should build");
    for forbidden in [
        "--tui-latency-warn-threshold",
        "--tui-latency-bad-threshold",
        "--tui-loss-warn-threshold",
        "--tui-loss-bad-threshold",
        "--tui-row-coloring",
        "--tui-hop-trend",
        "--tui-summary-jitter",
        "--tui-summary-percentiles",
    ] {
        assert!(!args.iter().any(|token| token == forbidden));
    }
}

#[test]
fn enhanced_ui_overrides_require_enhanced_mode() {
    let mut request = base_request();
    request.has_enhanced_overrides = true;

    assert!(matches!(
        build_probe_plan(&request),
        Err(ProbeError::InvalidOption(_))
    ));
}
