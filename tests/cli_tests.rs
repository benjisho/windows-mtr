use std::process::Command;

#[allow(dead_code)]
#[path = "../src/error.rs"]
mod error;

use error::MtrError;

#[test]
fn test_help_output() {
    let output = Command::new("cargo")
        .args(["run", "--", "--help"])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");

    assert!(stdout.contains("windows-mtr"));
    assert!(stdout.contains("Target host to trace"));
    assert!(stdout.contains("-T")); // TCP option
    assert!(stdout.contains("-U")); // UDP option
    assert!(stdout.contains("-P")); // Port option
    assert!(stdout.contains("-r")); // Report mode
    assert!(stdout.contains("--ui")); // UI preset wrapper
    assert!(stdout.contains("--enhanced-sparklines")); // Enhanced hop trend toggle
}

#[test]
fn test_version_output() {
    let output = Command::new("cargo")
        .args(["run", "--", "--version"])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");

    assert!(stdout.contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn test_enhanced_ui_is_soft_disabled_with_actionable_error() {
    let output = Command::new("cargo")
        .args(["run", "--", "--ui", "enhanced", "8.8.8.8"])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());

    let stderr = String::from_utf8(output.stderr).expect("Invalid UTF-8");
    assert!(stderr.contains("enhanced UI is not available with bundled Trippy 0.13.0"));
}

#[test]
fn test_insufficient_privileges_diagnostic_contract_is_stable() {
    let error = MtrError::InsufficientPrivileges;
    let message = error.to_string();

    assert!(
        matches!(error, MtrError::InsufficientPrivileges),
        "Expected explicit InsufficientPrivileges error category to remain present",
    );
    assert!(
        message.contains("Administrator privileges are required to run traceroute"),
        "Expected insufficient privilege diagnostics to keep user-readable summary",
    );
    assert!(
        message.contains("Run as administrator"),
        "Expected insufficient privilege diagnostics to include actionable guidance",
    );
}

#[test]
fn test_tcp_without_port_exits_with_error_and_actionable_guidance() {
    let output = Command::new("cargo")
        .args(["run", "--", "-T", "8.8.8.8"])
        .output()
        .expect("Failed to execute command");

    assert!(
        !output.status.success(),
        "Expected non-zero exit for TCP mode without required port",
    );

    let stderr = String::from_utf8(output.stderr).expect("Invalid UTF-8");
    assert!(
        stderr.contains("Port option required for TCP protocol"),
        "Expected protocol-specific error instead of silent fallback to another probe mode",
    );
    assert!(
        stderr.contains("Example: windows-mtr.exe -T -P 443 8.8.8.8"),
        "Expected actionable guidance for TCP privilege/usage diagnostics",
    );
}

#[test]
fn test_udp_without_port_exits_with_error_and_actionable_guidance() {
    let output = Command::new("cargo")
        .args(["run", "--", "-U", "8.8.8.8"])
        .output()
        .expect("Failed to execute command");

    assert!(
        !output.status.success(),
        "Expected non-zero exit for UDP mode without required port",
    );

    let stderr = String::from_utf8(output.stderr).expect("Invalid UTF-8");
    assert!(
        stderr.contains("Port option required for UDP protocol"),
        "Expected protocol-specific error instead of silent fallback to another probe mode",
    );
    assert!(
        stderr.contains("Example: windows-mtr.exe -U -P 443 8.8.8.8"),
        "Expected actionable guidance for UDP privilege/usage diagnostics",
    );
}

// This test would normally be run with #[ignore] as it requires network access
// and would be part of an integration test suite rather than unit tests
#[test]
#[ignore]
fn test_basic_execution() {
    let output = Command::new("cargo")
        .args(["run", "--", "localhost", "-c", "1", "-r"])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");

    // Check for banner
    assert!(stdout.contains("windows-mtr by Benji Shohet"));

    // Check for expected output format in report mode
    assert!(stdout.contains("Host"));
    assert!(stdout.contains("Loss%"));
}
