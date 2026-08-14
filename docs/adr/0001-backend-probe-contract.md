# ADR 0001: Backend probe and report contract

- Status: Proposed
- Date: 2026-08-15

## Context

Windows MTR currently has two execution paths. The Windows native ICMP path is
IPv4-only and accepts a smaller set of controls than the embedded Trippy path.
The dashboard, CLI reports, and REST API therefore do not yet share one
explicit capability or hop-result contract.

## Decision

1. A probe request is normalized before execution and records the selected
   backend (`native-icmp` or `trippy`) plus the capability decisions for every
   requested option.
2. A backend returns a versioned `ProbeReport` containing target metadata,
   ordered `HopReport` entries, probe timing, loss, and an explicit error or
   unavailable reason when a field cannot be collected.
3. Formatters consume `ProbeReport`; they must not reach into backend-specific
   process output or silently infer unsupported fields.
4. A capability is either implemented by the selected backend, routed to a
   backend that implements it, or rejected with an actionable validation error.
   Accepted-but-ignored flags are not permitted.
5. The public JSON/API schema is versioned independently from the internal
   Rust representation. Additive fields are preferred; breaking changes require
   a new schema version and migration notes.

## Consequences

- Native ICMPv6 and native flag parity can be implemented behind the same
  adapter without changing CLI/API formatters.
- Dashboard headless tests and REST API contract tests can use deterministic
  `ProbeReport` fixtures instead of launching a probe process.
- SNMP and ETW enrichments must attach to the normalized hop/report model and
  define their privilege, credential, and failure behavior before integration.
- Until the adapter exists, documentation must describe backend-specific
  limitations explicitly; this ADR does not claim those features are shipped.

## Rejected alternatives

- Keeping separate formatter-specific parsing for native and Trippy output.
- Silently falling back between backends when a requested option is unsupported.
- Treating the current REST target-success DTO as a complete MTR report.
