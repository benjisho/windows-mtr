use anyhow::Context;
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::env;
use std::io::Write;
use std::net::{IpAddr, ToSocketAddrs};
use std::process::{self, Command, Stdio};
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

mod error;
mod native_ui;
use error::{MtrError, Result};

const EMBEDDED_TRIPPY_ENV: &str = "WINDOWS_MTR_EMBEDDED_TRIPPY";

/// Windows-native clone of Linux mtr - a CLI that delivers ICMP/TCP/UDP traceroute & ping
#[derive(Parser, Debug, Clone)]
#[command(author = "Benji Shohet (benjisho)", version, about, long_about = None)]
#[command(after_help = "Examples:
  windows-mtr 8.8.8.8                           # Basic ICMP trace to Google DNS
  windows-mtr -T -P 443 github.com              # TCP trace to GitHub on port 443 (HTTPS)
  windows-mtr -U -P 53 1.1.1.1                  # UDP trace to Cloudflare DNS on port 53
  windows-mtr -r -c 10 example.com              # Report mode with 10 pings per hop
  windows-mtr --json -c 20 example.com          # JSON report output
  windows-mtr --rest-api --rest-api-bind 127.0.0.1:8080 # Start REST API server
  windows-mtr --native-ui 8.8.8.8               # Launch native Ratatui UI preview
  windows-mtr --trippy-flags '--tui-refresh-rate 150ms' example.com")]
struct Cli {
    /// Target host to trace (hostname or IP)
    #[arg(required_unless_present = "rest_api")]
    host: Option<String>,

    /// Start HTTP REST API server mode
    #[arg(long = "rest-api")]
    rest_api: bool,

    /// Bind address for REST API mode
    #[arg(long = "rest-api-bind", default_value = "127.0.0.1:8080")]
    rest_api_bind: String,

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
    #[arg(long = "trippy-flags", value_name = "FLAGS")]
    trippy_flags: Option<String>,

    /// Launch native Ratatui UI (tabs, hop table, charts)
    #[arg(long = "native-ui", conflicts_with_all = ["rest_api", "report", "json", "json_pretty"])]
    native_ui: bool,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum JsonFormat {
    Compact,
    Pretty,
}

#[derive(Debug, Deserialize)]
struct ApiTraceRequest {
    host: String,
    #[serde(default)]
    tcp: bool,
    #[serde(default)]
    udp: bool,
    port: Option<u16>,
    source_port: Option<u16>,
    count: Option<usize>,
    interval: Option<f32>,
    timeout: Option<f32>,
    #[serde(default)]
    report_wide: bool,
    #[serde(default)]
    no_dns: bool,
    max_hops: Option<u8>,
    #[serde(default)]
    show_asn: bool,
    #[serde(default)]
    dns_lookup_as_info: bool,
    packet_size: Option<u16>,
    src: Option<IpAddr>,
    interface: Option<String>,
    ecmp: Option<String>,
    dns_cache_ttl: Option<u64>,
    trippy_flags: Option<String>,
    #[serde(default)]
    pretty: bool,
}

#[derive(Debug, Serialize)]
struct ApiErrorResponse<'a> {
    error: &'a str,
}

fn json_format_from_args(args: &Cli) -> Option<JsonFormat> {
    if args.json {
        Some(JsonFormat::Compact)
    } else if args.json_pretty {
        Some(JsonFormat::Pretty)
    } else {
        None
    }
}

fn should_print_banner(args: &Cli) -> bool {
    json_format_from_args(args).is_none() && !args.rest_api && !args.native_ui
}

fn print_banner() {
    println!("windows-mtr by Benji Shohet (benjisho) — https://github.com/benjisho/windows-mtr");
}

fn validate_target(host: &str) -> Result<String> {
    match (host, 0).to_socket_addrs() {
        Ok(_) => Ok(host.to_string()),
        Err(_) => match host.parse::<IpAddr>() {
            Ok(_) => Ok(host.to_string()),
            Err(_) => Err(MtrError::HostResolutionError(host.to_string())),
        },
    }
}

