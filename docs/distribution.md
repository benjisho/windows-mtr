# Distribution model

## Canonical release model

**Source of truth:** GitHub Releases  
**Canonical artifact:** `windows-mtr-x86_64.zip`

Canonical ZIP contents:
- `mtr.exe`
- `windows-mtr.exe`
- `README.txt`
- `SHA256SUM`

Package managers must point to this ZIP and verify SHA256. They should not become separate binary sources.

See also: [docs/capability-validation.md](capability-validation.md).

## Channel status

| Channel | Readiness | Artifact required | Install command | Automation status | Manual steps | Known limitations |
|---|---|---|---|---|---|---|
| GitHub Releases | Supported / canonical | `windows-mtr-x86_64.zip`, `SHA256SUM` | download + extract | automated in release workflow | publish tag release | Windows-focused path |
| WinGet | Planned / manifest template prepared | canonical ZIP URL + SHA256 | `winget install --manifest .\packaging\winget` | local update script only | manual PR to `microsoft/winget-pkgs` | not auto-submitted |
| Scoop | Planned / manifest template prepared | canonical ZIP URL + hash | `scoop install .\packaging\scoop\windows-mtr.json` | local update script only | optional future bucket publish | not live in public bucket |
| Chocolatey | Planned / portable template prepared | canonical ZIP URL + checksum metadata | `choco pack` then local install | local update script only | manual push to chocolatey.org | not published yet |
| crates.io | Future developer channel | source crate | `cargo install windows-mtr --locked` | none | requires release readiness review | compiles locally; not ideal for normal Windows users |
| cargo-binstall | Future | stable GitHub artifact naming | `cargo binstall windows-mtr` (future) | none | enable after artifact naming stability | deferred |
| Docker/GHCR | Optional / partial | container image | `docker run --rm ghcr.io/benjisho/windows-mtr:latest --help` | workflow exists | optional manual Docker Hub setup | raw probe modes may need NET_RAW/privileged capabilities |
| Homebrew tap | Deferred | N/A | N/A | none | defer until macOS/Linux runtime validation | avoid `mtr` naming conflict |
| Snap | Deferred | N/A | N/A | none | defer until Linux runtime validation | avoid premature support claims |
| .deb | Deferred | N/A | N/A | none | defer until Linux runtime validation | avoid `/usr/bin/mtr` collision |
| .rpm | Deferred | N/A | N/A | none | defer until Linux runtime validation | avoid `/usr/bin/mtr` collision |

## WinGet maintainer notes

Templates:
- `packaging/winget/windows-mtr.yaml`
- `packaging/winget/windows-mtr.installer.yaml`
- `packaging/winget/windows-mtr.locale.en-US.yaml`

Validation/testing:
```powershell
winget validate .\packaging\winget
winget install --manifest .\packaging\winget
```

Update helper:
```powershell
scripts/release/update-winget-manifest.ps1 -Version <x.y.z> -Sha256 <SHA256>
```

## Scoop maintainer notes

Manifest:
- `packaging/scoop/windows-mtr.json`

Local test:
```powershell
scoop install .\packaging\scoop\windows-mtr.json
```

Future bucket flow:
```powershell
scoop bucket add benjisho https://github.com/benjisho/scoop-bucket
scoop install windows-mtr
```

Update helper:
```powershell
scripts/release/update-scoop-manifest.ps1 -Version <x.y.z> -Sha256 <SHA256>
```

## Chocolatey maintainer notes

Templates:
- `packaging/chocolatey/windows-mtr.portable.nuspec`
- `packaging/chocolatey/tools/VERIFICATION.txt.template`

Local workflow:
```powershell
choco pack
choco install windows-mtr.portable --source . -y
choco push windows-mtr.portable.<VERSION>.nupkg --source https://push.chocolatey.org/
```

Update helper:
```powershell
scripts/release/update-chocolatey.ps1 -Version <x.y.z> -Sha256 <SHA256>
```

## Official references

- WinGet packaging docs: https://learn.microsoft.com/windows/package-manager/package/
- WinGet manifest schema docs: https://learn.microsoft.com/windows/package-manager/package/manifest
- MSIX packaging guidance: https://learn.microsoft.com/windows/msix/
- Chocolatey package creation: https://docs.chocolatey.org/en-us/create/create-packages/
- Scoop manifest docs: https://github.com/ScoopInstaller/Scoop/wiki/App-Manifests
- crates.io/cargo install docs: https://doc.rust-lang.org/cargo/commands/cargo-install.html
- cargo-binstall docs: https://github.com/cargo-bins/cargo-binstall
- Docker image publishing docs: https://docs.github.com/packages/working-with-a-github-packages-registry/working-with-the-container-registry
- Homebrew formula docs (deferred): https://docs.brew.sh/Formula-Cookbook
- Snapcraft docs (deferred): https://snapcraft.io/docs
- Debian packaging notes (deferred): https://www.debian.org/doc/manuals/maint-guide/
- RPM packaging guide (deferred): https://rpm.org/docs/
