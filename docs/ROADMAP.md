# Windows MTR Roadmap

This page tracks delivery status for major features and platform capabilities.

For release-by-release details, see the [changelog](../CHANGELOG.md).

## Status legend

- ✅ Released
- 🚧 In Progress
- 📅 Planned
- 🛣️ Roadmap

## Delivery status

| Feature | Status | Timeline |
|---|---|---|
| Core MTR Functionality | ✅ Released | v1.0.0 |
| MSI Installer | 🛣️ Retired (portable ZIP is canonical) | — |
| IPv6 Support | ⚠️ Partial (native Windows ICMP is IPv4-only) | Follow-up |
| Docker Support | ⚠️ Partial | Follow-up |
| Single portable executable | ✅ Released | v1.1.3 |
| JSON Output | ✅ Released | v1.1.3 |
| DNS Caching (TTL) | ⚠️ Partial (Trippy paths only) | Follow-up |
| CI matrix coverage (Windows + Ubuntu, MSRV + stable) | ✅ Released | v1.2.x |
| CodeQL workflow for Rust | ⚠️ Partial (workflow exists; PR/alerting policy needs completion) | Follow-up |
| Container publishing to GHCR + Docker Hub | ✅ Released | v1.2.x |
| REST API (API key + trusted-ingress identity forwarding, rate limiting, concurrency controls) | ⚠️ Partial | Follow-up |
| SNMP Integration (optional) | 🛣️ Long-term roadmap | 2027+ |
| Dashboard UI (Overview/Hops/Charts, hop table, charts) | Experimental MVP via `--ui dashboard` (`--ui native` remains a deprecated alias) | H2 2026 |
| Dashboard fallback UI improvements (sorting, filtering, export, heatmaps) | Deferred | Post-MVP |
| ETW + Windows observability integrations (optional) | 🛣️ Long-term roadmap | 2027+ |
| Versioned JSON schema & CSV export | ✅ Released (`schema_version: "1.0"` added to CLI JSON output; `--csv <PATH>` introduced for CSV export) | v1.3.x |
| Release-artifact validation (JSON, CSV, TCP/UDP argument checks, REST API health) | ⚠️ Partial | Follow-up |
| Security hardening gates (cargo-deny + cargo-audit per PR; extended fuzz regression scheduled weekly) | ✅ Released | v1.3.x |
| Advanced security hardening (fuzz corpus/time-budget expansion, advisory cleanup) | 🛣️ Roadmap | H2 2026 |
| Cross-platform probe parity/privilege smoke tests | ⚠️ Partial (elevated Windows coverage is optional self-hosted) | Follow-up |
| GitHub Actions hardening (pin workflow actions by commit SHA) | ✅ Released | v1.2.x |
| API probe execution timeout and job lifecycle hardening | ⚠️ Partial (timeout does not prove child termination) | Follow-up |
| CLI/runtime cleanup (unused error variants, banner polish) | 🛣️ Roadmap | H2 2026 |

## Notes

Roadmap dates and priorities can change based on stability, security, and user feedback.

Issue #185 is a stale tracker; this table is the current roadmap summary.
