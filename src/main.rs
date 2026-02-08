use anyhow::Context;
use clap::Parser;
use std::env;
use std::net::{IpAddr, ToSocketAddrs};
use std::path::PathBuf;
use std::process::{self, Command};

mod error;
use error::{MtrError, Result};

/// Windows-native clone of Linux mtr - a CLI that delivers ICMP/TCP/UDP traceroute & ping
#[derive(Parser)]
#[command(author = "Benji Shohet (benjisho)", version, about, long_about = None)]
#[command(after_help = "Examples:
  windows-mtr 8.8.8.8                    # Basic ICMP trace to Google DNS
  windows-mtr -T -P 443 github.com       # TCP trace to GitHub on port 443 (HTTPS)
  windows-mtr -U -P 53 1.1.1.1           # UDP trace to Cloudflare DNS on port 53
  windows-mtr -r -c 10 example.com       # Report mode with 10 pings per hop")]
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

    /// Report mode (no continuous updates)
    #[arg(short = 'r')]
    report: bool,

    /// Number of pings to send to each host
    #[arg(short = 'c')]
    count: Option<usize>,

    /// Time in seconds between ICMP ECHO requests
    #[arg(short = 'i')]
    interval: Option<f32>,

    /// Maximum time in seconds to keep a probe alive
    #[arg(short = 'w')]
    timeout: Option<f32>,

    /// Don't perform reverse DNS lookups (faster)
    #[arg(short = 'n')]
    no_dns: bool,

    /// Maximum number of hops to trace
    #[arg(short = 'm')]
    max_hops: Option<u8>,
}

fn print_banner() {
    println!("windows-mtr by Benji Shohet (benjisho) — https://github.com/benjisho/windows-mtr");
}

fn validate_target(host: &str) -> Result<String> {
    // Try to resolve the hostname to check if it's valid
    match (host, 0).to_socket_addrs() {
        Ok(_) => Ok(host.to_string()),
        Err(_) => {
            // Maybe it's an IP without a port, try parsing as IpAddr
            match host.parse::<IpAddr>() {
                Ok(_) => Ok(host.to_string()),
                Err(_) => Err(MtrError::HostResolutionError(host.to_string())),
            }
        }
    }
}

fn find_trippy_binary() -> Result<PathBuf> {
    // First use the 'which' crate to locate the trippy binary in PATH
    if let Ok(path) = which::which("trippy") {
        return Ok(path);
    }

    // On Windows, also try looking for trip.exe (the trippy executable name on Windows)
    #[cfg(windows)]
    if let Ok(path) = which::which("trip") {
        return Ok(path);
    }

    // Check if we're running from a bundled binary that has trippy embedded
    let exe_dir = env::current_exe()?
        .parent()
        .ok_or_else(|| MtrError::Other("Failed to get executable directory".to_string()))?
        .to_path_buf();

    // Check for trippy.exe first (fallback name)
    let local_trippy = exe_dir.join(if cfg!(windows) {
        "trippy.exe"
    } else {
        "trippy"
    });
    if local_trippy.exists() {
        return Ok(local_trippy);
    }

    // On Windows, also check for trip.exe (the actual name)
    #[cfg(windows)]
    {
        let local_trip = exe_dir.join("trip.exe");
        if local_trip.exists() {
            return Ok(local_trip);
        }
    }

    // Try common program files directory (Windows)
    #[cfg(windows)]
    {
        let program_files =
            env::var("ProgramFiles").unwrap_or_else(|_| "C:\\Program Files".to_string());
        let windows_mtr_dir = PathBuf::from(program_files).join("Windows-MTR");

        // Check for both possible filenames (trippy.exe and trip.exe)
        let program_files_trippy = windows_mtr_dir.join("trippy.exe");
        let program_files_trip = windows_mtr_dir.join("trip.exe");

        if program_files_trippy.exists() {
            return Ok(program_files_trippy);
        }

        if program_files_trip.exists() {
            return Ok(program_files_trip);
        }
    }

    // If we reach here, we could not find trippy/trip.
    // Avoid attempting implicit installation to keep behavior predictable in managed environments.
    Err(MtrError::TrippyNotFound)
}

