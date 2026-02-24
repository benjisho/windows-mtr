# AI Instructions for Contributors and Automated Agents

This document defines best-practice standards for AI-assisted changes in `windows-mtr`.

## Purpose
Use AI to improve developer productivity **without reducing** code quality, security, or maintainability.

## Core principles
1. **Human intent first**: Align changes to the requested outcome; avoid speculative edits.
2. **Smallest effective change**: Minimize diff size while fully solving the task.
3. **Verifiable output**: Every behavioral change should be validated by tests or reproducible checks.
4. **Traceability**: Summaries must explain what changed and why.
5. **Safety by default**: Preserve secure defaults and robust error handling.

## What AI agents should always do
- Inspect relevant code and tests before editing.
- Reuse existing patterns from nearby modules.
- Add/adjust tests when behavior changes.
- Keep user-facing docs synchronized with code changes.
- Call out assumptions and environment limitations explicitly.

## What AI agents must avoid
- Fabricating test results or command outcomes.
- Introducing hidden behavior, telemetry, or tracking.
- Making unrelated opportunistic refactors in task-focused PRs.
- Weakening validation, linting, or security checks to “make CI pass.”

## Recommended validation checklist
Run what is feasible for the scope:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

For docs-only changes, at minimum verify formatting and markdown rendering quality.

## Change classification rubric
- **Docs-only**: No runtime behavior changed. Prefer no-code diff.
- **Bug fix**: Add regression coverage when practical.
- **Feature**: Include tests + docs + backward compatibility notes.
- **Refactor**: Must be behavior-preserving with test evidence.

## PR quality bar for AI-authored changes
A strong AI-authored PR should include:
- Concise problem statement.
- Focused solution summary.
- Risks/tradeoffs.
- Exact commands run for validation.
- Follow-up work (if any) separated from current scope.

## Suggested commit message format
- `docs: add AI and agent contribution guidance`
- `fix: handle <specific error path> in <module>`
- `feat: add <capability> for <use case>`

## Ownership and review
AI output is draft quality until reviewed by a human maintainer. Reviewers may request simplification, stronger tests, or tighter scope.

## GitHub-native instruction files
When applicable, keep these aligned:
- `.github/copilot-instructions.md` for Copilot behavior.
- `.github/agents/` for role-based agent specifications.
- `.github/instructions/*.instructions.md` for path-scoped Copilot rules.
- Root `AGENTS.md` for repository-wide agent policy.

If rules overlap, repository-wide policy should remain consistent across all files.
