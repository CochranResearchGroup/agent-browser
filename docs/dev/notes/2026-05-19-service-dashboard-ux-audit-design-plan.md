# Service Dashboard UX Audit And Design Plan

Date: 2026-05-19

## Scope

This note pauses feature implementation and audits the Service dashboard as an
operator workbench for the agent-browser service.

The audit covers the current React dashboard surface in
`packages/dashboard/src/app/page.tsx`,
`packages/dashboard/src/components/service-panel.tsx`, and
`packages/dashboard/src/app/globals.css`. It also reconciles the Service view
work against the existing roadmap notes:

- `docs/dev/notes/2026-04-22-agent-browser-service-roadmap.md`
- `docs/dev/notes/2026-04-22-service-roadmap-discipline-checkpoint.md`
- `docs/dev/notes/2026-05-18-service-view-redesign-plan.md`

Graphiti discovery for `agent_browser_main` was healthy and returned the
service-roadmap discipline note as relevant context. The important design rule
from that note still applies: the dashboard should consume authoritative
service state. It should not invent service semantics in the frontend.

## Product Position

The Service dashboard should be an operations console, not a decorative
dashboard. It is for operators who need to understand many concurrent services,
agents, profiles, browser processes, tabs, jobs, incidents, and remote-view
streams without losing the thread of ownership.

The dominant personality should be calm, dense, and precise:

- the work surface dominates
- supporting chrome recedes
- live fleet state comes before retained history
- every count says whether it is live, active, retained, or capped history
- every actionable row can explain who owns it, what task last touched it, and
  what the service recommends next

## Current-State Audit Findings

### 1. Vertical Document Layout Still Fights The Workbench Model

Severity: High

Evidence:

- `packages/dashboard/src/components/service-panel.tsx:4484` wraps the Service
  page in one vertical `ScrollArea`.
- `packages/dashboard/src/components/service-panel.tsx:4644` places Managed
  browsers in a card, then `packages/dashboard/src/components/service-panel.tsx:4667`
  places the workspace tabs below that card.
- `packages/dashboard/src/app/globals.css:1644` gives the browser table a
  horizontal scroll container, but no bounded vertical table viewport.

Impact:

The primary table can consume the page and push Profiles, Incidents, Sessions,
Jobs, Events, and Trace below the fold. That is workable for a report page, but
not for a service control plane where operators need stable access to list,
detail, and secondary evidence at the same time.

Design fix:

Move to a fixed-height workbench spine:

- sticky compact service header
- primary browser table in a bounded scroll region
- persistent workspace rail or lower split that remains reachable without
  scrolling past every browser row
- right inspector as the selected-record detail surface

### 2. The Browser Table Renders The Full Filtered Set

Severity: High

Evidence:

- `packages/dashboard/src/components/service-panel.tsx:1951` derives the
  filtered browser rows.
- `packages/dashboard/src/components/service-panel.tsx:2160` maps every
  filtered browser into the table body.
- The 2026-05-18 plan recorded real retained data with 178 browser records and
  331 profile records.

Impact:

Large retained-state sets are normal for this product. Rendering all filtered
browser rows is acceptable today only because the numbers are modest. The design
should assume hundreds or thousands of records as service adoption grows.

Design fix:

Use a bounded viewport first, then virtualize when the row count crosses 50.
The table should also expose an explicit result cap or row window in its status
line so operators understand whether they are seeing all results.

### 3. Service Header And Audit Actor Consume Prime Space

Severity: Medium

Evidence:

- `packages/dashboard/src/components/service-panel.tsx:4422` renders a dedicated
  Service control plane header.
- `packages/dashboard/src/components/service-panel.tsx:4459` renders a separate
  operator identity card before any service state.
- `packages/dashboard/src/app/globals.css:502` styles the operator card as its
  own full-width row.

Impact:

The audit actor matters, but it is not the operator's primary task. Keeping it
as a full row reinforces the earlier "stack of panels" problem and steals
vertical space from fleet state.

Design fix:

Move audit actor into the account chip menu or a compact service action menu.
The main header should show location, service health, and one action slot. The
actor can be displayed as a small chip with a clear label such as `Audit actor:
dashboard` and edited from a menu.

