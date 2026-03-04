# GitHub Actions Trigger and Execution Order Review

This review explains **what triggers each workflow** and **how they run relative to one another on the same event**.

## Key rule: cross-workflow ordering

There is currently **no cross-workflow dependency** (`workflow_run`, reusable workflow chaining, or external orchestrator) between workflows in this repository. As a result:

- When multiple workflows match one event, GitHub queues them independently.
- They typically start around the same time (runner availability dependent).
- Their completion order is non-deterministic.
- `concurrency` in `release.yml` only serializes runs of the `Release` workflow itself, not other workflows.

## Trigger matrix

| Workflow | Manual (`workflow_dispatch`) | `push` to `master` | `pull_request` to `master` | Tag push | Scheduled |
|---|---:|---:|---:|---:|---:|
| `ci.yml` | ✅ | ✅ (ignores `*.md` / `.gitignore`) | ✅ (ignores `*.md` / `.gitignore`) | ❌ | ❌ |
| `codeql.yml` | ✅ | ✅ (ignores `*.md` / `.gitignore`) | ✅ (ignores `*.md` / `.gitignore`) | ❌ | ✅ (`20 21 * * 3`) |
| `security.yml` | ✅ | ✅ (`master` + path allowlist) | ✅ (`master` + path allowlist) | ❌ | ❌ |
| `docker-scout.yml` | ❌ | ✅ (`master`) | ✅ (all PR branches) | ✅ (`v*`) | ❌ |
| `release.yml` | ✅ | ✅ (`master`, ignores docs/.gitignore) | ❌ | ✅ (`v*.*.*`) | ❌ |
| `windows-build.yml` | ✅ | ❌ | ❌ | ✅ (`v*`) | ❌ |

## Intra-workflow job order (deterministic)

### `ci.yml`
- `resolve-msrv` runs first.
- `test-msrv` waits for `resolve-msrv`.
- `test-stable` waits for both `resolve-msrv` and `test-msrv`, and only runs when MSRV != stable.
- `docker-build-amd64` is independent and can run in parallel with the Rust test jobs.
- `build-pr-binaries` only runs on PR events and waits for `test-msrv`, `test-stable`, and `docker-build-amd64`, with explicit success/skip guards.

### `release.yml`
- `build` runs first.
- `publish-package` depends on `build` and only executes on push events to `master` or tags.
- `release` depends on both `build` and `publish-package`, and runs only for tag pushes.

### `windows-build.yml`
- `build-windows` runs first.
- `release` waits for `build-windows` and runs only when ref is a tag.

### `codeql.yml`
- One job (`analyze`) with a matrix for `actions` and `rust`; matrix entries run in parallel.

### `security.yml` and `docker-scout.yml`
- Each has one top-level job and therefore no internal DAG complexity.

## Same-event execution review

### 1) Pull request to `master` (code change in `src/**`)
Triggered workflows:
- `CI`
- `CodeQL`
- `Security` (path match)
- `Docker Scout`

Relative ordering:
- All four workflows queue independently and may overlap.
- `CI/build-pr-binaries` runs late inside CI due to `needs`, but this does not block or sequence the other workflows.

### 2) Pull request to `master` (docs-only change)
Triggered workflows:
- `Docker Scout` (no path filter)

Not triggered:
- `CI` and `CodeQL` (both ignore markdown-only diffs)
- `Security` (path allowlist does not include docs)

### 3) Push to `master` (non-doc code change)
Triggered workflows:
- `CI`
- `CodeQL`
- `Release`
- `Docker Scout`
- `Security` (if changed files match allowlist)

Relative ordering:
- No guaranteed order across workflows.
- `Release/publish-package` will not start until `Release/build` passes.

### 4) Tag push `vX.Y.Z`
Triggered workflows:
- `Release` (`v*.*.*`)
- `Windows Build and Release` (`v*`)
- `Docker Scout` (`v*`)

Relative ordering:
- All three workflows can start together.
- There is no coordination between `release.yml` and `windows-build.yml` despite both creating GitHub releases.

## Critical ordering/race findings

1. **Potential dual-release race on tags**
   - `release.yml` and `windows-build.yml` both trigger on version tags and both call `softprops/action-gh-release`.
   - This can cause duplicate release creation attempts or asset upload conflicts depending on timing.

2. **No global sequencing between quality and release workflows**
   - `release.yml` does not depend on completion of `ci.yml`, `security.yml`, or `codeql.yml`.
   - A push to `master` can publish container images from `Release` even while other workflows are still running.