fn verify_options(args: &Cli) -> Result<()> {
    // Verify port is provided for TCP and UDP modes
    if (args.tcp || args.udp) && args.port.is_none() {
        let (protocol, flag) = if args.tcp { ("TCP", 'T') } else { ("UDP", 'U') };
        return Err(MtrError::PortRequired(protocol.to_string(), flag));
    }

    Ok(())
}

fn main() -> anyhow::Result<()> {
    print_banner();

    // Parse command-line arguments
    let args = Cli::parse();

    // Verify options
    verify_options(&args)
        .map_err(|e| anyhow::anyhow!(e.to_string()))
        .context("invalid command-line options")?;

    // Validate target
    let host = validate_target(&args.host)
        .map_err(|e| anyhow::anyhow!(e.to_string()))
        .with_context(|| format!("invalid target host: {}", args.host))?;

    // Find the trippy binary
    let trippy_path = match find_trippy_binary() {
        Ok(path) => path,
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!("\nTo fix this issue, please try one of the following:");
            eprintln!("1. Place 'trippy.exe' in the same directory as this executable");
            eprintln!("2. Install trippy manually: cargo install trippy");
            eprintln!("3. Download the full release package from GitHub which includes trippy");
            eprintln!("   https://github.com/benjisho/windows-mtr/releases");
            return Err(anyhow::anyhow!("Trippy binary required"));
        }
    };

    // Start building the trippy command
    let mut cmd = Command::new(trippy_path);

    // Protocol options - pass them correctly to the trippy binary
    if args.tcp {
        cmd.arg("--protocol").arg("tcp");
    } else if args.udp {
        cmd.arg("--protocol").arg("udp");
    }

    // Port - pass it as a separate argument
    if let Some(port) = args.port {
        cmd.arg("--port").arg(port.to_string());
    }

    // Add the target host
    cmd.arg(host);

    // Report mode
    if args.report {
        cmd.arg("--report");
    }

    // Max pings
    if let Some(count) = args.count {
        cmd.arg("--max-rounds").arg(count.to_string());
    }

    // Interval
    if let Some(interval) = args.interval {
        cmd.arg("--interval").arg(interval.to_string());
    }

    // Timeout
    if let Some(timeout) = args.timeout {
        cmd.arg("--grace-duration").arg(timeout.to_string());
    }

    // DNS lookups
    if args.no_dns {
        cmd.arg("--no-dns");
    }

    // Max hops
    if let Some(max_hops) = args.max_hops {
        cmd.arg("--max-ttl").arg(max_hops.to_string());
    }

    // Execute trippy with our arguments and forward its exit status
    let output = cmd.output().with_context(|| {
        format!(
            "failed to execute trippy binary at {}",
            cmd.get_program().to_string_lossy()
        )
    })?;

    // Check if the error is related to privileges
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Check for privilege errors in trippy's output
        if stderr.contains("privileges are required") || stderr.contains("permission denied") {
            return Err(anyhow::anyhow!(MtrError::InsufficientPrivileges.to_string()));
        }

        // For other errors, just print stderr and return the status code
        if !stderr.is_empty() {
            eprintln!("{}", stderr);
        }
    }

    process::exit(output.status.code().unwrap_or(2));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_cli() -> Cli {
        Cli {
            host: "8.8.8.8".to_string(),
            tcp: false,
            udp: false,
            port: None,
            report: false,
            count: None,
            interval: None,
            timeout: None,
            no_dns: false,
            max_hops: None,
        }
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
    fn validate_target_accepts_ip_and_hostname() {
        assert!(validate_target("8.8.8.8").is_ok());
        assert!(validate_target("localhost").is_ok());
    }

    #[test]
    fn validate_target_rejects_invalid_target() {
        assert!(validate_target("invalid host with spaces").is_err());
    }
}
