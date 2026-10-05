# Plan 0072: Workspace Inventory Authority

Date: 2026-07-09
Status: Completed

## Goal

Stabilize the dashboard left rail by separating workspace-node construction from inventory placement. The dashboard should render viable and actionable entries first, keep Needs Attention at the bottom, and exclude rows that do not map to an existing browser, retained record, or recovery action.

## Source Audit

Primary audit: `docs/dev/notes/2026-07-09-workspace-inventory-audit.md`

Relevant files:

- `packages/dashboard/src/lib/service-workspaces.ts`
- `packages/dashboard/src/components/workspace-navigator.tsx`
- `packages/dashboard/src/components/workspace-remote-viewport.tsx`
- `packages/dashboard/src/lib/selected-workspace-context.ts`
- `scripts/test-dashboard-workspace-nodes.js`
- `scripts/test-dashboard-workspace-navigator.js`

## Design

Add a pure inventory placement layer:

```text
WorkspaceInventoryPlacement
  lane: primary | detected | launcher | retained | attention | hidden
  reason
  rank
```

`WorkspaceNode.state` should remain descriptive. `WorkspaceInventoryPlacement.lane` should decide where the row appears and whether it appears initially.

## Implementation Steps

1. Add a pure placement helper in `service-workspaces.ts`.
   - Input: `WorkspaceNode` plus any authority verdicts already attached to the node.
   - Output: lane, reason, rank.
   - Keep backward-compatible `node.group` during the first slice.

2. Apply placement before rendering the initial rail.
   - Primary viable rows first.
   - Detected viable rows next.
   - Retained rows after active inventory.
   - Needs Attention last.
   - Hidden rows excluded from the rail.

3. Make non-existent rows explicit.
   - Rows without live browser authority, retained state, viable detected target, or profile action should be hidden.
   - Rows with a concrete recovery action can enter Needs Attention.

4. Preserve non-owned viable browser visibility.
   - Detected non-owned CDP rows with viable snapshot/read-only streams should remain visible even when owned rows are degraded.
   - Do not require control input for detected read-only viability.

5. Add focused tests.
   - Owned viable rows stay in primary lane.
   - Viable non-owned rows stay visible.
   - Needs Attention sorts last.
   - Non-existent rows without recovery action are hidden.

## Non-Goals

- Do not redesign the whole navigator UI.
- Do not remove retained history.
- Do not change daemon service-state shape in this slice.
- Do not make profile-sharing policy decisions here.

## Acceptance Criteria

- Inventory placement is a pure helper with deterministic tests.
- Needs Attention appears after viable inventory in all tested cases.
- Non-owned viable browsers remain visible.
- Non-existent, non-actionable rows are excluded.
- Existing workspace URL selection behavior remains compatible.

## Validation Commands

```bash
pnpm test:dashboard-workspace-nodes
pnpm test:dashboard-workspace-navigator
```

## Completion Evidence

Completed on 2026-07-09.

Validation passed:

- `pnpm test:dashboard-workspace-nodes`
- `pnpm test:dashboard-workspace-navigator`
- `pnpm --dir packages/dashboard exec tsc --noEmit --pretty false`

The implementation adds `WorkspaceInventoryPlacement`, filters hidden rows, keeps retained rows out of the default live rail, sorts Needs Attention after primary/detected/launcher rows, preserves viable non-owned detected browsers, and keeps stream-readiness-blocked live rows as attention rather than primary inventory.

Independent outcome audit: passed with no blocking findings; residual note was limited to disabled profile actionability semantics matching the tested contract.
