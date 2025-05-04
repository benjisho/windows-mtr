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
    #[arg(short = 'P', value_name = "PORT")]
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
                Err(_) => Err(MtrError::HostResolutionError(host.to_string()))
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
        .map_err(|e| MtrError::IoError(e))?
        .parent()
        .ok_or_else(|| MtrError::Other("Failed to get executable directory".to_string()))?
        .to_path_buf();
    
    // Check for trippy.exe first (fallback name)
    let local_trippy = exe_dir.join(if cfg!(windows) { "trippy.exe" } else { "trippy" });
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
        let program_files = env::var("ProgramFiles").unwrap_or_else(|_| "C:\\Program Files".to_string());
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
    
    // If we can't find trippy, try installing it via cargo
    if Command::new("cargo").arg("--version").output().is_ok() {
        eprintln!("Trippy binary not found. Trying to install it with cargo...");
        
        let cargo_install_status = Command::new("cargo")
            .args(["install", "trippy"])
            .status()
            .map_err(|e| MtrError::TrippyInstallFailed(e.to_string()))?;
            
        if cargo_install_status.success() {
            // Try again with the 'which' crate to locate the newly installed trippy
            #[cfg(windows)]
            if let Ok(path) = which::which("trip") {
                return Ok(path);
            }
            
            #[cfg(not(windows))]
            if let Ok(path) = which::which("trippy") {
                return Ok(path);
            }
        }
    } else {
        eprintln!("Cargo not found. Cannot automatically install trippy.");
    }
    
    // If we reach here, we couldn't find or install trippy
    Err(MtrError::TrippyNotFound)
}

fn verify_options(args: &Cli) -> Result<()> {
    // Verify port is provided for TCP and UDP modes
    if (args.tcp || args.udp) && args.port.is_none() {
        let (protocol, flag) = if args.tcp { ("TCP", 'T') } else { ("UDP", 'U') };
        return Err(MtrError::PortRequired(protocol.to_string(), flag));
    }
    
    // Add more informative documentation for port parameter
    if let Some(port) = args.port {
        if port > 65535 {
            return Err(MtrError::InvalidOption(format!("Port number {} is out of range (must be 1-65535)", port)));
        }
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
    
    // In newer trippy versions, TCP/UDP port is specified as part of the protocol argument
    if args.tcp {
        if let Some(port) = args.port {
            // Use the format: --protocol tcp:<port>
            cmd.arg("--protocol").arg(format!("tcp:{}", port));
        } else {
            cmd.arg("--tcp");
        }
    } else if args.udp {
        if let Some(port) = args.port {
            // Use the format: --protocol udp:<port>
            cmd.arg("--protocol").arg(format!("udp:{}", port));
        } else {
            cmd.arg("--udp");
        }
    }
    
    // Add the target host last
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
    let output = cmd.output()
        .map_err(|e| anyhow::anyhow!("Failed to execute trippy: {}", e))?;
        
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
