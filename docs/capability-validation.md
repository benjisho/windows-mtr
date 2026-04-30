# Capability Validation Matrix

Last reviewed: 2026-04-26.

Status rubric: **Full**, **Strong**, **Partial**, **Basic**, **Not implemented**, **Roadmap only**.

> A capability is marked **Full** only when implementation + tests + CI + docs + release/distribution runtime evidence are all present.

| Capability | Claimed status | Evidence in code | Evidence in tests | Evidence in CI | Evidence in release artifact | Evidence in docs | Final validated status | Notes / follow-up |
|---|---|---|---|---|---|---|---|---|
| ICMP probes | Supported | `src/service/mod.rs` request mapping | probe/report tests | CI runs cargo test | release smoke uses localhost report | README/USAGE | Strong | Needs release binary probe smoke retained per release workflow. |
| TCP probes | Supported | `--tcp`/port mapping | unit + CLI tests | CI cargo test | not directly smoke-tested from zip yet | README/USAGE | Strong | Add release artifact TCP smoke test later. |
| UDP probes | Supported | `--udp` mapping | unit tests | CI cargo test | not directly smoke-tested from zip yet | README/USAGE | Strong | Add release artifact UDP smoke test later. |
| IPv6 | Claimed | host/IP parser supports IP types | limited explicit coverage | no dedicated CI stage | no explicit release check | docs mention IPv6 | Partial | Add dedicated IPv6 integration test + release smoke. |
| ECMP/multipath | Claimed | `--ecmp` -> `--multipath-strategy` | unit mapping test | cargo test | no artifact test | USAGE parity table | Partial | Functional mapping exists; runtime validation missing. |
| packet size | Supported | `--packet-size` mapping | unit mapping test | cargo test | no artifact test | USAGE | Partial | Promote when release e2e includes scenario. |
| source IP/interface | Supported | `--src`,`--interface` mapping | unit mapping test | cargo test | no artifact test | USAGE | Partial | Add privileged smoke coverage. |
| Interactive TUI (embedded Trippy) | Supported | `run_embedded_trippy` + default/enhanced | CLI tests + report tests | cargo test on CI | `mtr.exe --help` + localhost report smoke | README/USAGE | Strong | Keep as default interactive mode. |
| Dashboard UI | Experimental | `src/native_ui.rs` + snapshot args builder | unit tests for parser/loss/args | cargo test | non-interactive validation path in tests | README/USAGE | Partial | Fallback mode only; not primary path. |
| Report mode | Supported | `--mode pretty` mapping | report tests | cargo test | release smoke runs `-n -r -c 1` | docs | Strong | Close to Full once release zip validation remains stable. |
| JSON output | Supported | `--mode json` + JSON passthrough | unit/report tests + fixture parser | cargo test | dashboard path uses JSON snapshots | USAGE/API docs | Strong | Add zip-level JSON smoke for Full. |
| CSV / wide output | Claimed | wide report supported, CSV absent | report-wide tests only | cargo test | no artifact test | docs mention wide; no CSV command | Basic | CSV should be described as not implemented. |
| REST API | Implemented | `src/service/rest_api.rs` + server | integration + contract tests | CI runs tests | not part of windows zip smoke | docs/API/security | Strong | Release artifact runtime path not validated yet. |
| OpenAPI spec | Implemented | `docs/api/openapi.yaml` | contract checks | schema script exists | n/a | docs/API | Strong | Keep schema validation in CI. |
| API key auth | Implemented | auth strategy code | security tests | CI tests | not zip-verified | docs/security | Strong | Runtime hardening exists. |
| mTLS auth (header model) | Implemented | mtls strategy code | security tests | CI tests | not zip-verified | docs/security | Partial | Documented as implemented/testing, not full mTLS transport termination. |
| rate limiting | Implemented | fixed-window limiters | API security tests | CI tests | not zip-verified | docs/security | Strong | |
| concurrency limiting | Implemented | max concurrent probes controls | API tests | CI tests | not zip-verified | docs/security | Strong | |
| payload limiting | Implemented | max request size checks | API tests | CI tests | not zip-verified | docs/security | Strong | |
| threat model docs | Claimed | security docs present | docs-focused checks only | indirect | n/a | `docs/security/rest-api.md` | Partial | Threat model exists for REST API scope. |
| security audit / CI checks | Claimed strong | security workflows present | n/a | CodeQL/semgrep/security workflows | n/a | README/security docs | Strong | Keep claims scoped to implemented workflows. |
| fuzz testing | Claimed | fuzz target exists | no CI execution evidence by default | not always-on | n/a | README/changelog mentions pending CI | Basic | Keep wording: harness exists, CI integration partial. |
| Docker/GHCR | Claimed | Dockerfile + release workflow push | no runtime probe tests seen | release workflow publishes | separate runtime path from Windows zip | README | Partial | Useful secondary channel; not primary install path. |
| Windows release ZIP | Supported target | release workflow creates zip | validation script + workflow checks | release workflow | canonical artifact `windows-mtr-x86_64.zip` | README/docs/distribution | Strong | Promote to Full after recurring tagged release validation history. |
| MSI installer | Historically claimed | no active MSI packaging in current workflow | none | none | not shipped in current release flow | README old claims corrected | Not implemented | Keep as roadmap only until reintroduced. |
| WinGet readiness | Claimed/planned | manifest templates included | dry-run update script | no auto-submit | points to GitHub release ZIP | docs/distribution | Partial | Manual submission checklist required. |
| Scoop readiness | Claimed/planned | manifest template included | dry-run update script | no publish CI | points to GitHub release ZIP | docs/distribution | Partial | |
| Chocolatey readiness | Claimed/planned | nuspec/template included | dry-run update script | no publish CI | points to GitHub release ZIP | docs/distribution | Partial | |
| Linux runtime support | Sometimes implied | Rust code mostly portable + raw probe caveats | no first-class Linux packaging tests in this repo | limited | not a Windows release artifact goal | docs/distribution (deferred) | Basic | Do not market as fully supported. |
| macOS runtime support | Sometimes implied | not validated | no dedicated tests | none | no artifact | docs/distribution (deferred) | Not implemented | Defer packaging/claims. |

## Summary

- Current strongest validated story: **Windows CLI + report/TUI + release ZIP + SHA256 verification**.
- **Dashboard UI** is intentionally marked experimental fallback.
- API/security features are implemented and tested in source/CI, but not yet validated from the downloadable Windows release artifact path.
- Distribution claims should remain: **GitHub Releases canonical**, WinGet/Scoop/Chocolatey **prepared/planned**, Linux/macOS channels **deferred**.
