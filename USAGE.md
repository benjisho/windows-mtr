# Windows MTR Usage Guide

## Basic Usage

```bash
mtr [options] <hostname-or-ip>
```

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

## REST API operational limits (v1 plan)

If you deploy the planned REST API wrapper, apply these defaults unless you have a reviewed reason to change them:

- Bind to `127.0.0.1:3000` by default
- Require explicit opt-in for non-local bind addresses
- Request timeout: `10s`
- Max concurrent probes: `8`
- Max targets per request: `8`
- Max request body size: `16 KiB`

Authentication decision for v1:
- Local-only bind: `none-local-only` is acceptable
- Non-local bind: require `X-API-Key` or mTLS

Input validation before probe execution:
- Hostnames/IPs normalized and validated
- Ports validated in `1..=65535` (required for TCP/UDP)
- Intervals/timeouts validated as positive finite numbers (`timeout >= interval`)

See [docs/security/rest-api.md](docs/security/rest-api.md).
