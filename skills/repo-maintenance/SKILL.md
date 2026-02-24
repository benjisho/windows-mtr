---
name: repo-maintenance
description: Use this skill when making routine maintenance changes in this repository, including docs updates, small bug fixes, and test-safe refactors that must follow repository guardrails and validation steps.
---

# repo-maintenance

## Use when
- The task is a focused repository update (docs, minor fixes, localized improvements).
- You need predictable process and validation requirements for this repo.

## Inputs
- User request.
- Files likely impacted.
- Current CI/toolchain constraints.

## Steps
1. Read root `AGENTS.md`, `README.md`, and relevant touched files.
2. Plan a smallest-safe diff and avoid unrelated cleanup.
3. Implement changes using existing local patterns.
4. Validate with repo checks when possible.
5. Summarize exact commands run, results, and limitations.

## Validation
```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

## References
- For repository policy and precedence details, read `references/instruction-precedence.md`.

## Output
- Concise change summary.
- File list and rationale.
- Exact validation command outcomes.
