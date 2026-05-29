# Left Pane Workspace Navigator Slice 1

Date: 2026-05-23

## Scope

Implemented the first campaign slice from
`docs/dev/notes/2026-05-23-left-pane-workspace-navigator-campaign.md`: a pure
dashboard-layer `WorkspaceNode` model.

New implementation:

- `packages/dashboard/src/lib/service-workspaces.ts`
- `scripts/test-dashboard-workspace-nodes.js`
- `pnpm test:dashboard-workspace-nodes`

The model derives workspace nodes from daemon sessions, cached daemon tabs,
service browsers, service sessions, service tabs, profile allocations, jobs,
incidents, and view-stream metadata. It does not mutate service state or launch
browser work.

## Covered Fixtures

The focused fixture smoke covers:

- live service browser linked to service session, tab, profile, and running job
- retained process-exited browser
- disconnected browser with a retained incident and repair action
- profile lease conflict with disabled launch action
- auth-ready profile with enabled launch action
- manual-seeding-required profile with enabled Seed action
- controllable remote-headed browser with `rdp_gateway` view stream metadata
- daemon-only browser session with cached tab state

## Display Inference

The following values are still derived for display only:

- workspace grouping from health, jobs, incidents, readiness, and view-stream
  capability
- primary label preference order from service, task, agent, display name,
  profile name, then raw ID
- primary tab choice from active service tab when present, then first tab
- job linkage from explicit IDs in job payloads or matching service, agent, and
  task names
- profile-only workspace rows for launch or seeding candidates that do not yet
  have a browser node

Cleaner backend fields would reduce inference:

- canonical workspace ID that links browser, session, profile, tab, jobs, and
  incidents
- canonical primary tab ID per browser
- explicit launch eligibility status per profile and browser build
- explicit operator takeover and resume state
- normalized profile readiness reason and action code

## Validation

Commands run:

- `pnpm test:dashboard-workspace-nodes`
- `pnpm build:dashboard`
- `pnpm validation:select -- --base HEAD`

Rendered `agent-browser` inspection was not applicable for this slice because
no visual component or placeholder navigator was added. The next slice replaces
the rendered left pane and must include desktop and mobile `agent-browser`
screenshots.
