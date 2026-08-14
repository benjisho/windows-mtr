# Feature Delivery Status

This document serves as the single source of truth for feature delivery status.

## Status Legend
- ✅ Released
- 🛣️ Roadmap (not shipped, not implemented)
- 🚧 In Progress

## JSON Output
- **Status**: ✅ Released in v1.1.3
- **Notes**: Fully implemented with `schema_version: "1.0"`; CSV export is available through `--csv <PATH>`. Check the [documentation](USAGE.md#output--report-options) for examples on usage.

## DNS Caching (TTL)
- **Status**: ⚠️ Partial
- **Notes**: Available on Trippy-backed paths; the native Windows ICMP route does not apply the TTL setting.

## REST API v1
- **Status**: ⚠️ Partial
- **Notes**: Implemented with API key and trusted-ingress identity headers, bounded request validation, rate limiting, and concurrency bookkeeping. Default bind is localhost-only (`127.0.0.1:3000`). It does not terminate TLS itself, does not yet return hop-by-hop MTR data, and timeout does not prove child-process cancellation; see [REST API Documentation](security/rest-api.md).

## Release Artifact Validation
- **Status**: ⚠️ Partial
- **Notes**: PR and release ZIPs smoke-test JSON, CSV, TCP/UDP argument parsing, and REST API health. They do not perform live TCP/UDP probes.

## Dashboard UI
- **Status**: 🚧 In Progress (experimental preview via `--ui dashboard`, with deprecated alias `--ui native`)
- **Notes**: Preview available on `master`. Provides Overview / Hops / Charts navigation, stable hop columns, explicit loading/poll-error states, and latency/loss charts that ignore invalid samples. Not yet promoted to a stable release; expect rough edges.

## ETW/Windows Observability Integration
- **Status**: 🛣️ Roadmap
- **Notes**: Planned for future release. This integration is critical for observability.

## Security Hardening (audit + scheduled fuzzing)
- **Status**: ✅ Core gates released; advanced hardening remains roadmap
- **Notes**: `cargo-deny` and `cargo-audit` run in the PR security gate. Extended fuzz regression runs weekly (and can be started manually) in `fuzz-regression.yml`, with its nightly toolchain and `cargo-fuzz` version pinned. Future work is fuzz corpus/time-budget expansion and advisory cleanup.
