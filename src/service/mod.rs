pub mod api_models;
pub mod rest_api;
pub mod rest_server;
use anyhow::Context;
use std::io::Write;
use std::net::{IpAddr, ToSocketAddrs};
use std::path::Path;
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum UiMode {
    Default,
    Enhanced,
    Dashboard,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum JsonOutput {
    Compact,
    Pretty,
}

#[derive(Debug, Clone, Copy)]
pub struct EnhancedUiConfig {
    pub latency_warn_ms: f32,
    pub latency_bad_ms: f32,
    pub loss_warn_pct: f32,
    pub loss_bad_pct: f32,
    pub row_coloring: bool,
    pub sparklines: bool,
    pub summary: bool,
}

#[derive(Debug, Clone)]
pub struct ProbeRequest {
    pub host: String,
    pub tcp: bool,
    pub udp: bool,
    pub port: Option<u16>,
    pub source_port: Option<u16>,
    pub report: bool,
    pub json_output: Option<JsonOutput>,
    pub count: Option<usize>,
    pub interval_seconds: Option<f32>,
    pub timeout_seconds: Option<f32>,
    pub report_wide: bool,
    pub no_dns: bool,
    pub max_hops: Option<u8>,
    pub show_asn: bool,
    pub dns_lookup_as_info: bool,
    pub packet_size: Option<u16>,
    pub src: Option<IpAddr>,
    pub interface: Option<String>,
    pub ecmp: Option<String>,
    pub dns_cache_ttl_seconds: Option<u64>,
    pub trippy_flags: Option<String>,
    pub ui_mode: UiMode,
    pub enhanced_ui: EnhancedUiConfig,
    pub has_enhanced_overrides: bool,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ProbePlan {
    pub validated_host: String,
    pub trippy_args: Vec<String>,
    pub json_output: Option<JsonOutput>,
    pub ui_mode: UiMode,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ProbeResult {
    pub exit_code: i32,
}

#[derive(thiserror::Error, Debug, Clone, Eq, PartialEq)]
pub enum ProbeError {
    #[error("Failed to resolve hostname: {0}")]
    HostResolutionError(String),

    #[error(
        "Port option required for {0} protocol\n\nExample: windows-mtr.exe -{1} -P 443 8.8.8.8"
    )]
    PortRequired(String, char),

    #[error("Invalid command-line option: {0}")]
    InvalidOption(String),
}

pub fn validate_target(host: &str) -> Result<String, ProbeError> {
    match (host, 0).to_socket_addrs() {
        Ok(_) => Ok(host.to_string()),
        Err(_) => match host.parse::<IpAddr>() {
            Ok(_) => Ok(host.to_string()),
            Err(_) => Err(ProbeError::HostResolutionError(host.to_string())),
        },
    }
}

pub fn verify_options(request: &ProbeRequest) -> Result<(), ProbeError> {
    if (request.tcp || request.udp) && request.port.is_none() {
        let (protocol, flag) = if request.tcp {
            ("TCP", 'T')
        } else {
            ("UDP", 'U')
        };
        return Err(ProbeError::PortRequired(protocol.to_string(), flag));
    }

    if request.report_wide && !request.report && request.json_output.is_none() {
        return Err(ProbeError::InvalidOption(
            "-w/--report-wide requires -r/--report or --json output mode".to_string(),
        ));
    }

    if (request.ui_mode == UiMode::Enhanced || request.ui_mode == UiMode::Dashboard)
        && (request.report || request.json_output.is_some())
    {
        let ui_name = match request.ui_mode {
            UiMode::Enhanced => "enhanced",
            UiMode::Dashboard => "dashboard",
            UiMode::Default => "default",
        };

        return Err(ProbeError::InvalidOption(format!(
            "--ui {ui_name} is only supported in interactive TUI mode"
        )));
    }

    if request.has_enhanced_overrides && request.ui_mode != UiMode::Enhanced {
        return Err(ProbeError::InvalidOption(
            "enhanced UI tuning flags require --ui enhanced".to_string(),
        ));
    }

    if request.ui_mode == UiMode::Enhanced {
        return Err(ProbeError::InvalidOption(
            "enhanced UI is not available with bundled Trippy 0.13.0; use default UI or --ui dashboard fallback".to_string(),
        ));
    }

    if let Some(flags) = &request.trippy_flags {
        let parsed = parse_passthrough_flags(flags)?;

        if request.ui_mode == UiMode::Dashboard
            && parsed
                .iter()
                .any(|token| has_dashboard_conflicting_flag(token))
        {
            return Err(ProbeError::InvalidOption(
                "--trippy-flags cannot set --mode/--report-cycles/--tui-* in --ui dashboard"
                    .to_string(),
            ));
        }
    }

    Ok(())
}

pub fn build_probe_plan(request: &ProbeRequest) -> Result<ProbePlan, ProbeError> {
    verify_options(request)?;
    let validated_host = validate_target(&request.host)?;
    let trippy_args = build_embedded_trippy_args(request, &validated_host)?;

    Ok(ProbePlan {
        validated_host,
        trippy_args,
        json_output: request.json_output,
        ui_mode: request.ui_mode,
    })
}

fn mode_from_request(request: &ProbeRequest) -> &'static str {
    if request.json_output.is_some() {
        "json"
    } else if request.report || request.report_wide {
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

fn split_wrapped_passthrough_token(token: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut active_quote: Option<char> = None;

    for ch in token.chars() {
        if let Some(quote) = active_quote {
            if ch == quote {
                active_quote = None;
            } else {
                current.push(ch);
            }
            continue;
        }

        if ch.is_whitespace() {
            if !current.is_empty() {
                result.push(std::mem::take(&mut current));
            }
            continue;
        }

        if (ch == '\'' || ch == '"') && current.is_empty() {
            active_quote = Some(ch);
            continue;
        }

        current.push(ch);
    }

    if let Some(quote) = active_quote {
        current.insert(0, quote);
    }

    if !current.is_empty() {
        result.push(current);
    }

    result
}

pub fn parse_passthrough_flags(flags: &str) -> Result<Vec<String>, ProbeError> {
    let parsed = shlex::split(flags).ok_or_else(|| {
        ProbeError::InvalidOption("--trippy-flags contains invalid shell quoting".to_string())
    })?;

    if parsed.len() == 1 {
        let token = &parsed[0];
        if token.starts_with("--") && token.contains(' ') {
            return Ok(split_wrapped_passthrough_token(token));
        }
    }

    Ok(parsed)
}

fn has_dashboard_conflicting_flag(token: &str) -> bool {
    token == "--mode"
        || token == "--report-cycles"
        || token.starts_with("--tui-")
        || token == "--json"
        || token == "--json-pretty"
}

pub fn build_json_snapshot_args(
    request: &ProbeRequest,
    host: &str,
) -> Result<Vec<String>, ProbeError> {
    let mut trippy_args = vec![
        "mtr".to_string(),
        "--mode".to_string(),
        "json".to_string(),
        "--report-cycles".to_string(),
        "1".to_string(),
    ];

    if request.tcp {
        trippy_args.push("--tcp".to_string());
    } else if request.udp {
        trippy_args.push("--udp".to_string());
    }

    if let Some(port) = request.port {
        trippy_args.extend(["--target-port".to_string(), port.to_string()]);
    }

    if let Some(source_port) = request.source_port {
        trippy_args.extend(["--source-port".to_string(), source_port.to_string()]);
    }

    if let Some(interval) = request.interval_seconds {
        trippy_args.extend([
            "--min-round-duration".to_string(),
            duration_seconds(interval),
        ]);
    }

    if let Some(timeout) = request.timeout_seconds {
        trippy_args.extend(["--grace-duration".to_string(), duration_seconds(timeout)]);
    }

    if let Some(max_hops) = request.max_hops {
        trippy_args.extend(["--max-ttl".to_string(), max_hops.to_string()]);
    }

    if request.show_asn || request.dns_lookup_as_info {
        trippy_args.push("--dns-lookup-as-info".to_string());
    }

    if let Some(packet_size) = request.packet_size {
        trippy_args.extend(["--packet-size".to_string(), packet_size.to_string()]);
    }

    if let Some(src) = request.src {
        trippy_args.extend(["--source-address".to_string(), src.to_string()]);
    }

    if let Some(interface) = &request.interface {
        trippy_args.extend(["--interface".to_string(), interface.clone()]);
    }

    if let Some(ecmp) = &request.ecmp {
        trippy_args.extend(["--multipath-strategy".to_string(), ecmp.clone()]);
    }

    if let Some(ttl) = request.dns_cache_ttl_seconds {
        trippy_args.extend(["--dns-ttl".to_string(), format!("{ttl}s")]);
    }

    if let Some(extra) = &request.trippy_flags {
        let parsed = parse_passthrough_flags(extra)?;
        if let Some(conflict) = parsed
            .iter()
            .find(|token| has_dashboard_conflicting_flag(token))
        {
            return Err(ProbeError::InvalidOption(format!(
                "--trippy-flags contains `{conflict}`, which conflicts with --ui dashboard JSON snapshot mode"
            )));
        }
        trippy_args.extend(parsed);
    }

    trippy_args.push(host.to_string());
    Ok(trippy_args)
}

pub fn build_embedded_trippy_args(
    request: &ProbeRequest,
    host: &str,
) -> Result<Vec<String>, ProbeError> {
    let mut trippy_args = vec!["mtr".to_string()];

    trippy_args.extend(["--mode".to_string(), mode_from_request(request).to_string()]);

    if request.tcp {
        trippy_args.push("--tcp".to_string());
    } else if request.udp {
        trippy_args.push("--udp".to_string());
    }

    if let Some(port) = request.port {
        trippy_args.extend(["--target-port".to_string(), port.to_string()]);
    }

    if let Some(source_port) = request.source_port {
        trippy_args.extend(["--source-port".to_string(), source_port.to_string()]);
    }

    if let Some(count) = request.count {
        trippy_args.extend(["--report-cycles".to_string(), count.to_string()]);
    }

    if let Some(interval) = request.interval_seconds {
        trippy_args.extend([
            "--min-round-duration".to_string(),
            duration_seconds(interval),
        ]);
    }

    if let Some(timeout) = request.timeout_seconds {
        trippy_args.extend(["--grace-duration".to_string(), duration_seconds(timeout)]);
    }

    if request.no_dns {
        trippy_args.extend(["--tui-address-mode".to_string(), "ip".to_string()]);
    }

    if let Some(max_hops) = request.max_hops {
        trippy_args.extend(["--max-ttl".to_string(), max_hops.to_string()]);
    }

    if request.show_asn || request.dns_lookup_as_info {
        trippy_args.push("--dns-lookup-as-info".to_string());
    }

    if let Some(packet_size) = request.packet_size {
        trippy_args.extend(["--packet-size".to_string(), packet_size.to_string()]);
    }

    if let Some(src) = request.src {
        trippy_args.extend(["--source-address".to_string(), src.to_string()]);
    }

    if let Some(interface) = &request.interface {
        trippy_args.extend(["--interface".to_string(), interface.clone()]);
    }

    if let Some(ecmp) = &request.ecmp {
        trippy_args.extend(["--multipath-strategy".to_string(), ecmp.clone()]);
    }

    if let Some(ttl) = request.dns_cache_ttl_seconds {
        trippy_args.extend(["--dns-ttl".to_string(), format!("{ttl}s")]);
    }

    if let Some(extra) = &request.trippy_flags {
        trippy_args.extend(parse_passthrough_flags(extra)?);
    }

    trippy_args.push(host.to_string());
    Ok(trippy_args)
}

pub fn run_embedded_trippy(
    current_exe: &Path,
    args: &[String],
    json_output: Option<JsonOutput>,
    embedded_env_name: &str,
) -> anyhow::Result<ProbeResult> {
    if let Some(format) = json_output {
        let output = Command::new(current_exe)
            .env(embedded_env_name, "1")
            .args(args.iter().skip(1))
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .output()
            .context("failed to launch embedded trippy runner")?;

        if !output.status.success() {
            anyhow::bail!(
                "embedded trippy exited with status {}",
                output.status.code().unwrap_or(2)
            );
        }

        match format {
            JsonOutput::Compact => {
                let value: serde_json::Value = serde_json::from_slice(&output.stdout)
                    .context("failed to parse trippy JSON output")?;
                serde_json::to_writer(std::io::stdout(), &value)
                    .context("failed to write compact JSON output")?;
                std::io::stdout().write_all(b"\n")?;
            }
            JsonOutput::Pretty => {
                std::io::stdout().write_all(&output.stdout)?;
            }
        }

        return Ok(ProbeResult {
            exit_code: output.status.code().unwrap_or(2),
        });
    }

    let status = Command::new(current_exe)
        .env(embedded_env_name, "1")
        .args(args.iter().skip(1))
        .status()
        .context("failed to launch embedded trippy runner")?;
    Ok(ProbeResult {
        exit_code: status.code().unwrap_or(2),
    })
}
