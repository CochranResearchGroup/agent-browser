# Remote Viewport Controller Audit

Date: 2026-07-09

## Context

The remote viewport remains unstable when selected browser state, stream readiness, route proof, Guacamole frame state, and recovery actions drift. The current surface can report browser unavailable, JSON parse failures, sign-in expiry, and stream proxy timeouts after the operator has already selected a browser.

This audit covers Candidate 4 from the architecture review: Remote Viewport Controller.

## Current Shape

`packages/dashboard/src/components/workspace-remote-viewport.tsx` is currently responsible for too many decisions:

- parsing workspace viewport URL selection
- resolving daemon sessions from selection
- converting selected workspace context into viewport browser records
- choosing the primary stream
- ranking viewport stream tiles
- probing frame URLs
- detecting frame failure
- handling Guacamole interaction settings
- rendering CDP snapshot fallback
- posting focus, recovery, take-over, and route-switch requests
- handling fullscreen behavior
- computing operator-facing readiness messages

Some helper logic exists in `packages/dashboard/src/lib/workspace-viewport-state.ts`, but the controller loop and side effects remain inside the React component.

## Failure Mechanism

The viewport has multiple asynchronous inputs with no single controller state machine:

- service status polling can replace browser and stream records
- URL selection can change independently
- stream preflight can complete after the selected browser changed
- iframe load/error timers can race preflight and route recovery
- recovery requests can mutate route state while old frame state is still visible
- Guacamole and CDP snapshot streams use different readiness paths

Because the state transitions are implicit in React effects, the viewport can show stale errors for the wrong browser, upgrade a stale stream after a delayed probe, or keep a degraded owned browser selected while viable non-owned streams are elsewhere in inventory.

## Deletion Test

Deleting `WorkspaceRemoteViewport` would remove the viewport experience entirely, which means it is carrying rendering, orchestration, and controller behavior at once.

Deleting `workspace-viewport-state.ts` would remove operator copy and status classification, but not the side-effect ordering. The controller is still inside the component.

Deleting Guacamole-specific helpers would not fix CDP snapshot or stream preflight races. The problem is the missing controller boundary, not one stream provider.

## Recommended Deep Module

Create a Remote Viewport Controller module in the dashboard data layer. It should be a pure reducer plus a small effect adapter:

- reducer: selected target, stream identity, preflight state, frame state, route recovery state, focus state
- commands: preflight stream, open focus, route recovery, route switch, take over, refresh snapshot
- guards: ignore stale command results that do not match current target token

The React component should render controller state and dispatch controller actions. It should not decide whether a late preflight is still valid.

## Required Interlocks

- Every selected viewport target needs a stable token derived from browser id, stream id or URL, route id, and mode.
- Async preflight, frame failure, focus, and recovery results must be ignored unless their token matches the current target.
- Browser-unavailable state must clear when the selected target changes to a viable target.
- Route recovery and take-over actions must transition through explicit pending, accepted, failed, and stale-result states.
- CDP snapshot refresh must use the same target token as stream preflight.

## Risks

- A large component rewrite would increase risk. Extract the reducer first and leave rendering mostly intact.
- A reducer without token guards would only move the race conditions to a new file.
- A controller that assumes Guacamole semantics would not stabilize CDP snapshot and CDP screencast.

## Acceptance For Candidate 4

- A plan exists to extract target-tokened viewport controller state.
- The first implementation slice should add a pure reducer with tests for stale preflight, target switch, frame failure, and recovery result handling.
- React effects should be able to adopt the reducer incrementally.
- No behavior should depend on native browser dialogs or manual page refresh.
