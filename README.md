# Windows MTR

Windows-focused MTR-style network diagnostics CLI with embedded Trippy runtime.

## Installation

> Canonical binary source: **GitHub Releases**.

Download `windows-mtr-x86_64.zip` from [GitHub Releases](https://github.com/benjisho/windows-mtr/releases), extract it, then run:

```powershell
.\mtr.exe 8.8.8.8
.\windows-mtr.exe -r -c 10 8.8.8.8
```

The ZIP contains exactly:
- `mtr.exe`
- `windows-mtr.exe`
- `README.txt`
- `SHA256SUM`

### Distribution matrix

| Method | Status | Command |
|---|---|---|
| GitHub Releases | supported (canonical) | Download ZIP, extract, run `.\mtr.exe` |
| WinGet | planned (manifest prepared) | `winget install --manifest .\packaging\winget` |
| Scoop | planned (manifest prepared) | `scoop install .\packaging\scoop\windows-mtr.json` |
| Chocolatey | planned (template prepared) | `choco pack` then local test install |
| crates.io | future | `cargo install windows-mtr --locked` |
| cargo-binstall | future | deferred until release naming is stable |
| Docker/GHCR | partial / optional | `docker run --rm ghcr.io/benjisho/windows-mtr:latest --help` |
| Homebrew / Snap / .deb / .rpm | deferred | pending Linux/macOS runtime validation |

See [docs/distribution.md](docs/distribution.md) for maintainer details.

## UI modes

- `--ui default`: embedded Trippy interactive TUI.
- `--ui enhanced`: embedded Trippy TUI + windows-mtr diagnostic thresholds/overlays.
- `--ui dashboard`: **experimental** windows-mtr dashboard that polls JSON snapshots; useful if embedded Trippy TUI crashes in your terminal.
- `--ui native`: deprecated compatibility alias for `--ui dashboard`.

## Quick start

```powershell
# interactive default TUI
mtr 8.8.8.8

# enhanced interactive TUI
mtr --ui enhanced 8.8.8.8

# dashboard fallback mode
mtr --ui dashboard 8.8.8.8

# stable report mode
mtr -n -r -c 5 8.8.8.8
```

### Troubleshooting

If interactive TUI crashes or exits with `0xC0000005`, try:

```powershell
.\mtr.exe --ui dashboard 8.8.8.8
```

For stable diagnostics in automation/scripts:

```powershell
.\mtr.exe -n -r -c 5 8.8.8.8
```

## Capability status

Strategic capability claims are validated in:
- [docs/capability-validation.md](docs/capability-validation.md)

Use that matrix as source-of-truth for what is **Full**, **Strong**, **Partial**, **Basic**, **Not implemented**, or **Roadmap only**.

## Docs

- [USAGE.md](USAGE.md)
- [docs/distribution.md](docs/distribution.md)
- [docs/capability-validation.md](docs/capability-validation.md)
- [docs/security/rest-api.md](docs/security/rest-api.md)

## License

Apache-2.0. See [LICENSE](LICENSE).
