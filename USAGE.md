# Windows MTR Usage Guide

## Basic Usage

```
mtr [options] <hostname or IP>
```

## Command Line Options

### Core Options

| Option | Description |
|--------|-------------|
| `<hostname or IP>` | Target host to trace (required) |
| `-T` | Use TCP SYN packets instead of ICMP echo (default is ICMP) |
| `-U` | Use UDP packets instead of ICMP echo (default is ICMP) |
| `-P <port>` | Specify target port for TCP/UDP modes (default: 80 for TCP, 33434 for UDP) |

### Output Control Options

| Option | Description |
|--------|-------------|
| `-r` | Report mode - output a report once and exit (no continuous updates) |
| `-c <count>` | Number of pings to send to each host (default: unlimited) |
| `-n` | Don't resolve hostnames (no DNS lookups - faster) |

### Timing Options

| Option | Description |
|--------|-------------|
| `-i <seconds>` | Time in seconds between ICMP ECHO requests (default: 1.0) |
| `-w <seconds>` | Maximum time in seconds to keep a probe alive (default: 5.0) |

### Limit Options

| Option | Description |
|--------|-------------|
| `-m <hops>` | Maximum number of hops (TTL) to probe (default: 30) |

## Example Commands

### Basic ICMP Trace
```
mtr 8.8.8.8
```

### TCP Trace to a Web Server
```
mtr -T -P 443 example.com
```

### UDP Trace to DNS Server
```
mtr -U -P 53 1.1.1.1
```

### Generate Static Report
```
mtr -c 10 -r 8.8.8.8
```

### Faster Trace (No DNS Lookups)
```
mtr -n 8.8.8.8
```

### Custom Timing Parameters
```
mtr -i 0.5 -w 3 8.8.8.8
```

## Output Format

### Live Mode
In the default live mode, Windows MTR will continuously update the traceroute results in real-time using a TUI (terminal user interface).

### Report Mode (-r)
When using report mode (`-r`), the output format will match Linux MTR's report format:

```
HOST: <hostname>                  Loss%   Snt   Last   Avg  Best  Wrst StDev
  1.|-- <hop1>                     0.0%    10    1.2   1.5   1.0   2.2   0.3
  2.|-- <hop2>                     0.0%    10   10.3  11.2  10.0  14.9   1.8
  ...
  N.|-- <destination>              0.0%    10   25.4  27.3  24.9  35.0   3.2
```

## Exit Codes

| Code | Description |
|------|-------------|
| 0 | Success - trace completed successfully |
| 1 | Argument error - invalid command line options |
| 2 | Runtime error - trace failed during execution |