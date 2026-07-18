#![cfg(windows)]

use std::process::Command;

fn run_mtr(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_mtr"))
        .args(args)
        .output()
        .expect("failed to run windows-mtr test binary")
}

#[test]
fn native_icmp_report_reaches_ipv4_loopback() {
    let output = run_mtr(&[
        "-n",
        "-r",
        "-c",
        "1",
        "-m",
        "1",
        "--timeout",
        "1",
        "127.0.0.1",
    ]);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("report must be UTF-8");
    assert!(stdout.contains("windows-mtr ICMP report for 127.0.0.1"));
    assert!(stdout.contains("Hop  Host"));
    assert!(stdout.contains("127.0.0.1"));
}

#[test]
fn native_icmp_json_reports_ipv4_loopback() {
    let output = run_mtr(&[
        "-n",
        "--json",
        "-c",
        "1",
        "-m",
        "1",
        "--timeout",
        "1",
        "127.0.0.1",
    ]);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("valid JSON report");
    assert_eq!(report["report"]["protocol"], "icmp");
    assert_eq!(report["report"]["backend"], "windows-icmp-helper");
    assert_eq!(report["report"]["hops"][0]["host"], "127.0.0.1");
}

#[test]
fn native_icmp_csv_reports_ipv4_loopback() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let csv_path = directory.path().join("report.csv");
    let output = run_mtr(&[
        "-n",
        "--csv",
        csv_path.to_str().expect("UTF-8 temporary path"),
        "-c",
        "1",
        "-m",
        "1",
        "--timeout",
        "1",
        "127.0.0.1",
    ]);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let csv = std::fs::read_to_string(csv_path).expect("CSV report must be written");
    assert!(csv.starts_with("hop,ip,hostname,avg_ms,best_ms,worst_ms,loss_pct"));
    assert!(csv.contains("1,127.0.0.1"));
}
