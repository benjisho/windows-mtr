#!/usr/bin/env python3
"""Conservative pre-tool policy checks for Bash commands."""

from __future__ import annotations

import json
import re
import shlex
import sys
from dataclasses import dataclass


@dataclass(frozen=True)
class Rule:
    pattern: re.Pattern[str]
    reason: str


DENY_RULES: tuple[Rule, ...] = (
    Rule(re.compile(r"\bgit\s+push\b[^\n]*\s--force(?:\b|=)"), "Refusing force push in routine repo work."),
    Rule(re.compile(r"\bgit\s+push\b[^\n]*\s-f\b"), "Refusing force push in routine repo work."),
    Rule(re.compile(r"\bgit\s+reset\s+--hard\b"), "Refusing hard reset because it can destroy local work."),
    Rule(re.compile(r"\bgit\s+clean\s+-fdx\b"), "Refusing git clean -fdx because it can remove untracked files irreversibly."),
    Rule(re.compile(r"\brm\s+-rf\s+(/\s*|~\s*|\.\.?\s*$|\*)"), "Refusing broad destructive rm -rf target."),
    Rule(re.compile(r"\bgit\s+tag\b[^\n]*\s(-a\s+)?v\d"), "Tag creation is release-sensitive; require explicit maintainer request."),
)


def load_input() -> dict:
    raw = sys.stdin.read().strip()
    if not raw:
        return {}
    try:
        data = json.loads(raw)
        return data if isinstance(data, dict) else {}
    except json.JSONDecodeError:
        return {}


def extract_command(payload: dict) -> str:
    for key in ("command", "input", "cmd"):
        value = payload.get(key)
        if isinstance(value, str) and value.strip():
            return value
    tool_input = payload.get("toolInput")
    if isinstance(tool_input, dict):
        for key in ("command", "cmd"):
            value = tool_input.get(key)
            if isinstance(value, str) and value.strip():
                return value
    return ""


def main() -> int:
    payload = load_input()
    command = extract_command(payload)
    normalized = " ".join(shlex.split(command)) if command else ""

    if not normalized:
        print(json.dumps({"decision": "allow"}))
        return 0

    for rule in DENY_RULES:
        if rule.pattern.search(normalized):
            print(json.dumps({"decision": "deny", "reason": rule.reason}))
            return 0

    print(json.dumps({"decision": "allow"}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
