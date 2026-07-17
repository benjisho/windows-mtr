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
- **Status**: ✅ Released in v1.1.3
- **Notes**: Provides improved performance. Refer to the [DNS Caching Documentation](USAGE.md#timing--dns-cache) for detailed implementation and considerations.

## REST API v1
- **Status**: ✅ Released in v1.1.3
- **Notes**: Implemented with API key and trusted-ingress mTLS identity forwarding, rate limiting, and concurrency controls. Default bind is localhost-only (`127.0.0.1:3000`). It does not terminate TLS itself; see [REST API Documentation](security/rest-api.md) for the deployment model and threat model.

## Release Artifact Validation
- **Status**: ✅ Released in v1.3.x
- **Notes**: PR and release ZIPs are smoke-tested for JSON, CSV, TCP, UDP, and REST API health paths.

## Dashboard UI
- **Status**: 🚧 In Progress (experimental preview via `--ui dashboard`, with deprecated alias `--ui native`)
- **Notes**: Preview available on `master`. Provides live hop table, latency/loss charts, and multi-tab interface. Not yet promoted to a stable release; expect rough edges.

## ETW/Windows Observability Integration
- **Status**: 🛣️ Roadmap
- **Notes**: Planned for future release. This integration is critical for observability.

## Security Hardening (audit + scheduled fuzzing)
- **Status**: ✅ Released in v1.3.x
- **Notes**: `cargo-deny` and `cargo-audit` run in the PR security gate. Extended fuzz regression runs weekly (and can be started manually) in `fuzz-regression.yml`, with its nightly toolchain and `cargo-fuzz` version pinned. Future work is fuzz corpus/time-budget expansion and advisory cleanup.
