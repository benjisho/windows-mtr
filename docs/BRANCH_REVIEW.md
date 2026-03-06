# Branch Review Snapshot

This document summarizes the current branch delta, what is complete, and what still needs edits to stay aligned with `windows-mtr` goals.

## PR aim (current branch)
The branch is focused on CI/CD maintenance (dependency updates in release automation), not runtime CLI or probe-engine behavior.

## What was introduced in this branch
From branch history in the reviewed range (`git log --oneline --name-only`), the following files changed:

1. `.github/workflows/release.yml`
   - Bumped `docker/login-action` from major `v3` to `v4`.
   - Bumped `docker/setup-qemu-action` from major `v3` to `v4`.
2. `.github/workflows/security.yml`
   - Added explicit workflow/job `permissions` to address least-privilege and code-scanning guidance.
3. `.github/workflows/windows-build.yml`
   - Added explicit workflow/job `permissions` to address least-privilege and code-scanning guidance.
4. `README.md`
   - Updated badge presentation/status messaging.
   - Updated roadmap/status text to reflect current priorities.

## 1) Completed in this PR

- Release workflow action versions were advanced to current major versions for Docker login and QEMU setup.
- Security-sensitive workflows now include explicit `permissions` declarations (`security.yml` and `windows-build.yml`).
- README documentation updates were included for badge and roadmap/status alignment.
- Container publishing path still targets both GHCR and Docker Hub and keeps multi-arch build intent (`linux/amd64`, `linux/arm64`).

## 2) Follow-up recommendations

- Pin third-party actions to full commit SHAs (not only major tags) for stronger supply-chain immutability.
- Keep least-privilege `permissions` review as part of recurring workflow maintenance.
- Add/confirm CI checks that validate workflow changes before merge (e.g., actionlint in CI, if adopted by maintainers).

## Online best-practice verification summary
A quick online validation against GitHub Actions security guidance confirms:
- Restrict token permissions where possible.
- Prefer pinning third-party actions to full-length commit SHAs for immutable references.
- Continue routine action dependency updates (Dependabot already aligns with this pattern).

Conclusion: this branch direction is valid and aligned with standard CI maintenance best practices, with SHA pinning as the highest-value remaining improvement.
