# API Reference

Windows MTR is primarily a CLI application. This document defines its operational API surfaces for automation:

1. Command-line interface (flags/options)
2. Exit behavior
3. Structured report output (JSON)
4. REST API endpoints (`--rest-api`)

## CLI Interface

Base syntax:

```bash
mtr [options] <hostname-or-ip>
```

Primary categories:

- **Probe selection:** `-T`, `-U`, `-P`, `--source-port`
- **Routing scope:** `-m`, `-S`, `--interface`
- **Output mode:** `-r`, `-w`, `--json`, `--json-pretty`, `--native-ui`
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

- Treat unknown fields as forward-compatible additions.
- Avoid strict ordering assumptions.
- Validate required fields in your own schema.



## Native UI mode

Use `--native-ui` to run the built-in Ratatui interface (tabs, hop table, gauges, and latency chart):

```bash
mtr --native-ui 8.8.8.8
```

This mode is terminal-interactive and intended for operator workflows, not JSON/REST automation.

## REST API (server mode)

Enable API mode:

```bash
mtr --rest-api --rest-api-bind 127.0.0.1:8080
```

Endpoints:

- `GET /health`
  - Response: `{"status":"ok"}`
- `POST /v1/report`
  - Request JSON includes a required `host` plus optional probe/output settings (`tcp`, `udp`, `port`, `count`, `timeout`, etc.).
  - Response JSON contains:
    - `exit_code`: embedded traceroute process exit code
    - `target`: resolved target string
    - `report`: traceroute report JSON object

Example:

```bash
curl -s -X POST http://127.0.0.1:8080/v1/report \
  -H "Content-Type: application/json" \
  -d '{"host":"1.1.1.1","count":5,"tcp":true,"port":443}'
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
