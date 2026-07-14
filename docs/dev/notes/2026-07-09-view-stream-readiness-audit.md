# View Stream Readiness Audit

Date: 2026-07-09

## Context

CDP screenshots and CDP streaming remain fragile even after Guacamole reliability improved. Operators see stream sign-in expiry, local proxy timeouts, screenshot JSON parse failures, and rows that appear selectable before the stream path is actually ready.

This audit covers Candidate 2 from the architecture review: View Stream Readiness.

## Current Shape

The current code spreads stream readiness across several modules:

- `cli/src/native/actions.rs` refreshes CDP screencast view streams by upserting stream records into service state.
- `cli/src/native/stream/dashboard.rs` proxies dashboard API requests to local loopback ports and reports timeout strings such as timed out connecting, writing, or reading from `127.0.0.1`.
- `cli/src/native/stream/dashboard.rs` also handles session screenshot requests and service API proxying in the broad dashboard connection handler.
- `packages/dashboard/src/lib/service-view-streams.ts` decides whether a stream can be embedded from provider and URL shape.
- `packages/dashboard/src/lib/workspace-viewport-state.ts` computes viewport readiness from selected browser state, stream provider, stream URL, stream readiness, preflight messages, frame issues, and focus messages.
- `packages/dashboard/src/components/workspace-remote-viewport.tsx` performs runtime stream preflight, frame-failure detection, Guacamole handling, focus, stale selection recovery, and route recovery.

No single module owns the claim: this view stream can be opened now.

## Failure Mechanism

The current readiness model is mostly optimistic. A stream can be represented as embeddable because it has a provider and frame URL, while a later probe discovers:

- the frame URL points at a stale or unreachable local proxy
- the public dashboard URL cannot reuse a local-only frame URL
- the backend returns an empty body that the client then parses as JSON
- the stream credential or sign-in state has expired
- a screenshot path reports success-shaped transport but no valid image payload
- the selected browser changed while preflight was still in flight

The result is a delayed failure inside the viewport rather than an earlier interlock at inventory time.

## Deletion Test

Deleting `service-view-streams.ts` would remove helper functions such as `canEmbedViewStream`, `viewStreamFrameUrl`, and `viewStreamDashboardFrameUrl`, but it would not remove stream readiness logic from the viewport or daemon.

Deleting the viewport preflight logic would make failures appear later or silently, because the daemon does not currently expose a complete view-readiness verdict.

Deleting `proxy_local_http_api_request` would break the dashboard proxy, but not replace it with a readiness contract. Its current interface returns bytes or a string error, which is too low-level for the dashboard to make a reliable inventory decision.

## Recommended Deep Module

Create a View Stream Readiness module with a daemon-owned verdict and a dashboard adapter:

- Daemon verdict: transport, auth, target, frame, screenshot, control-input, and freshness.
- Dashboard adapter: convert daemon verdict plus local preflight into stable display states.

The daemon verdict should become the primary gate for initial inventory. The dashboard may still run a short preflight, but it should refine the verdict rather than invent it.

## Required Interlocks

- A stream record must include a readiness verdict that distinguishes unknown, probing, ready, degraded, expired, unreachable, stale target, and unsupported provider.
- Dashboard embeddability must require both provider shape and readiness verdict. URL presence alone is not enough.
- Screenshot endpoints must return explicit JSON errors for empty or invalid backend responses. The client should not surface raw `Unexpected end of JSON input` as the main operator message.
- Stream proxy errors must be normalized into stable codes before they reach the dashboard.
- A selected stream must be tied to a browser identity and freshness token so late preflight results cannot upgrade a stale selection.

## Risks

- A dashboard-only fix would keep CLI, MCP, and service clients blind to broken streams.
- A daemon-only fix would not address late frame errors or browser-specific embed restrictions in the dashboard.
- Treating every probe failure as fatal could hide viable non-owned CDP browsers. The verdict needs an attention path, not just a binary hide/show decision.

## Acceptance For Candidate 2

- A durable view-stream readiness plan exists with daemon and dashboard responsibilities separated.
- The first implementation slice produces normalized readiness reasons for proxy and screenshot failures.
- Dashboard helpers consume stable readiness states before allowing initial embed/open actions.
- Tests cover empty backend response, proxy timeout normalization, and dashboard embeddability gating.
