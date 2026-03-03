# Installation Guide

This guide covers recommended installation methods for Windows MTR.

## Table of Contents

- [System Requirements](#system-requirements)
- [Install from GitHub Releases](#install-from-github-releases)
- [Portable Installation](#portable-installation)
- [Build from Source](#build-from-source)
- [Verification](#verification)
- [Uninstall](#uninstall)
- [Troubleshooting](#troubleshooting)

## System Requirements

- Windows 7 / Server 2012 R2 or later (recommended: modern supported Windows versions)
- Administrator privileges for full probe capabilities
- ~50 MB free disk space for binaries and dependencies

## Install from GitHub Releases

Recommended for most users.

1. Open the [latest release page](https://github.com/benjisho/windows-mtr/releases).
2. Download the MSI package for your architecture.
3. Run installer as Administrator.
4. Verify from terminal:

```powershell
mtr --help
```

## Portable Installation

1. Download `windows-mtr.zip` (or compressed variant).
2. Extract to a folder, e.g. `C:\Tools\windows-mtr`.
3. Run directly:

```powershell
.\mtr.exe --help
```

Optional: add folder to `PATH`.

## Build from Source

### Prerequisites

- Rust (1.88.0+)
- Visual Studio Build Tools (Desktop development with C++)

### Build steps

```bash
git clone https://github.com/benjisho/windows-mtr.git
cd windows-mtr
cargo build --release
```

Binary output:

- `target\release\mtr.exe`

## Verification

Run a quick report test:

```powershell
mtr -r -c 5 8.8.8.8
```

JSON verification:

```powershell
mtr --json -c 5 1.1.1.1
```

## Uninstall

- MSI install: remove via **Apps & Features**.
- Portable install: delete extracted folder and any PATH entry.

## Troubleshooting

### Command not found

- Restart terminal after modifying PATH.
- Use full path to `mtr.exe` to verify binary is valid.

### Insufficient privileges

- Launch terminal as Administrator and rerun command.

### Endpoint filtering blocks probes

- Validate local firewall/network security policy.
- Try different protocol mode (`-T` or `-U`) for diagnostics.

For operational usage details, see [USAGE.md](USAGE.md).
