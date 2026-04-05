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
| MSI Installer | ✅ Released | v1.0.0 |
| IPv6 Support | ✅ Released | v1.0.0 |
| Docker Support | ✅ Released | v1.0.0 |
| Single portable executable | ✅ Released | v1.1.3 |
| JSON Output | ✅ Released | v1.1.3 |
| DNS Caching (TTL) | ✅ Released | v1.1.3 |
| CI matrix coverage (Windows + Ubuntu, MSRV + stable) | ✅ Released | v1.2.x |
| CodeQL workflow for Rust | ✅ Released | v1.2.x |
| Container publishing to GHCR + Docker Hub | ✅ Released | v1.2.x |
| REST API (API key + mTLS auth, rate limiting, concurrency controls) | ✅ Released | v1.1.3 |
| SNMP Integration | 📅 Planned | H2 2026 |
| Native Ratatui UI (tabs, hop table, charts) | 🚧 In Progress (Experimental Preview via `--ui native`) | H2 2026 |
| ETW + Windows observability integrations | 🛣️ Roadmap | H2 2026 |
| Versioned JSON schema + CSV export | 🛣️ Roadmap | H2 2026 |
| Security hardening gates (cargo-audit + fuzz harness in CI) | 🚧 In Progress (cargo-audit live, fuzz harness pending) | H2 2026 |
| Cross-platform probe parity/privilege smoke tests | ✅ Released (CI `Probe parity (windows-latest\|ubuntu-latest)` + privilege smoke lanes: `Privilege probe smoke (ubuntu-latest\|windows-latest, non-elevated)`, `Privilege probe smoke (ubuntu-latest, elevated)`, and optional `Privilege probe smoke (windows, elevated self-hosted)`; coverage includes non-elevated failures on Windows/Ubuntu and elevated success on Ubuntu + Windows self-hosted; constraint: elevated Windows lane requires self-hosted runner because GitHub-hosted `windows-latest` cannot be interactively elevated.) | H2 2026 |
| GitHub Actions hardening (pin workflow actions by commit SHA) | ✅ Released | v1.2.x |
| CLI/runtime cleanup (unused error variants, banner polish) | 🛣️ Roadmap | H2 2026 |

## Notes

Roadmap dates and priorities can change based on stability, security, and user feedback.
