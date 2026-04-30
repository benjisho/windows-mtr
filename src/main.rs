use anyhow::Context;
use clap::{Args, Parser, ValueEnum};
use std::env;
use std::net::{IpAddr, SocketAddr};
use std::process;
use std::time::Duration;
use windows_mtr::service::rest_api::{AuthStrategy, RestApiConfig};
use windows_mtr::service::rest_server::run_rest_api_server;
use windows_mtr::service::{
    EnhancedUiConfig, JsonOutput, ProbeError, ProbeRequest, UiMode, build_probe_plan,
    run_embedded_trippy,
};

mod error;
mod native_ui;
use error::MtrError;

const EMBEDDED_TRIPPY_ENV: &str = "WINDOWS_MTR_EMBEDDED_TRIPPY";

/// Windows-native clone of Linux mtr - a CLI that delivers ICMP/TCP/UDP traceroute & ping
#[derive(Parser, Debug)]
#[command(author = "Benji Shohet (benjisho)", version, about, long_about = None)]
#[command(after_help = "Examples:
  windows-mtr 8.8.8.8                           # Basic ICMP trace to Google DNS
  windows-mtr -T -P 443 github.com              # TCP trace to GitHub on port 443 (HTTPS)
  windows-mtr -U -P 53 1.1.1.1                  # UDP trace to Cloudflare DNS on port 53
  windows-mtr -r -c 10 example.com              # Report mode with 10 pings per hop
  windows-mtr --json -c 20 example.com          # JSON report output
  windows-mtr --api                              # Run REST API runtime
  windows-mtr --trippy-flags '--tui-refresh-rate 150ms' example.com")]
struct Cli {
    /// Run in REST API mode instead of probe CLI mode
    #[arg(long = "api")]
    api: bool,

    /// Bind address for API mode
    #[arg(long = "api-bind", value_name = "ADDR")]
    api_bind: Option<SocketAddr>,

    /// REST API authentication strategy [none-local-only|api-key|mtls]
    #[arg(long = "api-auth", value_enum, value_name = "STRATEGY")]
    api_auth: Option<ApiAuthPreset>,

    /// Inline API key for `--api-auth api-key` (prefer --api-key-env for safety)
    #[arg(long = "api-key", value_name = "KEY", conflicts_with = "api_key_env")]
    api_key: Option<String>,

    /// Environment variable name that stores API key for `--api-auth api-key`
    #[arg(
        long = "api-key-env",
        value_name = "ENV_VAR",
        conflicts_with = "api_key"
    )]
    api_key_env: Option<String>,

    /// Maximum number of REST API requests allowed per fixed window
    #[arg(long = "api-max-requests-per-window", value_name = "COUNT")]
    api_max_requests_per_window: Option<usize>,

    /// Fixed rate-limit window duration in seconds for REST API requests
    #[arg(long = "api-rate-limit-window-seconds", value_name = "SECONDS")]
    api_rate_limit_window_seconds: Option<u64>,

    /// Maximum completed/failed probe jobs retained in API in-memory store
    #[arg(long = "api-max-completed-jobs", value_name = "COUNT")]
    api_max_completed_jobs: Option<usize>,

    /// TTL in seconds for completed/failed probe jobs retained in API in-memory store
    #[arg(long = "api-completed-job-ttl-seconds", value_name = "SECONDS")]
    api_completed_job_ttl_seconds: Option<u64>,

    /// Trusted ingress source IP(s) allowed to forward mTLS identity headers (repeatable)
    #[arg(long = "api-mtls-trusted-ingress", value_name = "IP")]
    api_mtls_trusted_ingress: Vec<IpAddr>,

    #[command(flatten)]
    trace: TraceCli,
}

#[derive(Args, Debug)]
struct TraceCli {
    /// Target host to trace (hostname or IP)
    host: Option<String>,

    /// Use TCP SYN for probes (default is ICMP)
    #[arg(short = 'T', conflicts_with = "udp")]
    tcp: bool,

    /// Use UDP for probes (default is ICMP)
    #[arg(short = 'U', conflicts_with = "tcp")]
    udp: bool,

    /// Target port for TCP/UDP modes (required when using -T or -U)
    #[arg(short = 'P', value_name = "PORT", value_parser = clap::value_parser!(u16).range(1..=65535))]
    port: Option<u16>,

