# Service View Redesign Plan

Date: 2026-05-18

## Trigger

The dashboard Service tab currently works technically, but its information
architecture is wrong for an operator surface. It presents large generic panels,
unexplained retained-state counts, a row-list managed browser section, and
always-visible left and right panes whose role is unclear in a service-control
view.

This note is a planning checkpoint before further Service view implementation.
The goal is to develop the page intentionally instead of polishing the current
card stack.

## Current Findings

Live service-state checks on 2026-05-18 showed:

- `profiles`: 331 service profile records
- runtime profile directories under `~/.agent-browser/runtime-profiles`: 22
- `browsers`: 178 retained browser records
- browser health split: 168 `not_started`, 8 `process_exited`, 1 `faulted`, 1 `ready`
- `sessions`: 161 retained session records
- `tabs`: 97 retained tab records
- `jobs`: 200 retained jobs
- `events`: 100 retained events
- control plane: worker `Ready`, browser health `Ready`, queue depth `0` of `256`

These numbers are not all live fleet counts. Several are retained-state and
bounded-log counts. Displaying them as equally prominent KPI cards misleads the
operator.

Specific semantics that need to be explicit:

- Profile count is a service profile record count, not a count of real profile
  directories. It includes configured, persisted, observed, custom-path, and
  runtime profile records.
- Browser count is a retained browser-record count, not live browser count.
  `not_started` records are placeholders or retained lifecycle records, not
  active Chrome processes.
- Session count is retained service session state. It can include expired,
  released, abandoned, and historical session records unless pruned.
- Job count is a retained bounded job log. A value of `200` means the job log is
  at its retention cap, not that 200 jobs are currently running.
- `8 recent control jobs` is the current Service view query window, not a
  meaningful health signal by itself.
- Operator identity defaulting to `dashboard` means the UI will attribute
  incident actions to a generic dashboard actor when no operator name is set.
  This is acceptable as a fallback but poor UX. It should be labelled as an
  audit actor, not a logged-in user identity.

## Product Direction

The Service tab should become an operations console for a browser-control
service. It should prioritize:

- live fleet state first
- retained-state explanations second
- traceability by service, agent, task, profile, browser, and tab
- fast triage of degraded browsers, blocked profiles, stuck jobs, and monitor
  incidents
- direct inspection of browser view streams when available

The page should avoid decorative dashboards. If an element does not help an
operator decide what to inspect or do next, it should be removed or collapsed.

## Layout Plan

### Shell

- Add small attractive collapse buttons for the left and right panes.
- The left pane should be optional context, not mandatory chrome.
- The right pane should be optional detail or activity, not a permanent space
  tax.
- Collapsed panes should preserve state and reopen to the previous size.
- Use compact icon buttons with clear hover labels, keyboard focus states, and
  local persistence.

Recommended pane roles:

- Left pane: navigation and browser or service hierarchy.
- Center pane: primary Service operations surface.
- Right pane: selected-detail inspector, trace drawer, chat, or activity.

When Service is the active top-level section, the default should be a wide
center operations surface with both side panes collapsible.

### Top Status Strip

Replace the five large health cards with a single compact status strip:

- worker state light
- queue depth light
- browser health light
- live browser count light
- incident or monitor attention light
- reconciliation age light

Each indicator should have:

- semantic color
- short label
- hover or click detail
- optional link to a filtered view

Remove the manual refresh button. The page auto-refreshes already. Keep
`Reconcile` as an explicit operator action, but move it to an overflow or
actions menu because it mutates service state.

### Managed Entity Summary

Remove the large managed entity stats card from the primary scan path.

Replace it with a compact retained-state disclosure:

- Live: active browsers, active sessions, leased profiles, queue depth
- Retained: profiles, browser records, sessions, jobs, events
- Health/noise warning when retained counts are dominated by inert records

The disclosure should explain record semantics in plain language and link to
cleanup actions or docs. It should not occupy prime vertical space.

## Managed Browsers Table

The managed browser section should become a data table, not a card list.

Required columns:

- health
- browser ID
- live state or lifecycle
- host
- PID
- profile
- active sessions
- tabs
- view stream availability
- last observed
- last error or reason
- service and task labels when known

Table requirements:

- sortable columns
- adjustable column widths
- column visibility controls
- text filter
- health filter
- host filter
- live-only toggle
- retained-only or inert toggle
- view-stream available toggle
- default sort: non-ready and live records first, then last observed descending
- row click opens the right-pane detail inspector
- View button opens the remote-view dialog when a stream exists

This table should be the main Service view surface because browser lifecycle is
the operator's primary concern.

## Jobs, Sessions, Profiles, And Incidents

These should be secondary tabs or drawers below or beside the browser table,
not stacked full-width panels.