3. **Path-filter asymmetry causes intentional but surprising behavior**
   - Docs-only PRs still run `Docker Scout` while `CI`, `CodeQL`, and `Security` skip.

## Recommendations (SRE/DevOps)

1. Pick a **single release authority** for tag events:
   - Keep `release.yml` for full release and make `windows-build.yml` manual-only, or
   - Keep `windows-build.yml` for draft releases and remove tag release creation from `release.yml`.

2. If policy requires quality gates before release publication, orchestrate with:
   - `workflow_run` chaining, or
   - branch protections + required status checks + protected tagging process.

3. Align path filters intentionally:
   - Either add path filters to `docker-scout.yml` for docs-only PR efficiency,
   - or explicitly document why image scanning is desired for all PRs.

4. Optional: add repository-level concurrency groups for release-related workflows to avoid overlapping tag-release attempts.


## Implemented changes observed on this branch

After re-reading the current branch workflows, these implementation updates are now in place:

1. **Docker Scout auth guard for fork/secretless contexts**
   - In `docker-scout.yml`, registry login is now conditional (`if: secrets.REGISTRY_USER != '' && secrets.REGISTRY_TOKEN != ''`), so PRs without registry secrets no longer hard-fail at login.

2. **CodeQL upload behavior is explicitly disabled in this workflow**
   - In `codeql.yml`, `github/codeql-action/analyze` is configured with `upload: false`, keeping this workflow compatible with repositories using Code Scanning default setup and avoiding duplicate SARIF upload patterns.

3. **Core ordering risks are still present**
   - The tag dual-publisher risk remains (`release.yml` + `windows-build.yml`).
   - Cross-workflow quality gating before publish is still not enforced by orchestration.

### Optimization decision (current state)

- **Mandatory optimization:** consolidate to a single tag-release publisher (highest risk reduction).
- **Strongly recommended:** add explicit release gating policy (required checks and protected tagging, or `workflow_run` orchestration).
- **Optional optimization:** path-gate Docker Scout on PRs if CI cost/latency matters more than always-on image comparison.

## Industry best-practice alignment check (online benchmark)

This section was cross-checked against GitHub official guidance and representative open-source Rust projects.

### Sources reviewed

Official GitHub references:
- Events and trigger behavior: https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows
- Concurrency controls: https://docs.github.com/en/actions/using-jobs/using-concurrency
- Token permission hardening (`GITHUB_TOKEN` least privilege): https://docs.github.com/en/actions/security-guides/automatic-token-authentication
- Reusable workflows / orchestration options: https://docs.github.com/en/actions/reference/workflows-and-actions/reusing-workflow-configurations

Comparable project workflows:
- ripgrep CI: https://raw.githubusercontent.com/BurntSushi/ripgrep/master/.github/workflows/ci.yml
- bat CICD: https://raw.githubusercontent.com/sharkdp/bat/master/.github/workflows/CICD.yml
- tokio CI: https://raw.githubusercontent.com/tokio-rs/tokio/master/.github/workflows/ci.yml

### Where this repository aligns well

1. **Least-privilege defaults are generally applied**
   - Several workflows explicitly set narrow `permissions`, which is consistent with GitHub security guidance and patterns used by mature projects.

2. **Path filtering is used to reduce unnecessary runs**
   - `paths`/`paths-ignore` is a common optimization and is widely used across larger repositories to control CI cost and queue pressure.

3. **Intra-workflow DAGs are explicit with `needs`**
   - Existing job dependencies are clear and predictable inside each workflow, matching common CI graph design in projects like `tokio` and `bat`.

### Where this repository diverges from common best practice

1. **Two workflows can publish GitHub releases from the same tag event**
   - Most mature projects keep a single release authority per tag to avoid race conditions and ambiguous release ownership.

2. **No cross-workflow quality gate before publish**
   - A common pattern is to gate publish/release via required checks and/or explicit orchestration, rather than relying on independent workflow completion timing.

3. **No shared concurrency guard across release-capable workflows**
   - Concurrency is scoped to `release.yml`; a broader strategy is often used where multiple workflows can mutate the same release assets.

4. **Docker Scout runs on docs-only PRs**
   - This may be intentional for supply-chain visibility, but in many projects image-focused jobs are path-gated to container/build-relevant changes.

### Practical target state (recommended)

- Keep one tag-driven release publisher.
- Add explicit release gating policy (required checks + protected tagging and/or chained workflow orchestration).
- Add cross-workflow concurrency policy for release/tag mutation paths.
- Document intentional exceptions (for example always-on Docker Scout) to reduce maintainer confusion.