    /// Source port for TCP/UDP probes
    #[arg(long, value_name = "PORT", value_parser = clap::value_parser!(u16).range(1..=65535))]
    source_port: Option<u16>,

    /// Report mode (no continuous updates)
    #[arg(short = 'r')]
    report: bool,

    /// Generate JSON report output
    #[arg(short = 'j', long = "json", conflicts_with = "json_pretty")]
    json: bool,

    /// Generate pretty-formatted JSON report output
    #[arg(long = "json-pretty", conflicts_with = "json")]
    json_pretty: bool,

    /// Number of pings (cycles) to send to each host
    #[arg(short = 'c')]
    count: Option<usize>,

    /// Minimum time in seconds between rounds
    #[arg(short = 'i')]
    interval: Option<f32>,

    /// Maximum time in seconds to keep a probe alive
    #[arg(short = 'W', long = "timeout")]
    timeout: Option<f32>,

    /// Report mode with wider host name field (Linux mtr parity)
    #[arg(short = 'w', long = "report-wide")]
    report_wide: bool,

    /// Don't perform reverse DNS lookups (faster)
    #[arg(short = 'n')]
    no_dns: bool,

    /// Maximum number of hops to trace
    #[arg(short = 'm')]
    max_hops: Option<u8>,

    /// Show ASN data in reports and lookups (Linux parity)
    #[arg(short = 'b', long = "show-asn")]
    show_asn: bool,

    /// DNS/ASN lookup mode (Linux parity shortcut)
    #[arg(short = 'z')]
    dns_lookup_as_info: bool,

    /// Packet size for probes
    #[arg(short = 's', long = "packet-size", value_name = "BYTES")]
    packet_size: Option<u16>,

    /// Source IP address to bind probes from (Linux parity)
    #[arg(short = 'S', long = "src")]
    src: Option<IpAddr>,

    /// Source network interface
    #[arg(long = "interface")]
    interface: Option<String>,

    /// Equal-cost multipath strategy [classic|paris|dublin]
    #[arg(long = "ecmp", value_parser = ["classic", "paris", "dublin"])]
    ecmp: Option<String>,

    /// DNS cache TTL in seconds for this run
    #[arg(long = "dns-cache-ttl", value_name = "SECONDS")]
    dns_cache_ttl: Option<u64>,

