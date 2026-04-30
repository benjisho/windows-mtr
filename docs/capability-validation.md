# Capability Validation Matrix

This matrix validates strategic claims against repository evidence (code, tests, CI workflows, docs, and release artifact paths).

Status values: **Full**, **Strong**, **Partial**, **Basic**, **Not implemented**, **Roadmap only**.

> Rule: `Full` requires implementation + tests + CI + docs + release/runtime validation path.

| Capability | Claimed status | Evidence in code | Evidence in tests | Evidence in CI | Evidence in release artifact | Evidence in docs | Final validated status | Notes / follow-up |
|---|---|---|---|---|---|---|---|---|
| ICMP probes | Full | Trippy args builder defaults to ICMP | CLI/report tests | CI `cargo test --all` | ZIP smoke probe command in release workflow | README/USAGE | Strong | Upgrade to Full once ZIP smoke command is consistently validated in release job |
| TCP probes | Full | `-T` maps to `--tcp` | unit/cli tests for TCP + port requirement | CI tests | release smoke currently basic | README/USAGE | Strong | Add artifact-level TCP smoke if practical |
| UDP probes | Full | `-U` maps to `--udp` | unit/cli tests for UDP + port requirement | CI tests | release smoke currently basic | README/USAGE | Strong | Add artifact-level UDP smoke if practical |
| IPv6 | Claimed | No dedicated IPv6-specific code path | no explicit IPv6 integration test | not explicit | not explicit | docs claim parity broadly | Partial | Add IPv6 integration tests and release smoke validation |
| ECMP / multipath | Claimed | `--ecmp` -> `--multipath-strategy` | unit mapping tests | CI tests | not artifact-smoked | USAGE mapping table | Partial | Runtime validation needed |
| custom packet size | Claimed | `--packet-size` support in builders | unit coverage | CI tests | not artifact-smoked | USAGE mapping table | Partial | Add release artifact smoke command |
| source IP / interface | Claimed | `--source-address`, `--interface` mapping | unit mapping tests | CI tests | not artifact-smoked | USAGE mapping table | Partial | environment-specific runtime validation pending |
| interactive TUI | Full | embedded Trippy mode | CLI tests around interactive failures | CI tests compile path | release `--help` checks only | README/USAGE | Strong | Full requires artifact-level interactive runtime validation path |
| dashboard UI | Experimental | `src/native_ui.rs` + dashboard snapshot args | unit tests for parser/loss and snapshot args | CI tests | not direct TUI smoke | README/USAGE | Partial | Experimental fallback by design |
| report mode | Full | `--mode pretty` mapping | report tests | CI test suite | release smoke uses `-n -r -c 1 127.0.0.1` | README/USAGE | Full | |
| JSON output | Full | `--mode json`, parser paths | unit tests + API integration JSON usage | CI tests | dashboard uses JSON snapshot runtime path | README/USAGE | Strong | promote to Full after direct artifact JSON assertion |
| CSV / wide output | Claimed | wide mode exists (`-w`) | report tests | CI tests | not artifact-smoked | USAGE | Partial | CSV not explicitly implemented |
| REST API | Implemented | `src/service/rest_api.rs`, server runtime | API contract + integration tests | CI includes API tests | not in ZIP smoke commands | docs/API and security docs | Strong | Release artifact API smoke not yet present |
| OpenAPI spec | Implemented | `docs/api/openapi.yaml` | contract tests + schema validator | CI checks schema scripts | runtime path documented | docs/API | Strong | keep schema compatibility checks in CI |
| API key auth | Implemented | auth config + validation | rest_api_security_tests | CI test suite | no artifact smoke | security docs | Strong | Add runtime smoke in CI if possible |
| mTLS auth | Implemented (header trust model) | auth strategy + trusted ingress config | security tests | CI tests | no artifact smoke | security docs | Partial | Needs end-to-end cert-based validation docs/tests |
| rate limiting | Implemented | REST config values and enforcement | rest_api_security_tests | CI tests | no artifact smoke | README/USAGE | Strong | |
| concurrency limiting | Implemented | max concurrent probes in REST config | API tests | CI tests | no artifact smoke | security docs | Strong | |
| payload limiting | Implemented | max body size + target limits | security/API tests | CI tests | no artifact smoke | security docs | Strong | |
| threat model docs | Claimed | security docs present | n/a | docs lint/CI | n/a | `docs/security/rest-api.md` | Strong | Keep updated with implementation changes |
| security audit / CI checks | Claimed | security workflows + codeql/semgrep configs | n/a | dedicated workflows exist | n/a | README badges/docs | Strong | Depends on external workflow success |
| fuzz testing | Claimed | `fuzz/` target exists | no regular CI fuzz execution | not in default CI | n/a | README notes pending CI integration | Basic | Keep labeled as in progress |
| Docker/GHCR | Claimed | Dockerfile + release workflow publish job | docker smoke in CI | CI/release jobs publish (GHCR) | separate artifact path, not primary | README install section | Partial | Optional channel; requires privilege caveats for probes |
| Windows release ZIP | Full | release workflow builds zip | verification script + workflow checks | release workflow | ZIP includes expected files + SHA checks | docs/distribution | Full | Canonical distribution path |
| MSI/installer | Claimed historically | no active MSI generation in workflows | none | none | no MSI artifact mandated | legacy docs references | Not implemented | docs corrected to ZIP-first |
| WinGet readiness | Claimed | manifest templates in `packaging/winget` | dry-run updater script | CI dry-run script step | points to GitHub release ZIP | docs/distribution | Partial | manual publication pending |
| Scoop readiness | Claimed | manifest in `packaging/scoop` | dry-run updater script | CI dry-run script step | points to GitHub release ZIP | docs/distribution | Partial | manual bucket publication pending |
| Chocolatey readiness | Claimed | nuspec/tools templates | dry-run updater script | CI dry-run script step | references GitHub release ZIP | docs/distribution | Partial | manual package publication pending |
| Linux runtime support | Claimed broadly | builds on Linux in CI | Linux CI test jobs | CI ubuntu jobs | Windows ZIP only | docs now cautious | Partial | Avoid `/usr/bin/mtr` naming collision; use `windows-mtr` if added later |
| macOS runtime support | Claimed broadly | no macOS CI runner | no macOS tests | none | none | docs now deferred | Not implemented | Defer package channels until validated |

## Summary

- The product is strongest today as a Windows-first CLI/TUI binary distributed via GitHub Releases.
- API/security capabilities are implemented and tested, but not yet validated directly from the release ZIP in automated smoke checks.
- Cross-platform and package-manager publication claims should remain conservative until runtime and publication evidence is complete.
