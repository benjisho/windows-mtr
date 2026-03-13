use serde_json::Value;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Debug)]
struct Case {
    id: String,
    os: String,
    privilege: String,
    probe_mode: String,
    args: Vec<String>,
    outcome: String,
    exit_code: i32,
    failure_class: Option<String>,
    stderr_contains: Vec<String>,
    enforce_in_ci: bool,
}

#[derive(Debug)]
struct CaseResult {
    id: String,
    os: String,
    privilege: String,
    probe_mode: String,
    enforced: bool,
    status: &'static str,
    note: String,
}

fn current_os_label() -> &'static str {
    if cfg!(windows) { "windows" } else { "linux" }
}

fn get_str<'a>(row: &'a Value, key: &str) -> &'a str {
    row[key]
        .as_str()
        .unwrap_or_else(|| panic!("missing or invalid string field: {key}"))
}

fn get_bool(row: &Value, key: &str) -> bool {
    row[key]
        .as_bool()
        .unwrap_or_else(|| panic!("missing or invalid bool field: {key}"))
}

fn parse_cases(path: &Path) -> Vec<Case> {
    let raw = fs::read_to_string(path).expect("failed to read parity fixture");
    let value: Value = serde_json::from_str(&raw).expect("failed to parse parity fixture json");
    let rows = value
        .as_array()
        .expect("parity fixture root must be a JSON array");

    rows.iter()
        .map(|row| {
            let expected = &row["expected"];
            let exit_code: i32 = expected["exit_code"]
                .as_i64()
                .expect("missing expected.exit_code")
                .try_into()
                .expect("expected.exit_code must fit i32");

            Case {
                id: get_str(row, "id").to_string(),
                os: get_str(row, "os").to_string(),
                privilege: get_str(row, "privilege").to_string(),
                probe_mode: get_str(row, "probe_mode").to_string(),
                args: row["args"]
                    .as_array()
                    .expect("missing args")
                    .iter()
                    .map(|arg| arg.as_str().expect("args must be strings").to_string())
                    .collect(),
                outcome: expected["outcome"]
                    .as_str()
                    .expect("missing expected.outcome")
                    .to_string(),
                exit_code,
                failure_class: expected["failure_class"].as_str().map(ToString::to_string),
                stderr_contains: expected["stderr_contains"]
                    .as_array()
                    .map(|items| {
                        items
                            .iter()
                            .map(|item| {
                                item.as_str()
                                    .expect("stderr_contains values must be strings")
                            })
                            .map(ToString::to_string)
                            .collect()
                    })
                    .unwrap_or_default(),
                enforce_in_ci: get_bool(row, "enforce_in_ci"),
            }
        })
        .collect()
}

fn validate_matrix(cases: &[Case]) {
    assert!(!cases.is_empty(), "parity fixture must not be empty");

    let mut seen_axes = HashSet::new();
    let mut seen_ids = HashSet::new();

    for case in cases {
        assert!(
            matches!(case.os.as_str(), "linux" | "windows"),
            "invalid os for {}: {}",
            case.id,
            case.os
        );
        assert!(
            matches!(case.privilege.as_str(), "unprivileged" | "elevated"),
            "invalid privilege for {}: {}",
            case.id,
            case.privilege
        );
        assert!(
            matches!(case.probe_mode.as_str(), "icmp" | "tcp" | "udp"),
            "invalid probe_mode for {}: {}",
            case.id,
            case.probe_mode
        );
        assert!(
            matches!(case.outcome.as_str(), "success" | "failure"),
            "invalid outcome for {}: {}",
            case.id,
            case.outcome
        );
        assert!(
            seen_ids.insert(case.id.clone()),
            "duplicate id row found: {}",
            case.id
        );

        let expected_prefix = format!("{}-{}-{}-", case.os, case.privilege, case.probe_mode);
        assert!(
            case.id.starts_with(&expected_prefix),
            "row id must start with axis prefix '{}': {}",
            expected_prefix,
            case.id
        );

        let axis = format!("{}|{}|{}", case.os, case.privilege, case.probe_mode);
        assert!(
            seen_axes.insert(axis),
            "duplicate axis row found: {}",
            case.id
        );
    }

    let expected_axes = ["linux", "windows"]
        .into_iter()
        .flat_map(|os| {
            ["unprivileged", "elevated"]
                .into_iter()
                .flat_map(move |privilege| {
                    ["icmp", "tcp", "udp"]
                        .into_iter()
                        .map(move |probe| format!("{os}|{privilege}|{probe}"))
                })
        })
        .collect::<HashSet<_>>();

    assert_eq!(
        seen_axes, expected_axes,
        "parity fixture must include every OS × privilege × probe_mode row exactly once"
    );
}

