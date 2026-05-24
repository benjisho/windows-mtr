use std::net::AddrParseError;
use thiserror::Error;

/// Custom error types for Windows MTR
#[derive(Error, Debug)]
pub enum MtrError {
    /// Error resolving hostname
    #[error("Failed to resolve hostname: {0}")]
    HostResolutionError(String),

    /// Invalid IP address
    #[error("Invalid IP address: {0}")]
    InvalidIpAddress(#[from] AddrParseError),

    /// Insufficient privileges (used in API error contract and diagnostic tests)
    #[allow(dead_code)]
    #[error(
        "Administrator privileges are required to run traceroute\n\nPlease right-click on the command prompt/terminal and select 'Run as administrator' before running windows-mtr."
    )]
    InsufficientPrivileges,

    /// Invalid command-line options
    #[error("Invalid command-line option: {0}")]
    InvalidOption(String),

    /// Invalid port for protocol
    #[error(
        "Port option required for {0} protocol\n\nExample: windows-mtr.exe -{1} -P 443 8.8.8.8"
    )]
    PortRequired(String, char),

    /// IO error
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// Catch-all for unexpected failures (used in API error contract)
    #[allow(dead_code)]
    #[error("Error: {0}")]
    Other(String),
}
