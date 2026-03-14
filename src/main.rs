use anyhow::Context;
use clap::{Args, Parser, ValueEnum};
use std::env;
use std::net::{IpAddr, SocketAddr};
use std::process;
use windows_mtr::service::rest_api::RestApiConfig;
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

    /// UI preset for interactive mode
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
    Native,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum OnOff {
    On,
    Off,
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

fn print_banner() {
    println!("windows-mtr by Benji Shohet (benjisho) — https://github.com/benjisho/windows-mtr");
}

fn ui_mode_from_cli(ui: UiPreset) -> UiMode {
    match ui {
        UiPreset::Default => UiMode::Default,
        UiPreset::Enhanced => UiMode::Enhanced,
        UiPreset::Native => UiMode::Native,
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
        if let Some(bind) = args.api_bind {
            config.bind_addr = bind;
        }
        config
            .validate_security_defaults()
            .map_err(|e| anyhow::anyhow!(
                "invalid REST API security configuration: {e}. Action: keep localhost defaults or set --api-bind with a secure auth strategy in config"
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

    if plan.ui_mode == UiMode::Native {
        let code = native_ui::run_native_ui(&plan.validated_host, &plan.trippy_args)?;
        process::exit(code);
    }

    let current_exe = env::current_exe().context("failed to locate current executable")?;

    let result = run_embedded_trippy(
        &current_exe,
        &plan.trippy_args,
        plan.json_output,
        EMBEDDED_TRIPPY_ENV,
    )
    .context("failed to run embedded trippy")?;
    process::exit(result.exit_code);
}
