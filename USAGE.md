# Windows MTR Usage Guide

## Basic Usage

```bash
mtr [options] <hostname-or-ip>
```

> On Windows portable downloads, replace `mtr` with your actual executable name (for example `windows-mtr-x86_64.exe`).

## Core Options

| Option | Description |
|---|---|
| `<hostname-or-ip>` | Target host to trace (required) |
| `-T` | TCP SYN probes |
| `-U` | UDP probes |
| `-P, --port <PORT>` | Target port for TCP/UDP (`-T`/`-U`) |
| `--source-port <PORT>` | Source port for TCP/UDP |
| `-S, --src <IP>` | Source IP address |
| `--interface <NAME>` | Source interface |
| `-m <HOPS>` | Max TTL/hops |
| `-s, --packet-size <BYTES>` | Probe packet size |

## Output & Report Options

| Option | Description |
|---|---|
| `-r` | One-shot report mode (pretty table) |
| `-w, --report-wide` | Wide report output mode |
| `-j, --json` | JSON report mode |
| `--json-pretty` | Pretty JSON report mode |
| `-c <COUNT>` | Probe/report cycles |
| `-n` | Disable reverse DNS rendering (show IP only) |
| `-b, --show-asn` | Enable ASN lookup/rendering |
| `-z` | DNS ASN lookup shortcut |
| `--ui <default\|enhanced\|native>` | Interactive UI preset (enhanced enables diagnostic overlays) |

## Native Ratatui UI

Use `--ui native` to run the built-in Ratatui interface with live hop data, a hop table, and charts.

```bash
mtr --ui native 8.8.8.8
```

Controls:
- `Tab` / `→` switch to next tab
- `←` switch to previous tab
- `q` quit

When probe snapshots fail repeatedly, both the help footer and the Hop table empty-state surface the latest poll error and live troubleshooting hints (run with Administrator privileges, review firewall policy, or try report mode with `-r`).

`--ui native` accepts the standard probe and output tuning options, and renders them in the native Ratatui view.

## Enhanced UI options

The `enhanced` preset applies defaults tuned for quicker incident triage:

- Latency color bands: `--latency-warn-ms 100`, `--latency-bad-ms 250`
- Loss color bands: `--loss-warn-pct 2`, `--loss-bad-pct 5`
- Row coloring: `--enhanced-row-color on`
- Per-hop trend/sparkline column: `--enhanced-sparklines on`
- Percentile + jitter summary area: `--enhanced-summary on`

| Option | Description |
|---|---|
| `--latency-warn-ms <MS>` | Warning threshold for per-hop latency coloring |
| `--latency-bad-ms <MS>` | Critical threshold for per-hop latency coloring |
| `--loss-warn-pct <PCT>` | Warning threshold for packet loss coloring |
| `--loss-bad-pct <PCT>` | Critical threshold for packet loss coloring |
| `--enhanced-row-color <on\|off>` | Enable/disable row band coloring in enhanced mode |
| `--enhanced-sparklines <on\|off>` | Enable/disable per-hop trend/sparkline column |
| `--enhanced-summary <on\|off>` | Enable/disable percentile and jitter summary area |

## Timing & DNS Cache

| Option | Description |
|---|---|
| `-i <SECONDS>` | Minimum round duration |
| `-W, --timeout <SECONDS>` | Probe grace timeout |
| `--dns-cache-ttl <SECONDS>` | Per-run DNS cache TTL |

## Power User Passthrough

| Option | Description |
|---|---|
| `--trippy-flags "<FLAGS>"` | Forwards native Trippy flags verbatim |
| `--ecmp <classic\|paris\|dublin>` | ECMP/multipath strategy |

## Linux `mtr` parity mapping (minimum viable)

