use std::process::Command;
use std::{fs, path::Path};

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
fn test_enhanced_ui_conflicts_with_report_mode() {
    let output = Command::new("cargo")
        .args(["run", "--", "--ui", "enhanced", "-r", "8.8.8.8"])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());

    let stderr = String::from_utf8(output.stderr).expect("Invalid UTF-8");
    assert!(stderr.contains("--ui enhanced is only supported in interactive TUI mode"));
}

#[test]
fn test_enhanced_wrapper_conflicts_with_passthrough_override() {
    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "--ui",
            "enhanced",
            "--trippy-flags",
            "--tui-summary-percentiles false",
            "8.8.8.8",
        ])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());

    let stderr = String::from_utf8(output.stderr).expect("Invalid UTF-8");
    assert!(
        stderr.contains("--trippy-flags cannot override windows-mtr enhanced UI wrapper settings")
    );
}

#[test]
fn test_insufficient_privileges_diagnostic_contract_is_stable() {
    let error_rs = fs::read_to_string(Path::new("src/error.rs"))
        .expect("should be able to read src/error.rs for contract assertions");

    assert!(
        error_rs.contains("InsufficientPrivileges"),
        "Expected explicit InsufficientPrivileges error category to remain present",
    );
    assert!(
        error_rs.contains("Administrator privileges are required to run traceroute"),
        "Expected insufficient privilege diagnostics to keep user-readable summary",
    );
    assert!(
        error_rs.contains("Run as administrator"),
        "Expected insufficient privilege diagnostics to include actionable guidance",
    );
}

#[test]
fn test_icmp_path_does_not_silently_fallback_to_tcp_or_udp() {
    let main_rs = fs::read_to_string(Path::new("src/main.rs"))
        .expect("should be able to read src/main.rs for transport guardrail assertions");

    assert!(
        main_rs.contains("if args.tcp {") && main_rs.contains("} else if args.udp {"),
        "Expected transport selection to remain explicit and mutually-exclusive",
    );
    assert!(
        main_rs.contains("trippy_args.push(\"--tcp\".to_string());")
            && main_rs.contains("trippy_args.push(\"--udp\".to_string());"),
        "Expected TCP/UDP probes to be opt-in only",
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
