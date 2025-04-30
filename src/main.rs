use anyhow::Context;
use clap::Parser;
use std::env;
use std::net::{IpAddr, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::process::{self, Command};

mod error;
use error::{MtrError, Result};

/// Windows-native clone of Linux mtr - a CLI that delivers ICMP/TCP/UDP traceroute & ping
#[derive(Parser)]
#[command(author = "Benji Shohet (benjisho)", version, about, long_about = None)]
struct Cli {
    /// Target host to trace (hostname or IP)
    host: String,

    /// Use TCP SYN for probes (default is ICMP)
    #[arg(short = 'T', conflicts_with = "udp")]
    tcp: bool,

    /// Use UDP for probes (default is ICMP)
    #[arg(short = 'U', conflicts_with = "tcp")]
    udp: bool,

    /// Target port for TCP/UDP modes
    #[arg(short = 'P')]
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
        Err(e) => {
            // Maybe it's an IP without a port, try parsing as IpAddr
            match host.parse::<IpAddr>() {
                Ok(_) => Ok(host.to_string()),
                Err(_) => Err(MtrError::HostResolutionError(host.to_string()))
            }
        }
    }
}

fn find_trippy_binary() -> Result<PathBuf> {
    // First try to use trippy from PATH
    if let Ok(status) = Command::new("trippy").arg("--version").output() {
        if status.status.success() {
            return Ok(PathBuf::from("trippy"));
        }
    }
    
    // Try to find trippy in the cargo bin directory
    if let Some(home_dir) = dirs::home_dir() {
        let cargo_bin = home_dir.join(".cargo").join("bin");
        let trippy_path = cargo_bin.join(if cfg!(windows) { "trippy.exe" } else { "trippy" });
        
        if trippy_path.exists() {
            return Ok(trippy_path);
        }
    }
    
    // Check if we're running from a bundled binary that has trippy embedded
    let exe_dir = env::current_exe()
        .map_err(|e| MtrError::IoError(e))?
        .parent()
        .ok_or_else(|| MtrError::Other("Failed to get executable directory".to_string()))?
        .to_path_buf();
    
    let local_trippy = exe_dir.join(if cfg!(windows) { "trippy.exe" } else { "trippy" });
    
    if local_trippy.exists() {
        return Ok(local_trippy);
    }
    
    // If we can't find trippy, try installing it via cargo
    eprintln!("Trippy binary not found. Trying to install it with cargo...");
    
    let cargo_install_status = Command::new("cargo")
        .args(["install", "trippy"])
        .status()
        .map_err(|e| MtrError::TrippyInstallFailed(e.to_string()))?;
        
    if cargo_install_status.success() {
        // Try again with the newly installed trippy
        if let Some(home_dir) = dirs::home_dir() {
            let cargo_bin = home_dir.join(".cargo").join("bin");
            let trippy_path = cargo_bin.join(if cfg!(windows) { "trippy.exe" } else { "trippy" });
            
            if trippy_path.exists() {
                return Ok(trippy_path);
            }
        }
    }
    
    Err(MtrError::TrippyNotFound)
}

fn verify_options(args: &Cli) -> Result<()> {
    // Verify port is provided for TCP and UDP modes
    if (args.tcp || args.udp) && args.port.is_none() {
        let protocol = if args.tcp { "TCP" } else { "UDP" };
        return Err(MtrError::PortRequired(protocol.to_string()));
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
    let trippy_path = find_trippy_binary()
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    
    // Start building the trippy command
    let mut cmd = Command::new(trippy_path);
    cmd.arg(host);
    
    // Protocol options
    if args.tcp {
        cmd.arg("--tcp");
    } else if args.udp {
        cmd.arg("--udp");
    }
    
    // Port
    if let Some(port) = args.port {
        cmd.arg("--port").arg(port.to_string());
    }
    
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
    let status = cmd.status()
        .map_err(|e| anyhow::anyhow!("Failed to execute trippy: {}", e))?;
        
    process::exit(status.code().unwrap_or(2));
}
