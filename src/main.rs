use anyhow::Context;
use clap::{Parser, ValueEnum};
use std::env;
use std::io::Write;
use std::net::{IpAddr, ToSocketAddrs};
use std::process::{self, Command, Stdio};
use windows_mtr::passthrough::parse_passthrough_flags as parse_passthrough_flags_core;

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
  windows-mtr --trippy-flags '--tui-refresh-rate 150ms' example.com")]
struct Cli {
    /// Target host to trace (hostname or IP)
    host: String,

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

fn json_format_from_args(args: &Cli) -> Option<JsonOutput> {
    if args.json {
        Some(JsonOutput::Compact)
    } else if args.json_pretty {
        Some(JsonOutput::Pretty)
    } else {
        None
    }
}

fn should_print_banner(args: &Cli) -> bool {
    json_format_from_args(args).is_none()
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

fn enhanced_ui_config_from_cli(args: &Cli) -> EnhancedUiConfig {
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

fn verify_options(args: &Cli) -> Result<()> {
    if (args.tcp || args.udp) && args.port.is_none() {
        let (protocol, flag) = if args.tcp { ("TCP", 'T') } else { ("UDP", 'U') };
        return Err(MtrError::PortRequired(protocol.to_string(), flag));
    }

    if args.report_wide && !args.report && !args.json && !args.json_pretty {
        return Err(MtrError::InvalidOption(
            "-w/--report-wide requires -r/--report or --json output mode".to_string(),
        ));
    }

    let has_enhanced_specific_options = args.latency_warn_ms.is_some()
        || args.latency_bad_ms.is_some()
        || args.loss_warn_pct.is_some()
        || args.loss_bad_pct.is_some()
        || args.enhanced_row_color.is_some()
        || args.enhanced_sparklines.is_some()
        || args.enhanced_summary.is_some();

    if (args.ui == UiPreset::Enhanced || args.ui == UiPreset::Native)
        && (args.report || args.json || args.json_pretty)
    {
        let ui_name = match args.ui {
            UiPreset::Enhanced => "enhanced",
            UiPreset::Native => "native",
            UiPreset::Default => "default",
        };

        return Err(MtrError::InvalidOption(format!(
            "--ui {ui_name} is only supported in interactive TUI mode"
        )));
    }

    if has_enhanced_specific_options && args.ui != UiPreset::Enhanced {
        return Err(MtrError::InvalidOption(
            "enhanced UI tuning flags require --ui enhanced".to_string(),
        ));
    }

    if args.ui == UiPreset::Enhanced {
        let config = EnhancedUiConfig::from_cli(args);
        if config.latency_warn_ms >= config.latency_bad_ms {
            return Err(MtrError::InvalidOption(
                "--latency-warn-ms must be lower than --latency-bad-ms".to_string(),
            ));
        }
        if config.loss_warn_pct >= config.loss_bad_pct {
            return Err(MtrError::InvalidOption(
                "--loss-warn-pct must be lower than --loss-bad-pct".to_string(),
            ));
        }
    }

    if let Some(flags) = &args.trippy_flags {
        let parsed = parse_passthrough_flags(flags)?;
        let conflicting = [
            "--tui-latency-warn-threshold",
            "--tui-latency-bad-threshold",
            "--tui-loss-warn-threshold",
            "--tui-loss-bad-threshold",
            "--tui-row-coloring",
            "--tui-hop-trend",
            "--tui-summary-jitter",
            "--tui-summary-percentiles",
        ];

        if args.ui == UiPreset::Enhanced
            && parsed
                .iter()
                .any(|token| conflicting.iter().any(|flag| token == flag))
        {
            return Err(MtrError::InvalidOption(
                "--trippy-flags cannot override windows-mtr enhanced UI wrapper settings"
                    .to_string(),
            ));
        }
    }

    Ok(())
}

fn mode_from_args(args: &Cli) -> &'static str {
    if json_format_from_args(args).is_some() {
        "json"
    } else if args.report || args.report_wide {
        "pretty"
    } else {
        "tui"
    }
}

fn duration_seconds(value: f32) -> String {
    if value.fract() == 0.0 {
        format!("{}s", value as u64)
    } else {
        format!("{value}s")
    }
}

fn parse_passthrough_flags(flags: &str) -> Result<Vec<String>> {
    parse_passthrough_flags_core(flags).map_err(MtrError::InvalidOption)
}

fn build_embedded_trippy_args(args: &Cli, host: &str) -> Result<Vec<String>> {
    let mut trippy_args = vec!["mtr".to_string()];

    trippy_args.extend(["--mode".to_string(), mode_from_args(args).to_string()]);

    if args.tcp {
        trippy_args.push("--tcp".to_string());
    } else if args.udp {
        trippy_args.push("--udp".to_string());
    }

    if let Some(port) = args.port {
        trippy_args.extend(["--target-port".to_string(), port.to_string()]);
    }

    if let Some(source_port) = args.source_port {
        trippy_args.extend(["--source-port".to_string(), source_port.to_string()]);
    }

    if let Some(count) = args.count {
        trippy_args.extend(["--report-cycles".to_string(), count.to_string()]);
    }

    if let Some(interval) = args.interval {
        trippy_args.extend([
            "--min-round-duration".to_string(),
            duration_seconds(interval),
        ]);
    }

    if let Some(timeout) = args.timeout {
        trippy_args.extend(["--grace-duration".to_string(), duration_seconds(timeout)]);
    }

    if args.no_dns {
        trippy_args.extend(["--tui-address-mode".to_string(), "ip".to_string()]);
    }

    if let Some(max_hops) = args.max_hops {
        trippy_args.extend(["--max-ttl".to_string(), max_hops.to_string()]);
    }

    if args.show_asn || args.dns_lookup_as_info {
        trippy_args.push("--dns-lookup-as-info".to_string());
    }

    if let Some(packet_size) = args.packet_size {
        trippy_args.extend(["--packet-size".to_string(), packet_size.to_string()]);
    }

    if let Some(src) = args.src {
        trippy_args.extend(["--source-address".to_string(), src.to_string()]);
    }

    if let Some(interface) = &args.interface {
        trippy_args.extend(["--interface".to_string(), interface.clone()]);
    }

    if let Some(ecmp) = &args.ecmp {
        trippy_args.extend(["--multipath-strategy".to_string(), ecmp.clone()]);
    }

    if let Some(ttl) = args.dns_cache_ttl {
        trippy_args.extend(["--dns-ttl".to_string(), format!("{ttl}s")]);
    }

    if args.ui == UiPreset::Enhanced {
        let ui = EnhancedUiConfig::from_cli(args);
        trippy_args.extend([
            "--tui-latency-warn-threshold".to_string(),
            format!("{}ms", ui.latency_warn_ms),
            "--tui-latency-bad-threshold".to_string(),
            format!("{}ms", ui.latency_bad_ms),
            "--tui-loss-warn-threshold".to_string(),
            ui.loss_warn_pct.to_string(),
            "--tui-loss-bad-threshold".to_string(),
            ui.loss_bad_pct.to_string(),
            "--tui-row-coloring".to_string(),
            ui.row_coloring.to_string(),
            "--tui-hop-trend".to_string(),
            ui.sparklines.to_string(),
            "--tui-summary-jitter".to_string(),
            ui.summary.to_string(),
            "--tui-summary-percentiles".to_string(),
            ui.summary.to_string(),
        ]);
    }

    if let Some(extra) = &args.trippy_flags {
        trippy_args.extend(parse_passthrough_flags(extra)?);
    }

    trippy_args.push(host.to_string());
    Ok(trippy_args)
}

fn run_embedded_trippy(args: &[String], json_format: Option<JsonFormat>) -> anyhow::Result<i32> {
    // SAFETY: `current_exe` is only used to re-exec this process for output formatting,
    // not for any trust or authorization decision.
    let current_exe =
        // nosemgrep: rust.lang.security.current-exe.current-exe
        env::current_exe().context("failed to locate current executable")?;

    if let Some(format) = json_format {
        let output = Command::new(&current_exe)
            .env(EMBEDDED_TRIPPY_ENV, "1")
            .args(args.iter().skip(1))
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .output()
            .context("failed to launch embedded trippy runner")?;

        match format {
            JsonFormat::Compact => {
                let value: serde_json::Value = serde_json::from_slice(&output.stdout)
                    .context("failed to parse trippy JSON output")?;
                serde_json::to_writer(std::io::stdout(), &value)
                    .context("failed to write compact JSON output")?;
                std::io::stdout().write_all(b"\n")?;
            }
            JsonFormat::Pretty => {
                std::io::stdout().write_all(&output.stdout)?;
            }
        }

        return Ok(output.status.code().unwrap_or(2));
    }

    let status = Command::new(current_exe)
        .env(EMBEDDED_TRIPPY_ENV, "1")
        .args(args.iter().skip(1))
        .status()
        .context("failed to launch embedded trippy runner")?;
    Ok(status.code().unwrap_or(2))
}

fn main() -> anyhow::Result<()> {
    if env::var_os(EMBEDDED_TRIPPY_ENV).is_some() {
        return trippy_tui::trippy();
    }

    let args = Cli::parse();

    if should_print_banner(&args) {
        print_banner();
    }

    let request = build_probe_request(&args);
    verify_options(&request)
        .map_err(to_cli_error)
        .map_err(|e| anyhow::anyhow!(e.to_string()))
        .context("invalid command-line options")?;

    let host = validate_target(&request.host)
        .map_err(to_cli_error)
        .map_err(|e| anyhow::anyhow!(e.to_string()))
        .with_context(|| format!("invalid target host: {}", request.host))?;

    let trippy_args = build_embedded_trippy_args(&request, &host)
        .map_err(to_cli_error)
        .map_err(|e| anyhow::anyhow!(e.to_string()))
        .context("failed to translate windows-mtr options into trippy options")?;

    if args.ui == UiPreset::Native {
        let code = native_ui::run_native_ui(&host, &trippy_args)?;
        process::exit(code);
    }

    // SAFETY: `current_exe` is only used to re-exec this process for output formatting,
    // not for any trust or authorization decision.
    let current_exe =
        // nosemgrep: rust.lang.security.current-exe.current-exe
        env::current_exe().context("failed to locate current executable")?;

    let result = run_embedded_trippy(
        &current_exe,
        &trippy_args,
        request.json_output,
        EMBEDDED_TRIPPY_ENV,
    )?;
    process::exit(result.exit_code);
}
