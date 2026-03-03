# GitHub Agents Guidance

This directory contains role-specific guidance for automated or semi-automated agents used in CI/CD and repository maintenance.

## Current files
- `maintainer-agent.md`: baseline guidance for repo-aware maintenance tasks.

## Usage model
- Keep agent scopes narrow, explicit, and easy to review.
- Prefer analysis-first behavior before any write operation.
- Require exact validation commands for all code-writing tasks.
- Avoid overlapping ownership between agent specs.
- Prune broad refactors unless the task explicitly calls for them.

## Required spec sections for any new agent
Each agent spec should define:
1. Scope and mission.
2. Allowed actions.
3. Prohibited actions.
4. Validation requirements.
5. Output contract.

## Change management
- Update this directory when introducing a new automation role.
- Keep guidance aligned with `.github/copilot-instructions.md` and repository `AGENTS.md`.
- When repository structure changes, refresh path references and examples.
