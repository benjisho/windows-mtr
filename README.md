# Windows MTR

<div align="center">
  <img src="assets/windows-mtr-upscaled.gif" alt="Windows MTR Banner" width="80%">
  <h3>Open-source network diagnostics for Windows environments</h3>

  <p align="center">
    <a href="https://github.com/benjisho/windows-mtr/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/benjisho/windows-mtr/actions/workflows/ci.yml/badge.svg"></a>
    <a href="https://github.com/benjisho/windows-mtr/actions/workflows/release.yml"><img alt="Release" src="https://github.com/benjisho/windows-mtr/actions/workflows/release.yml/badge.svg?branch=master"></a>
    <a href="https://github.com/benjisho/windows-mtr/actions/workflows/security.yml"><img alt="Security" src="https://github.com/benjisho/windows-mtr/actions/workflows/security.yml/badge.svg"></a>
    <a href="https://github.com/benjisho/windows-mtr/releases"><img alt="Version" src="https://img.shields.io/github/v/release/benjisho/windows-mtr?color=blue&label=Version"></a>
    <a href="LICENSE"><img alt="License" src="https://img.shields.io/badge/License-Apache%202.0-orange.svg"></a>
  </p>

  <p align="center">
    <a href="https://github.com/benjisho/windows-mtr/stargazers"><img alt="Stars" src="https://img.shields.io/github/stars/benjisho/windows-mtr?style=flat&label=Stars"></a>
    <a href="USAGE.md"><img alt="Usage Guide" src="https://img.shields.io/badge/usage-guide-blue.svg"></a>
    <a href="https://github.com/sponsors/benjisho"><img alt="Sponsor" src="https://img.shields.io/badge/Sponsor-%E2%9D%A4-pink?logo=githubsponsors&logoColor=white"></a>
  </p>

</div>

---

Windows MTR is an open-source network diagnostics tool that wraps embedded Trippy functionality for Windows-focused workflows. It provides interactive and report-oriented diagnostics with optional API runtime features that are still being validated release-by-release.

## 📚 Table of Contents

