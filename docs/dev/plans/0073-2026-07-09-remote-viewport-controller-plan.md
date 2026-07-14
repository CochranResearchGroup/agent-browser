# Plan 0073: Remote Viewport Controller

Date: 2026-07-09
Status: Completed

## Goal

Extract the remote viewport orchestration into a target-tokened controller so stream preflight, frame failure, focus, and route recovery cannot update the wrong selected browser. The first slice should add the reducer and tests, then wire the highest-risk preflight path.

## Source Audit

Primary audit: `docs/dev/notes/2026-07-09-remote-viewport-controller-audit.md`

Relevant files:

- `packages/dashboard/src/components/workspace-remote-viewport.tsx`
- `packages/dashboard/src/lib/workspace-viewport-state.ts`
- `packages/dashboard/src/lib/selected-workspace-context.ts`
- `packages/dashboard/src/lib/service-view-streams.ts`

## Design

Add a dashboard data-layer module:

```text
WorkspaceViewportControllerState
  targetToken
  target
  preflight
  frame
  focus
  recovery
```

Each async action must carry the `targetToken` that was current when it started. The reducer must ignore success or failure events with stale tokens.

## Implementation Steps

1. Add `packages/dashboard/src/lib/workspace-viewport-controller.ts`.
   - Define target identity and token helpers.
   - Define state, events, and reducer.
   - Keep it pure and testable.

2. Add focused tests.
   - Target switch clears stale browser-unavailable state.
   - Late preflight success for an old token is ignored.
   - Late preflight failure for an old token is ignored.
   - Frame failure applies only to the current token.
   - Recovery accepted and recovery failed apply only to the current token.

3. Wire stream preflight first.
   - Replace local preflight state updates in `workspace-remote-viewport.tsx` with controller events.
   - Keep current rendering and text where possible.

4. Wire frame failure and recovery in later small slices.
   - Move iframe load/error timer effects onto the controller.
   - Move route recovery request state onto the controller.

## Non-Goals

- Do not redesign the viewport UI.
- Do not replace Guacamole.
- Do not change daemon route acquisition semantics.
- Do not rewrite all viewport effects in one patch.

## Acceptance Criteria

- A pure controller reducer exists with deterministic tests.
- Preflight results cannot mutate state after the selected target changes.
- Existing viewport rendering remains compatible.
- The controller can represent CDP snapshot, CDP screencast, and Guacamole streams.

## Validation Commands

```bash
pnpm test:workspace-viewport-controller
pnpm test:dashboard-view-streams
```

If the React component is touched:

```bash
pnpm test:dashboard-selected-workspace-context
```

## Completion Evidence

- Added `packages/dashboard/src/lib/workspace-viewport-controller.ts` with a pure target-tokened reducer.
- Added `scripts/test-workspace-viewport-controller.js` and `pnpm test:workspace-viewport-controller`.
- Wired stream preflight in `packages/dashboard/src/components/workspace-remote-viewport.tsx`.
- Fixed the audit-blocking render race by forcing render preflight to `idle` unless the controller token matches the currently selected target token.
- Preserved explicit stale selected-workspace diagnostics through `includeHidden` while keeping hidden rows out of default workspace-node inventory.

Validation:

```bash
pnpm test:workspace-viewport-controller
pnpm test:dashboard-view-streams
pnpm test:dashboard-selected-workspace-context
pnpm test:dashboard-workspace-nodes
pnpm --dir packages/dashboard exec tsc --noEmit --pretty false
```

Independent audit: Candidate 4 initially found a stale-ready render race; the remediation above closed that blocker.
