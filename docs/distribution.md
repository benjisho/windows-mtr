# Distribution Plan

## Source of truth

`windows-mtr` uses **GitHub Releases** as the canonical binary source.

- Canonical artifact host: GitHub Releases
- Canonical artifact name: `windows-mtr-x86_64.zip`
- Canonical ZIP contents: `mtr.exe`, `windows-mtr.exe`, `README.txt`, `SHA256SUM`
- Package managers are metadata/discovery layers that reference this ZIP and verify SHA256.

## Channel matrix

| Channel | Readiness | Purpose | Artifact required | Install command | Release automation | Manual steps | Limitations |
|---|---|---|---|---|---|---|---|
| GitHub Releases | Supported / canonical | Primary user distribution | `windows-mtr-x86_64.zip` | Extract ZIP, run `.\mtr.exe` | Implemented in release workflow + `scripts/release/verify-release-artifacts.ps1` | Publish tag release | Windows-first artifact only |
| WinGet | Planned / manifest template prepared | Windows package index/discovery | Canonical ZIP + SHA256 | `winget install --manifest .\packaging\winget` (local) | Local dry-run update script only | Manual PR to `microsoft/winget-pkgs` | Not auto-submitted |
| Scoop | Planned / manifest template prepared | Lightweight portable Windows install | Canonical ZIP + SHA256 | `scoop install .\packaging\scoop\windows-mtr.json` | Local dry-run update script only | Optional future bucket PR | Not published yet |
| Chocolatey | Planned / template prepared | Enterprise Windows automation channel | Canonical ZIP + SHA256 | `choco pack` then install local package | Local dry-run update script only | Manual push to Chocolatey community feed | Not auto-published |
| crates.io | Future | Rust developer channel | Rust crate source | `cargo install windows-mtr --locked` | Not enabled | Publish crate when metadata/release readiness is complete | Requires local Rust compilation |
| cargo-binstall | Future | Fast Rust developer binary install | Stable release naming + checksums | deferred | Not enabled | Enable after naming/checksum conventions are stable | Depends on release naming conventions |
| Docker/GHCR | Optional/partial | Container-based runtime/testing | Container image | `docker run --rm ghcr.io/benjisho/windows-mtr:latest --help` | Existing workflow publishes container tags | None if using published tags | Raw probes may need NET_RAW/privileged mode |
| Homebrew tap | Deferred | macOS/Linux package channel | Deferred | deferred | Not enabled | deferred | Deferred until runtime validation |
| Snap | Deferred | Linux package channel | Deferred | deferred | Not enabled | deferred | Deferred until runtime validation |
| `.deb` | Deferred | Linux distro package | Deferred | deferred | Not enabled | deferred | Deferred until runtime validation |
| `.rpm` | Deferred | Linux distro package | Deferred | deferred | Not enabled | deferred | Deferred until runtime validation |

## Maintainer workflow

1. Build release binary and package canonical ZIP.
2. Run `scripts/release/verify-release-artifacts.ps1`.
3. Update package-manager metadata locally:
   - `scripts/release/update-winget-manifest.ps1`
   - `scripts/release/update-scoop-manifest.ps1`
   - `scripts/release/update-chocolatey.ps1`
4. Validate manifests locally (no auto-submit).
5. Publish release; then submit package manager PRs manually.

## WinGet checklist (manual)

- Update manifests under `packaging/winget/` with release version/url/hash.
- Validate: `winget validate <manifest-folder>`
- Local install test: `winget install --manifest <manifest-folder>`
- Open PR to `microsoft/winget-pkgs` manually.

## Scoop checklist (manual)

- Update `packaging/scoop/windows-mtr.json` with version/url/hash.
- Local install test: `scoop install .\packaging\scoop\windows-mtr.json`
- Future bucket flow:
  - `scoop bucket add benjisho https://github.com/benjisho/scoop-bucket`
  - `scoop install windows-mtr`

## Chocolatey checklist (manual)

- Update nuspec/template values via script.
- Build package: `choco pack`
- Test local source: `choco install windows-mtr.portable --source . -y`
- Push when ready: `choco push windows-mtr.portable.<VERSION>.nupkg --source https://push.chocolatey.org/`

## Reference links

- WinGet manifests: <https://learn.microsoft.com/windows/package-manager/package/manifest>
- WinGet create/submit flow: <https://learn.microsoft.com/windows/package-manager/package/>
- MSIX CLI packaging guidance: <https://learn.microsoft.com/windows/msix/packaging-tool/tool-overview>
- Chocolatey package creation: <https://docs.chocolatey.org/en-us/create/create-packages/>
- Scoop manifest docs: <https://github.com/ScoopInstaller/Scoop/wiki/App-Manifests>
- crates.io/cargo install: <https://doc.rust-lang.org/cargo/commands/cargo-install.html>
- cargo-binstall docs: <https://github.com/cargo-bins/cargo-binstall>
- Docker publish docs: <https://docs.docker.com/build/ci/github-actions/>
- Homebrew formula docs (deferred): <https://docs.brew.sh/Formula-Cookbook>
- Snapcraft docs (deferred): <https://snapcraft.io/docs>
- Debian packaging notes (deferred): <https://www.debian.org/doc/manuals/maint-guide/>
- RPM packaging guide (deferred): <https://rpm.org/docs/latest/manual/packaging-guide.html>

See also: [docs/capability-validation.md](capability-validation.md).
