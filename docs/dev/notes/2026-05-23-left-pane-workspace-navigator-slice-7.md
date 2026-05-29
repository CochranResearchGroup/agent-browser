# Left Pane Workspace Navigator Slice 7

Date: 2026-05-23

## Scope

Slice 7 implemented the dashboard side of human takeover visibility. The
service contracts already expose retained session lease state with
`lease: "human_takeover"`, owner, cleanup, profile lease disposition, conflict
session IDs, and lease timestamps. They do not yet expose a service-owned
resume or takeover-release request action.

This slice therefore treats takeover as service-owned observed state only:

- derive takeover state from service session records in `WorkspaceNode`
- mark affected browser and retained-session nodes as blocked attention rows
- show owner, queue impact, selected browser, selected tab, cleanup, conflicts,
  and lease timing in the Service inspector
- surface a disabled Resume affordance with the backend contract gap as the
  reason
- keep true resume/release behavior out of the dashboard until the backend
  exposes a mutating service request action

## Implementation

- Added `WorkspaceNodeTakeover` to the dashboard workspace model.
- Browser-backed and retained session workspace rows now detect
  `human_takeover` leases and carry queue-impact, conflict, and waiting-job
  details.
- The left pane labels takeover rows with a `takeover` state badge, includes
  takeover owner and queue impact in search text, and preserves remote Control
  or View as the primary action when the affected browser still exposes a
  controllable stream. Resume remains the disabled fallback when no stream can
  be controlled.
- The Service session inspector now has an `Operator Takeover` section. Queue
  impact and the disabled Resume reason are ordered before timestamps so the
  decision state is visible in the first viewport.
- Service session rows use a warning tone when the retained lease is
  `human_takeover`.
- Service workspace content now packs grid rows to the top. Rendered QA showed
  the prior grid behavior stretched rows vertically and pushed session records
  toward the bottom of the panel.

## Contract Gap

Takeover is visible, but not reversible yet. The current service request action
set used by the fixture contained `view_focus`, `service_browser_close`,
`service_browser_repair`, and `service_tab`, but no resume or
takeover-release action.

The next backend slice should add a service-owned action that can release a
human takeover lease and resume paused or waiting agent work. Until then, the
dashboard must keep Resume disabled and must not synthesize release behavior
client-side.

## Rendered Inspection

Rendered inspection used `agent-browser` against a local dashboard at
`http://127.0.0.1:3407/`, backed by a local fixture service on
`http://127.0.0.1:3497`.

Screenshots:

- `/tmp/agent-browser-dashboard-workspace-navigator-slice-7/desktop-takeover-home.png`
- `/tmp/agent-browser-dashboard-workspace-navigator-slice-7/desktop-service-sessions-takeover-dense-final.png`
- `/tmp/agent-browser-dashboard-workspace-navigator-slice-7/desktop-service-session-inspector-takeover-final.png`
- `/tmp/agent-browser-dashboard-workspace-navigator-slice-7/mobile-workspaces-takeover-final.png`
- `/tmp/agent-browser-dashboard-workspace-navigator-slice-7/mobile-service-session-inspector-takeover-top.png`

Findings:

- Desktop left pane shows the takeover row in the first viewport with owner
  context, takeover badge, remote Control when a controllable stream exists,
  disabled Resume fallback, and queue impact.
- Service Sessions rows now appear immediately below filters instead of being
  pushed down by stretched grid rows.
- The right inspector first viewport shows owner, queue impact, and the
  disabled Resume reason without requiring the operator to scroll.
- Mobile Workspaces view preserves the takeover badge, remote Control when
  available, disabled Resume fallback, and queue-impact copy without horizontal
  overflow.
- The dashboard URL retained workspace, browser, session, tab, profile, and
  job identity after selecting the takeover row.

## Validation

Passed:

- `pnpm test:dashboard-workspace-nodes`
- `pnpm test:dashboard-workspace-navigator`
- `pnpm test:dashboard-inspector-actions`
- `pnpm test:dashboard-browser-table`
- `pnpm test:dashboard-view-streams`
- `pnpm test:dashboard-browser-row-actions-render`
- `pnpm test:dashboard-launcher-eligibility`
- `pnpm test:dashboard-profile-allocation`
- `pnpm build:dashboard`
- `pnpm validation:select -- --base HEAD --json`
- `git diff --check`

The validation selector also recommended Rust, docs, service-client, and skill
sync checks because the working tree already contains broader campaign changes
outside this slice. This slice changed only dashboard takeover presentation,
dashboard CSS density, and focused dashboard contract tests.

## Remaining Risk

- Resume and takeover release are not implemented because the backend contract
  is missing.
- Live takeover, paused, cooperative, and resume states still need backend
  authority and live smokes before this can be called a complete takeover flow.
- The disabled Resume affordance is intentionally conservative. It should be
  wired to a service-owned request only after contract, schema, MCP, HTTP, and
  generated client surfaces agree on the action. This must not suppress Control
  or View for browsers whose retained stream metadata is still controllable.
