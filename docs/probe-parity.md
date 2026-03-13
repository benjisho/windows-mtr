# Probe parity matrix

This document defines expected probe behavior parity across **OS × privilege × probe mode**.

The source of truth for automated checks is:

- `tests/fixtures/probe_parity_matrix.json`

The integration test `tests/probe_parity_tests.rs` reads that table and enforces all rows marked `"enforce_in_ci": true` on the active OS. Rows marked `false` are still validated for matrix completeness and documented intent, but are not executed in CI.

## Axes

- **OS**: `linux`, `windows`
- **Privilege**: `unprivileged`, `elevated`
- **Probe mode**: `icmp`, `tcp`, `udp`

## Outcome model

Each matrix row defines:

- command args to execute,
- expected `exit_code`,
- expected outcome (`success` or `failure`),
- optional failure class and message substring checks.

## Current enforceable coverage

To keep CI deterministic and network-independent, enforceable parity checks currently cover:

- CLI-level option validation parity,
- host validation parity,
- exit status consistency,
- stable failure class/message mapping.

This means roadmap progress can rely on a machine-enforced parity baseline without needing raw socket/network behavior in CI.
