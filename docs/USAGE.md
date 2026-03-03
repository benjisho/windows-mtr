# Usage Guide

This guide provides practical command patterns for Windows MTR.

## Command Syntax

```bash
mtr [options] <hostname-or-ip>
```

## Core Probe Modes

### ICMP (default)

```bash
mtr 8.8.8.8
```

### TCP SYN

```bash
mtr -T -P 443 github.com
```

### UDP

```bash
mtr -U -P 53 1.1.1.1
```

## Report and Automation Modes

### Pretty report

```bash
mtr -r -c 10 example.com
```

### Wide report

```bash
mtr -r -w -c 10 example.com
```

### JSON report

```bash
mtr --json -c 20 8.8.4.4
```

### Pretty JSON

```bash
mtr --json-pretty -c 20 8.8.4.4
```

## Useful Flags

- `-n` — disable reverse DNS
- `-b` / `--show-asn` — include ASN info
- `-m <hops>` — max hops/TTL
- `-i <seconds>` — minimum round duration
- `-W <seconds>` — probe grace timeout
- `-S <ip>` — source IP
- `--interface <name>` — source interface

## Common Workflows

### Validate HTTPS path stability

```bash
mtr -T -P 443 -r -c 25 your-service.example
```

### Compare DNS-on vs DNS-off latency visibility

```bash
mtr -r -c 15 your-target.example
mtr -r -n -c 15 your-target.example
```

### Feed JSON output into automation

```bash
mtr --json -c 30 your-target.example > report.json
```

See [API.md](API.md) for JSON/report field guidance.

## Troubleshooting

### No response from first hops

- This can be normal due to ICMP filtering/rate-limiting.
- Compare with TCP mode: `mtr -T -P 443 ...`.

### Inconsistent results between runs

- Increase cycles (`-c`) to smooth transient variation.
- Run during similar traffic windows for comparison.

### Name resolution appears slow

- Disable reverse DNS (`-n`) to isolate probe timing from DNS overhead.

For concise option reference, see [../USAGE.md](../USAGE.md).