fn verify_options(args: &Cli) -> Result<()> {
    if !args.rest_api && args.host.is_none() {
        return Err(MtrError::InvalidOption(
            "host is required unless --rest-api is enabled".to_string(),
        ));
    }

    if (args.tcp || args.udp) && args.port.is_none() {
        let (protocol, flag) = if args.tcp { ("TCP", 'T') } else { ("UDP", 'U') };
        return Err(MtrError::PortRequired(protocol.to_string(), flag));
    }

    if args.report_wide && !args.report && !args.json && !args.json_pretty {
        return Err(MtrError::InvalidOption(
            "-w/--report-wide requires -r/--report or --json output mode".to_string(),
        ));
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

fn parse_passthrough_flags(flags: &str) -> Result<Vec<String>> {
    let parsed = shlex::split(flags).ok_or_else(|| {
        MtrError::InvalidOption("--trippy-flags contains invalid shell quoting".to_string())
    })?;

    if parsed.len() == 1 {
        let token = &parsed[0];
        if token.starts_with("--") && token.contains(' ') {
            return Ok(split_wrapped_passthrough_token(token));
        }
    }

    Ok(parsed)
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

    if let Some(extra) = &args.trippy_flags {
        trippy_args.extend(parse_passthrough_flags(extra)?);
    }

    trippy_args.push(host.to_string());
    Ok(trippy_args)
}

fn run_embedded_trippy(args: &[String], json_format: Option<JsonFormat>) -> anyhow::Result<i32> {
    let current_exe = env::current_exe().context("failed to locate current executable")?;

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

fn execute_report_json(
    cli: &Cli,
    host: &str,
    pretty: bool,
) -> anyhow::Result<(i32, serde_json::Value)> {
    let mut run_args = cli.clone();
    run_args.json = !pretty;
    run_args.json_pretty = pretty;
    run_args.report = false;

    let trippy_args = build_embedded_trippy_args(&run_args, host)
        .map_err(|e| anyhow::anyhow!(e.to_string()))
        .context("failed to translate windows-mtr options into trippy options")?;

    let current_exe = env::current_exe().context("failed to locate current executable")?;
    let output = Command::new(&current_exe)
        .env(EMBEDDED_TRIPPY_ENV, "1")
        .args(trippy_args.iter().skip(1))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .context("failed to launch embedded trippy runner")?;

    let code = output.status.code().unwrap_or(2);
    let json: serde_json::Value = serde_json::from_slice(&output.stdout)
        .context("failed to parse trippy JSON output for REST response")?;
    Ok((code, json))
}

fn api_request_to_cli(request: ApiTraceRequest) -> Cli {
    Cli {
        host: Some(request.host),
        rest_api: false,
        rest_api_bind: "127.0.0.1:8080".to_string(),
        tcp: request.tcp,
        udp: request.udp,
        port: request.port,
        source_port: request.source_port,
        report: false,
        json: !request.pretty,
        json_pretty: request.pretty,
        count: request.count,
        interval: request.interval,
        timeout: request.timeout,
        report_wide: request.report_wide,
        no_dns: request.no_dns,
        max_hops: request.max_hops,
        show_asn: request.show_asn,
        dns_lookup_as_info: request.dns_lookup_as_info,
        packet_size: request.packet_size,
        src: request.src,
        interface: request.interface,
        ecmp: request.ecmp,
        dns_cache_ttl: request.dns_cache_ttl,
        trippy_flags: request.trippy_flags,
        native_ui: false,
    }
}

fn content_type_header(value: &str) -> Header {
    Header::from_bytes(&b"Content-Type"[..], value.as_bytes()).expect("valid header")
}

fn respond_json(request: Request, status: u16, value: &serde_json::Value) {
    let body = value.to_string();
    let response = Response::from_string(body)
        .with_status_code(StatusCode(status))
        .with_header(content_type_header("application/json"));
    let _ = request.respond(response);
}

fn respond_json_error(request: Request, status: u16, message: &str) {
    let payload = serde_json::to_value(ApiErrorResponse { error: message }).unwrap_or_else(
        |_| serde_json::json!({"error": "internal response serialization failure"}),
    );
    respond_json(request, status, &payload);
}

fn handle_rest_request(mut request: Request) {
    let method = request.method().clone();
    let path = request.url().to_string();

    if method == Method::Get && path == "/health" {
        respond_json(request, 200, &serde_json::json!({"status": "ok"}));
        return;
    }

    if method == Method::Post && path == "/v1/report" {
        let mut body = String::new();
        if let Err(err) = request.as_reader().read_to_string(&mut body) {
            respond_json_error(request, 400, &format!("failed to read request body: {err}"));
            return;
        }

        let parsed: ApiTraceRequest = match serde_json::from_str(&body) {
            Ok(value) => value,
            Err(err) => {
                respond_json_error(request, 400, &format!("invalid JSON body: {err}"));
                return;
            }
        };

        let cli = api_request_to_cli(parsed);
        if let Err(err) = verify_options(&cli) {
            respond_json_error(request, 400, &err.to_string());
            return;
        }

        let Some(host_ref) = cli.host.as_deref() else {
            respond_json_error(request, 400, "host is required");
            return;
        };

        let host = match validate_target(host_ref) {
            Ok(valid) => valid,
            Err(err) => {
                respond_json_error(request, 400, &err.to_string());
                return;
            }
        };

        match execute_report_json(&cli, &host, cli.json_pretty) {
            Ok((exit_code, report)) => {
                respond_json(
                    request,
                    200,
                    &serde_json::json!({
                        "exit_code": exit_code,
                        "target": host,
                        "report": report
                    }),
                );
            }
            Err(err) => {
                respond_json_error(request, 500, &format!("trace execution failed: {err}"));
            }
        }
        return;
    }

    respond_json_error(request, 404, "route not found");
}

fn run_rest_api(bind_addr: &str) -> anyhow::Result<()> {
    let server = Server::http(bind_addr)
        .map_err(|e| anyhow::anyhow!("failed to bind REST API server on {bind_addr}: {e}"))?;

    eprintln!("windows-mtr REST API listening on http://{bind_addr}");
    for request in server.incoming_requests() {
        handle_rest_request(request);
    }

    Ok(())
}

fn main() -> anyhow::Result<()> {
    if env::var_os(EMBEDDED_TRIPPY_ENV).is_some() {
        return trippy_tui::trippy();
    }

    let args = Cli::parse();

    if should_print_banner(&args) {
        print_banner();
    }

    verify_options(&args)
        .map_err(|e| anyhow::anyhow!(e.to_string()))
        .context("invalid command-line options")?;

    if args.rest_api {
        return run_rest_api(&args.rest_api_bind);
    }

    if args.native_ui {
        let host = args
            .host
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("host is required for --native-ui"))?;
        return native_ui::run(host);
    }

    let host_input = args
        .host
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("host is required"))?;

    let host = validate_target(host_input)
        .map_err(|e| anyhow::anyhow!(e.to_string()))
        .with_context(|| format!("invalid target host: {host_input}"))?;

    let trippy_args = build_embedded_trippy_args(&args, &host)
        .map_err(|e| anyhow::anyhow!(e.to_string()))
        .context("failed to translate windows-mtr options into trippy options")?;

    let code = run_embedded_trippy(&trippy_args, json_format_from_args(&args))?;
    process::exit(code);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_cli() -> Cli {
        Cli {
            host: Some("8.8.8.8".to_string()),
            rest_api: false,
            rest_api_bind: "127.0.0.1:8080".to_string(),
            tcp: false,
            udp: false,
            port: None,
            source_port: None,
            report: false,
            json: false,
            json_pretty: false,
            count: None,
            interval: None,
            timeout: None,
            report_wide: false,
            no_dns: false,
            max_hops: None,
            show_asn: false,
            dns_lookup_as_info: false,
            packet_size: None,
            src: None,
            interface: None,
            ecmp: None,
            dns_cache_ttl: None,
            trippy_flags: None,
            native_ui: false,
        }
    }

    #[test]
    fn verify_options_requires_host_when_not_rest_api() {
        let mut args = base_cli();
        args.host = None;
        assert!(matches!(
            verify_options(&args),
            Err(MtrError::InvalidOption(_))
        ));
    }

    #[test]
    fn verify_options_allows_missing_host_in_rest_api_mode() {
        let mut args = base_cli();
        args.host = None;
        args.rest_api = true;
        assert!(verify_options(&args).is_ok());
    }

    #[test]
    fn verify_options_requires_port_for_tcp_udp() {
        let mut tcp = base_cli();
        tcp.tcp = true;
        assert!(matches!(
            verify_options(&tcp),
            Err(MtrError::PortRequired(_, 'T'))
        ));

        let mut udp = base_cli();
        udp.udp = true;
        assert!(matches!(
            verify_options(&udp),
            Err(MtrError::PortRequired(_, 'U'))
        ));
    }

    #[test]
    fn verify_options_accepts_valid_ports() {
        for port in [1, 443, 65535] {
            let mut args = base_cli();
            args.tcp = true;
            args.port = Some(port);
            assert!(verify_options(&args).is_ok());
        }
    }

    #[test]
    fn verify_options_rejects_report_wide_without_report_mode() {
        let mut args = base_cli();
        args.report_wide = true;
        assert!(matches!(
            verify_options(&args),
            Err(MtrError::InvalidOption(_))
        ));
    }

    #[test]
    fn validate_target_accepts_ip_and_hostname() {
        assert!(validate_target("8.8.8.8").is_ok());
        assert!(validate_target("localhost").is_ok());
    }

    #[test]
    fn validate_target_rejects_invalid_target() {
        assert!(validate_target("invalid host with spaces").is_err());
    }

    #[test]
    fn parse_passthrough_flags_splits_single_quoted_token() {
        let parsed =
            parse_passthrough_flags("\"--tui-refresh-rate 150ms\"").expect("flags should parse");
        assert_eq!(parsed, vec!["--tui-refresh-rate", "150ms"]);
    }

    #[test]
    fn parse_passthrough_flags_preserves_inner_quoted_values() {
        let parsed = parse_passthrough_flags("\"--log-filter 'warn info' --verbose\"")
            .expect("flags should parse");
        assert_eq!(parsed, vec!["--log-filter", "warn info", "--verbose"]);
    }

    #[test]
    fn parse_passthrough_flags_allows_literal_apostrophe_values() {
        let parsed =
            parse_passthrough_flags("\"--interface O'Reilly\"").expect("flags should parse");
        assert_eq!(parsed, vec!["--interface", "O'Reilly"]);
    }

    #[test]
    fn parse_passthrough_flags_rejects_invalid_shell_quoting() {
        assert!(matches!(
            parse_passthrough_flags("--foo 'bar"),
            Err(MtrError::InvalidOption(_))
        ));
    }

    #[test]
    fn build_embedded_trippy_args_maps_core_flags() {
        let args = Cli {
            host: Some("example.com".to_string()),
            rest_api: false,
            rest_api_bind: "127.0.0.1:8080".to_string(),
            tcp: true,
            udp: false,
            port: Some(443),
            source_port: Some(50000),
            report: true,
            json: false,
            json_pretty: false,
            count: Some(10),
            interval: Some(0.5),
            timeout: Some(3.0),
            report_wide: true,
            no_dns: true,
            max_hops: Some(20),
            show_asn: true,
            dns_lookup_as_info: false,
            packet_size: Some(128),
            src: Some("192.0.2.2".parse().expect("valid test ip")),
            interface: Some("Ethernet".to_string()),
            ecmp: Some("paris".to_string()),
            dns_cache_ttl: Some(120),
            trippy_flags: Some("--log-format json --verbose".to_string()),
            native_ui: false,
        };

        let trippy_args =
            build_embedded_trippy_args(&args, "example.com").expect("args should build");
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
        let mut args = base_cli();
        args.json = true;

        let trippy_args = build_embedded_trippy_args(&args, "8.8.8.8").expect("args should build");
        assert_eq!(trippy_args, vec!["mtr", "--mode", "json", "8.8.8.8"]);
    }

    #[test]
    fn api_request_to_cli_enables_json_mode() {
        let request = ApiTraceRequest {
            host: "1.1.1.1".to_string(),
            tcp: true,
            udp: false,
            port: Some(443),
            source_port: None,
            count: Some(3),
            interval: None,
            timeout: None,
            report_wide: false,
            no_dns: true,
            max_hops: None,
            show_asn: false,
            dns_lookup_as_info: false,
            packet_size: None,
            src: None,
            interface: None,
            ecmp: None,
            dns_cache_ttl: None,
            trippy_flags: None,
            pretty: false,
        };

        let cli = api_request_to_cli(request);
        assert!(cli.json);
        assert!(!cli.report);
        assert_eq!(cli.host.as_deref(), Some("1.1.1.1"));
    }

    #[test]
    fn should_not_print_banner_for_json_modes() {
        let mut args = base_cli();
        args.json = true;
        assert!(!should_print_banner(&args));

        let mut args = base_cli();
        args.json_pretty = true;
        assert!(!should_print_banner(&args));
    }

    #[test]
    fn should_not_print_banner_for_rest_api_mode() {
        let mut args = base_cli();
        args.rest_api = true;
        assert!(!should_print_banner(&args));
    }

    #[test]
    fn should_not_print_banner_for_native_ui_mode() {
        let mut args = base_cli();
        args.native_ui = true;
        assert!(!should_print_banner(&args));
    }

    #[test]
    fn should_print_banner_for_non_json_modes() {
        let args = base_cli();
        assert!(should_print_banner(&args));
    }
}