### 4. Status Lights Are Better Than KPI Cards, But Still Too Card-Like

Severity: Medium

Evidence:

- `packages/dashboard/src/components/service-panel.tsx:4486` renders six status
  lights.
- `packages/dashboard/src/app/globals.css:595` styles each status light with a
  border, background, shadow, hover lift, and animation.
- `packages/dashboard/src/components/service-panel.tsx:895` implements status
  lights as focusable tooltip triggers using a `div`.

Impact:

The status strip is directionally right, but the lights still visually compete
with the table. The focusable `div` also behaves like an interactive control
even though it does not perform an action.

Design fix:

Make status lights quieter and more semantic:

- use status chips with minimal tint and no hover lift unless clicking filters
  the page
- if a light has no action, expose it as status text with tooltip detail and do
  not make it look clickable
- if a light filters the page, make it a real `button` with an accessible label
  and pressed state

### 5. Secondary Workspace Copy Is Too Generic

Severity: Medium

Evidence:

- `packages/dashboard/src/components/service-panel.tsx:4674` uses the heading
  `Secondary work surfaces`.
- `packages/dashboard/src/components/service-panel.tsx:4675` says browser
  records stay primary and these tabs are for routing, sessions, incidents,
  jobs, and trace.

Impact:

The phrase "Secondary work surfaces" describes the layout implementation, not
the operator's task. It does not help an operator decide where to go.

Design fix:

Rename this area to `Service records` or `Operational records`. Make each tab
label task-oriented:

- Profiles: identity and routing
- Incidents: attention and remedies
- Sessions: leases and tabs
- Jobs: queue and history
- Events: timeline and trace

### 6. Profiles Need A Product Model, Not Just Rows

Severity: High

Evidence:

- `packages/dashboard/src/components/service-panel.tsx:4698` makes profiles one
  workspace tab.
- `packages/dashboard/src/components/service-panel.tsx:4701` labels them
  `profile allocation rows`.
- The service roadmap defines profiles as durable identity, user data dir,
  login hints, site policy, and profile storage settings.

Impact:

Profile management is central to agent-browser. Operators think in terms of
target site, login identity, browser build compatibility, seeding state,
keyring posture, and account readiness. A flat allocation-row list will not be
enough for routing or debugging.

Design fix:

Design the Profiles workspace around an identity matrix:

- site or account identity
- primary browser build and host
- compatible browser families
- runtime profile path and storage policy
- login readiness and seeding state
- lease state
- last verified target service
- conflicts, stale records, and repair actions

### 7. Right Inspector Is Correct, But Its Relationship To Rows Is Understated

Severity: Medium

Evidence:

- `packages/dashboard/src/app/page.tsx:83` opens the right pane when a service
  selection is inspected.
- `packages/dashboard/src/app/page.tsx:133` routes service selections into
  `ServiceDetailInspector`.
- `packages/dashboard/src/app/globals.css:2233` styles the inspector as a
  separate detail surface.

Impact:

The inspector is the right pattern. The table and record rows need to make the
list-detail relationship more obvious: selection, focus restoration, selected
row state, and empty inspector copy should all reinforce that row detail lives
on the right.

Design fix:

Add selected-row state across browser, profile, incident, session, tab, and job
records. Keep row actions separate from row selection. The empty inspector
should explain: "Select a browser, profile, session, tab, job, or incident to
inspect service-owned detail."

### 8. Mobile Service Mode Is A Reduced Copy, Not A Designed Mobile Workbench

Severity: Medium

Evidence:

- `packages/dashboard/src/app/page.tsx:322` switches mobile to top tabs.
- `packages/dashboard/src/app/page.tsx:352` embeds the Service panel in a padded
  dashboard pane.

Impact:

Mobile does not need full operator parity, but it needs a deliberate mode:
quick health, incidents, active browsers, and remote-view handoff. A direct
copy of the desktop vertical panel is likely to become cramped.

Design fix:

Define mobile as a triage view:

- status strip
- active and degraded browser rows
- incident attention
- remote-view open action
- detail drawer rather than persistent right inspector

