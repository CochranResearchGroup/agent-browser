# Left Pane Workspace Navigator Slice 2 Checkpoint

Date: 2026-05-23

## Scope

Implemented the first visible left-pane refactor from the workspace navigator
campaign.

New implementation:

- `packages/dashboard/src/components/workspace-navigator.tsx`
- `scripts/test-dashboard-workspace-navigator.js`
- `pnpm test:dashboard-workspace-navigator`

Dashboard page changes:

- desktop left pane now renders `WorkspaceNavigator` instead of `SessionTree`
- mobile tab label is `Workspaces` and can render the navigator directly
- mobile tab state no longer falls through to the viewport when Workspaces is
  selected

The navigator derives rows through `deriveWorkspaceNodes`, groups them into
Needs attention, Active, and Retained, and includes search plus scope controls
for All, Active, Attention, and Retained.

## Preserved Actions

The navigator still uses the existing daemon session actions:

- create workspace through the existing create-session atom
- close all daemon sessions through the existing close-all atom
- close a daemon-backed workspace through the existing close-session atom
- kill a daemon-backed workspace through the existing kill-session atom
- add or switch tabs through the existing tab atoms when the selected node has
  a daemon-backed port

Destructive actions use shadcn `AlertDialog`, not native dialogs.

## Rendered QA

Rendered inspection used `agent-browser` against local dev server
`http://127.0.0.1:3103`.

Screenshots:

- desktop Service route:
  `/tmp/agent-browser-dashboard-workspace-navigator-slice-2/service-desktop-fixed.png`
- mobile Workspaces tab:
  `/tmp/agent-browser-dashboard-workspace-navigator-slice-2/workspaces-mobile-fixed.png`

Observed result:

- desktop first viewport now shows grouped workspace rows in the left pane
- mobile Workspaces tab now renders the navigator instead of the viewport
- rows are dense enough to show multiple attention and active rows without
  oversized explanatory chrome

## Remaining Slice 2 Gaps

This checkpoint does not complete every Slice 2 detail:

- collapsed desktop left pane still uses the existing show or hide panel
  behavior rather than a persistent icon rail with workspace health badges
- service-owned browser close and repair actions are not yet exposed from the
  left pane
- retained rows are dense, but large retained-state sets still need a stronger
  default filter before this can be called final UX

## Validation

Commands run:

- `pnpm test:dashboard-workspace-nodes`
- `pnpm test:dashboard-workspace-navigator`
- `pnpm test:dashboard-inspector-actions`
- `pnpm test:dashboard-browser-table`
- `pnpm build:dashboard`
- `git diff --check`

`pnpm validation:select -- --base HEAD` still recommends broader Rust, service,
docs, and skill-copy checks because the current worktree contains pre-existing
changes outside this navigator slice.
