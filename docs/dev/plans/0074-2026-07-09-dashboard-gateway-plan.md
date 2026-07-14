# Plan 0074: Dashboard Gateway

Date: 2026-07-09
Status: Completed

## Goal

Create a typed dashboard gateway interface so local backend transport failures, invalid payloads, and service fallback decisions become stable JSON envelopes instead of raw strings or empty responses.

## Source Audit

Primary audit: `docs/dev/notes/2026-07-09-dashboard-gateway-audit.md`

Relevant files:

- `cli/src/native/stream/dashboard.rs`
- `cli/src/native/stream/http.rs`
- `packages/dashboard/src/lib/dashboard-api.ts`
- `packages/dashboard/src/components/workspace-remote-viewport.tsx`

## Design

Introduce a small gateway vocabulary:

```text
backend_connect_timeout
backend_write_timeout
backend_read_timeout
backend_empty_response
backend_invalid_http
invalid_backend_payload
backend_unavailable
fallback_used
```

Represent gateway failures as:

```json
{
  "success": false,
  "error": "short operator message",
  "code": "backend_read_timeout",
  "details": {
    "port": 37889,
    "path": "/api/stream/37889/frame"
  }
}
```

## Implementation Steps

1. Add typed gateway error helpers in `dashboard.rs` or a new sibling module.
   - Keep the public shape small and serializable.
   - Preserve existing CORS and HTTP status behavior.

2. Normalize local proxy failures.
   - Return structured errors for connect, write, read, size limit, and empty response.
   - Preserve enough details for diagnostics without exposing raw internals as the main message.

3. Normalize JSON API response expectations.
   - For service API and screenshot API paths, ensure empty or invalid backend bodies become structured JSON errors.
   - Keep raw byte proxying only for paths that intentionally stream non-JSON payloads.

4. Add focused Rust tests.
   - Empty backend response.
   - Read timeout code mapping.
   - Invalid HTTP response.
   - JSON envelope serialization.

5. Update dashboard client helpers only if response shape changes require it.
   - Prefer compatibility: existing `error` remains string.
   - Add optional `code` and `details` handling.

## Non-Goals

- Do not redesign dashboard routing.
- Do not remove CLI fallback.
- Do not replace stream transport.
- Do not change authentication semantics.

## Acceptance Criteria

- Dashboard API proxy failures always return valid JSON envelopes for JSON endpoints.
- Timeout and invalid-payload cases have stable codes.
- Empty backend responses cannot surface as `Unexpected end of JSON input`.
- Focused Rust tests pass.

## Validation Commands

```bash
cargo test --manifest-path cli/Cargo.toml dashboard_gateway
pnpm test:dashboard-view-streams
```

## Completion Evidence

- Added stable `DashboardReadinessError` codes and diagnostic details in `cli/src/native/stream/dashboard.rs`.
- Normalized local backend connect, write, read, empty-response, invalid-HTTP, invalid-JSON, and backend-unavailable failures into code-bearing JSON envelopes.
- Preserved the existing `success:false` and `error` response shape while adding optional `code` and `details`.
- Validated service API and focus-command proxy responses as JSON before forwarding.

Validation:

```bash
cargo test --manifest-path cli/Cargo.toml dashboard_gateway
cargo fmt --manifest-path cli/Cargo.toml -- --check
```

Independent audit: Candidate 5 passed with no blocking findings.