    /// Forward additional native trippy options verbatim
    #[arg(
        long = "trippy-flags",
        value_name = "FLAGS",
        allow_hyphen_values = true
    )]
    trippy_flags: Option<String>,

    /// UI preset for interactive mode (dashboard is experimental; native is a deprecated alias)
    #[arg(long = "ui", value_enum, default_value_t = UiPreset::Default)]
    ui: UiPreset,

    /// Latency warning threshold in milliseconds for enhanced UI coloring
    #[arg(long = "latency-warn-ms", value_name = "MS")]
    latency_warn_ms: Option<f32>,

    /// Latency critical threshold in milliseconds for enhanced UI coloring
    #[arg(long = "latency-bad-ms", value_name = "MS")]
    latency_bad_ms: Option<f32>,

    /// Packet loss warning threshold percentage for enhanced UI coloring
    #[arg(long = "loss-warn-pct", value_name = "PCT")]
    loss_warn_pct: Option<f32>,

    /// Packet loss critical threshold percentage for enhanced UI coloring
    #[arg(long = "loss-bad-pct", value_name = "PCT")]
    loss_bad_pct: Option<f32>,

    /// Toggle row coloring bands in enhanced UI
    #[arg(long = "enhanced-row-color", value_enum, value_name = "on|off")]
    enhanced_row_color: Option<OnOff>,

    /// Toggle per-hop trend/sparkline column in enhanced UI
    #[arg(long = "enhanced-sparklines", value_enum, value_name = "on|off")]
    enhanced_sparklines: Option<OnOff>,

    /// Toggle percentile/jitter summary in enhanced UI
    #[arg(long = "enhanced-summary", value_enum, value_name = "on|off")]
    enhanced_summary: Option<OnOff>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum UiPreset {
    Default,
    Enhanced,
    #[value(alias = "native")]
    Dashboard,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum OnOff {
    On,
    Off,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum ApiAuthPreset {
    ApiKey,
    Mtls,
    NoneLocalOnly,
}

impl OnOff {
    fn as_bool(self) -> bool {
        matches!(self, Self::On)
    }
}

fn json_output_from_cli(args: &TraceCli) -> Option<JsonOutput> {
    if args.json {
        Some(JsonOutput::Compact)
    } else if args.json_pretty {
        Some(JsonOutput::Pretty)
    } else {
        None
    }
}

fn should_print_banner(args: &Cli) -> bool {
    !args.api && json_output_from_cli(&args.trace).is_none()
}

fn should_print_interactive_troubleshooting_hint(request: &ProbeRequest, exit_code: i32) -> bool {
    request.ui_mode == UiMode::Default
        && !request.report
        && request.json_output.is_none()
        && exit_code != 0
}

fn format_exit_code(exit_code: i32) -> String {
    if exit_code < 0 {
        format!("{exit_code} (0x{:08X})", exit_code as u32)
    } else {
        exit_code.to_string()
    }
}

fn windows_exit_diagnostic(exit_code: i32) -> Option<&'static str> {
    match exit_code as u32 {
        0xC0000005 => Some(
            "Detected a Windows access violation from the embedded Trippy UI. \
             Retry with `mtr --ui dashboard <target>` or use report mode (`mtr -r -c 5 <target>`).",
        ),
        _ => None,
    }
}

fn print_banner() {
    println!("windows-mtr by Benji Shohet (benjisho) — https://github.com/benjisho/windows-mtr");
}

fn auth_strategy_from_cli(value: ApiAuthPreset) -> AuthStrategy {
    match value {
        ApiAuthPreset::ApiKey => AuthStrategy::ApiKey,
        ApiAuthPreset::Mtls => AuthStrategy::Mtls,
        ApiAuthPreset::NoneLocalOnly => AuthStrategy::NoneLocalOnly,
    }
}

fn api_key_from_cli(args: &Cli) -> anyhow::Result<Option<String>> {
    if let Some(env_name) = &args.api_key_env {
        let raw = env::var(env_name).with_context(|| {
            format!(
                "--api-key-env was set to `{env_name}`, but that environment variable is not present"
            )
        })?;

        let key = raw.trim().to_string();
        if key.is_empty() {
            anyhow::bail!(
                "--api-key-env was set to `{env_name}`, but that environment variable is empty"
            );
        }

        return Ok(Some(key));
    }

    Ok(args.api_key.clone())
}

fn apply_rest_api_cli_overrides(args: &Cli, config: &mut RestApiConfig) -> anyhow::Result<()> {
    if let Some(bind) = args.api_bind {
        config.bind_addr = bind;
    }

    if let Some(auth) = args.api_auth {
        config.auth_strategy = auth_strategy_from_cli(auth);
    }

    if !config.bind_addr.ip().is_loopback() {
        config.allow_non_local_bind = true;
    }

    let api_key = api_key_from_cli(args)?;
    if api_key.is_some() && config.auth_strategy != AuthStrategy::ApiKey {
        anyhow::bail!(
            "API key input (--api-key/--api-key-env) requires '--api-auth api-key'; current strategy is '{:?}'",
            config.auth_strategy
        );
    }
    if config.auth_strategy == AuthStrategy::ApiKey && api_key.is_none() {
        anyhow::bail!(
            "'--api-auth api-key' requires key input via '--api-key-env <ENV_VAR>' (preferred) or '--api-key <KEY>'"
        );
    }

    config.api_key = api_key;

    if let Some(max_requests) = args.api_max_requests_per_window {
        config.max_requests_per_window = max_requests;
    }

    if let Some(window_seconds) = args.api_rate_limit_window_seconds {
        config.rate_limit_window = Duration::from_secs(window_seconds);
    }

    if let Some(max_completed_jobs) = args.api_max_completed_jobs {
        config.max_completed_jobs = max_completed_jobs;
    }

    if let Some(completed_job_ttl_seconds) = args.api_completed_job_ttl_seconds {
        config.completed_job_ttl = Duration::from_secs(completed_job_ttl_seconds);
    }

    if !args.api_mtls_trusted_ingress.is_empty() {
        config.trusted_mtls_ingress_ips = args.api_mtls_trusted_ingress.clone();
    }

    Ok(())
}

fn ui_mode_from_cli(ui: UiPreset) -> UiMode {
    match ui {
        UiPreset::Default => UiMode::Default,
        UiPreset::Enhanced => UiMode::Enhanced,
        UiPreset::Dashboard => UiMode::Dashboard,
    }
}

fn enhanced_ui_config_from_cli(args: &TraceCli) -> EnhancedUiConfig {
    EnhancedUiConfig {
        latency_warn_ms: args.latency_warn_ms.unwrap_or(100.0),
        latency_bad_ms: args.latency_bad_ms.unwrap_or(250.0),
        loss_warn_pct: args.loss_warn_pct.unwrap_or(2.0),
        loss_bad_pct: args.loss_bad_pct.unwrap_or(5.0),
        row_coloring: args.enhanced_row_color.unwrap_or(OnOff::On).as_bool(),
        sparklines: args.enhanced_sparklines.unwrap_or(OnOff::On).as_bool(),
        summary: args.enhanced_summary.unwrap_or(OnOff::On).as_bool(),
    }
}

fn build_probe_request(args: &TraceCli) -> anyhow::Result<ProbeRequest> {
    let host = args
        .host
        .clone()
        .ok_or_else(|| anyhow::anyhow!("missing host argument (or run with --api)"))?;

    let has_enhanced_overrides = args.latency_warn_ms.is_some()
        || args.latency_bad_ms.is_some()
        || args.loss_warn_pct.is_some()
        || args.loss_bad_pct.is_some()
        || args.enhanced_row_color.is_some()
        || args.enhanced_sparklines.is_some()
        || args.enhanced_summary.is_some();

    Ok(ProbeRequest {
        host,
        tcp: args.tcp,
        udp: args.udp,
        port: args.port,
        source_port: args.source_port,
        report: args.report,
        json_output: json_output_from_cli(args),
        count: args.count,
        interval_seconds: args.interval,
        timeout_seconds: args.timeout,
        report_wide: args.report_wide,
        no_dns: args.no_dns,
        max_hops: args.max_hops,
        show_asn: args.show_asn,
        dns_lookup_as_info: args.dns_lookup_as_info,
        packet_size: args.packet_size,
        src: args.src,
        interface: args.interface.clone(),
        ecmp: args.ecmp.clone(),
        dns_cache_ttl_seconds: args.dns_cache_ttl,
        trippy_flags: args.trippy_flags.clone(),
        ui_mode: ui_mode_from_cli(args.ui),
        enhanced_ui: enhanced_ui_config_from_cli(args),
        has_enhanced_overrides,
    })
}

fn to_cli_error(error: ProbeError) -> MtrError {
    match error {
        ProbeError::HostResolutionError(host) => MtrError::HostResolutionError(host),
        ProbeError::PortRequired(protocol, flag) => MtrError::PortRequired(protocol, flag),
        ProbeError::InvalidOption(detail) => MtrError::InvalidOption(detail),
    }
}

fn main() -> anyhow::Result<()> {
    if env::var_os(EMBEDDED_TRIPPY_ENV).is_some() {
        return trippy_tui::trippy();
    }

    let args = Cli::parse();

    if args.api {
        let mut config = RestApiConfig::default();
        apply_rest_api_cli_overrides(&args, &mut config)?;

        config
            .validate_security_defaults()
            .map_err(|e| anyhow::anyhow!(
                "invalid REST API security configuration: {e}. Action: for remote binds, set '--api-bind 0.0.0.0:PORT --api-auth api-key --api-key-env <ENV_VAR>' or '--api-auth mtls'; keep default localhost when unauthenticated"
            ))?;

        let runtime = tokio::runtime::Runtime::new().context("failed to start tokio runtime")?;
        return runtime.block_on(run_rest_api_server(config));
    }

    if should_print_banner(&args) {
        print_banner();
    }

    let request = build_probe_request(&args.trace)?;
    let plan = build_probe_plan(&request)
        .map_err(to_cli_error)
        .map_err(|error| anyhow::anyhow!(error.to_string()))
        .context("invalid command-line options")?;

    if plan.ui_mode == UiMode::Dashboard {
        let snapshot_args = plan
            .dashboard_snapshot_args
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("missing dashboard JSON snapshot args in probe plan"))?;
        let code = native_ui::run_dashboard_ui(&plan.validated_host, snapshot_args)?;
        process::exit(code);
    }

    // SAFETY: this path is used only to re-exec ourselves for local output handling,
    // not for trust, auth, or authorization decisions.
    let current_exe =
        // nosemgrep: rust.lang.security.current-exe.current-exe
        env::current_exe().context("failed to locate current executable")?;

    let result = run_embedded_trippy(
        &current_exe,
        &plan.trippy_args,
        plan.json_output,
        EMBEDDED_TRIPPY_ENV,
    )
    .context("failed to run embedded trippy")?;

    if should_print_interactive_troubleshooting_hint(&request, result.exit_code) {
        let diagnostic = windows_exit_diagnostic(result.exit_code)
            .map(|message| format!("\n{message}"))
            .unwrap_or_default();
        eprintln!(
            "windows-mtr interactive mode exited with code {}.\n\
             Try report mode to validate probe execution: `mtr -r -c 5 {}`.\n\
             If report mode also fails, verify Administrator privileges and local firewall/security policy.{}",
            format_exit_code(result.exit_code),
            plan.validated_host,
            diagnostic
        );
    }

    process::exit(result.exit_code);
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn cli_accepts_api_mode_without_host() {
        let cli = Cli::try_parse_from(["mtr", "--api"]).expect("api mode should parse");
        assert!(cli.api);
        assert!(cli.trace.host.is_none());
    }

    #[test]
    fn interactive_troubleshooting_hint_only_for_default_interactive_failures() {
        let request = ProbeRequest {
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
        };

        assert!(should_print_interactive_troubleshooting_hint(&request, 1));
        assert!(!should_print_interactive_troubleshooting_hint(&request, 0));
    }

    #[test]
    fn format_exit_code_renders_windows_status_hex_for_negative_codes() {
        assert_eq!(format_exit_code(-1073741819), "-1073741819 (0xC0000005)");
        assert_eq!(format_exit_code(1), "1");
    }

    #[test]
    fn windows_exit_diagnostic_flags_access_violation() {
        assert!(windows_exit_diagnostic(-1073741819).is_some());
        assert!(windows_exit_diagnostic(1).is_none());
    }

    #[test]
    fn cli_parses_api_bind_override() {
        let cli = Cli::try_parse_from(["mtr", "--api", "--api-bind", "127.0.0.1:4000"])
            .expect("api bind should parse");
        assert_eq!(cli.api_bind, Some("127.0.0.1:4000".parse().unwrap()));
    }

    #[test]
    fn cli_parses_rate_limit_controls() {
        let cli = Cli::try_parse_from([
            "mtr",
            "--api",
            "--api-max-requests-per-window",
            "20",
            "--api-rate-limit-window-seconds",
            "30",
        ])
        .expect("rate limit options should parse");

        assert_eq!(cli.api_max_requests_per_window, Some(20));
        assert_eq!(cli.api_rate_limit_window_seconds, Some(30));
    }

    #[test]
    fn cli_parses_retention_controls() {
        let cli = Cli::try_parse_from([
            "mtr",
            "--api",
            "--api-max-completed-jobs",
            "512",
            "--api-completed-job-ttl-seconds",
            "1200",
        ])
        .expect("retention options should parse");

        assert_eq!(cli.api_max_completed_jobs, Some(512));
        assert_eq!(cli.api_completed_job_ttl_seconds, Some(1200));
    }

    #[test]
    fn cli_parses_mtls_trusted_ingress_controls() {
        let cli = Cli::try_parse_from([
            "mtr",
            "--api",
            "--api-auth",
            "mtls",
            "--api-mtls-trusted-ingress",
            "127.0.0.1",
            "--api-mtls-trusted-ingress",
            "10.0.0.10",
        ])
        .expect("mTLS trusted ingress options should parse");

        assert_eq!(cli.api_mtls_trusted_ingress.len(), 2);
        assert_eq!(
            cli.api_mtls_trusted_ingress[0],
            "127.0.0.1".parse::<IpAddr>().unwrap()
        );
        assert_eq!(
            cli.api_mtls_trusted_ingress[1],
            "10.0.0.10".parse::<IpAddr>().unwrap()
        );
    }

    #[test]
    fn cli_parses_api_auth_with_api_key_env() {
        let cli = Cli::try_parse_from([
            "mtr",
            "--api",
            "--api-bind",
            "0.0.0.0:4000",
            "--api-auth",
            "api-key",
            "--api-key-env",
            "WINDOWS_MTR_API_KEY",
        ])
        .expect("api auth flags should parse");

        assert_eq!(cli.api_auth, Some(ApiAuthPreset::ApiKey));
        assert_eq!(cli.api_key_env.as_deref(), Some("WINDOWS_MTR_API_KEY"));
    }

    #[test]
    fn cli_rejects_api_key_sources_used_together() {
        let err = Cli::try_parse_from([
            "mtr",
            "--api",
            "--api-auth",
            "api-key",
            "--api-key",
            "secret",
            "--api-key-env",
            "WINDOWS_MTR_API_KEY",
        ])
        .expect_err("clap should reject conflicting API key inputs");

        assert!(err.to_string().contains("cannot be used with"));
    }

    #[test]
    fn api_mode_rejects_non_api_key_auth_with_key_input() {
        let cli =
            Cli::try_parse_from(["mtr", "--api", "--api-auth", "mtls", "--api-key", "secret"])
                .expect("flags should parse for validation test");

        let mut config = RestApiConfig::default();
        let err = apply_rest_api_cli_overrides(&cli, &mut config)
            .expect_err("key input with mtls should fail validation");
        assert!(err.to_string().contains("requires '--api-auth api-key'"));
    }

    #[test]
    fn api_mode_applies_rate_limit_overrides() {
        let cli = Cli::try_parse_from([
            "mtr",
            "--api",
            "--api-max-requests-per-window",
            "20",
            "--api-rate-limit-window-seconds",
            "30",
        ])
        .expect("flags should parse for override validation");

        let mut config = RestApiConfig::default();
        apply_rest_api_cli_overrides(&cli, &mut config).expect("overrides should apply");

        assert_eq!(config.max_requests_per_window, 20);
        assert_eq!(config.rate_limit_window, Duration::from_secs(30));
    }

    #[test]
    fn api_mode_applies_retention_overrides() {
        let cli = Cli::try_parse_from([
            "mtr",
            "--api",
            "--api-max-completed-jobs",
            "512",
            "--api-completed-job-ttl-seconds",
            "1200",
        ])
        .expect("flags should parse for retention override validation");

        let mut config = RestApiConfig::default();
        apply_rest_api_cli_overrides(&cli, &mut config).expect("overrides should apply");

        assert_eq!(config.max_completed_jobs, 512);
        assert_eq!(config.completed_job_ttl, Duration::from_secs(1200));
    }

    #[test]
    fn api_mode_applies_mtls_trusted_ingress_overrides() {
        let cli = Cli::try_parse_from([
            "mtr",
            "--api",
            "--api-auth",
            "mtls",
            "--api-mtls-trusted-ingress",
            "10.10.10.10",
        ])
        .expect("flags should parse for mTLS ingress override validation");

        let mut config = RestApiConfig::default();
        apply_rest_api_cli_overrides(&cli, &mut config).expect("overrides should apply");

        assert_eq!(
            config.trusted_mtls_ingress_ips,
            vec!["10.10.10.10".parse::<IpAddr>().unwrap()]
        );
    }

    #[test]
    fn api_mode_rejects_missing_api_key_for_api_key_auth() {
        let cli = Cli::try_parse_from(["mtr", "--api", "--api-auth", "api-key"])
            .expect("flags should parse for validation test");

        let mut config = RestApiConfig::default();
        let err = apply_rest_api_cli_overrides(&cli, &mut config)
            .expect_err("api-key auth without key should fail validation");
        assert!(err.to_string().contains("requires key input"));
    }

    #[test]
    fn probe_mode_requires_host_argument() {
        let cli = Cli::try_parse_from(["mtr"]).expect("empty CLI should still parse");
        let err = build_probe_request(&cli.trace).expect_err("probe mode should require host");
        assert!(
            err.to_string()
                .contains("missing host argument (or run with --api)")
        );
    }

    #[test]
    fn ui_dashboard_accepts_native_alias_for_compatibility() {
        let dashboard = Cli::try_parse_from(["mtr", "--ui", "dashboard", "8.8.8.8"])
            .expect("dashboard value should parse");
        let native_alias = Cli::try_parse_from(["mtr", "--ui", "native", "8.8.8.8"])
            .expect("native alias should parse");

        assert_eq!(dashboard.trace.ui, UiPreset::Dashboard);
        assert_eq!(native_alias.trace.ui, UiPreset::Dashboard);
    }
}
