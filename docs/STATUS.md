# Feature Delivery Status

This document serves as the single source of truth for feature delivery status.

## Status Legend
- ✅ Released  
- 🛣️ Roadmap (not shipped, not implemented)  
- 🚧 In Progress

## JSON Output  
- **Status**: ✅ Released in v1.1.3  
- **Notes**: Fully implemented and available for use. Check the [documentation](#) for examples on usage.

## DNS Caching (TTL)
- **Status**: ✅ Released in v1.1.3  
- **Notes**: Provides improved performance. Refer to the [DNS Caching Documentation](#) for detailed implementation and considerations.

## REST API v1
- **Status**: ✅ Released in v1.1.3  
- **Notes**: Implemented with API key and mTLS authentication, rate limiting, and concurrency controls. Default bind is localhost-only (`127.0.0.1:3000`). See [REST API Documentation](security/rest-api.md) for the full threat model and usage examples.

## Dashboard UI
- **Status**: 🚧 In Progress (experimental preview via `--ui dashboard`, with deprecated alias `--ui native`)  
- **Notes**: Preview available on `master`. Provides live hop table, latency/loss charts, and multi-tab interface. Not yet promoted to a stable release; expect rough edges.

## ETW/Windows Observability Integration
- **Status**: 🛣️ Roadmap  
- **Notes**: Planned for future release. This integration is critical for observability.

## Security Hardening (cargo-audit + fuzz CI)
- **Status**: 🚧 In Progress  
- **Notes**: Security hardening initiatives are underway. cargo-audit is live, while fuzz harness is pending. More information can be found in the [Security Documentation](#).