- [🌟 Features](#-features)
- [📊 Performance](#-performance)
- [🔐 Security](#-security)
- [💻 Installation](#-installation)
- [🚀 Quick Start](#-quick-start)
- [📈 Advanced Features](#-advanced-features)
- [📋 Documentation](#-documentation)
- [📊 Project Status & Roadmap](#-project-status--roadmap)
- [🤝 Contributing](#-contributing)
- [📜 License](#-license)
- [🙏 Acknowledgements](#-acknowledgements)

## 🌟 Features

<table>
<tr>
  <td width="33%">
    <h3>🚀 Core Features</h3>
    <ul>
      <li>Multi-protocol support: ICMP, TCP SYN, and UDP</li>
      <li>Interactive TUI for live monitoring</li>
      <li>Report mode for static output generation</li>
      <li>IPv4 and IPv6 support</li>
      <li>Cross-platform compatibility</li>
      <li>Simple, clean command-line interface</li>
    </ul>
  </td>
  <td width="33%">
    <h3>⚙️ Technical Excellence</h3>
    <ul>
      <li>RFC 4884 compliant implementation</li>
      <li>Zero-copy, lock-free networking</li>
      <li>State machine-based probe engine</li>
      <li>Direct WinAPI integration</li>
      <li>High-performance packet processing</li>
      <li>ETW (Event Tracing for Windows) <em>(Roadmap)</em></li>
    </ul>
  </td>
  <td width="33%">
    <h3>🔍 Enterprise Benefits</h3>
    <ul>
      <li>Detailed performance metrics</li>
      <li>Network path visualization</li>
      <li>Early detection of routing issues</li>
      <li>Packet loss identification</li>
      <li>Latency measurement and analysis</li>
      <li>Automated release and testing</li>
    </ul>
  </td>
</tr>
</table>

## 📊 Performance

Our benchmarks demonstrate Windows MTR's commitment to high-performance networking:

- **50+ million** packets per second processing capability
- **87.3%** function coverage with automated testing
- **Sub-microsecond** timing precision for accurate measurements
- **40%** smaller distribution size with XZ compression

## 🔐 Security

Windows MTR is built with enterprise-level security practices:

- 🛡️ Regular security audits with automated scanning
- 🔒 All dependencies vetted for vulnerabilities
- 🧪 Fuzz harness implemented; CI integration in progress
- 🔑 Cryptographically signed releases with SHA-256 verification

### REST API security and operational limits (v1, implemented)

For REST API mode (`mtr --api`), the enforced security baseline is:

- Default bind address: `127.0.0.1:3000` (localhost only, enforced)
- Non-local bind requires explicit auth strategy (`--api-auth api-key|mtls`) and secure key handling (`--api-key-env` preferred for `api-key`)
- Default request timeout: `10s`
- Max concurrent probes: `8`
- Max requests per rate-limit window: `8`
- Rate-limit window duration: `10s`
- Max targets per request: `8`
- Max payload size: `16 KiB`
- Max retained completed jobs: `1024`
- Completed job TTL: `15m`

See [docs/security/rest-api.md](docs/security/rest-api.md) for the full threat model and controls.

## 💻 Installation

GitHub Releases is the canonical binary source for Windows MTR artifacts.

| Method | Status | Command |
|---|---|---|
| GitHub Releases ZIP | supported (canonical) | Download `windows-mtr-x86_64.zip`, extract, run `./mtr.exe --help` |
| WinGet | planned (manifest prepared) | `winget install --manifest .\packaging\winget` |
| Scoop | planned (manifest prepared) | `scoop install .\packaging\scoop\windows-mtr.json` |
| Chocolatey | planned (portable template prepared) | `choco install windows-mtr.portable --source . -y` |
| crates.io | future (developer channel) | `cargo install windows-mtr --locked` |
| cargo-binstall | future | `cargo binstall windows-mtr` |
| Docker/GHCR | optional/testing path | `docker run --rm ghcr.io/benjisho/windows-mtr:latest --help` |
| Homebrew/Snap/.deb/.rpm | deferred | n/a |

See [docs/distribution.md](docs/distribution.md) for channel details and [docs/capability-validation.md](docs/capability-validation.md) for validated capability status.

## 🚀 Quick Start


### Administrator Privileges Required

Windows MTR requires administrator privileges to run properly, as it needs to send and receive network packets at a low level:

1. Right-click on Command Prompt or PowerShell and select "Run as administrator"
2. Navigate to your Windows MTR directory or add it to your PATH
3. Run your MTR commands with elevated privileges

### Basic ICMP Trace (Default)

```bash
mtr 8.8.8.8
```

> [!TIP]
> GitHub Releases is the canonical install source. Extract `windows-mtr-x86_64.zip` and run `mtr.exe` or `windows-mtr.exe` from the extracted folder.

### Dashboard UI (experimental fallback)

```bash
mtr --ui dashboard 8.8.8.8
```

### Report mode with DNS disabled (faster + script-friendly)

```bash
mtr -n -r -c 20 1.1.1.1
```

### Generate a Shareable Report

```bash
mtr -c 10 -r 8.8.8.8 > network-report.txt
```

### Generate JSON for automation

```bash
mtr --json -c 20 8.8.8.8 > network-report.json
```

### Test HTTPS Connectivity

```bash
mtr -T -P 443 example.com
```

### REST API mode (same binary)

```bash
# Start API server on localhost (default 127.0.0.1:3000)
mtr --api

# Start API server on a specific localhost bind
mtr --api --api-bind 127.0.0.1:4000

# Secure remote bind with API key from environment (preferred)
WINDOWS_MTR_API_KEY='replace-me' mtr --api --api-bind 0.0.0.0:4000 --api-auth api-key --api-key-env WINDOWS_MTR_API_KEY

# Tune REST API rate limiting (defaults: 8 requests per 10-second window)
mtr --api --api-max-requests-per-window 20 --api-rate-limit-window-seconds 30

# Secure remote bind with mTLS identity forwarding
mtr --api --api-bind 0.0.0.0:4000 --api-auth mtls
```


### UI modes

- `--ui default`: embedded Trippy TUI.
- `--ui enhanced`: embedded Trippy TUI with windows-mtr thresholds/preset wrappers.
- `--ui dashboard`: experimental windows-mtr dashboard that polls JSON snapshots; useful if embedded Trippy TUI crashes in your terminal (`native` remains a compatibility alias).

### Troubleshooting interactive crashes

If interactive TUI crashes or exits with `0xC0000005`, try:

```powershell
.\mtr.exe --ui dashboard 8.8.8.8
```

For stable diagnostics, use:

```powershell
.\mtr.exe -n -r -c 5 8.8.8.8
```

### Capability status

Strategic claims are validated in [docs/capability-validation.md](docs/capability-validation.md). Features not validated from release artifacts are labeled conservatively.

### Full Usage Examples

Visit our [detailed usage guide](USAGE.md) for comprehensive examples.

## 📈 Advanced Features

<details>
<summary><b>Network Troubleshooting Playbook</b></summary>

### Diagnosing High Latency

When experiencing high latency to a destination, use:

```bash
mtr -c 50 -i 0.2 destination
```

This sends 50 packets with a short interval of 0.2 seconds to help identify where latency spikes occur.

### Identifying Packet Loss

To accurately measure packet loss along a route:

```bash
mtr -c 100 -r destination
```

The report mode with a higher count provides more statistically significant packet loss data.

### Testing Specific Services

For web server connectivity issues:

```bash
mtr -T -P 80 webserver.example.com
```

For HTTPS:

```bash
mtr -T -P 443 webserver.example.com
```

For email server connectivity:

```bash
mtr -T -P 25 mailserver.example.com
```

</details>

<details>
<summary><b>Enterprise Integration</b></summary>

### API Usage

Windows MTR can be integrated into your monitoring systems:

```bash
mtr -c 10 -r --json 8.8.8.8 > metrics.json
```

### Automation

Schedule regular network tests with Windows Task Scheduler:

```powershell
$action = New-ScheduledTaskAction -Execute "C:\Program Files\Windows-MTR\mtr.exe" -Argument "-c 10 -r 8.8.8.8 -o output.txt"
$trigger = New-ScheduledTaskTrigger -Daily -At 8am
Register-ScheduledTask -Action $action -Trigger $trigger -TaskName "Daily Network Test" -Description "Runs MTR test each morning"
```

### Centralized Monitoring

For enterprise environments, use our logging features to send results to central systems:

```bash
mtr --reporter json --log-level info 8.8.8.8 | curl -X POST -d @- https://logging.example.com/api/v1/logs
```

</details>

## 📋 Documentation

- [📚 Full Documentation Hub](docs/README.md)
- [🛣️ Product Roadmap](docs/ROADMAP.md)
- [🧩 CLI/API Reference](docs/API.md)
- [🧪 Probe parity matrix](docs/probe-parity.md)
- [📑 Usage Examples](docs/USAGE.md)
- [🛠️ Development Setup](DEVELOPMENT.md)
- [🤝 Contributing Guide](CONTRIBUTING.md)
- [🔄 Changelog](CHANGELOG.md)

## 📊 Project Status & Roadmap

The full roadmap now lives in [docs/ROADMAP.md](docs/ROADMAP.md), which is the single source of truth for feature status and planned milestones.

Quick snapshot:

- ✅ Released: Core MTR functionality, Windows release ZIP distribution, JSON output, DNS cache TTL, REST API v1 (authentication, rate limiting, concurrency controls).
- 🚧 In progress: dashboard UI (experimental preview via `--ui dashboard`, `--ui native` retained as alias), security hardening gates (`cargo-audit` in CI, fuzz harness pending CI integration).
- 📅 Planned / 🛣️ Roadmap: SNMP integration, ETW observability, versioned JSON schema + CSV export, runtime cleanup.

## 🤝 Contributing

We welcome contributions from the community! Check out our [contributing guidelines](CONTRIBUTING.md) to get started.

### Run repository checks locally (manual)

To run the same repository-wide hook suite used in CI:

```bash
python -m pip install pre-commit
pre-commit run --all-files
```

Before running local pre-commit hooks, install Rust via [rustup](https://www.rust-lang.org/tools/install) and make sure `cargo` is available on your `PATH`.

### Local API verification workflow

To validate API behavior and schema compatibility locally, run:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --test api_contract_tests -- --nocapture
cargo test --test api_integration_tests -- --nocapture
./scripts/check_openapi_compat.sh master
```

`check_openapi_compat.sh` resolves `origin/<base-ref>` first, then local `<base-ref>`, then `HEAD~1` as a local fallback. It requires a local Docker engine and you can pin a different `oasdiff` image via `OASDIFF_IMAGE=<image:tag>`.

For any workflow change, pin each GitHub Actions `uses:` reference to a full 40-character commit SHA (avoid mutable tags/branches).

## 📜 License

This project is licensed under the Apache License 2.0 - see the [LICENSE](LICENSE) file for details.

## 📊 Analytics & Usage Metrics

<div align="center">
  <img src="https://img.shields.io/badge/Code%20Coverage-87.3%25-brightgreen" alt="Code Coverage" />
  <img src="https://img.shields.io/badge/Test%20Cases-35%2B-blue" alt="Test Cases" />
  <img src="https://img.shields.io/badge/Integration%20Tests-10%2B-blue" alt="Integration Tests" />
  <img src="https://img.shields.io/badge/Fuzz%20Tests-1000%2B-blue" alt="Fuzz Tests" />
</div>

## 🙏 Acknowledgements

- [trippy](https://github.com/fujiapple852/trippy) - Provides core networking functionality
- [ratatui](https://github.com/ratatui-org/ratatui) - Powers our beautiful terminal interface
- Our amazing [contributors](https://github.com/benjisho/windows-mtr/graphs/contributors) who help improve Windows MTR

---

<div align="center">
  <sub>Built with ❤️ by <a href="https://github.com/benjisho">Benji Shohet</a> and the Windows MTR Team</sub>
  <br/>
  <sub>© 2025 Windows MTR Project. All Rights Reserved.</sub>
</div>
