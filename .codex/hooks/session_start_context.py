#!/usr/bin/env python3
"""Emit short repo-specific context at Codex session start."""

from __future__ import annotations

import json
import subprocess
from pathlib import Path


def git_root() -> Path:
    try:
        out = subprocess.check_output(
            ["git", "rev-parse", "--show-toplevel"], text=True, stderr=subprocess.DEVNULL
        ).strip()
        if out:
            return Path(out)
    except Exception:
        pass
    return Path.cwd()


def main() -> int:
    root = git_root()
    agents = root / "AGENTS.md"

    lines = [
        "windows-mtr Codex context:",
        "- Keep diffs focused and preserve CLI/runtime compatibility unless explicitly requested.",
        "- For Rust changes, run: cargo fmt --all -- --check; cargo clippy --all-targets --all-features -- -D warnings; cargo test --all.",
        "- Update docs for CLI/output/API/install/release behavior changes (README/USAGE/docs/CHANGELOG as relevant).",
        "- Treat workflow/release/security-related edits as high-sensitivity and minimize scope.",
        "- Run OpenAPI checks only when API contract/spec changed: python3 scripts/validate_openapi_schema.py and scripts/check_openapi_compat.sh <base-ref>.",
    ]

    if not agents.exists():
        lines.append("- Note: AGENTS.md not found at git root; rely on repository docs/instructions.")

    print(json.dumps({"additionalContext": "\n".join(lines)}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
