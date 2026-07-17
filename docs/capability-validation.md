# Capability Validation Matrix

This document validates strategic capability claims against repository reality.

Validation scale: `Full`, `Strong`, `Partial`, `Basic`, `Not implemented`, `Roadmap only`.

`Full` requires code + tests + CI + docs + release-artifact/runtime-path validation.

| Capability | Claimed status | Evidence in code | Evidence in tests | Evidence in CI | Evidence in release artifact | Evidence in docs | Final validated status | Notes / required follow-up |
|---|---|---|---|---|---|---|---|---|
| ICMP probes | Supported | Probe request/arg mapping in service | unit + report tests | CI runs cargo test | mandatory release ZIP smoke runs a localhost report | README/USAGE | Full | Runtime behavior still depends on host privileges and network policy |
| TCP probes | Supported | `-T`, `--target-port` support | unit/probe parity tests | CI tests | mandatory release ZIP smoke runs `-T -P 443` | README/USAGE | Full | Runtime behavior still depends on host privileges and network policy |
| UDP probes | Supported | `-U` mapping in service | unit/probe parity tests | CI tests | mandatory release ZIP smoke runs `-U -P 53` | README/USAGE | Full | Runtime behavior still depends on host privileges and network policy |
| IPv6 | Supported | host parsing and trippy passthrough | limited tests | generic CI only | no release artifact validation | docs mention | Partial | Add explicit IPv6 integration test |
| ECMP/multipath | Supported | `--ecmp` to `--multipath-strategy` | unit test mapping | cargo test in CI | no release artifact check | USAGE mapping table | Partial | Add runtime test with deterministic fixture |
| Custom packet size | Supported | `--packet-size` mapping | unit tests | CI tests | no release artifact smoke | docs present | Partial | Add release artifact smoke for parsing |
| Source IP/interface | Supported | `--src`, `--interface` mapping | unit tests | CI tests | no release artifact smoke | docs present | Partial | Privilege/network env sensitive |
| Interactive TUI (embedded Trippy) | Supported | default/enhanced mode | CLI + unit tests | CI cargo test | release `--help`/`--version` checks; interactive runtime not automated | README/USAGE | Strong | Add optional Windows interactive smoke where feasible |
| Dashboard UI (`--ui dashboard`) | Experimental | custom ratatui dashboard + JSON polling | native_ui unit tests + fixture parsing | cargo test in CI | release runtime path documented, no interactive artifact automation | README/USAGE | Partial | Alias `--ui native` kept for compatibility |
| Report mode | Supported | `-r` to pretty mode | report tests | CI tests | mandatory release ZIP smoke runs `-n -r -c 1 127.0.0.1` | README/USAGE | Full | Runtime behavior still depends on host privileges and network policy |
| JSON output | Supported | `--json`/`--json-pretty` handling | report/unit tests | CI tests | mandatory release ZIP smoke parses output and checks `schema_version: "1.0"` | USAGE/docs | Full | Keep schema-version compatibility policy explicit |
| CSV output | Supported | `--csv <PATH>` writes a normalized report | report/unit tests | CI tests | mandatory release ZIP smoke checks CSV creation and header | README/USAGE/API | Full | Keep CSV header compatibility documented |
| Wide report output | Supported | `--report-wide` handling | limited option tests | CI tests | no dedicated release artifact smoke | README/USAGE | Partial | Add deterministic wide-report runtime assertion |
| REST API | Implemented | `rest_api`, `rest_server` modules | API integration/security tests | CI runs API tests | mandatory release ZIP smoke starts API and checks health | README/USAGE/docs/security | Strong | Add a packaged authenticated probe lifecycle test for Full |
| OpenAPI spec | Implemented | `docs/api/openapi.yaml` + schema checks | contract tests + schema script | CI runs schema validation scripts | not part of release ZIP execution | docs/API | Strong | Keep compatibility check gate |
| API key auth | Implemented | Auth strategy + key env | security tests | CI tests | release runtime path not explicitly validated | security docs | Strong | Add release API smoke in future |
| mTLS identity forwarding | Implemented (trusted ingress) | mtls strategy + ingress controls | security tests | CI tests | no release artifact proof | security docs | Partial | This is not native TLS termination; add cert-based end-to-end testing only if direct TLS is adopted |
| Rate limiting | Implemented | REST config controls | security/integration tests | CI tests | no release artifact proof | docs/security | Strong |  |
| Concurrency limiting | Implemented | REST config controls | integration tests | CI tests | no release artifact proof | docs/security | Strong |  |
| Payload limiting | Implemented | REST body limit config | security tests | CI tests | no artifact proof | docs/security | Strong |  |
| Threat model docs | Claimed | `docs/security/rest-api.md` | doc-only | markdown checks only | N/A | present | Basic | Expand beyond REST scope |
| Security audit / CI checks | Implemented | dedicated security workflow | audit policy and workflow configuration | `cargo-deny` + `cargo-audit` run per PR | N/A | README/roadmap/status | Strong | Keep advisory policy current |
| Fuzz testing | Implemented | `fuzz/` harness exists | fuzz target build and execution | extended all-target regression runs weekly and manually with pinned tooling | N/A | README/roadmap/status | Strong | Expand corpus and time budget based on observed runtime |
| Docker/GHCR | Claimed | Dockerfile + release workflow publish job | workflow-level only | tag workflow publishes images | not part of Windows ZIP | README/docs | Partial | Optional channel; not primary install |
| Windows release ZIP | Supported canonical | release workflow packaging logic | layout and executable smoke scripts | PR and release workflows run both checks | ZIP includes required files and executes JSON, CSV, TCP, UDP, and REST API health paths | README/distribution docs | Full | Keep the smoke script aligned with documented artifact contents |
| MSI/installer | Historically referenced | no active MSI build in release workflow | none | none | no MSI artifact in canonical plan | docs may reference old MSI | Not implemented | remove/soften MSI claims |
| WinGet readiness | Planned | manifests in `packaging/winget` | local validation docs/scripts | no auto-submit CI | points to canonical ZIP | distribution docs | Partial | prepared but unpublished |
| Scoop readiness | Planned | manifest in `packaging/scoop` | local test commands documented | no publish CI | points to canonical ZIP | distribution docs | Partial | prepared but unpublished |
| Chocolatey readiness | Planned | nuspec/template added | local commands documented | no publish CI | points to canonical ZIP | distribution docs | Partial | prepared but unpublished |
| Linux runtime support | Sometimes implied | Rust code is cross-platform but Windows-first UX | limited Linux smoke job | ubuntu smoke uses cargo check | no Linux binary release path | docs now defer | Basic | do not claim first-class Linux runtime yet |
| macOS runtime support | Sometimes implied | no dedicated macOS workflow/release | none | none | no macOS artifacts | docs now defer | Not implemented | defer package channels |

## Summary

- The product is strongest today as a Windows-first CLI + embedded Trippy runtime with report and API features.
- Dashboard mode is valuable but explicitly experimental.
- Canonical distribution should remain GitHub Releases ZIP until package-manager publication and validation are complete.
