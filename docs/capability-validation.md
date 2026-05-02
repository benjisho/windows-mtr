# Capability Validation Matrix

This document validates strategic capability claims against repository reality.

Validation scale: `Full`, `Strong`, `Partial`, `Basic`, `Not implemented`, `Roadmap only`.

`Full` requires code + tests + CI + docs + release-artifact/runtime-path validation.

| Capability | Claimed status | Evidence in code | Evidence in tests | Evidence in CI | Evidence in release artifact | Evidence in docs | Final validated status | Notes / required follow-up |
|---|---|---|---|---|---|---|---|---|
| ICMP probes | Supported | Probe request/arg mapping in service | unit + report tests | CI runs cargo test | release binary smoke probe planned | README/USAGE | Strong | Promote to Full once release ZIP probe smoke is enforced on tags |
| TCP probes | Supported | `-T`, `--target-port` support | unit/probe parity tests | CI tests | release smoke includes CLI parse only | README/USAGE | Strong | Keep release smoke for `-T -P` path |
| UDP probes | Supported | `-U` mapping in service | unit/probe parity tests | CI tests | not directly smoke-tested in release artifact | README/USAGE | Partial | Add release smoke for UDP |
| IPv6 | Supported | host parsing and trippy passthrough | limited tests | generic CI only | no release artifact validation | docs mention | Partial | Add explicit IPv6 integration test |
| ECMP/multipath | Supported | `--ecmp` to `--multipath-strategy` | unit test mapping | cargo test in CI | no release artifact check | USAGE mapping table | Partial | Add runtime test with deterministic fixture |
| Custom packet size | Supported | `--packet-size` mapping | unit tests | CI tests | no release artifact smoke | docs present | Partial | Add release artifact smoke for parsing |
| Source IP/interface | Supported | `--src`, `--interface` mapping | unit tests | CI tests | no release artifact smoke | docs present | Partial | Privilege/network env sensitive |
| Interactive TUI (embedded Trippy) | Supported | default/enhanced mode | CLI + unit tests | CI cargo test | release `--help`/`--version` checks; interactive runtime not automated | README/USAGE | Strong | Add optional Windows interactive smoke where feasible |
| Dashboard UI (`--ui dashboard`) | Experimental | custom ratatui dashboard + JSON polling | native_ui unit tests + fixture parsing | cargo test in CI | release runtime path documented, no interactive artifact automation | README/USAGE | Partial | Alias `--ui native` kept for compatibility |
| Report mode | Supported | `-r` to pretty mode | report tests | CI tests | release smoke includes `-n -r -c 1 127.0.0.1` | README/USAGE | Strong | Move to Full after release ZIP smoke mandatory |
| JSON output | Supported | `--json`/`--json-pretty` handling | report/unit tests | CI tests | dashboard path consumes JSON snapshots | USAGE/docs | Strong | Add release ZIP JSON smoke |
| CSV/wide output | Wide yes, CSV unclear | `--report-wide` exists | tests for wide options limited | CI tests | no artifact validation | docs mention wide; CSV not explicit | Basic | Avoid claiming CSV until implemented/tested |
| REST API | Implemented | `rest_api`, `rest_server` modules | API integration/security tests | CI runs API tests | not packaged as separate server artifact; same binary path | README/USAGE/docs/security | Strong | Full requires explicit release-runtime API smoke |
| OpenAPI spec | Implemented | `docs/api/openapi.yaml` + schema checks | contract tests + schema script | CI runs schema validation scripts | not part of release ZIP execution | docs/API | Strong | Keep compatibility check gate |
| API key auth | Implemented | Auth strategy + key env | security tests | CI tests | release runtime path not explicitly validated | security docs | Strong | Add release API smoke in future |
| mTLS auth | Implemented | mtls strategy + ingress controls | security tests | CI tests | no release artifact proof | security docs | Partial | Needs end-to-end cert-based test |
| Rate limiting | Implemented | REST config controls | security/integration tests | CI tests | no release artifact proof | docs/security | Strong |  |
| Concurrency limiting | Implemented | REST config controls | integration tests | CI tests | no release artifact proof | docs/security | Strong |  |
| Payload limiting | Implemented | REST body limit config | security tests | CI tests | no artifact proof | docs/security | Strong |  |
| Threat model docs | Claimed | `docs/security/rest-api.md` | doc-only | markdown checks only | N/A | present | Basic | Expand beyond REST scope |
| Security audit / CI checks | Claimed | workflows include security jobs | CI workflow config | security workflow present | N/A | README mentions | Partial | Keep wording conservative |
| Fuzz testing | Claimed | `fuzz/` harness exists | local fuzz target tests | CI integration not complete | N/A | README notes in-progress | Basic | Do not claim full fuzz CI coverage |
| Docker/GHCR | Claimed | Dockerfile + release workflow publish job | workflow-level only | tag workflow publishes images | not part of Windows ZIP | README/docs | Partial | Optional channel; not primary install |
| Windows release ZIP | Supported canonical | release workflow packaging logic | new verify script | release workflow validation | ZIP includes required files | README/distribution docs | Strong | Full after mandatory verify step is enforced |
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
