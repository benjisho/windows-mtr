# Distribution Model

## Canonical source of truth

`windows-mtr` uses **GitHub Releases** as the canonical binary source.

- Canonical artifact name: `windows-mtr-x86_64.zip`
- Canonical ZIP contents:
  - `mtr.exe`
  - `windows-mtr.exe`
  - `README.txt`
  - `SHA256SUM`
- Package managers are metadata/discovery layers that reference this ZIP and verify SHA256.

This avoids maintaining separate binary truths across channels.

## Channel matrix

| Channel | Readiness | Purpose | Artifact | Expected user command | Automation status | Limitations |
|---|---|---|---|---|---|---|
| GitHub Releases | supported / canonical | Primary Windows user install | `windows-mtr-x86_64.zip` + `SHA256SUM` | Extract ZIP, run `./mtr.exe --help` | Release workflow builds and validates ZIP | Requires manual download unless wrapped by package manager |
| WinGet | planned / manifest template prepared | Windows enterprise install/discovery | GitHub Release ZIP URL | `winget install --manifest .\\packaging\\winget` (local validate/test) | Local dry-run manifest update script only | No auto-submit to `microsoft/winget-pkgs` |
| Scoop | planned / manifest template prepared | Developer-friendly portable install | GitHub Release ZIP URL + hash | `scoop install .\\packaging\\scoop\\windows-mtr.json` | Local dry-run manifest update script only | Public bucket publication is manual |
| Chocolatey | planned / package template prepared | Enterprise automation compatibility | GitHub Release ZIP URL + checksum in tools script | `choco install windows-mtr.portable --source . -y` | Local dry-run template update script only | No auto-push to Chocolatey feed |
| crates.io | future / Rust developer channel | Source-based install for Rust users | crate source | `cargo install windows-mtr --locked` | Not published from CI | Compiles locally; not ideal for normal Windows users |
| cargo-binstall | future | Prebuilt binary install for Rust users | GitHub Release naming compatibility | `cargo binstall windows-mtr` | Deferred until artifact naming is stable | Not validated in release workflow yet |
| Docker/GHCR | optional/testing path | API/testing workflows, not primary Windows install | container image | `docker run --rm ghcr.io/benjisho/windows-mtr:latest --help` | GHCR publish exists; Docker Hub gated by secrets | Probe modes may require container NET_RAW/privileged capabilities |
| Homebrew tap | deferred | Potential future macOS/Linux channel | n/a | n/a | none | Deferred pending validated macOS/Linux runtime support |
| Snap | deferred | Potential Linux channel | n/a | n/a | none | Deferred pending validated Linux runtime support |
| `.deb` | deferred | Potential Debian/Ubuntu channel | n/a | n/a | none | Deferred pending validated Linux runtime support |
| `.rpm` | deferred | Potential RPM channel | n/a | n/a | none | Deferred pending validated Linux runtime support |

## Manual submission checklists

### WinGet (manual)
1. Run `scripts/release/update-winget-manifest.ps1 -Version <version> -ZipPath <zip>`.
2. Validate: `winget validate packaging/winget`.
3. Test install locally: `winget install --manifest packaging/winget`.
4. Submit PR manually to `microsoft/winget-pkgs`.

### Scoop (manual)
1. Run `scripts/release/update-scoop-manifest.ps1 -Version <version> -ZipPath <zip>`.
2. Validate locally: `scoop install .\\packaging\\scoop\\windows-mtr.json`.
3. Publish to custom bucket manually.

### Chocolatey (manual)
1. Run `scripts/release/update-chocolatey.ps1 -Version <version> -ZipPath <zip>`.
2. Package: `choco pack packaging/chocolatey/windows-mtr.portable.nuspec`.
3. Local test: `choco install windows-mtr.portable --source . -y`.
4. Push manually: `choco push <nupkg> --source https://push.chocolatey.org/`.

## Maintainer references

- WinGet manifests: <https://learn.microsoft.com/windows/package-manager/package/manifest>
- Microsoft MSIX packaging tools: <https://learn.microsoft.com/windows/msix/packaging-tool/tool-overview>
- Chocolatey package creation: <https://docs.chocolatey.org/en-us/create/create-packages/>
- Scoop manifest docs: <https://github.com/ScoopInstaller/Scoop/wiki/App-Manifests>
- crates.io / cargo install: <https://doc.rust-lang.org/cargo/commands/cargo-install.html>
- cargo-binstall: <https://github.com/cargo-bins/cargo-binstall>
- Docker publishing (GHCR): <https://docs.github.com/packages/working-with-a-github-packages-registry/working-with-the-container-registry>
- Homebrew taps (deferred): <https://docs.brew.sh/How-to-Create-and-Maintain-a-Tap>
- Snapcraft (deferred): <https://snapcraft.io/docs>
- Debian packaging notes (deferred): <https://www.debian.org/doc/manuals/maint-guide/>
- RPM packaging guide (deferred): <https://rpm-packaging-guide.github.io/>

## Philosophy

- GitHub Releases is the binary truth.
- WinGet/Scoop/Chocolatey should reference release artifacts, not replace them.
- Capability claims must align with `docs/capability-validation.md` and current release validation.