| Linux mtr | windows-mtr | trippy |
|---|---|---|
| `-b` | `-b`, `--show-asn` | `--dns-lookup-as-info` |
| `-s` | `-s`, `--packet-size` | `--packet-size` |
| `-S` (source IP) | `-S`, `--src` | `--source-address` |
| `-z` | `-z` | `--dns-lookup-as-info` |
| `--ecmp` | `--ecmp` | `--multipath-strategy` |
| `-w` (report wide) | `-w`, `--report-wide` | `--mode pretty` |
| `-n` | `-n` | `--tui-address-mode ip` |
| `-c` | `-c` | `--report-cycles`, `--max-rounds` |
| `-i` | `-i` | `--min-round-duration` |
| `-W` | `-W`, `--timeout` | `--grace-duration` |
| `-m` | `-m` | `--max-ttl` |

## Examples

```bash
# Interactive TUI
mtr 8.8.8.8

# Interactive TUI (enhanced diagnostic preset)
mtr --ui enhanced 8.8.8.8

# Enhanced mode with custom threshold bands + toggles
mtr --ui enhanced --latency-warn-ms 80 --latency-bad-ms 180 --loss-warn-pct 1 --loss-bad-pct 3 --enhanced-sparklines off 8.8.8.8

# TCP report
mtr -T -P 443 -c 15 -r github.com

# JSON output for automation
mtr --json -c 20 1.1.1.1

# Force source IP + packet size
mtr -S 192.0.2.10 -s 128 8.8.4.4

# Advanced trippy tuning passthrough
mtr --trippy-flags "--log-format json --verbose --tui-refresh-rate 150ms" 8.8.8.8
```

## Default vs enhanced mode (side-by-side)

| Default mode | Enhanced mode |
|---|---|
| `mtr 8.8.8.8` | `mtr --ui enhanced 8.8.8.8` |
| Standard hop table | Adds threshold-based row coloring |
| Average-focused quick view | Adds percentile + jitter summary |
| No explicit trend column | Optional per-hop sparkline trend column |

![Default mode demo](assets/windows-mtr-m.gif)
![Enhanced mode demo](assets/windows-mtr-upscaled.gif)

## REST API startup and operational limits (v1, implemented)

Use API mode from the main binary and keep these defaults unless you have a reviewed reason to change them:

```bash
# Start API server on default localhost bind
mtr --api

# Override bind address on localhost
mtr --api --api-bind 127.0.0.1:4000

# Secure remote bind with API key from environment (preferred)
WINDOWS_MTR_API_KEY='replace-me' mtr --api --api-bind 0.0.0.0:4000 --api-auth api-key --api-key-env WINDOWS_MTR_API_KEY

# Tune REST API request rate limiting
mtr --api --api-max-requests-per-window 20 --api-rate-limit-window-seconds 30

# Tune REST API completed-job retention controls
mtr --api --api-max-completed-jobs 512 --api-completed-job-ttl-seconds 1200

# Secure remote bind with mTLS
mtr --api --api-bind 0.0.0.0:4000 --api-auth mtls

# mTLS ingress trust list (header-based mode) for non-loopback trusted reverse proxy hops
mtr --api --api-bind 0.0.0.0:4000 --api-auth mtls --api-mtls-trusted-ingress 10.0.0.10
```

- Bind to `127.0.0.1:3000` by default
- Require explicit opt-in for non-local bind addresses
- Request timeout: `10s`
- Max concurrent probes: `8`
- Max requests per rate-limit window: `8`
- Rate-limit window duration: `10s`
- Max targets per request: `8`
- Max request body size: `16 KiB`
- Max retained completed jobs: `1024`
- Completed job TTL: `15m`

Authentication enforcement in v1:
- Local-only bind: `none-local-only` is acceptable
- Non-local bind: require explicit `--api-auth api-key|mtls`
- For `api-key`, prefer `--api-key-env <ENV_VAR>` over inline `--api-key` to avoid exposing secrets in shell history
- For `mtls` header mode, trusted ingress IP sources are configurable via repeatable `--api-mtls-trusted-ingress <IP>`

Input validation before probe execution:
- Hostnames/IPs normalized and validated
- Ports validated in `1..=65535` (required for TCP/UDP)
- Intervals/timeouts validated as positive finite numbers (`timeout >= interval`)

See [docs/security/rest-api.md](docs/security/rest-api.md).
