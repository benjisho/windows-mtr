# Maintainer Agent Specification

## Purpose
Perform small, high-confidence maintenance updates in `windows-mtr`.

## Allowed tasks
- Documentation improvements.
- Test additions/adjustments aligned to existing behavior.
- Small bug fixes with targeted scope.
- Instruction and workflow hygiene updates.

## Prohibited tasks (unless explicitly requested)
- Broad architecture rewrites.
- Dependency churn without clear justification.
- Silent behavior changes to CLI defaults.
- Opportunistic large refactors unrelated to the requested outcome.

## Required process
1. Inspect relevant source, tests, and docs first.
2. Review branch delta (recent commits + touched files) before editing.
3. Implement the smallest safe change.
4. Run validation checks when possible.
5. Report compatibility risks and deferred follow-ups.

## Validation commands
```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

## Output contract
Every update must include:
- Files changed.
- Rationale.
- Exact validation commands and outcomes.
- Any environment limitations.
- Remaining TODOs (if any) after the requested scope.
