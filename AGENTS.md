# AGENTS.md

Codex operating notes for `windows-mtr` (repository scope).

## Mission and guardrails
- Keep `windows-mtr` reliable, secure, and predictable for network diagnostics.
- Prefer small, reviewable pull requests with clear user impact.
- Preserve CLI compatibility unless a change is explicitly requested and documented.

## Required workflow
1. Read context first: `README.md`, `USAGE.md`, and touched source/tests/docs.
2. Plan a minimal safe diff (avoid unrelated refactors).
3. Implement with explicit error handling and no silent network fallbacks.
4. Validate with relevant checks.
5. Update docs/changelog when behavior, UX, install, API, or release flow changes.

## Repository map
- `src/` — application code.
- `tests/` — integration/unit/report tests.
- `examples/` — sample usage.
- `xtask/` — automation helpers.
- `docs/` — project documentation.

## Toolchain and validation
- Rust/toolchain expectations follow repo docs and CI (currently Rust 1.88.0+ / MSRV from `Cargo.toml`).
- For Rust changes, run when available:
  - `cargo fmt --all -- --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test --all`
- For workflow/docs hygiene changes, run targeted checks (for example `pre-commit run --all-files`, `actionlint`, markdown lint).
- If a command cannot run, report the exact command and concrete environment blocker.

## API/OpenAPI checks (when API contract/spec changes)
- `python3 scripts/validate_openapi_schema.py`
- `scripts/check_openapi_compat.sh <base-ref>`

## Security and sensitive-change caution
- Treat CLI args, hostnames, packet data, and files as untrusted inputs.
- Avoid logging sensitive data or adding telemetry/data-exfil behavior.
- Keep dependencies minimal and justified.
- Use extra care and minimal diffs for:
  - `.github/workflows/**`, release/version/tagging/packaging automation.
  - Security policy/audit/dependency configuration.

## Documentation expectations
Update docs when changing:
- CLI flags/options/defaults.
- Output/report/API schema.
- Build/install/run/release behavior.

Primary docs typically include `README.md`, `USAGE.md`, `docs/`, and `CHANGELOG.md`.

## Non-goals
- Do not rewrite large modules unless explicitly requested.
- Do not change license/legal notices without maintainer instruction.
- Do not commit generated artifacts unless repository convention requires it.

## Repo instruction surfaces
- `.github/copilot-instructions.md` — repository-wide assistant guidance.
- `.github/instructions/*.instructions.md` — path-scoped rules.
- `.github/agents/` — role-specific agent specs.
- `skills/<skill-name>/SKILL.md` — repeatable repository-local workflows.
