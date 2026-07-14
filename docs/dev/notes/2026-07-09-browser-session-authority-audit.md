# Browser Session Authority Audit

Date: 2026-07-09

## Context

The latest failure mode shows the browser runtime is still brittle when stale operating-system processes, service-state browser rows, profile route proof, and view streams drift apart. The visible symptoms are sluggish left-rail population, viable non-owned browsers missing from inventory, degraded owned browsers staying selectable, CDP screenshot and stream failures, and resource pressure from accumulated stale `agent-browser` and browser processes.

This audit covers Candidate 1 from the architecture review: Browser Session Authority.

## Current Shape

The current code has several shallow owners instead of one deep authority:

- `cli/src/native/actions.rs` builds service status and refreshes multiple projections before returning JSON. It injects browser process stats into the serialized `service_state`, but the result is a process-stat decoration rather than an authority verdict.
- `cli/src/native/control_plane.rs` has a separate service-status path that reconciles and persists service state, then returns `control_plane`, profile allocations, retained display allocations, launch config, and service state. This path does not obviously share the same process-stat enrichment as the action path.
- `cli/src/native/service_resources.rs` can classify process samples into protected, candidate, and observed records. That module already knows about current dashboard main PID, retained browser allocations, retained display allocations, and unowned `agent-browser` processes.
- `packages/dashboard/src/lib/service-workspaces.ts` independently computes browser node `live`, `state`, `attentionReason`, `profileActionability`, route-bound ownership, and primary view stream. Those calculations do not receive a single lifecycle verdict from the daemon.
- `packages/dashboard/src/components/workspace-remote-viewport.tsx` independently probes status, stream frame URLs, Guacamole routes, stale selected tabs, focus, route recovery, and take-over actions. It is too broad to be the lifecycle authority.

## Failure Mechanism

The stale-process incident is not just a cleanup bug. It is an authority split:

- The operating system can contain stale or unowned `agent-browser` processes.
- Service state can still report a smaller, cleaner browser set.
- The dashboard can render rows before it has a viable stream or lifecycle verdict.
- The remote viewport can select a browser whose CDP or stream endpoint is already unhealthy.
- Resource cleanup exists, but it is an operator action path. It is not a status interlock that can prevent stale or out-of-date rows from entering the primary rail.

Because each layer infers locally, a stale or degraded edge can pass through as a usable browser until a later stream probe fails with messages such as unexpected end of JSON, stream sign-in expiry, or local stream proxy timeout.

## Deletion Test

Deleting `service_resources.rs` would remove process classification, but it would not reveal one replacement authority. The remaining code would still infer lifecycle from service state, route proof, stream shape, and dashboard selection state. That means `service_resources.rs` is useful, but it is not yet deep enough to stabilize the system.

Deleting `workspace-remote-viewport.tsx` would remove many symptoms, but the lost behavior is too broad: rendering, stream preflight, selected-tab recovery, route recovery, focus, and diagnostics. It is not the right module to own process and lifecycle authority.

Deleting `service-workspaces.ts` would remove dashboard inventory projection, but not the source of truth. The dashboard should project authority, not invent it.

## Recommended Deep Module

Create a Browser Session Authority interface in the daemon. Its job is to reconcile:

- service-state browser rows
- process samples and resource classification
- profile and route ownership
- view-stream readiness inputs, when available
- stale and unowned process pressure

The first slice should be read-only. It should not kill processes automatically. It should produce an explicit verdict that downstream layers must consume before presenting a browser as viable.

## Required Interlocks

- Service status must expose a top-level `browserSessionAuthority` object with resource pressure and lifecycle verdicts.
- Each service-state browser should have a lifecycle verdict or a stable lookup key into the top-level authority snapshot.
- Dashboard inventory must suppress non-viable rows from the initial left rail and place actionable degraded rows at the bottom attention area.
- Unowned or stale process pressure must be visible in status even when those processes are not candidates for automatic cleanup.
- The control-plane status path and action status path must share the same authority calculation.

## Risks

- If the first slice tries to solve cleanup, stream readiness, and routing in one edit, it will widen the blast radius too much.
- If the authority is dashboard-only, stale process pressure remains invisible to CLI, MCP, and service clients.
- If the authority only reports aggregate pressure and not per-browser verdicts, the left rail still has to infer viability locally.

## Acceptance For Candidate 1

- A daemon-side authority snapshot exists and is exposed through service status.
- Resource pressure from stale or unowned `agent-browser` processes appears in that snapshot.
- The dashboard can read the snapshot without breaking existing service-status fixtures.
- Focused tests cover the authority snapshot and dashboard projection path.
- No automatic termination is introduced in this slice.
