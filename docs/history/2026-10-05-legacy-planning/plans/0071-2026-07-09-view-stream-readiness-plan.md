# Plan 0071: View Stream Readiness

Date: 2026-07-09
Status: Completed

## Goal

Create a deep View Stream Readiness interface so CDP screencast, screenshot, and dashboard embed decisions share one readiness contract. The first slice should normalize stream and screenshot failures, then make dashboard open/embed decisions depend on that contract.

## Source Audit

Primary audit: `docs/dev/notes/2026-07-09-view-stream-readiness-audit.md`

Relevant files:

- `cli/src/native/actions.rs`
- `cli/src/native/stream/dashboard.rs`
- `packages/dashboard/src/lib/service-view-streams.ts`
- `packages/dashboard/src/lib/workspace-viewport-state.ts`
- `packages/dashboard/src/components/workspace-remote-viewport.tsx`

## Design

Introduce a small readiness vocabulary shared by daemon status and dashboard helpers:

```text
ready
probing
unreachable
auth_expired
stale_target
invalid_payload
unsupported_provider
unknown
```

Each stream should report:

- `state`
- `reason`
- `checkedAt`
- `targetBrowserId`
- `targetTabId`, when available
- `transport`, for example local proxy, dashboard proxy, Guacamole, or CDP
- `blocking`, whether initial inventory should treat the stream as non-viable

## Implementation Steps

1. Normalize daemon proxy and screenshot failures.
   - Convert local proxy timeout and empty-response errors into stable readiness codes.
   - Ensure screenshot API failures return explicit JSON errors with code and message.
   - Avoid raw empty JSON parse failures at the dashboard boundary.

2. Attach readiness to stream records.
   - Extend CDP screencast stream records with readiness fields where they are created or refreshed.
   - Preserve compatibility with stream records that do not have readiness.

3. Update dashboard stream helpers.
   - Make `canOpenViewStream` and `canOpenControlViewStream` consider blocking readiness states.
   - Keep fallback behavior for older payloads with missing readiness.
   - Add tests for ready, unknown, unreachable, auth expired, and unsupported provider.

4. Stabilize viewport preflight.
   - Tie in-flight preflight results to the selected browser and stream URL.
   - Ignore late preflight results for stale selections.
   - Render normalized readiness messages instead of transport internals.

5. Validate.
   - Add Rust tests around proxy/screenshot error normalization.
   - Add dashboard tests for helper gating and stale preflight behavior if the viewport changes.

## Non-Goals

- Do not replace Guacamole routing.
- Do not remove existing CDP screencast support.
- Do not treat non-owned browsers as unusable solely because they lack control input.
- Do not make the dashboard responsible for daemon process cleanup.

## Acceptance Criteria

- Empty or timed-out backend responses produce explicit JSON errors with stable codes.
- Stream records can carry readiness without breaking older clients.
- Dashboard open/embed helpers block known-bad readiness states.
- Late preflight results cannot make a stale selected stream look ready.
- Focused Rust and dashboard tests pass.

## Validation Commands

```bash
cargo test --manifest-path cli/Cargo.toml dashboard_proxy
pnpm test:dashboard-view-streams
```

If viewport state changes:

```bash
pnpm test:dashboard-selected-workspace-context
```

## Completion Evidence

Completed on 2026-07-09.

Validation passed:

- `pnpm test:dashboard-view-streams`
- `pnpm test:dashboard-workspace-nodes`
- `pnpm --dir packages/dashboard exec tsc --noEmit --pretty false`
- `cargo test --manifest-path cli/Cargo.toml dashboard_proxy_normalizes -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml screenshot_errors_map_to_stable_readiness_codes -- --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`

The implementation normalizes dashboard proxy and screenshot failures into stable code-bearing JSON errors, blocks known-bad readiness states in stream helpers, gates navigator View/Control through readiness-aware stream projection, and prevents the viewport render path from bypassing `canOpenViewStream`.

Independent outcome audit: initial audit found viewport and navigator action bypasses; remediation was re-audited with no remaining blocker.