### 9. Visual System Needs Fewer Surface Treatments

Severity: Medium

Evidence:

- `packages/dashboard/src/app/globals.css:580` shares card treatment across
  summary, timeline, trace, and empty states.
- `packages/dashboard/src/app/globals.css:800` adds a separate workspace card.
- `packages/dashboard/src/app/globals.css:1567` adds a table shell inside the
  summary card.

Impact:

The UI has moved away from the original pile of panels, but still uses many
nested surfaces with borders, tints, radius, and shadow. Dense product UI should
use boundaries only where they clarify interaction.

Design fix:

Define three surface tiers:

- page chrome: header, nav, collapse controls
- work surface: table or workspace region, mostly borderless
- detail or intervention surface: inspector, alert, dialog, confirmation

Then remove card treatment from regions that are plain layout.

## Target Information Architecture

### Desktop Workbench

The desired desktop layout is a three-zone workbench:

1. Left context rail
   - collapsible
   - service, browser, profile, and session hierarchy
   - not required to understand the current table

2. Center work surface
   - compact service header
   - status lights and filters
   - bounded primary browser table
   - persistent operational-record workspace

3. Right inspector
   - collapsible
   - selected record details
   - incident actions, job cancellation, profile verification, stream launch
   - remote-view controls when available

The center work surface should not feel like a sequence of cards. It should
feel like a tool.

### Primary Browser Table

The browser table remains the main object on the page. It should answer these
questions without opening detail:

- Is this browser live, degraded, retained, or inert?
- Which host and browser build owns it?
- Which profile is attached?
- Which service, agent, and task last touched it?
- Does it have active sessions or tabs?
- Is remote viewing available?
- What is the latest actionable error?
- What should the operator do next?

Required next table improvements:

- bounded vertical viewport
- selected row state
- virtualized rows or capped row window above 50 rows
- row action group that distinguishes inspect, view, focus, close, and repair
- explicit live, actionable, retained, and all-record filters
- service, agent, task, host, health, browser build, and view-stream filters

### Operational Records Workspace

The lower or adjacent workspace should remain reachable without scrolling past
all browser rows. It should have compact tabs:

- Profiles: identity, routing, readiness
- Incidents: attention, grouped remedies, acknowledgement
- Sessions: leases, tabs, owners
- Jobs: queue, active work, retained history
- Events: timeline, trace, health transitions

The workspace is not secondary because it is unimportant. It is secondary
because browser lifecycle is the first scan path. Copy should say what the
workspace does, not call it secondary.

### Remote View Integration

The roadmap says raw Guacamole or VNC screens are implementation details. The
dashboard root remains the React operations console. Remote view belongs behind
a browser or tab row affordance.

Desired behavior:

- rows show a remote-view indicator only when the service has a view stream
- clicking the indicator opens an iframe view owned by the dashboard
- fullscreen is available from the dashboard chrome
- before opening, the service focuses or maximizes the intended browser or tab
  inside the remote desktop viewport
- human takeover is recorded as a lease and can pause or coordinate queued work

## Implementation Plan

### Phase 1: Workbench Layout Contract

Goal: make the page structure correct before adding more features.

Tasks:

- Replace the single long Service `ScrollArea` with a workbench layout that
  bounds the browser table and keeps operational records reachable.
- Move audit actor into a compact account or service action menu.
- Keep the status strip sticky and quieter.
- Add selected-row state and selected-row styling for browser records.
- Update empty right-inspector copy so the list-detail model is explicit.

Validation:

- `pnpm test:dashboard-browser-table`
- `pnpm test:dashboard-inspector-actions`
- `pnpm build:dashboard`
- live visual smoke at desktop width and narrow width

### Phase 2: Table Scale And Interaction

Goal: make retained-state scale safe.

Tasks:

- Add a bounded table viewport.
- Add virtualization or a visible row-window once filtered rows exceed 50.
- Keep header sorting and resize handles working inside the bounded viewport.
- Add filters for health, host, browser build, service, agent, task, and
  view-stream availability when backed by service data.

Validation:

- focused table contract test
- visual smoke with a large retained-state fixture or live state
- keyboard pass for filtering, sorting, row selection, column resizing, and
  action buttons

