# windows-mtr After PR #167 (Post-merge Review)

_Date: 2026-05-03_

## Bottom line

PR #167 is merged into `master`, and the repository is in a significantly stronger state. The key architectural objectives landed:

- `--ui native` was replaced conceptually by `--ui dashboard` (with `native` retained as a deprecated alias).
- Distribution now centers around one canonical GitHub Release ZIP artifact.
- Capability claims are now validated against code, tests, docs, CI, and release/runtime evidence.

## Highlights of what improved

### 1) Runtime naming and UX consistency

Current UI model:

- `--ui default` → embedded Trippy TUI
- `--ui enhanced` → embedded Trippy TUI + windows-mtr thresholds/preset
- `--ui dashboard` → experimental JSON snapshot dashboard fallback
- `--ui native` → deprecated compatibility alias

This removes the old mismatch where “native” did not represent the actual behavior.

### 2) Dashboard argument architecture

Dashboard mode now uses a dedicated JSON snapshot argument builder (`build_json_snapshot_args(request, host)`) instead of mutating TUI-oriented arguments.

The builder creates a clear snapshot command shape:

- `mtr --mode json --report-cycles 1 ...probe flags... <host>`

And explicitly rejects conflicting flags such as:

- `--mode`, `--report-cycles`, `--json`, `--json-pretty`, and `--tui-*`

### 3) Release artifact strategy

Canonical distribution artifact:

- `windows-mtr-x86_64.zip`
  - `mtr.exe`
  - `windows-mtr.exe`
  - `README.txt`
  - `SHA256SUM`

Verification scripts/docs now validate ZIP structure, internal checksums, and package-manager manifest alignment.

## Status summary

- PR #167 merged: ✅
- CI green: ✅
- Security workflow green: ✅
- Pre-commit green: ✅
- Semgrep green: ✅
- Capability validation doc added: ✅

## Remaining follow-ups

Priority follow-ups captured in the review:

1. Track and remove RustSec ignores by upgrading the Trippy/hickory dependency chain.
2. Review/soften README claims that require benchmark/signing evidence.
3. Decide whether Chocolatey remains template-only or becomes truly installable.
4. Publish release-level ZIP checksum assets for package-manager verification.
5. Validate WinGet/Scoop/Chocolatey flows against a real tagged release.
6. Promote selected capability statuses only after release/runtime verification.
7. Optionally rename internal `native_*` symbols to `dashboard_*` in a later cleanup.

## Overall verdict

The project moved from a split-brain release/runtime story to a coherent Windows-first diagnostics CLI with a consistent UI narrative, clearer packaging model, and an explicit capability-evidence framework.

