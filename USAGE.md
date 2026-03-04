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

## Timing & DNS Cache

| Option | Description |
|---|---|
| `-i <SECONDS>` | Minimum round duration |
| `-W, --timeout <SECONDS>` | Probe grace timeout |
| `--dns-cache-ttl <SECONDS>` | Per-run DNS cache TTL |
| `--rest-api` | Run as HTTP API server instead of CLI trace mode |
| `--rest-api-bind <ADDR>` | REST API bind address (default `127.0.0.1:8080`) |
| `--native-ui` | Launch native Ratatui UI (tabs, hops, charts) |

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


## REST API

Start the server:

```bash
mtr --rest-api --rest-api-bind 127.0.0.1:8080
```

Endpoints:

- `GET /health` → `{"status":"ok"}`
- `POST /v1/report` with JSON body:

```json
{
  "host": "1.1.1.1",
  "count": 5,
  "tcp": true,
  "port": 443
}
```

Response includes `exit_code`, `target`, and a `report` object from Trippy JSON output.

## Examples

```bash
# Interactive TUI (embedded trippy)
mtr 8.8.8.8

# Native Ratatui UI preview
mtr --native-ui 8.8.8.8

# TCP report
mtr -T -P 443 -c 15 -r github.com

# JSON output for automation
mtr --json -c 20 1.1.1.1

# Force source IP + packet size
mtr -S 192.0.2.10 -s 128 8.8.4.4

# Advanced trippy tuning passthrough
mtr --trippy-flags "--log-format json --verbose --tui-refresh-rate 150ms" 8.8.8.8
```
