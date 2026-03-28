#!/usr/bin/env python3
"""Lightweight post-tool reminder layer for Bash activity."""

from __future__ import annotations

import json
import subprocess
import sys
from typing import Iterable


def load_input() -> dict:
    raw = sys.stdin.read().strip()
    if not raw:
        return {}
    try:
        data = json.loads(raw)
        return data if isinstance(data, dict) else {}
    except json.JSONDecodeError:
        return {}


def changed_files_from_git() -> list[str]:
    try:
        out = subprocess.check_output(
            ["git", "status", "--porcelain"], text=True, stderr=subprocess.DEVNULL
        )
    except Exception:
        return []

    files: list[str] = []
    for line in out.splitlines():
        if len(line) < 4:
            continue
        path = line[3:]
        if " -> " in path:
            path = path.split(" -> ", 1)[1]
        files.append(path)
    return files


def has_prefix(paths: Iterable[str], prefix: str) -> bool:
    return any(p.startswith(prefix) for p in paths)


def has_suffix(paths: Iterable[str], suffix: str) -> bool:
    return any(p.endswith(suffix) for p in paths)


def main() -> int:
    _payload = load_input()
    paths = changed_files_from_git()
    if not paths:
        print(json.dumps({}))
        return 0

    reminders: list[str] = []

    if has_prefix(paths, "src/") or has_prefix(paths, "tests/") or has_prefix(paths, "examples/") or has_prefix(paths, "xtask/"):
        reminders.append(
            "Rust code changed: run cargo fmt/clippy/test loop before finalizing."
        )

    if has_prefix(paths, ".github/workflows/") or "Dockerfile" in paths:
        reminders.append(
            "Workflow/Docker scope changed: keep edits minimal and run workflow hygiene checks (e.g., pre-commit/actionlint) when available."
        )

    if has_suffix(paths, ".md"):
        reminders.append(
            "Markdown changed: ensure docs remain aligned with behavior and run markdown/pre-commit checks if available."
        )

    if "docs/api/openapi.yaml" in paths or "scripts/validate_openapi_schema.py" in paths or "scripts/check_openapi_compat.sh" in paths:
        reminders.append(
            "OpenAPI-related files changed: run schema validation and compatibility checks as appropriate."
        )

    if not reminders:
        print(json.dumps({}))
        return 0

    message = "Post-tool reminders:\n- " + "\n- ".join(reminders)
    print(json.dumps({"additionalContext": message}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
