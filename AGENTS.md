# AGENTS.md

This file provides instructions for autonomous coding agents working in this repository.

## 1) Mission and contribution goals
- Keep `windows-mtr` reliable, secure, and predictable for networking diagnostics.
- Prefer small, reviewable pull requests with clear user impact.
- Preserve CLI compatibility unless change is explicitly documented.

## 2) Required workflow for every change
1. **Understand context first**
   - Read `README.md`, `USAGE.md`, and relevant source/tests before editing.
2. **Plan before editing**
   - Identify affected modules, risks, and validation steps.
3. **Implement minimal safe change**
   - Avoid broad refactors unless required by the task.
4. **Validate locally**
   - Run formatting, linting, and tests relevant to touched code.
5. **Document and summarize**
   - Update docs/changelog when behavior or UX changes.

## 3) Repository map
- `src/` — application source.
- `tests/` — integration/unit/report tests and fixtures.
- `examples/` — sample usage.
- `xtask/` — project automation helpers.
- `README.md` and `USAGE.md` — user documentation and commands.

## 4) Coding standards
- Follow existing Rust style and naming patterns in nearby files.
- Prioritize correctness and explicit error handling over cleverness.
- Do not introduce silent fallbacks for network errors without justification.
- Keep functions cohesive and avoid unnecessary public surface expansion.
- Add or update tests for bug fixes and behavior changes.

## 5) Testing and quality gates
When available, prefer running these commands before finalizing:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

If a command is not runnable in the environment, report exactly why.

## 6) Security and safety rules
- Treat all external inputs (CLI args, hostnames, packets, files) as untrusted.
- Avoid logging sensitive data or unnecessary host-identifying details.
- Do not add telemetry, network beacons, or data exfiltration behavior.
- Keep dependencies minimal; justify and document any new dependency.

## 7) Performance guidelines
- Be mindful of hot paths in probe/report loops.
- Avoid needless allocations and blocking operations in frequently executed paths.
- Include a short performance note in PRs for non-trivial algorithmic changes.

## 8) Documentation expectations
Update docs when any of these change:
- CLI flags/options/defaults.
- Output formats or report fields.
- Installation/build/run instructions.

## 9) Commit and PR guidance
- Use imperative, specific commit messages.
- PR descriptions should include:
  - What changed.
  - Why it changed.
  - How it was tested.
  - Any compatibility or risk notes.

## 10) Non-goals and restrictions
- Do not rewrite large modules unless explicitly requested.
- Do not change license or legal notices without maintainer instruction.
- Do not commit generated artifacts unless repository convention requires it.

## 11) GitHub automation files
- Keep Copilot guidance in `.github/copilot-instructions.md`.
- Keep role-specific agent specs in `.github/agents/`.
- Ensure agent specs define scope, prohibited actions, validation, and output contract.

## 12) Repository-local skills
- Place project-specific skills under `skills/<skill-name>/SKILL.md`.
- Prefer narrow skills that automate repeatable tasks (diagnostics, release prep, docs sync).
- Reuse scripts/templates from a skill directory instead of duplicating long instructions.
