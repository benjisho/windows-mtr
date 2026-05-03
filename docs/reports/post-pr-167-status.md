# windows-mtr Status Report After PR #167

_Date: 2026-05-03_

## Bottom line

PR #167 is merged and delivered the intended architecture cleanup:

- `--ui dashboard` is now the canonical name for the JSON snapshot fallback mode.
- `--ui native` remains as a deprecated compatibility alias.
- Release distribution now centers on one canonical GitHub Releases ZIP.
- Capability claims are tracked against code, tests, docs, CI, and release/runtime evidence.

## Confirmed outcomes

### Runtime UX and naming

Old behavior was misleading (`native` was not actually an embedded native UI). The current mapping is coherent:

- `--ui default` → embedded Trippy TUI
- `--ui enhanced` → embedded Trippy TUI + windows-mtr presets
- `--ui dashboard` → experimental JSON snapshot dashboard fallback
- `--ui native` → deprecated compatibility alias

### Dashboard argument construction

Dashboard mode now uses a dedicated JSON snapshot argument builder (`build_json_snapshot_args`) instead of mutating TUI arguments. Conflicting flags (for example `--mode`, `--report-cycles`, JSON mode flags, and `--tui-*`) are explicitly rejected.

### Release artifact model

Canonical distribution source is GitHub Releases ZIP:

- `windows-mtr-x86_64.zip`
  - `mtr.exe`
  - `windows-mtr.exe`
  - `README.txt`
  - `SHA256SUM`

Validation scripts now check ZIP layout, checksums, and package-manager manifest alignment.

### Validation and trust layer

Repository now includes capability validation guidance that requires evidence across:

- implementation code
- tests
- CI outcomes
- docs
- release/runtime validation

## Remaining follow-up items

1. Track and remove RustSec ignore entries by upgrading the Trippy/hickory dependency chain.
2. Soften or evidence README performance and security-signing claims.
3. Decide whether Chocolatey remains template-only or becomes fully installable.
4. Add/publish ZIP-level checksum assets for package-manager consumption.
5. Validate WinGet/Scoop/Chocolatey flows against a real tagged release.
6. Promote capability statuses only after release/runtime validation is completed.
7. Optionally rename remaining internal `native_*` symbols to `dashboard_*` later.

## Recommended next step

Cut the next GitHub Release from current `master`, verify the ZIP artifact contents/checksums, then run package-manager validation against that concrete release artifact.
