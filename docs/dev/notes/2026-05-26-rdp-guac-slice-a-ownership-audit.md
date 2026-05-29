# RDP And Guacamole Slice A Ownership Audit

Date: 2026-05-26
State: IMPLEMENTED
Plan: `docs/dev/plans/0001-2026-05-26-rdp-guac-hardening-test-plan.md`

## Baseline

The implementation started from an active dirty worktree that already included
the dashboard workspace navigator, RDP viewport work, service contracts, docs,
and generated client changes from the current lane. This slice kept new edits
bounded to dashboard-side diagnostics, viewport state mapping, tests, and this
handoff note.

Validation base used for selector guidance in this turn: `HEAD`.

Touched surfaces:

- `packages/dashboard/src/lib/service-workspaces.ts`
- `packages/dashboard/src/lib/workspace-viewport-state.ts`
- `packages/dashboard/src/components/workspace-remote-viewport.tsx`
- `packages/dashboard/src/components/workspace-navigator.tsx`
- `scripts/test-dashboard-view-streams.js`
- `scripts/test-dashboard-workspace-nodes.js`
- `docs/dev/plans/0001-2026-05-26-rdp-guac-hardening-test-plan.md`

## State Inventory

The dashboard now treats these existing service fields as the Slice A ownership
inventory for an RDP-backed workspace:

- browser identity: `id`, `profileId`, `host`, `browserBuild`, `health`, `pid`,
  `cdpEndpoint`, `activeSessionIds`
- display and stream identity: `displayName`, `viewStreams[].provider`,
  `viewStreams[].url`, `viewStreams[].controlInput`, `viewStreams[].readOnly`
- session identity: `id`, `browserIds`, `tabIds`, `lease`, `profileId`,
  owner fields, and profile-lease conflict fields
- tab identity: `id`, `browserId`, `sessionId`, `ownerSessionId`, `targetId`,
  `lifecycle`, `title`, and `url`

No service API schema change was required for this slice. The diagnostics are
derived from already exposed service records.

## Diagnostics Added

`deriveWorkspaceOwnershipDiagnostics` now emits no-launch diagnostics for:

- duplicate CDP endpoints across browser records
- duplicate remote displays across RDP-capable remote-headed browser records
- duplicate Guacamole routes across RDP gateway stream records
- duplicate live CDP target IDs across browser records
- stale retained target identity on a live browser, with fallback guidance

`deriveWorkspaceNodes` attaches matching diagnostics to browser and session
workspace nodes. The workspace navigator includes diagnostic text in row search
and row metadata, so duplicate or stale ownership is visible before a live
stream is opened.

## Stale Target Recovery

The workspace viewport now distinguishes stale retained target identity from
browser failure. When a URL-selected tab is closed, blank, or otherwise stale,
the viewport selects the best current live nonblank tab for `view_focus` and
marks the viewport with the `stale_target_recovered` UX state.

The viewport also exposes the shared UX state vocabulary through
`data-ux-state`, currently including connected, connecting, provider
unavailable, browser unavailable, takeover ready, preparing focus, and stale
target recovered states.

## Guacamole Behavior Evidence

The current campaign authority records live validation on 2026-05-25 and
2026-05-26: the Guacamole-backed RDP connection can render the UPS browser, but
behaves as a single-active-viewer path when two clients attach to the same
connection. This slice preserves that as the expected baseline for Slice B.

Current dashboard behavior detects Guacamole's disconnected page text and
renders an explicit Take over action rather than leaving a silent white or
black viewport.

## Validation

Passed in this slice:

- `pnpm test:dashboard-view-streams`
- `pnpm test:dashboard-workspace-nodes`
- `pnpm test:dashboard-workspace-navigator`
- `pnpm test:dashboard-browser-row-actions-render`
- `pnpm test:dashboard-browser-table`
- `pnpm test:dashboard-launcher-eligibility`
- `pnpm test:dashboard-inspector-actions`
- `pnpm build:dashboard`
- `pnpm validation:select -- --base HEAD`
- `git diff --check`

`pnpm validation:select -- --base HEAD` also recommended Rust, service-client,
docs, and installed-skill checks because the broader active worktree already
contains Rust, contract, generated-client, docs-site, and skill changes outside
the Slice A dashboard diagnostics delta. Run those lane-level gates before a
merge, release, or handoff that claims the whole dirty worktree is ready.

No live runtime state was repaired in this slice. A two-client live takeover
smoke remains the first requirement of Slice B.
