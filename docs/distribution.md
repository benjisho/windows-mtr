# Distribution Plan

## Source of truth

`windows-mtr` uses **one canonical binary source**:

1. Build once in GitHub Actions.
2. Publish one canonical portable artifact: `windows-mtr-x86_64.zip`.
3. Include `mtr.exe`, `windows-mtr.exe`, `README.txt`, and `SHA256SUM`.
4. Package managers reference that artifact URL + SHA256 (no separate binary forks).

See [capability validation](./capability-validation.md) for current readiness levels.

## Channel matrix

| Channel | Readiness | Purpose | Artifact | Install command | Automation status | Known limitations |
|---|---|---|---|---|---|---|
| GitHub Releases | **Supported / Canonical** | Primary Windows install path | `windows-mtr-x86_64.zip` | Download, unzip, run `.\mtr.exe` | release workflow builds/validates artifact | Requires manual download unless package manager used |
| WinGet | Planned (manifest prepared) | Windows package index/discovery | GitHub release ZIP URL + SHA256 | `winget install BenjiShohet.WindowsMTR` (after publish) | local manifest + update script; no auto-submit | Not published until maintainer PR to `winget-pkgs` |
| Scoop | Planned (manifest prepared) | Dev/admin-friendly portable install | GitHub release ZIP URL + SHA256 | `scoop install .\packaging\scoop\windows-mtr.json` (local) | local manifest + update script | Not yet in public bucket |
| Chocolatey | Planned (template prepared) | Enterprise automation / legacy Windows ecosystems | GitHub release ZIP + checksum verification | `choco install windows-mtr.portable --source . -y` (local package) | local template + update script | Not yet pushed to community feed |
| crates.io | Future | Rust developer install path | crate source build | `cargo install windows-mtr --locked` | not automated here | compiles locally; not ideal default for Windows end users |
| cargo-binstall | Future | Rust dev prebuilt binary install | GitHub release naming conventions | `cargo binstall windows-mtr` (future) | deferred pending stable artifact naming | depends on release asset convention stability |
| Docker/GHCR | Secondary/optional | containerized runtime/testing | container images | `docker run --rm ghcr.io/benjisho/windows-mtr:latest --help` | workflow exists when configured | raw probing may require NET_RAW/privileged modes |
| Homebrew tap | Deferred | potential macOS/Linux distribution | n/a | n/a | deferred | macOS/Linux runtime validation incomplete |
| Snap | Deferred | Linux package distribution | n/a | n/a | deferred | Linux runtime validation incomplete |
| .deb | Deferred | Debian/Ubuntu packages | n/a | n/a | deferred | Linux runtime validation incomplete |
| .rpm | Deferred | RHEL/Fedora packages | n/a | n/a | deferred | Linux runtime validation incomplete |

## Maintainer workflows

### WinGet local validation

```powershell
winget validate .\packaging\winget
winget install --manifest .\packaging\winget
```

Manual submission checklist:
1. Run release and obtain actual ZIP SHA256.
2. Run `scripts/release/update-winget-manifest.ps1`.
3. Validate manifests locally.
4. Submit PR to `microsoft/winget-pkgs` manually.

### Scoop local validation

```powershell
scoop install .\packaging\scoop\windows-mtr.json
```

Future bucket flow (after bucket exists):

```powershell
scoop bucket add benjisho https://github.com/benjisho/scoop-bucket
scoop install windows-mtr
```

### Chocolatey local validation

```powershell
choco pack .\packaging\chocolatey\windows-mtr.portable.nuspec
choco install windows-mtr.portable --source . -y
choco push windows-mtr.portable.<VERSION>.nupkg --source https://push.chocolatey.org/
```

## Official references

- WinGet manifests and packaging: <https://learn.microsoft.com/windows/package-manager/package/manifest>
- Microsoft MSIX/packaging tooling guidance: <https://learn.microsoft.com/windows/msix/>
- Chocolatey package authoring: <https://docs.chocolatey.org/en-us/create/create-packages/>
- Scoop manifest reference: <https://github.com/ScoopInstaller/Scoop/wiki/App-Manifests>
- crates.io / cargo install: <https://doc.rust-lang.org/cargo/commands/cargo-install.html>
- cargo-binstall: <https://github.com/cargo-bins/cargo-binstall>
- Docker image publishing (GHCR): <https://docs.github.com/packages/working-with-a-github-packages-registry/working-with-the-container-registry>
- Homebrew taps/formulas (deferred): <https://docs.brew.sh/How-to-Create-and-Maintain-a-Tap>
- Snapcraft docs (deferred): <https://snapcraft.io/docs>
- Debian packaging notes (deferred): <https://www.debian.org/doc/manuals/maint-guide/>
- RPM packaging guide (deferred): <https://rpm.org/docs/>
