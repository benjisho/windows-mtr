# API Reference

Windows MTR is primarily a CLI application. This document defines its operational API surfaces for automation:

1. Command-line interface (flags/options)
2. Exit behavior
3. Structured report output (JSON)

## CLI Interface

Base syntax:

```bash
mtr [options] <hostname-or-ip>
```

Primary categories:

- **Probe selection:** `-T`, `-U`, `-P`, `--source-port`
- **Routing scope:** `-m`, `-S`, `--interface`
- **Output mode:** `-r`, `-w`, `--json`, `--json-pretty`, `--csv <PATH>`
- **Sampling/timing:** `-c`, `-i`, `-W`
- **Name/ASN rendering:** `-n`, `-b`, `-z`

For full option examples, see [USAGE.md](USAGE.md) and [../USAGE.md](../USAGE.md).

## Exit Behavior

Typical expectations:

- `0` for successful command execution and report generation
- non-zero for invalid arguments, runtime/probe errors, or initialization failures

Automation guidance:

- Check exit code first
- Parse stdout only on success
- Capture stderr for diagnostic logging

## JSON Output Contract

When `--json` or `--json-pretty` is used, output is machine-readable and intended for downstream tooling.

Example pattern:

```bash
mtr --json -c 10 1.1.1.1 > mtr-report.json
```

Consumer best practices:

- Read and validate the top-level `schema_version` string (current value: `"1.0"`).
- If the JSON structure changes in a future release, the schema version will be bumped.
- Treat unknown fields as forward-compatible additions.
- Avoid strict ordering assumptions.
- Validate required fields in your own schema.

## CSV Output Contract

When `--csv <PATH>` is used, windows-mtr writes a CSV file at the provided path. CSV mode is mutually exclusive with `--json` and `--json-pretty`.

Current header set:

- `hop,ip,hostname,avg_ms,best_ms,worst_ms,loss_pct`

## REST API Response Headers

When running in REST API mode (`--api`), probe creation endpoints emit rate-limit metadata for both success responses and throttled responses (`429 Too Many Requests`):

- `X-RateLimit-Limit`: maximum requests allowed in the active window.
- `X-RateLimit-Remaining`: requests left in the active window.
- `X-RateLimit-Reset`: **seconds until** the current window resets (not an epoch timestamp).
- `RateLimit-Limit`: standards-aligned companion header carrying the same limit value.
- `RateLimit-Remaining`: standards-aligned companion header carrying the same remaining value.
- `RateLimit-Reset`: standards-aligned companion header carrying the same seconds-until-reset value.

All REST responses also include:

- `X-Request-ID`: per-request correlation identifier for logs and troubleshooting.

## API Probe Execution Timeout

API-launched probes are bounded by a configurable execution timeout (default: **60 seconds**). If a probe does not complete within the timeout, the job transitions to `failed` with an error message like `"probe timed out after 60.0s"` (sub-second durations render in milliseconds, e.g. `"1.0ms"`). The concurrency permit is released immediately when the timeout fires.

Configure via CLI flag:

```bash
mtr --api --api-probe-timeout-seconds 120
```

## Compatibility Notes

- CLI compatibility with Linux `mtr` is a goal, but not every flag is identical.
- `--trippy-flags` may expose advanced behavior from underlying implementation.
- Cross-platform/runtime differences may affect network probe behavior.

## Security Considerations for Integrators

- Treat all target/input values as untrusted.
- Avoid logging sensitive destination metadata in shared logs.
- Run with least privilege where feasible; elevate only when required for diagnostics.

## Related Docs

- [Installation](INSTALLATION.md)
- [Usage](USAGE.md)
- [Contributing](../CONTRIBUTING.md)
