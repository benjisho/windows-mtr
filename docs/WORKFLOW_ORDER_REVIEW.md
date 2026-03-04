# Workflow Order Review Policy

This repository enforces publish ordering through a `workflow_run` gate.

## Required workflow chain for `master`

Container publication for `master` is allowed only after these workflows have completed successfully for the **same commit SHA**:

1. `CI`
2. `CodeQL`
3. `Security`

The gate is implemented in `.github/workflows/publish-gate.yml` and verifies workflow conclusions before invoking reusable publish logic.

## Reusable publish logic

To avoid duplicated release/publish code, image publishing is centralized in `.github/workflows/reusable-publish.yml` using `workflow_call`.

- `publish-gate.yml` calls the reusable workflow for `master` publishes after required checks pass.
- `release.yml` calls the same reusable workflow for tag-based releases before GitHub Release creation.

## Tag release behavior

`release.yml` now handles tag-based release flow (`v*.*.*`) and no longer publishes directly from branch pushes.

This separation keeps release publication deterministic and makes policy review straightforward: checks first, then gated publish.
