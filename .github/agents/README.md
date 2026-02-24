# GitHub Agents Guidance

This directory contains role-specific guidance for automated or semi-automated agents used in CI/CD and repository maintenance.

## Current files
- `maintainer-agent.md`: baseline guidance for repo-aware maintenance tasks.

## Usage model
- Keep agent scopes narrow and explicit.
- Prefer read-only analysis agents for triage/reporting tasks.
- Require explicit tests/checks for any code-writing agent.
- Avoid overlapping ownership between agent specs.

## Change management
- Update this directory when introducing a new automation role.
- Document trigger conditions, allowed actions, and output expectations.
