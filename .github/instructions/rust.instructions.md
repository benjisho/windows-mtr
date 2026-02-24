---
applyTo: "src/**/*.rs,tests/**/*.rs,examples/**/*.rs,xtask/**/*.rs"
---
# Rust code generation instructions for windows-mtr

- Preserve CLI compatibility unless explicitly requested.
- Follow existing error handling patterns and avoid silent fallbacks.
- Add or adjust tests for behavior changes.
- Prefer minimal diffs; avoid broad refactors unrelated to the task.
- Validate with:
  - `cargo fmt --all -- --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test --all`
- If checks cannot run, state the exact environment blocker and stop short of fabrication.
