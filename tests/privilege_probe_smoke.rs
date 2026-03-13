use std::env;
use std::process::Command;

fn parse_args(raw: &str) -> Vec<String> {
    shlex::split(raw).unwrap_or_else(|| raw.split_whitespace().map(ToString::to_string).collect())
}

fn parse_alternatives(raw: &str) -> Vec<String> {
    raw.split("||")
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(ToString::to_string)
        .collect()
}

#[test]
fn privilege_probe_smoke() {
    let binary = env!("CARGO_BIN_EXE_mtr");
    let args = env::var("SMOKE_ARGS").unwrap_or_else(|_| "-n -r -c 1 127.0.0.1".to_string());
    let expected_exit = match env::var("EXPECT_EXIT_CODE") {
        Ok(value) => value
            .parse::<i32>()
            .expect("EXPECT_EXIT_CODE must be an integer"),
        Err(_) => {
            eprintln!("skipping privilege smoke: EXPECT_EXIT_CODE is not set");
            return;
        }
    };

    let output = Command::new(binary)
        .args(parse_args(&args))
        .output()
        .expect("failed to execute smoke probe command");

    let actual_exit = output.status.code().unwrap_or(2);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(
        actual_exit, expected_exit,
        "unexpected exit code for `{binary} {args}`\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    if let Ok(expected_stdout) = env::var("EXPECT_STDOUT_CONTAINS") {
        assert!(
            stdout.contains(&expected_stdout),
            "expected stdout to contain `{expected_stdout}`\nactual stdout:\n{stdout}"
        );
    }

    if let Ok(expected_stderr_any_of) = env::var("EXPECT_STDERR_ANY_OF") {
        let alternatives = parse_alternatives(&expected_stderr_any_of);
        assert!(
            alternatives.iter().any(|needle| stderr.contains(needle)),
            "expected stderr to contain one of {alternatives:?}\nactual stderr:\n{stderr}",
        );
    }
}

#[test]
fn parse_args_supports_shell_quoting() {
    let args = parse_args("--trippy-flags \"--log-format json --verbose\" -c 1 127.0.0.1");
    assert_eq!(
        args,
        vec![
            "--trippy-flags",
            "--log-format json --verbose",
            "-c",
            "1",
            "127.0.0.1",
        ]
    );
}

#[test]
fn parse_alternatives_drops_empty_entries() {
    let values = parse_alternatives("foo|| ||bar||");
    assert_eq!(values, vec!["foo", "bar"]);
}
