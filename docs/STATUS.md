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
- **Notes**: Available on Trippy-backed paths; native Windows ICMP does not apply the TTL setting.

## REST API v1
- **Status**: ⚠️ Partial
- **Notes**: API key, trusted-ingress identity forwarding, rate limiting, and concurrency controls are implemented. It does not terminate TLS, return hop-by-hop data, or guarantee child-process cancellation.

## Release Artifact Validation
- **Status**: ⚠️ Partial
- **Notes**: JSON, CSV, TCP/UDP argument handling, and REST API health are smoke-tested; TCP/UDP live probes are not.

## Dashboard UI
- **Status**: 🚧 In Progress (experimental preview via `--ui dashboard`, with deprecated alias `--ui native`)
- **Notes**: Preview available on `master`. Provides Overview / Hops / Charts navigation, stable hop columns, explicit loading/poll-error states, and latency/loss charts that ignore invalid samples. Not yet promoted to a stable release; expect rough edges.

## ETW/Windows Observability Integration
- **Status**: 🛣️ Roadmap
- **Notes**: Planned for future release. This integration is critical for observability.

## Security Hardening (audit + scheduled fuzzing)
- **Status**: ✅ Core gates released; advanced hardening remains roadmap
- **Notes**: `cargo-deny` and `cargo-audit` run in the PR security gate. Extended fuzz regression runs weekly (and can be started manually) in `fuzz-regression.yml`, with its nightly toolchain and `cargo-fuzz` version pinned. Future work is fuzz corpus/time-budget expansion and advisory cleanup.
