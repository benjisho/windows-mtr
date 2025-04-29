use std::process::Command;

#[test]
fn test_help_output() {
    let output = Command::new("cargo")
        .args(["run", "--", "--help"])
        .output()
        .expect("Failed to execute command");
    
    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    
    assert!(stdout.contains("windows-mtr"));
    assert!(stdout.contains("Target host to trace"));
    assert!(stdout.contains("-T"));  // TCP option
    assert!(stdout.contains("-U"));  // UDP option
    assert!(stdout.contains("-P"));  // Port option
    assert!(stdout.contains("-r"));  // Report mode
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