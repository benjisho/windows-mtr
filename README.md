# Windows MTR

A Windows-native clone of Linux MTR — cross-platform Rust CLI that delivers ICMP/TCP/UDP traceroute & ping in a single portable executable (`mtr.exe`). Built by Benji Shohet (benjisho).

![Windows MTR Banner](https://via.placeholder.com/800x200.png?text=Windows+MTR)

![CI](.github.com/benjisho/windows-mtr/workflows/CI/badge.svg)

![Release](https://img.shields.io/github/v/release/benjisho/windows-mtr?label=release)

## Features

- **Identical to Linux mtr**: Same command-line flags, output formats, and behavior.
- **Cross-platform**: Works on Windows, Linux, and macOS.
- **Multiple protocols**: ICMP (default), TCP SYN, and UDP probing.
- **Rich TUI**: Beautiful terminal interface for live monitoring.
- **Report mode**: Generate static, one-shot reports identical to Linux mtr.
- **Single binary**: Just download and run, no installation required.
- **No admin rights needed on Windows**: Uses Windows ICMP API, no external dependencies like Npcap.

## Installation

### Windows

1. Download the latest `windows-mtr.zip` from [GitHub Releases](https://github.com/benjisho/windows-mtr/releases).
2. Extract the ZIP file.
3. Use either `mtr.exe` or `windows-mtr.exe` - both are identical.

### Build from Source

```
git clone https://github.com/benjisho/windows-mtr.git
cd windows-mtr
cargo build --release
```

The binary will be available at `target/release/mtr.exe` (Windows) or `target/release/mtr` (Linux/macOS).

## Usage Examples

### Basic ICMP Trace (default)

```
mtr 8.8.8.8
```

### TCP Mode (e.g., testing HTTPS connectivity)

```
mtr -T -P 443 example.com
```

### UDP Mode (e.g., testing DNS connectivity)

```
mtr -U -P 53 8.8.8.8
```

### Generate Static Report (10 pings)

```
mtr -c 10 -r 8.8.8.8
```

### Set Custom Interval and Timeout

```
mtr -i 0.5 -w 2.0 8.8.8.8
```

## Command-line Options

| Option | Description |
|--------|-------------|
| `-T` | Use TCP SYN for probes (default is ICMP) |
| `-U` | Use UDP for probes (default is ICMP) |
| `-P <port>` | Target port for TCP/UDP modes |
| `-r` | Report mode (no continuous updates) |
| `-c <count>` | Number of pings to send to each host |
| `-i <seconds>` | Time between ICMP ECHO requests |
| `-w <seconds>` | Maximum time to keep a probe alive |

## FAQ

### Do I need admin rights to run on Windows?

No, unlike many other network tools, Windows MTR does not require administrator privileges to run. It uses the Windows ICMP API, which is accessible to normal users.

### Do I need to install WinPcap or Npcap?

No, Windows MTR has no external dependencies. It runs as a single executable file.

### Is the output identical to Linux mtr?

Yes, we've taken great care to ensure that the output format, especially in report mode (`-r`), matches the Linux mtr output byte-for-byte.

## License

Apache License 2.0 - See [LICENSE](LICENSE) for details.

---

Built with [trippy](https://github.com/fujiapple852/trippy) and [ratatui](https://github.com/ratatui-org/ratatui).
