# Left Pane Workspace Navigator Slice 6

Date: 2026-05-23

## Scope

Implemented the workspace remote viewport path for selected service-owned
browsers that report an embeddable view stream.

The dashboard now supports `view=workspace:view` and
`view=workspace:control` on the overview route. A left-pane View or Control
action pushes a durable workspace URL containing workspace, browser, session,
tab, profile, and job identity where available. The center pane then renders
dashboard-owned viewport chrome around the service-owned stream.

## Implementation Notes

- Added `WorkspaceRemoteViewport` in
  `packages/dashboard/src/components/workspace-remote-viewport.tsx`.
- Wrapped the overview viewport in `WorkspaceRemoteViewport`, with the
  existing CDP screencast viewport as fallback when no workspace viewport URL
  is active.
- Made the no-session desktop empty state yield to valid workspace viewport
  URLs so service-owned browsers can be viewed even when no daemon session is
  active.
- Added left-pane View and Control URL push behavior from
  `WorkspaceNavigator`.
- Fixed primary action priority so controllable service-owned streams expose
  Control before daemon Focus.
- In control mode, the viewport queues `view_focus` before embedding the
  stream when a stable tab index is available.
- Preserved service tab ordering when deriving the focus index, so the
  dashboard does not send an index from a UI-sorted list.
- Added compact icon-only viewport tools for refresh, external open, and
  fullscreen. The mobile action row keeps the controls on one line and
  truncates the provider badge instead of wrapping.

## View Focus Request Shape

Rendered QA used a local fixture service and verified the dashboard submitted:

```json
{
  "action": "view_focus",
  "serviceName": "agent-browser-dashboard",
  "agentName": "operator",
  "taskName": "workspace-viewport-control",
  "params": {
    "index": 0,
    "maximize": true
  },
  "jobTimeoutMs": 5000
}
```

Clicking the left-pane Control action from `/` pushed:

```text
/?view=workspace%3Acontrol&workspace=browser%3Afixture-browser&browser=fixture-browser&session=fixture-session&tab=fixture-tab&profile=fixture-profile
```

The center viewport then mounted with an iframe and showed the queued focus
notice.

## Visual Inspection

Rendered inspection was run with `agent-browser` against a local dashboard at
`http://127.0.0.1:3400/`, backed by a local fixture service on
`http://127.0.0.1:3499`.

Screenshots:

- `/tmp/agent-browser-dashboard-workspace-navigator-slice-6/desktop-workspace-viewport-control-fixture-final.png`
- `/tmp/agent-browser-dashboard-workspace-navigator-slice-6/mobile-workspace-viewport-control-fixture-final.png`
- `/tmp/agent-browser-dashboard-workspace-navigator-slice-6/desktop-workspace-viewport-control-fixture.png`
- `/tmp/agent-browser-dashboard-workspace-navigator-slice-6/mobile-workspace-viewport-control-fixture.png`

Findings:

- Desktop shows the left pane and remote viewport in the first viewport without
  forcing the operator to scroll past summary chrome.
- The viewport header is compact enough to keep the iframe dominant, and the
  right rail toggle no longer collides with viewport controls.
- Mobile keeps the route state, iframe, focus notice, provider badge, and three
  viewport tools visible without horizontal overflow.
- The accessibility snapshot exposes the left-pane primary action as Control
  for controllable streams.

## Validation

Passed:

- `pnpm test:dashboard-workspace-navigator`
- `pnpm test:dashboard-view-streams`
- `pnpm test:dashboard-workspace-nodes`
- `pnpm test:dashboard-inspector-actions`
- `pnpm test:dashboard-browser-row-actions-render`
- `pnpm test:dashboard-browser-table`
- `pnpm test:dashboard-launcher-eligibility`
- `pnpm test:dashboard-profile-allocation`
- `pnpm build:dashboard`
- `pnpm validation:select -- --base HEAD --json`
- `git diff --check`

`pnpm build:dashboard` emitted the existing Next.js export warning about
rewrites, but the build completed successfully.

`pnpm validation:select -- --base HEAD --json` also recommended Rust, service
client, docs, and installed-skill checks because the worktree contains earlier
campaign changes in those areas. Those broader checks were already part of the
Slice 5 validation pass and were not rerun for this viewport-only increment.

## Remaining Gaps

- This slice proves dashboard chrome, URL restoration, iframe embedding, and
  the `view_focus` request shape with a deterministic fixture. It does not
  prove true interaction with a live Guacamole or `rdp_gateway` deployment.
- The viewport requeues `view_focus` on a full page reload because the control
  route is treated as an operator intent to focus before opening.
- Human takeover, lease display, and resume semantics remain Slice 7 work.

## Next Recommended Slice

Proceed to Slice 7: model operator takeover, pause or cooperative control, and
resume behavior through service-owned state so manual Guac control is explicit
and reversible.