Recommended sections:

- Browsers
- Profiles
- Sessions
- Jobs
- Incidents
- Events and trace

Profiles should distinguish:

- real runtime profile directories
- configured profile records
- custom external profile paths
- observed profile records
- stale or orphaned records
- lease state
- authenticated target readiness

Sessions should distinguish:

- active leases
- released records
- expired records
- abandoned candidates
- linked browser and profile
- owning service, agent, and task

Jobs should distinguish:

- queue depth and currently running job
- recent retained jobs
- retention cap
- failed or timed-out jobs
- cancellable queued or waiting jobs

Incidents should remain prominent when non-empty, but they should be grouped by
recommended action instead of placed below a long stack of generic panels.

## Copy And Semantics

Rename or clarify the misleading labels:

- `Profiles` becomes `profile records` unless showing real directories.
- `Browsers` becomes `browser records` unless filtered to live browsers.
- `Sessions` becomes `retained sessions` unless filtered to active leases.
- `Jobs` becomes `retained jobs` or `job log` when showing the capped log.
- `Operator identity` becomes `Audit actor`.
- Default placeholder should be `Set audit actor`, with fallback text saying
  actions use `dashboard` if unset.

The UI should define these terms inline with hover text or a help drawer. The
operator should not need to know the storage model to understand the page.

## Implementation Slices

### Slice 1: Layout Discipline

- Add collapsible left and right panes.
- Persist collapsed state and widths locally.
- Hide duplicate Service panel surfaces.
- Move Service view to a wide center-first default.
- Remove the refresh affordance and move `Reconcile` into an action menu.

Validation:

- dashboard build
- keyboard focus check for collapse buttons
- visual smoke at the installed dashboard route

### Slice 2: Status Strip And Count Semantics

- Replace five health cards with compact indicator lights.
- Add hover/click details for worker, queue, browser health, live browsers,
  incidents, and reconciliation.
- Replace managed entity card with a compact retained-state disclosure.
- Clarify profile, browser, session, and job count semantics.

Validation:

- dashboard build
- live status smoke against `/api/service/status`
- verify counts match service JSON

### Slice 3: Managed Browser Table

- Replace `BrowserRow` list with a table component.
- Add sorting, filtering, column resizing, and column visibility.
- Add live-only and health filters.
- Keep view-stream inspection as a row action.
- Open browser details in the right pane when available.

Validation:

- dashboard build
- unit or script smoke for sort/filter behavior
- live smoke with current retained browser records

Status on 2026-05-18: partially implemented. The table now defaults to an
actionable browser-record filter so live, degraded, and otherwise inspectable
records stay in the primary scan path while inert `not_started` placeholders
move behind retained or all-record filters. Column visibility controls let an
operator hide secondary fields without losing row inspection.

### Slice 4: Secondary Work Surfaces

- Move profiles, sessions, jobs, incidents, events, and trace into deliberate
  tabs or drawers.
- Make Jobs explain retention cap and current queue separately.
- Make Profiles separate actual directories from service records.
- Make Sessions distinguish active from retained or stale records.

Validation:

- dashboard build
- live smoke against profiles, sessions, jobs, incidents, events, and trace
  endpoints

Status on 2026-05-18: implemented as a compact tabbed workspace below the
managed browser table. Profiles, incidents, sessions and tabs, jobs, and
events or trace now share one secondary surface so browser records remain the
primary scan path.

### Slice 5: Retained-State Cleanup UX

- Add warnings when retained-state counts are dominated by inert records.
- Surface dry-run cleanup actions without applying mutation by default.
- Link to `service prune-retained` and `service repair-retained` semantics.
- Keep destructive cleanup behind explicit confirmation.

Validation:

- no native dialogs
- dashboard build
- dry-run cleanup command smoke only

Status on 2026-05-18: implemented as a retained-state warning with dry-run
prune and repair actions, summarized candidate and skipped counts, and a
shadcn-style confirmation dialog before applying reviewed cleanup. The dashboard
uses the existing service request path and does not add dashboard-only cleanup
authority.

## Open Questions

- Should browser records default to live-only, with retained records behind a
  filter, or should all records stay visible with a prominent live/retained
  split?
- Should audit actor be local browser state only, or should it eventually bind
  to the authenticated dashboard user once ingress authentication identity is
  available?
- Should stale retained records be automatically summarized into a cleanup
  recommendation, or should the dashboard only expose explicit operator tools?
- Should the left pane show sessions, services, or a combined hierarchy of
  service → agent → task → browser?

## Recommended Next Step

Implement Slice 1 first. Collapsible panes and a center-first Service layout
will create the space needed for the browser table without committing to table
schema details too early.
