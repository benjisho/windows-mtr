#!/usr/bin/env python3
"""Validate OpenAPI YAML syntax, duplicate keys, and schema structure."""

from __future__ import annotations

import sys
from pathlib import Path

from openapi_spec_validator import OpenAPIV31SpecValidator
from ruamel.yaml import YAML
from ruamel.yaml.constructor import DuplicateKeyError


def main() -> int:
    path = Path(sys.argv[1] if len(sys.argv) > 1 else "docs/api/openapi.yaml")
    if not path.exists():
        print(f"OpenAPI spec not found: {path}", file=sys.stderr)
        return 2

    yaml = YAML(typ="safe")
    yaml.allow_duplicate_keys = False

    try:
        with path.open("r", encoding="utf-8") as handle:
            spec = yaml.load(handle)
    except DuplicateKeyError as error:
        print(f"Duplicate YAML key detected in {path}:\n{error}", file=sys.stderr)
        return 1
    except Exception as error:  # noqa: BLE001
        print(f"Failed to parse {path}: {error}", file=sys.stderr)
        return 1

    errors = list(OpenAPIV31SpecValidator(spec).iter_errors())
    if errors:
        print(f"OpenAPI structural validation failed for {path}:", file=sys.stderr)
        for error in errors:
            print(f"- {error.message}", file=sys.stderr)
        return 1

    print(f"OpenAPI schema validation passed: {path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
