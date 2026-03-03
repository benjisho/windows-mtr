# Copilot Instructions for `windows-mtr`

These instructions guide GitHub Copilot and AI code assistants when proposing changes in this repository.

## Project intent
- Keep `windows-mtr` reliable, secure, and predictable for network diagnostics.
- Preserve CLI compatibility unless a task explicitly documents a compatibility change.
- Favor small, reviewable diffs over broad rewrites.

## Refactor policy (prune unsafe refactors)
- Prefer targeted fixes over structural refactors.
- Do not rename/move modules, change public CLI surface, or rework core probe/report flow unless explicitly requested.
- If a refactor is needed, include:
  1. scope boundaries,
  2. compatibility impact,
  3. rollback plan,
  4. focused tests proving parity.

## Engineering expectations
- Follow nearby Rust patterns and naming.
- Use explicit, typed error paths and actionable user-facing messages.
- Treat CLI args, hostnames, packet data, and files as untrusted.
- Avoid silent network fallbacks and hidden retries.
- Avoid new dependencies unless justified in PR rationale.

## Branch-delta review checklist (before coding)
1. Review commit history and changed files to understand intent and drift.
2. Identify what is complete vs what remains (tests, docs, compatibility notes).
3. Confirm instructions still match current repository layout.
4. Implement only the minimum required change set.

## Required validation checks
Run what is available in the environment:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

If a check cannot run, state the exact blocker and never fabricate results.

## Documentation discipline
Update docs when changing:
- CLI flags/options/defaults.
- Output/report schema.
- Build, install, release, or packaging flow.

Primary docs:
- `README.md`
- `USAGE.md`
- `CHANGELOG.md` (for user-facing behavior)
- `docs/` pages for workflow details

## Pull request quality
PR descriptions should include:
1. What changed.
2. Why it changed.
3. How it was validated (exact commands).
4. Compatibility/risk notes.
5. Follow-up work (if intentionally deferred).
