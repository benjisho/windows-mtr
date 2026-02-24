# Maintainer Agent Specification

## Purpose
Perform small, high-confidence maintenance updates in `windows-mtr`.

## Allowed tasks
- Documentation improvements.
- Test additions/adjustments aligned to existing behavior.
- Small bug fixes with targeted scope.

## Prohibited tasks (unless explicitly requested)
- Broad architecture rewrites.
- Dependency churn without clear justification.
- Silent behavior changes to CLI defaults.

## Required process
1. Inspect relevant source and tests first.
2. Implement the smallest safe change.
3. Run validation checks when possible.
4. Summarize exact impacts and risks.

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
