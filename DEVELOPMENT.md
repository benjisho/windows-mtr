# Development Guide

This document covers local setup, day-to-day development workflow, and troubleshooting for **Windows MTR**.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Clone and Build](#clone-and-build)
- [Local Workflow](#local-workflow)
- [Testing and Quality Gates](#testing-and-quality-gates)
- [Project Layout](#project-layout)
- [Troubleshooting](#troubleshooting)
- [Related Documentation](#related-documentation)

## Prerequisites

### Required

- Rust toolchain (Rust 1.88.0+ recommended)
- Git
- [pre-commit](https://pre-commit.com/)
- `hadolint` (required for local Dockerfile pre-commit hooks)

### Windows builds

- Visual Studio Build Tools with **Desktop development with C++** workload
- Administrator shell for raw socket/probe-related runtime tests

### Optional but useful

- `cargo-audit` for security checks
- `cargo-nextest` for faster local test loops

## Clone and Build

```bash
git clone https://github.com/benjisho/windows-mtr.git
cd windows-mtr
cargo build
```

Release build:

```bash
cargo build --release
```

Run CLI help:

```bash
cargo run -- --help
```

## Local Workflow

1. Create a branch from latest main/master.
2. Implement a focused change.
3. Add/update tests for behavior changes.
4. Update docs when CLI/output/install behavior changes.
5. Run quality gates before pushing.

Recommended commit style: imperative and specific, e.g. `docs: add API guide for JSON output`.

## Testing and Quality Gates

Run all checks when possible:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

Targeted test examples:

```bash
cargo test --test cli_tests
cargo test --test report_tests
```

For repository pre-commit checks:

```bash
pre-commit run --all-files
```

Ensure `hadolint` is installed locally before running pre-commit so Dockerfile lint hooks can run.

If you are working in a restricted environment and cannot install it, you can explicitly skip the hook:

```bash
SKIP=hadolint pre-commit run --all-files
```

## Security & Fuzzing

- `cargo audit` runs in CI and fails builds on known vulnerabilities/advisories.
- `cargo fuzz` builds all harnesses in `fuzz/` and runs a short smoke target on every PR/push.
- Keep local checks aligned with CI by running:

```bash
cargo audit --deny warnings --deny unmaintained --deny unsound --deny yanked
cargo fuzz build --fuzz-dir fuzz
cargo fuzz run <target> --fuzz-dir fuzz -- -max_total_time=20
```

## Project Layout

- `src/` — core application code
- `tests/` — integration, unit, and report-focused tests
- `examples/` — usage and integration examples
- `xtask/` — automation helpers
- `docs/` — expanded project documentation

## Troubleshooting

### Build fails with linker/toolchain errors (Windows)

- Confirm Visual Studio Build Tools are installed.
- Ensure the `x86_64-pc-windows-msvc` target is active.

### Runtime probe behavior differs without admin rights

- Some probe modes depend on privileges/capabilities.
- Re-run in elevated terminal for diagnostics.

### DNS or ASN results look inconsistent

- Use `-n` to isolate DNS from routing/probe behavior.
- Compare JSON output across multiple runs to confirm variability.

## Related Documentation

- [Contributing Guidelines](CONTRIBUTING.md)
- [Documentation Index](docs/README.md)
- [Installation Guide](docs/INSTALLATION.md)
- [Usage Guide](docs/USAGE.md)
- [API Reference](docs/API.md)
