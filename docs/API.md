# API Reference

Windows MTR is primarily a CLI application. This document defines its operational API surfaces for automation:

1. Command-line interface (flags/options)
2. Exit behavior
3. Structured report output (JSON)
4. REST API response envelope and operational headers

## CLI Interface

Base syntax:

```bash
mtr [options] <hostname-or-ip>
```

Primary categories:

- **Probe selection:** `-T`, `-U`, `-P`, `--source-port`
- **Routing scope:** `-m`, `-S`, `--interface`
- **Output mode:** `-r`, `-w`, `--json`, `--json-pretty`
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


## REST API Envelope and Headers

For `mtr --api`, consumers should rely on the versioned response envelope and standard operational headers.

Current baseline:

- Response body includes `meta.schema_version` (currently `v1`) for compatibility checks.
- Rate limiting is enforced server-side; expose and document rate-limit headers when enabled for client-side throttling (`X-RateLimit-Limit`, `X-RateLimit-Remaining`, `X-RateLimit-Reset`).
- Return a request correlation header (`X-Request-ID` or equivalent) so clients can map failures to server logs.

Client guidance:

- Do not hard-fail on unknown envelope fields.
- Prefer schema-version checks over ad-hoc field detection.
- On `429`, back off using returned header values when present.

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