### Phase 3: Profiles As Identity Routing

Goal: make profile management match the agent-browser product model.

Tasks:

- Rename the Profiles workspace around identity and routing.
- Separate profile records, runtime directories, custom paths, and observed
  state.
- Show site or account identity, target-service readiness, browser-build
  compatibility, seeding state, keyring posture, lease state, and verification
  freshness where the service exposes those fields.
- Do not invent missing readiness fields in the frontend. If the service does
  not expose them, add service-owned fields in the same slice.

Validation:

- service contract tests for any new authoritative profile fields
- dashboard test for profile grouping and retained-versus-real directory copy
- docs update for profile semantics if fields become user-facing

### Phase 4: Operational Records And Remote View

Goal: connect service records, tabs, and streams into an inspectable control
plane.

Tasks:

- Make Sessions and Tabs show active leases first, retained history second.
- Add row affordances for remote view only when a view stream is available.
- Keep Guacamole and other providers hidden behind dashboard-owned view chrome.
- Add human takeover status and queue coordination only when the service owns
  the lease state.

Validation:

- view-stream smoke with iframe/fullscreen route
- service event or job smoke proving view open and human takeover are recorded
- dashboard inspector action tests

### Phase 5: Visual And Accessibility Polish

Goal: make the workbench feel intentional without adding noise.

Tasks:

- Collapse surface tiers to page chrome, work surface, and detail or
  intervention surface.
- Remove hover lift from passive status indicators.
- Replace focusable non-action `div` status lights with semantic status or real
  buttons.
- Ensure every icon-only control has an accessible name.
- Verify keyboard traversal from status, filters, table rows, workspace tabs,
  inspector actions, and remote-view controls.
- Respect reduced motion for page and pane animations.

Validation:

- keyboard-only smoke
- reduced-motion visual check
- `pnpm build:dashboard`

## Non-Goals

- Do not add new frontend-only service concepts.
- Do not add more cards to explain unclear state. Fix the state model or the
  copy.
- Do not make Guacamole or any stream provider the root UX.
- Do not block backend work on a perfect dashboard redesign.

## Recommended Next Step

Implement Phase 1 first. The best next slice is the workbench layout contract:
bounded browser table, reachable operational records, compact audit actor, and
explicit selected-row to right-inspector behavior. This directly addresses the
current UX failure while preserving the roadmap rule that the dashboard consumes
service-owned state.

## Progress

### 2026-05-19 Phase 1 Start

Implemented the first workbench-layout slice:

- moved the full-width operator identity row into the Service actions menu as
  an `Audit actor` control
- renamed `Secondary work surfaces` to `Operational records`
- bounded the browser table viewport so operational records remain reachable
- made the status strip sticky and visually quieter
- added selected browser-row state so the table reflects right-pane inspection
- added dashboard contract assertions for the table bound, selected-row wiring,
  operational-record copy, and removed operator card

Validation:

- `pnpm test:dashboard-browser-table`
- `pnpm test:dashboard-inspector-actions`
- `pnpm build:dashboard`
- live dashboard DOM smoke at `http://127.0.0.1:4850` with an isolated profile
  confirmed the Service tab rendered, `Operational records` was visible, the
  full-width operator card was absent, the status strip was sticky, and the
  browser table used a bounded auto-scroll viewport

Visual evidence:

- `/tmp/agent-browser-service-workbench-phase1.png`

### 2026-05-19 Phase 2 Table Windowing

Implemented the first table-scale slice without adding a frontend-only data
model:

- added an initial 50-row browser table window for large retained-state sets
- added explicit hidden-row feedback with `Show more` and `Show all` controls
- reset the row window when filtering or sorting changes
- kept selected-row, sorting, resizing, density, and column visibility behavior
  on the same table path
- added contract assertions for the row limit constants, visible row window,
  hidden-row controls, and compact row-window layout

Validation:

- `pnpm test:dashboard-browser-table`
- `pnpm test:dashboard-inspector-actions`
- `pnpm test:dashboard-view-streams`
- `pnpm build:dashboard`
- git whitespace check
- validation selector JSON mode
- live dashboard DOM smoke at `http://127.0.0.1:4850` with an isolated profile
  confirmed 50 visible browser rows from 180 filtered records, row-window
  controls showing 130 hidden records, and the bounded table viewport still
  using auto scroll

