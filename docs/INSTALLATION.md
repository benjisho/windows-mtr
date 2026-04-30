# Installation

## Primary supported path: GitHub Releases

1. Download `windows-mtr-x86_64.zip` from the project releases page.
2. Verify checksum with `SHA256SUM`.
3. Extract and run one of:

```powershell
.\mtr.exe 8.8.8.8
.\windows-mtr.exe -r -c 10 8.8.8.8
```

## Package manager readiness (not yet published)

- WinGet: template manifests prepared in `packaging/winget/`
- Scoop: template manifest prepared in `packaging/scoop/windows-mtr.json`
- Chocolatey: portable template prepared in `packaging/chocolatey/`

All packaging channels should reference the same GitHub Release ZIP.

For status, see:
- [docs/distribution.md](distribution.md)
- [docs/capability-validation.md](capability-validation.md)
