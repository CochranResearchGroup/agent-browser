# Left Pane Workspace Navigator Slice 4

Date: 2026-05-23

## Scope

Implemented the launch eligibility preview layer for the left-pane workspace
navigator campaign.

This slice adds a no-launch launcher model that derives browser/profile
eligibility from service-owned inputs:

- runtime profile records and profile allocation rows
- target readiness rows
- browser capability registry hosts, executables, capabilities,
  compatibility rows, and validation evidence
- access-plan responses when an operator fetches a plan for a candidate row
- service request action metadata from `/api/service/contracts`

The New workspace dialog now shows a compact browser/profile eligibility
preview. Rows stay visible when blocked, and disabled states use service-owned
reasons such as missing access-plan evidence, missing profile compatibility,
missing validation evidence, manual seeding, exclusive lease conflict, and
unsupported service request actions.

The slice intentionally does not add the mutating service launch button. The
existing local session Create behavior remains in place.

## Implementation Notes

- Added `packages/dashboard/src/lib/launcher-eligibility.ts` as a pure derived
  model for launcher rows and summary counts.
- Added `scripts/test-dashboard-launcher-eligibility.js` and
  `pnpm test:dashboard-launcher-eligibility`.
- Updated `WorkspaceNavigator` to fetch `/api/service/contracts` and
  `/api/service/browser-capability-registry` alongside `/api/service/status`.
- Added access-plan preview fetching for a selected launcher row through
  `/api/service/access-plan`.
- Updated the validation selector so launcher eligibility changes recommend
  the focused launcher test.
- Fixed a mobile mounting issue where the top New session action could set
  dialog state while `WorkspaceNavigator` was not mounted. On mobile, that
  action now switches to Workspaces before opening the dialog.

## Visual Inspection

Rendered inspection was run against the local dashboard at
`http://127.0.0.1:3104/service?view=service%3Aprofiles`.

Screenshots:

- `/tmp/agent-browser-dashboard-workspace-navigator-slice-4/desktop-launcher-preview-agent-browser.png`
- `/tmp/agent-browser-dashboard-workspace-navigator-slice-4/mobile-launcher-preview-agent-browser.png`
- `/tmp/agent-browser-dashboard-workspace-navigator-slice-4/desktop-launcher-preview-tight.png`
- `/tmp/agent-browser-dashboard-workspace-navigator-slice-4/mobile-launcher-preview-open.png`

Findings:

- Desktop preview is dense enough to show summary counts and four candidate
  rows without pushing the dialog below the viewport.
- Mobile preview opens from the top action after switching to the Workspaces
  tab, and the row list scrolls inside the dialog without overlapping the
  footer actions.
- The live service state has hundreds of retained profiles, so the dialog
  summarizes the visible rows instead of showing an all-combo blocked count.
- Path-like profile names are shortened to the final path segments so rows are
  scannable.

## Validation

Passed:

- `pnpm test:dashboard-launcher-eligibility`
- `pnpm test:dashboard-workspace-navigator`
- `pnpm test:dashboard-workspace-nodes`
- `pnpm test:dashboard-profile-allocation`
- `pnpm test:dashboard-browser-table`
- `pnpm test:dashboard-inspector-actions`
- `pnpm test:dashboard-view-streams`
- `pnpm test:dashboard-browser-row-actions-render`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:service-client-contract`
- `pnpm test:service-client-types`
- `pnpm build:dashboard`
- `pnpm validation:select -- --base HEAD --json`
- `git diff --check`

`pnpm build:dashboard` emitted the existing Next.js export warning about
rewrites, but the build completed successfully.

Selector recommendations for Rust, docs, and installed skill checks were not
run in this slice because they are triggered by pre-existing dirty surfaces
outside the launcher eligibility changes.

## Backend And UX Gaps

- There is no batched service-owned launcher eligibility endpoint yet. The UI
  derives candidates from status and registry evidence, then fetches
  access-plan evidence per visible candidate.
- The retained profile set is large enough that Slice 5 should add guided
  filtering before enabling launch submission.
- Many browser/profile combinations are blocked by missing profile
  compatibility or validation evidence. That is correct for this slice, but
  operators will need registry/preflight evidence before those rows can become
  launchable.
- The first visible stealth row currently reports no controllable viewport
  evidence in service state, so Guacamole or `rdp_gateway` interaction remains
  a Slice 6 deliverable.

## Next Recommended Slice

Proceed to Slice 5: guided browser/profile launch through the service request
queue. Do not bypass the access-plan and capability gates added in this slice.