fn run_case(case: &Case) -> CaseResult {
    let output = Command::new(env!("CARGO_BIN_EXE_mtr"))
        .args(&case.args)
        .output()
        .expect("failed to execute mtr binary");

    let exit_code = output.status.code().unwrap_or(-1);
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    let mut failures = Vec::new();

    if exit_code != case.exit_code {
        failures.push(format!(
            "exit code mismatch: expected {}, got {}",
            case.exit_code, exit_code
        ));
    }

    match case.outcome.as_str() {
        "success" if !output.status.success() => {
            failures.push("expected success status but command failed".to_string())
        }
        "failure" if output.status.success() => {
            failures.push("expected failure status but command succeeded".to_string())
        }
        _ => {}
    }

    for needle in &case.stderr_contains {
        if !stderr.contains(needle) {
            failures.push(format!("stderr missing expected substring: {needle}"));
        }
    }

    let failure_class = case
        .failure_class
        .clone()
        .unwrap_or_else(|| "n/a".to_string());

    if failures.is_empty() {
        CaseResult {
            id: case.id.clone(),
            os: case.os.clone(),
            privilege: case.privilege.clone(),
            probe_mode: case.probe_mode.clone(),
            enforced: case.enforce_in_ci,
            status: "PASS",
            note: format!("id={} exit={exit_code} class={failure_class}", case.id),
        }
    } else {
        CaseResult {
            id: case.id.clone(),
            os: case.os.clone(),
            privilege: case.privilege.clone(),
            probe_mode: case.probe_mode.clone(),
            enforced: case.enforce_in_ci,
            status: "FAIL",
            note: format!("id={} {}", case.id, failures.join("; ")),
        }
    }
}

fn parity_summary(results: &[CaseResult], os: &str) -> String {
    let mut out = String::new();
    out.push_str("## Probe parity summary\n\n");
    out.push_str(&format!("Runtime OS: `{os}`\n\n"));
    out.push_str("| OS | Privilege | Probe mode | Enforced | Result | Notes |\n");
    out.push_str("|---|---|---|---|---|---|\n");

    for row in results {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} |\n",
            row.os,
            row.privilege,
            row.probe_mode,
            if row.enforced { "yes" } else { "no" },
            row.status,
            row.note.replace('|', "\\|")
        ));
    }

    out
}

#[test]
fn probe_parity_matrix_is_enforced() {
    let fixture_path = Path::new("tests/fixtures/probe_parity_matrix.json");
    let cases = parse_cases(fixture_path);
    validate_matrix(&cases);

    let runtime_os = current_os_label();
    let mut results = Vec::new();
    let mut failures = Vec::new();

    for case in &cases {
        if case.os != runtime_os {
            results.push(CaseResult {
                id: case.id.clone(),
                os: case.os.clone(),
                privilege: case.privilege.clone(),
                probe_mode: case.probe_mode.clone(),
                enforced: false,
                status: "SKIP",
                note: "not executable on this CI runtime OS".to_string(),
            });
            continue;
        }

        if case.enforce_in_ci {
            let result = run_case(case);
            if result.status == "FAIL" {
                failures.push(format!("{}: {}", result.id, result.note));
            }
            results.push(result);
        } else {
            results.push(CaseResult {
                id: case.id.clone(),
                os: case.os.clone(),
                privilege: case.privilege.clone(),
                probe_mode: case.probe_mode.clone(),
                enforced: false,
                status: "SKIP",
                note: "documented expectation (not enforceable in CI runtime)".to_string(),
            });
        }
    }

    let summary = parity_summary(&results, runtime_os);
    println!("{summary}");

    if let Ok(path) = env::var("PARITY_SUMMARY_PATH") {
        fs::write(path, summary).expect("failed to write parity summary");
    }

    assert!(
        results.iter().any(|r| r.enforced),
        "no enforceable parity rows were executed for os={runtime_os}"
    );

    assert!(
        failures.is_empty(),
        "probe parity expectations failed:\n{}",
        failures.join("\n")
    );
}
