# P47.3 Canonical Workspace Inventory Plan

Date: 2026-06-24
State: DONE
Lane: P47.3
Parent Plan: `docs/dev/plans/0047-2026-06-24-remote-view-foundational-architecture-plan.md`

## Goal

`/goal execute P47 goal 3: publish canonical workspace inventory so the dashboard renders ownership, role, actionability, and live-control state from one record instead of reconstructing it from row shape, URL params, or stream URLs`

## Audit Findings

- `packages/dashboard/src/lib/service-workspaces.ts` already creates a
  canonical `WorkspaceNode` with role, inventory class, view stream,
  diagnostics, related ids, and actions.
- Existing dashboard tests cover viewer-client classification, route-bound
  proof blockers, disabled view/control actions, and live rail exclusion for
  attention rows.
- The remaining shallow boundary is that consumers can still be tempted to
  infer control eligibility from raw stream URLs or selected route params
  instead of the finalized `WorkspaceNode` record.

## Implementation Plan

1. Add a small exported `workspaceNodeLiveControlEligibility` helper that
   derives view/control eligibility from the canonical `WorkspaceNode`.
2. Treat viewer clients, retained history, and diagnostic rows as
   non-controllable even when raw stream-looking fields are present.
3. Extend the dashboard workspace node smoke test so URL-bearing retained,
   viewer-client, and diagnostic records cannot regain live-control eligibility.
4. Preserve existing dashboard record shape. Rust-published inventory records
   remain a larger contract change to execute only after this high-level
   boundary is stable.

## Validation

- PASS: `pnpm test:dashboard-workspace-nodes`
- PASS: `node --check scripts/test-dashboard-workspace-nodes.js`
- PASS: `git diff --check -- packages/dashboard/src/lib/service-workspaces.ts scripts/test-dashboard-workspace-nodes.js docs/dev/plans/0047-3-2026-06-24-canonical-workspace-inventory-plan.md`

## Closeout

Added `workspaceNodeLiveControlEligibility` as a canonical `WorkspaceNode`
guard for live view/control eligibility. Extended workspace-node fixtures so
diagnostic, viewer-client, and retained records with stream-looking fields
remain non-controllable.
