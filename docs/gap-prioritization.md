# Gap Prioritization and Scope Decisions

This document captures current prioritization decisions for commonly requested feature gaps so roadmap and implementation choices remain explicit and predictable.

## High-priority (near-term)

These items are considered high-value for API reliability, integration safety, and client compatibility:

1. Return standard rate-limit response metadata for REST API requests (for example: limit/remaining/reset semantics).
2. Return a stable request-correlation header (for example `X-Request-ID`) for traceability between clients and logs.
3. Add a version marker to JSON output schemas to protect downstream parsers from breaking changes.
4. Ensure probe timeout behavior is explicit and consistently enforced at the wrapper level.

## Medium-priority (planned convenience)

These items are useful but not required for baseline correctness:

1. CSV export mode (JSON remains the canonical machine-readable format).
2. Additional packaging channels where maintenance cost is justified (for example Chocolatey hardening from template to publish-ready).

## Optional / scope-dependent

These items are valid ideas but depend on product direction and user demand:

1. OAuth2/JWT auth flows for enterprise API environments (API key and mTLS remain sufficient defaults for many deployments).
2. Webhook callback delivery model in addition to poll-based APIs.
3. SNMP integration (often better handled by dedicated network-management tooling).
4. ETW provider instrumentation for advanced Windows observability.

## Platform and distribution scope notes

- Windows remains the primary supported target.
- macOS CI/release and Homebrew distribution are roadmap-level initiatives and require explicit signing/notarization/release process design before enablement.
- GitHub Releases ZIP remains the canonical binary distribution source.

## UX policy

- Default embedded interactive mode is the recommended primary experience.
- Dashboard mode is a fallback path for environments where embedded TUI stability is insufficient.
- Unsupported UI flags should fail with clear validation guidance.
