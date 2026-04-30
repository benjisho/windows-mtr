# Capability validation matrix

Validation date: 2026-04-26.

Status scale used:
- Full
- Strong
- Partial
- Basic
- Not implemented
- Roadmap only

A capability is only marked **Full** if implemented, tested, documented, and validated from release artifact or documented runtime path.

| Capability | Claimed status | Evidence in code | Evidence in tests | Evidence in CI | Evidence in release artifact | Evidence in docs | Final validated status | Notes / required follow-up |
|---|---|---|---|---|---|---|---|---|
| ICMP probes | strong | probe request/build pipeline in `src/service/mod.rs` | unit/probe tests | `cargo test --all` in CI/release | release smoke command uses local report | README/USAGE examples | Strong | Needs explicit artifact-level multi-hop validation. |
| TCP probes | strong | `-T` + `--target-port` mapping | CLI/unit tests | CI test suite | not run in release smoke | docs examples | Partial | Add release smoke test for TCP mode. |
| UDP probes | strong | `-U` mapping | CLI/unit tests | CI test suite | not run in release smoke | docs examples | Partial | Add release smoke test for UDP mode. |
| IPv6 | claimed | parser accepts host/IP | no explicit IPv6 artifact test found | not explicit | none | docs mention parity | Basic | Needs validated runtime tests in CI/release. |
| ECMP / multipath | claimed | `--ecmp` -> `--multipath-strategy` | unit mapping tests | CI tests | none | usage docs | Partial | No release-artifact validation yet. |
| custom packet size | claimed | `--packet-size` mapping | unit mapping tests | CI tests | none | usage docs | Partial | Add artifact smoke with `-s`. |
| source IP / interface | claimed | `--src` / `--interface` mapping | unit mapping tests | CI tests | none | usage docs | Partial | Needs privileged artifact-path testing. |
| interactive TUI | strong | embedded trippy launch path in `src/main.rs` | CLI/tests | CI help/version checks | release smoke `--help` only | docs | Strong | Runtime depends on terminal/privileges. |
| dashboard UI | experimental | `src/native_ui.rs` + JSON snapshot args builder | unit tests + fixture | CI test suite | not full interactive in release workflow | README/USAGE | Partial | Non-interactive parser/snapshot tested; interactive stability varies. |
| report mode | strong | `--mode pretty` plan logic | tests/report tests | CI tests | release smoke runs `-n -r -c 1 127.0.0.1` | docs | Strong | Good baseline coverage. |
| JSON output | strong | `--json` / pretty flow | tests | CI tests | dashboard consumes json snapshots | docs | Strong | Add explicit release artifact JSON smoke later. |
| CSV / wide output | mixed | wide mode mapped; CSV not seen | report tests for wide | CI tests | not in artifact tests | docs mention wide | Basic | CSV appears not implemented; keep wording cautious. |
| REST API | implemented | `src/service/rest_api.rs`, `rest_server.rs` | API integration/security tests | CI `cargo test --all` | not exercised from release zip | docs/security + API docs | Partial | Strong code/tests but no release-path validation yet. |
| OpenAPI spec | implemented | `docs/api/openapi.yaml` | api contract tests | CI tests include contract checks | not artifact-level | docs/api | Partial | Keep as available for testing. |
| API key auth | implemented | auth strategy config | security tests | CI tests | not artifact-level | security docs | Partial | Needs deployment validation guidance. |
| mTLS auth | implemented | auth strategy config | security tests | CI tests | not artifact-level | security docs | Partial | Header-forwarding trust model documented. |
| rate limiting | implemented | API config defaults/enforcement | rest security tests | CI tests | not artifact-level | docs/security | Partial | Add e2e API runtime smoke in release path. |
| concurrency limiting | implemented | API config controls | tests | CI tests | not artifact-level | docs/security | Partial | Same as above. |
| payload limiting | implemented | API request size controls | tests | CI tests | not artifact-level | docs/security | Partial | Same as above. |
| threat model docs | claimed | docs file exists | doc checks only | none explicit | n/a | `docs/security/rest-api.md` | Strong | Keep updated with runtime reality. |
| security audit / CI checks | claimed | workflow config | CI workflows present | yes | n/a | README/docs | Partial | Avoid overstating "enterprise-grade" guarantees. |
| fuzz testing | claimed | `fuzz/` target exists | no regular CI fuzz run proven | none strong | n/a | README/docs | Basic | Mark as available locally, not continuous CI yet. |
| Docker/GHCR | claimed | Dockerfile + release workflow job | docker smoke in CI | yes | not Windows artifact | docs mention container usage | Partial | Optional channel; not primary install path. |
| Windows release ZIP | should be canonical | release workflow assembles canonical zip | verify script + smoke checks | release workflow validates | yes | README/distribution docs | Strong | Primary supported distribution path. |
| MSI/installer | previously claimed | no active MSI build in release workflow | none | none | not shipped | old docs referenced | Not implemented | Claims removed; ZIP is canonical. |
| WinGet readiness | planned | templates under `packaging/winget` | dry-run update script | can be validated manually | references release zip | distribution docs | Partial | Manual submission only (no auto-submit). |
| Scoop readiness | planned | `packaging/scoop/windows-mtr.json` | dry-run script | manual test path documented | references release zip | distribution docs | Partial | Not yet published in a bucket. |
| Chocolatey readiness | planned | nuspec/template present | dry-run script | manual packaging docs | references release zip | distribution docs | Partial | Not yet published on chocolatey.org. |
| Linux runtime support | often implied | Rust code may compile, but not primary target | limited smoke job only | ubuntu smoke exists | no Linux release artifact | docs now defer | Basic | Do not claim full Linux packaging/runtime yet. |
| macOS runtime support | sometimes implied | no explicit target workflow seen | none clear | none clear | none | docs now defer | Not implemented | Defer Homebrew until validated runtime support. |

## Summary

- Canonical deliverable is now the GitHub Release ZIP.
- Package manager manifests are metadata layers that should reference the canonical ZIP + SHA256.
- Claims were reduced where validation is incomplete (especially installer, cross-platform maturity, and enterprise wording).