Visual evidence:

- `/tmp/agent-browser-service-table-row-window-phase2.png`

### 2026-05-19 Phase 2 Service-Backed Field Filters

Continued the table-scale slice by adding field filters that are backed by
service browser records:

- added native select filters for browser health, browser host, and view-stream
  availability
- added a browser-build filter only when service browser records expose
  `browserBuild` values
- included `browserBuild` in browser search text when the field exists
- reset the row window when any field filter changes
- kept browser-build semantics service-owned rather than inferring build from
  host, executable path, or profile name
- added contract assertions for filter state, option derivation, filter logic,
  conditional browser-build rendering, and compact select styling

Validation:

- `pnpm test:dashboard-browser-table`
- `pnpm test:dashboard-inspector-actions`
- `pnpm test:dashboard-view-streams`
- `pnpm build:dashboard`
- git whitespace check
- validation selector JSON mode
- live dashboard DOM smoke at `http://127.0.0.1:4850` confirmed health, host,
  and stream filters rendered from current service data. Browser-build filter
  was correctly hidden because current browser records did not expose
  `browserBuild`. Selecting health `faulted` reduced the table count from
  `50 of 180 filtered` to `1 of 1 filtered`; selecting `View stream available`
  after that reduced it to `0 of 0 filtered`.

Visual evidence:

- `/tmp/agent-browser-service-table-field-filters-phase2.png`

### 2026-05-19 Phase 2 Keyboard Row Navigation

Completed the table-scale keyboard hardening slice for the managed browser
table:

- added stable row button refs so keyboard focus can move within the current
  visible row window
- added Arrow Up and Arrow Down navigation from browser row links, with
  selection kept in sync with the right-side inspector
- added Home and End navigation to jump to the first or last visible browser
  row without escaping the current filtered window
- added a screen-reader hint that documents the row navigation keys
- kept field-filter traversal on native select controls in DOM order rather
  than adding custom keyboard traps
- added contract assertions for row refs, keyboard handling, and the accessible
  keyboard hint

Validation:

- `pnpm test:dashboard-browser-table`
- `pnpm test:dashboard-inspector-actions`
- `pnpm test:dashboard-view-streams`
- `pnpm build:dashboard`
- git whitespace check
- validation selector JSON mode
- live dashboard DOM smoke at `http://127.0.0.1:4850` confirmed the Service
  tab rendered, browser row links exposed the keyboard hint, and Arrow Down,
  Arrow Up, End, and Home changed the selected browser within the visible row
  window

Visual evidence:

- `/tmp/agent-browser-service-table-keyboard-phase2.png`

### 2026-05-19 Phase 3 Profile Identity And Routing

Started the Profiles workspace product-model slice using service-owned profile
allocation fields:

- renamed the visible Profiles count from allocation rows to identity and
  routing rows
- added a compact profile routing strip for target identities, login
  identities, authenticated targets, profiles with browsers, pinned browser
  builds, and readiness attention
- extended dashboard profile allocation typing to include service contract
  fields for browser build, account identities, and browser summaries
- changed each profile row from a generic holder summary into a routing row
  that shows target identity, login identity, browser build, keyring policy,
  primary browser, holder count, waiting count, tab count, service, agent,
  task, and conflicts
- added a readiness attention badge when service readiness indicates manual
  seeding, stale state, failed verification, or unverified post-close state
- extended profile allocation detail with primary target, primary login,
  primary browser, account identities, browser build, and browser summaries
- kept the dashboard as a consumer of authoritative service allocation data
  rather than deriving profile suitability from profile names or browser paths

Validation:

- `pnpm test:dashboard-profile-allocation`
- `pnpm test:dashboard-browser-table`
- `pnpm test:dashboard-inspector-actions`
- `pnpm test:dashboard-view-streams`
- `pnpm build:dashboard`
- git whitespace check
- validation selector JSON mode
- live dashboard DOM smoke at `http://127.0.0.1:4850` confirmed the Profiles
  tab rendered the identity and routing strip plus routing cells for target,
  login, browser build, and keyring

