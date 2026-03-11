# Contributing to Windows MTR

Thank you for your interest in contributing to **Windows MTR**. This guide explains how to contribute safely and efficiently to an enterprise-grade network diagnostics tool.

## Table of Contents

- [Code of Conduct Expectations](#code-of-conduct-expectations)
- [Before You Start](#before-you-start)
- [Development Workflow](#development-workflow)
- [Testing Requirements](#testing-requirements)
- [Documentation Requirements](#documentation-requirements)
- [Security Reporting](#security-reporting)
- [Release Process Overview](#release-process-overview)
- [Getting Help](#getting-help)

## Code of Conduct Expectations

By participating in this project, you agree to:

- Be respectful and constructive in all discussions.
- Assume good intent while giving clear, actionable feedback.
- Focus on technical outcomes and user impact.
- Avoid harassment, discrimination, and hostile behavior.

If you encounter unacceptable behavior, open a private maintainer contact through a GitHub issue asking for a confidential follow-up.

## Before You Start

1. Read the main [README](README.md).
2. Review command behavior in [USAGE.md](USAGE.md) and [docs/USAGE.md](docs/USAGE.md).
3. Check open issues and existing pull requests to avoid duplicated effort.

For local setup details, see [DEVELOPMENT.md](DEVELOPMENT.md).

## Development Workflow

1. **Fork and branch**
   - Create a topic branch with a clear name: `fix/icmp-timeout`, `docs/usage-json`.
2. **Keep changes minimal**
   - Prefer small, reviewable pull requests.
   - Preserve CLI compatibility unless intentionally documented.
3. **Implement with safety first**
   - Treat CLI input, hostnames, packets, and files as untrusted input.
4. **Validate locally**
   - Run formatting, linting, and tests before opening a PR.
5. **Open a pull request**
   - Use `.github/PULL_REQUEST_TEMPLATE.md`.
   - Clearly describe what changed, why, and how you tested it.

## Testing Requirements

Run these checks before submitting:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

All pull requests run dedicated **Workflow QA** checks for `actionlint` and `pre-commit --all-files` to validate workflow syntax and repository hygiene.

To mirror CI locally, install [pre-commit](https://pre-commit.com/) and run:

> [!IMPORTANT]
> Install Rust via [rustup](https://www.rust-lang.org/tools/install) first, and ensure `cargo` is available on your `PATH` before running pre-commit, because repository hooks execute Rust tooling.

```bash
pre-commit run --all-files
```

The repository pre-commit configuration includes workflow QA, baseline hygiene checks (such as `check-merge-conflict`, `check-yaml`, trailing whitespace, and end-of-file normalization), Rust checks (`cargo fmt`, `cargo check`, and `cargo clippy`), and Dockerfile linting via `hadolint`.

Install `hadolint` locally before running pre-commit so Dockerfile lint hooks can execute without missing-tool failures.

If you are in a restricted/offline environment and cannot install `hadolint`, run pre-commit with an explicit skip:

```bash
SKIP=hadolint pre-commit run --all-files
```

Markdown linting uses a **two-tier policy**:

1. **Baseline full-repository lint must pass** using the repository's relaxed legacy profile in `.markdownlint.jsonc`.
2. **Any Markdown file changed in your branch must pass strict lint** using `strict.markdownlint.jsonc`.

Run the baseline check:

```bash
npx -y markdownlint-cli2 "**/*.md" "!target/**" "!vendor/**"
```

Run strict lint for Markdown files changed from `master`:

```bash
changed_md_files=$(git diff --name-only --diff-filter=ACMR origin/master...HEAD -- '*.md')
if [ -n "$changed_md_files" ]; then
  printf '%s\n' "$changed_md_files" | xargs -r npx -y markdownlint-cli2 --config strict.markdownlint.jsonc
fi
```

This approach keeps legacy documentation debt scoped while ensuring newly touched Markdown meets stricter standards.

If you only want to run workflow QA directly, run:

```bash
actionlint
```

If any command cannot run in your environment, include:

- The exact command
- The exact failure output
- Why the limitation is environmental

## Documentation Requirements

Update docs whenever you change:

- CLI flags/options/default behavior
- Report/output formats (table/JSON)
- Build or installation instructions

Recommended docs to update:

- [README.md](README.md)
- [USAGE.md](USAGE.md)
- [docs/README.md](docs/README.md)
- [docs/API.md](docs/API.md)

## Security Reporting

Do **not** disclose vulnerabilities publicly in issues.

Instead:

1. Open a private security advisory in GitHub Security (preferred), or
2. Open an issue requesting private security contact details without exposing exploit details.

Include impact, affected versions, reproduction details, and mitigation ideas.

## Release Process Overview

High-level release flow:

1. Update relevant docs and `CHANGELOG.md`.
2. Ensure CI checks pass for all targets.
3. Tag and publish release artifacts (MSI, ZIP, checksums/signatures if available).
4. Validate installation and smoke-test core probe modes (ICMP/TCP/UDP).

For local contributor work, you usually only need to ensure your changes are release-ready and documented.

## Getting Help

- Usage questions: open a GitHub discussion/issue with command examples and outputs.
- Bug reports: use the [bug template](.github/ISSUE_TEMPLATE/bug_report.md).
- Feature proposals: use the [feature template](.github/ISSUE_TEMPLATE/feature_request.md).
