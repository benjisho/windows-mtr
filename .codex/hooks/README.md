# Codex hooks for `windows-mtr`

This directory contains **Codex-specific repository tooling** for maintainer workflow support.

It does **not** change Windows-MTR product/runtime behavior. Hooks only influence Codex session guidance and Bash tool policy/review.

## Files
- `session_start_context.py`: emits short repo-specific startup context (safety, validation loop, docs expectations).
- `pre_tool_use_policy.py`: conservatively blocks clearly risky Bash actions (force-push, hard reset, destructive clean/remove, accidental release-style tag creation).
- `post_tool_use_review.py`: emits lightweight reminders only when changed files suggest useful follow-up checks.

## Design limitations
- Hooks intentionally use a small policy surface to avoid over-blocking normal development.
- Logic is stdlib-only Python for deterministic behavior and easy review.
- Scripts are defensive around missing/invalid JSON input and missing git context.

## Disable or adjust
- Disable all hooks by removing or renaming `.codex/hooks.json`.
- Tune policy/reminders by editing individual scripts under `.codex/hooks/`.
- Keep changes focused and aligned with root `AGENTS.md` plus repository docs.
