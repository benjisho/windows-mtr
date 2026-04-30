# Windows MTR Usage Guide

## Basic usage

```bash
mtr [options] <hostname-or-ip>
```

On portable ZIP installs, run `mtr.exe` or `windows-mtr.exe` from the extracted folder.

## UI modes

| Mode | Command | Notes |
|---|---|---|
| default | `mtr 8.8.8.8` | Embedded Trippy TUI (primary interactive mode). |
| enhanced | `mtr --ui enhanced 8.8.8.8` | Embedded Trippy TUI plus threshold-based overlays. |
| dashboard | `mtr --ui dashboard 8.8.8.8` | Experimental windows-mtr dashboard that polls JSON snapshots; fallback if embedded TUI crashes. |
| native (alias) | `mtr --ui native 8.8.8.8` | Deprecated compatibility alias for `dashboard`. |

## Common options

| Option | Description |
|---|---|
| `-T` | TCP SYN probes |
| `-U` | UDP probes |
| `-P, --port <PORT>` | Target port for TCP/UDP |
| `--source-port <PORT>` | Source port |
| `-S, --src <IP>` | Source IP |
| `--interface <NAME>` | Source interface |
| `-m <HOPS>` | Max hops |
| `-s, --packet-size <BYTES>` | Packet size |
| `-r` | Report mode |
| `-w, --report-wide` | Wide report output |
| `-j, --json` | JSON output |
| `--json-pretty` | Pretty JSON output |
| `-c <COUNT>` | Report cycles |
| `-n` | Disable reverse DNS |
| `-b, --show-asn` | ASN rendering |
| `-z` | DNS/ASN lookup shortcut |
| `--ecmp <classic\|paris\|dublin>` | Multipath strategy |
| `--trippy-flags "<FLAGS>"` | Advanced passthrough flags |

## Troubleshooting

If embedded interactive TUI fails with `0xC0000005`:

```powershell
.\mtr.exe --ui dashboard 8.8.8.8
```

For stable non-interactive diagnostics:

```powershell
.\mtr.exe -n -r -c 5 8.8.8.8
```

## REST API mode (implemented, validate before production)

```bash
mtr --api
mtr --api --api-bind 127.0.0.1:4000
WINDOWS_MTR_API_KEY='replace-me' mtr --api --api-bind 0.0.0.0:4000 --api-auth api-key --api-key-env WINDOWS_MTR_API_KEY
mtr --api --api-bind 0.0.0.0:4000 --api-auth mtls
```

See [docs/security/rest-api.md](docs/security/rest-api.md) and [docs/capability-validation.md](docs/capability-validation.md) for current validation status.
