# REST API Security Threat Model (v1)

## Scope

This document defines baseline security assumptions and controls for the implemented windows-mtr REST API v1.

## Assets

- Probe execution capability (ICMP/TCP/UDP diagnostics).
- Host and network metadata in probe requests/results.
- Service availability for legitimate operators.

## Trust boundaries

- **Inbound HTTP boundary**: every request payload and header is untrusted.
- **Network target boundary**: target hostnames/IPs/ports are attacker-controlled input.
- **Execution boundary**: probe runtime can generate privileged network activity and must be constrained.

## Authentication strategy decision (v1)

1. **Default local-only mode**: `none-local-only`.
   - Allowed only when server binds to loopback (`127.0.0.1` / `::1`).
2. **Non-local deployments**: explicit authentication is required.
   - Choose one of:
     - **API key** (`X-API-Key`) for simple service-to-service deployments.
     - **mTLS** for environments with certificate-based workload identity.
3. **Prohibited**: non-local bind with no authentication.

## Secure defaults

- Bind address default: `127.0.0.1:3000`.
- Non-local bind requires explicit opt-in.
- Request timeout default: `10s`.
- Max concurrent probes default: `8`.
- Max targets per request default: `8`.
- Max payload size default: `16KiB`.

## Input validation and normalization requirements

All untrusted request fields MUST be validated before probe execution:

- **Hostnames/IPs**
  - Trim surrounding whitespace.
  - Accept canonical IP literals (IPv4/IPv6).
  - Hostnames normalized to lowercase.
  - Reject empty values, invalid characters, overlong labels (>63), or overlong hostnames (>253).
- **Ports**
  - Required for TCP/UDP probes.
  - Integer in range `1..=65535`.
- **Intervals/timeouts**
  - Positive finite numbers only.
  - `timeout_seconds >= interval_seconds` when both are present.

## Abuse prevention controls

- **Rate limiting**: reject request bursts above configured fixed-window limit.
- **Concurrency limiting**: reject probe starts when in-flight probe count exceeds limit.
- **Payload limiting**: reject oversized request bodies with 413.
- **Target cardinality limiting**: reject requests with too many targets.

## Threats and mitigations

- **Unauthenticated remote use** → prevented by local-only default and non-local auth requirement.
- **SSRF-like probing abuse** → constrained by auth, request validation, and target count limits.
- **Resource exhaustion (CPU/socket saturation)** → constrained by timeout + concurrency + rate limits.
- **Large-body denial attempts** → constrained by payload size cap.
- **Input parsing edge cases** → constrained by strict normalization + explicit validation failures.

## Residual risks

- API key leakage in logs or process env if deployed improperly.
- Operator misconfiguration of network ACLs around non-local deployments.
- Legitimate but high-cost probes can still consume available concurrency budget.

## Operational guidance

- Keep v1 local-only unless an integration requires remote access.
- Prefer mTLS over API keys in production.
- Add perimeter controls (firewall/ingress allow-list) even when auth is enabled.
- Monitor 413/429 rates for abuse or client misconfiguration.

## CLI examples for secure remote bind

```bash
# Local-only default (no auth required)
mtr --api

# Remote bind with API key loaded from environment (preferred)
WINDOWS_MTR_API_KEY='replace-me' mtr --api --api-bind 0.0.0.0:4000 --api-auth api-key --api-key-env WINDOWS_MTR_API_KEY

# Remote bind with mTLS (identity header provided by trusted TLS terminator)
mtr --api --api-bind 0.0.0.0:4000 --api-auth mtls
```

## Industry baseline alignment

This v1 model aligns with common API hardening guidance:

- **OWASP API Security Top 10 (2023)** emphasis on authentication, unrestricted resource consumption controls, and input validation.
- **HTTP Semantics (RFC 9110)** status-code usage for payload and request throttling outcomes (e.g., 413/429).

These references were checked during this update to ensure the guardrails match mainstream operator expectations.