Visual evidence:

- `/tmp/agent-browser-service-profile-routing-phase3.png`

### 2026-05-19 Phase 3 Profile Routing Filters

Continued the Profiles workspace findability slice:

- added native select filters for target identity, login identity, browser
  build, and readiness attention
- derived target identity options from service profile readiness, target
  service IDs, and authenticated service IDs
- derived login identity options from service profile readiness login IDs and
  account identity fields
- kept browser-build filtering conditional on service allocations exposing a
  `browserBuild` value
- applied profile field filters before text search so the free-text query
  remains a secondary narrowing tool
- preserved the dashboard as a service-owned-state consumer and avoided
  profile-name or path-based routing inference
- added contract assertions for profile filter state, option derivation,
  filter logic, rendered labels, and compact filter styling

Validation:

- `pnpm test:dashboard-profile-allocation`
- `pnpm test:dashboard-browser-table`
- `pnpm test:dashboard-inspector-actions`
- `pnpm test:dashboard-view-streams`
- `pnpm build:dashboard`
- git whitespace check
- validation selector JSON mode
- live dashboard DOM smoke at `http://127.0.0.1:4850` confirmed target,
  login, readiness, and conditional browser-build filters rendered in the
  Profiles tab. The current service data exposed no target or login identity
  option values, but did expose one browser build option and the readiness
  filter labels.

Visual evidence:

- `/tmp/agent-browser-service-profile-filters-phase3.png`

### 2026-05-19 Phase 3 Profile Row Selection

Completed the Profiles workspace list-detail behavior slice:

- added selected profile allocation row state so the Profiles workspace mirrors
  the browser table list-detail pattern
- kept selected profile rows synchronized with right-inspector profile detail
  by setting the selected profile ID before profile allocation lookup
- added stable profile row refs for keyboard focus movement
- added Arrow Up and Arrow Down navigation between visible profile routing rows
- added Home and End navigation to jump to the first or last visible profile
  routing row
- added a screen-reader hint documenting profile row keyboard navigation
- added selected-row styling for profile allocation rows
- added contract assertions for selected state, row refs, keyboard handling,
  rendered accessibility hooks, and selected-row styling

Validation:

- `pnpm test:dashboard-profile-allocation`
- `pnpm test:dashboard-browser-table`
- `pnpm test:dashboard-inspector-actions`
- `pnpm test:dashboard-view-streams`
- `pnpm build:dashboard`
- git whitespace check
- validation selector JSON mode
- live dashboard DOM smoke at `http://127.0.0.1:4850` confirmed profile row
  selection, selected-row styling, `aria-current`, and Arrow Down, Arrow Up,
  End, and Home keyboard navigation within the visible profile row window

Visual evidence:

- `/tmp/agent-browser-service-profile-selection-phase3.png`

### 2026-05-19 Phase 4 Browser Ownership Chips

Started the primary browser table ownership slice:

- added an `Ownership` browser table column for service, agent, and task
  evidence
- derived browser ownership from service sessions linked by `browserIds` and
  browser `activeSessionIds`
- included ownership evidence in browser table text search
- added compact ownership chips for service, agent, and task values
- passed service sessions into the browser table instead of inferring ownership
  from browser names, profile IDs, or paths
- kept ownership as a visible-table column that can still be hidden through the
  existing column layout menu
- added contract assertions for service-backed ownership derivation, ownership
  search, table column wiring, row props, and chip styling

Validation:

- `pnpm test:dashboard-browser-table`
- `pnpm test:dashboard-profile-allocation`
- `pnpm test:dashboard-inspector-actions`
- `pnpm test:dashboard-view-streams`
- `pnpm build:dashboard`
- git whitespace check
- validation selector JSON mode
- live dashboard DOM smoke at `http://127.0.0.1:4850` confirmed the
  `Ownership` header rendered after resetting the table view, 50 ownership
  cells rendered, and ownership search could find a service-owned browser row

Visual evidence:

- `/tmp/agent-browser-service-browser-ownership-phase4.png`
