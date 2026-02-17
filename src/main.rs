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
    let exe_dir = env::current_exe()
        .map_err(MtrError::IoError)?
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
    verify_options(&args).map_err(|e| anyhow::anyhow!(e.to_string()))?;
    
    // Validate target
    let host = validate_target(&args.host)
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    
    // Find the trippy binary
    let trippy_path = match find_trippy_binary() {
        Ok(path) => path,
        Err(e) => {
            eprintln!("Error: {e}");
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
        trippy_args.extend(["--protocol".to_string(), "tcp".to_string()]);
    } else if args.udp {
        trippy_args.extend(["--protocol".to_string(), "udp".to_string()]);
    }

    // Port - pass it as a separate argument
    if let Some(port) = args.port {
        trippy_args.extend(["--port".to_string(), port.to_string()]);
    }

    // Add the target host
    trippy_args.push(host.to_string());

    // Report mode
    if args.report {
        trippy_args.push("--report".to_string());
    }

    // Max pings
    if let Some(count) = args.count {
        trippy_args.extend(["--max-rounds".to_string(), count.to_string()]);
    }

    // Interval
    if let Some(interval) = args.interval {
        trippy_args.extend(["--interval".to_string(), interval.to_string()]);
    }

    // Timeout
    if let Some(timeout) = args.timeout {
        trippy_args.extend(["--grace-duration".to_string(), timeout.to_string()]);
    }

    // DNS lookups
    if args.no_dns {
        trippy_args.push("--no-dns".to_string());
    }

    // Max hops
    if let Some(max_hops) = args.max_hops {
        trippy_args.extend(["--max-ttl".to_string(), max_hops.to_string()]);
    }

    trippy_args
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

    let trippy_args = build_trippy_args(&args, &host);
    cmd.args(&trippy_args);

    // Execute trippy with our arguments and forward its exit status
    let output = cmd.output()
        .map_err(|e| anyhow::anyhow!("Failed to execute trippy: {e}"))?;
        
    // Check if the error is related to privileges
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Check for privilege errors in trippy's output
        if stderr.contains("privileges are required") || stderr.contains("permission denied") {
            return Err(anyhow::anyhow!(MtrError::InsufficientPrivileges.to_string()));
        }

        // For other errors, just print stderr and return the status code
        if !stderr.is_empty() {
            eprintln!("{stderr}");
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

    #[test]
    fn build_trippy_args_maps_cli_flags_to_expected_trippy_args() {
        let args = Cli {
            host: "example.com".to_string(),
            tcp: true,
            udp: false,
            port: Some(443),
            report: true,
            count: Some(10),
            interval: Some(0.5),
            timeout: Some(3.0),
            no_dns: true,
            max_hops: Some(20),
        };

        let trippy_args = build_trippy_args(&args, "example.com");
        assert_eq!(
            trippy_args,
            vec![
                "--protocol",
                "tcp",
                "--port",
                "443",
                "example.com",
                "--report",
                "--max-rounds",
                "10",
                "--interval",
                "0.5",
                "--grace-duration",
                "3",
                "--no-dns",
                "--max-ttl",
                "20"
            ]
        );
    }

    #[test]
    fn build_trippy_args_keeps_default_icmp_behavior_without_port() {
        let args = base_cli();
        let trippy_args = build_trippy_args(&args, "8.8.8.8");
        assert_eq!(trippy_args, vec!["8.8.8.8"]);
    }
}
