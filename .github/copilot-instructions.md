# Copilot Instructions for `windows-mtr`

These instructions guide GitHub Copilot and AI code assistants when proposing changes in this repository.

## Project intent
- Maintain a reliable, secure Windows-oriented MTR experience.
- Keep CLI behavior stable unless the change explicitly documents compatibility impact.
- Favor focused changes over broad refactors.

## Engineering expectations
- Follow patterns already used in nearby Rust modules.
- Prefer explicit, typed error paths and clear user-facing messages.
- Treat hostnames, packet data, and CLI input as untrusted.
- Avoid adding dependencies unless there is a strong, documented reason.

## Required checks before suggesting completion
Run what is available in the environment:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

If a check cannot run, state the exact blocker and do not fabricate results.

## Documentation discipline
Update docs when changing:
- CLI flags/options/defaults.
- Output/report format.
- Build or installation flow.

Primary docs:
- `README.md`
- `USAGE.md`
- `CHANGELOG.md` (when user-facing behavior changes)

## Pull request quality
PR descriptions should include:
1. What changed.
2. Why it changed.
3. How it was validated (exact commands).
4. Risks and compatibility notes.
