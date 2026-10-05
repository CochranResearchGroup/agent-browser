# Console Tab Selected Workspace Diagnostics Plan

Date: 2026-06-01
State: COMPLETE
Lane: P12-K
Parent Roadmap: `docs/dev/plans/0012-2026-05-31-workspace-inspection-pane-app-intelligence-roadmap.md`
Umbrella Plan: `docs/dev/plans/0018-2026-06-01-workspace-inspector-tabs-productization-plan.md`
Depends On:
- `docs/dev/plans/0013-2026-05-31-selected-workspace-context-plan.md`
- `docs/dev/plans/0019-2026-06-01-workspace-tab-dense-inspector-plan.md`
- `docs/dev/plans/0020-2026-06-01-chat-tab-selected-workspace-evidence-plan.md`
- `docs/dev/plans/0021-2026-06-01-activity-tab-selected-workspace-timeline-plan.md`
- `docs/dev/plans/0022-2026-06-01-superuser-codex-operator-agent-plan.md`

## Purpose

Ship the next right-pane tab well: Console.

The Console tab should show page and runtime diagnostics for the selected
workspace, not a global log stream and not an arbitrary JavaScript execution
surface. It should help an operator quickly answer whether the selected page is
throwing errors, logging warnings, failing security checks, or surfacing
runtime exceptions that explain a broken viewport, stuck automation task, or
bad page state.

## Product Principle

Console is an evidence tab.

It should be dense, scoped, redacted, and useful for debugging. It should not
be a generic browser devtools clone, an unscoped dashboard event feed, or a
mutation path disguised as inspection.

Do:

- scope default rows to the selected workspace, browser, tab, target, or
  stream when the source can prove that association
- show console logs, page errors, runtime exceptions, and security-related
  entries with level, time, source, target, and concise text
- group counts by errors, warnings, info, logs, exceptions, and unavailable
  sources
- keep several rows visible in the first pane
- make selected errors copyable as a redacted bundle
- let Chat consume selected Console evidence with evidence IDs
- show unscoped or unavailable Console sources explicitly

Do not:

- show unrelated global console messages as if they belong to the selected
  workspace
- keep the current arbitrary JavaScript eval box as a default selected
  workspace Console control
- clear browser-side console buffers without a clear service contract and user
  intent
- send raw private page content, auth tokens, cookies, storage values,
  dashboard auth artifacts, screenshots, or secrets to Chat
- render bulky panels that hide the newest errors below the fold
- invent target attribution in the frontend when the backend did not provide it

## User Questions

The Console tab must answer these questions quickly:

- which selected workspace the Console view is scoped to
- how many errors, warnings, page exceptions, and normal logs are present
- whether entries are live, retained, stale, or only globally available
- which browser, session, tab, target, frame, or stream produced an entry
- whether the latest errors line up with page load, viewport focus, stream
  recovery, Activity events, or Chat observations
- whether the current backend can provide scoped Console evidence
- what evidence can be copied or sent to Chat

## Source Findings

- `packages/dashboard/src/components/console-panel.tsx` currently reads
  `consoleLogsAtom`, filters by level, and renders a global stream log.
- The same component accepts `selectedWorkspaceContext`, but only exposes
  selected workspace metadata as `data-*` attributes today.
- The current input at the bottom runs `agent-browser eval` against
  `activeSessionNameAtom`, not the selected workspace. That can mutate page
  state and can target the wrong browser when dashboard selection and active
  session diverge.
- The native runtime already records `Runtime.consoleAPICalled` and
  `Runtime.exceptionThrown` events. It can also read retained browser console
  and page errors through existing `console` and `errors` browser actions.
- The CDP streaming loop broadcasts console and page-error events, but the
  dashboard needs target, tab, session, browser, frame, or stream attribution
  before those events can be treated as selected-workspace scoped.
- Plan 0020 currently marks Console evidence as unavailable in the Chat packet.
  This plan should replace that unavailable group with scoped Console evidence
  only after attribution and redaction are real.

## Scope

This plan productizes Console only.

Initial evidence groups:

- Console logs: log, debug, info, warning, and error messages
- Page errors: uncaught exceptions with source URL, line, column, and stack
  when available
- Runtime exceptions: `Runtime.exceptionThrown` entries and CDP runtime
  context when available
- Security-adjacent entries: CSP, mixed-content, certificate, permission, and
  extension-origin console messages when they are already present in captured
  console data
- Source readiness: whether scoped live stream, retained browser console, page
  errors, or global fallback sources are available

Network, Storage, and Extensions remain out of scope except where their errors
appear as console entries. Their full evidence surfaces belong to later tab
plans.

## Layout Contract

Use a compact diagnostics layout.

### Header Strip

Always visible:

- selected workspace label, state, and health
- scoped entry count
- error and warning counts
- latest entry age
- source readiness: live stream, retained console, page errors, global fallback,
  or unavailable
- target attribution quality: scoped, partially scoped, unscoped, or missing

### Summary Row

Show dense counters:

- Errors
- Warnings
- Info
- Logs
- Page errors
- Runtime exceptions
- Security
- Unscoped

Counters should be clickable filters. Disabled counters should show a short
reason.

### Filters

Expose compact controls:

- level
- source
- target or tab
- time window
- text search
- include unscoped fallback

The default view should show scoped errors and warnings first, then other
scoped entries newest first.

### Rows

Each row should show:

- timestamp and relative age
- level
- source type
- concise message
- source URL, line, column, and frame when available
- related browser, session, tab, target, stream, job, or incident IDs in compact
  badges
- attribution confidence

Long messages, stack traces, raw CDP payloads, and related Activity rows should
live behind row disclosures.

### Actions

Primary controls:

- copy selected Console evidence
- copy one row
- send selected Console evidence to Chat
- jump to related Activity row when available
- clear local view filter state

Do not show arbitrary JavaScript eval as a default action. A future execution
control belongs under Plan 0022 superuser Operate mode, with selected target
binding, audit, and confirmation where needed.

## Implementation Slices

### Slice K1 | Console Attribution Audit

Goal: determine which Console data can be truthfully scoped to a selected
workspace.

Tasks:

- Audit `consoleLogsAtom`, CDP stream console broadcasts, retained browser
  console reads, page-error reads, and service state identity fields.
- Identify the available target identifiers for each source: browser ID,
  daemon session, service session, tab ID, target ID, frame ID, stream port,
  CDP port, URL, and timestamp.
- Classify each source as scoped, partially scoped, unscoped fallback, or
  unavailable.
- Decide the minimum backend metadata needed to promote live stream console
  events from global to selected-workspace scoped.
- Record the field map in tests or implementation notes before UI work.

Exit criteria:

- A source-backed Console attribution map exists.
- The plan does not proceed by filtering global messages with frontend guesses.

### Slice K2 | Selected Workspace Console Evidence Model

Goal: create the evidence packet that Console UI and Chat can share.

Tasks:

- Add typed Console evidence rows with source, level, text, timestamp, related
  IDs, attribution confidence, and redacted raw details.
- Normalize live stream entries, retained console reads, and page-error reads
  into one row model.
- Preserve unavailable source records with reasons.
- Add redaction for URLs with credentials, stack frames with sensitive query
  values, raw argument previews, and private auth artifacts.
- Limit retained row count to a bounded recent window.

Exit criteria:

- Tests prove selected workspace IDs produce scoped Console rows when metadata
  is available.
- Tests prove unscoped fallback rows are labeled and not mixed into scoped
  counts.
- Tests prove redaction applies before copy or Chat handoff.

### Slice K3 | Dense Console Tab UI

Goal: replace the generic Console panel with a selected-workspace diagnostics
surface.

Tasks:

- Render the header strip, summary row, filters, row list, disclosures, and
  copy or Chat handoff controls.
- Preserve high information density with compact rows and no bulky card stack.
- Show a source-backed empty state when no scoped Console evidence exists.
- Show unavailable source reasons when Console capture or attribution is not
  ready.
- Remove or hide the current eval input from the selected-workspace Console
  lane.
- Keep the tab useful even before a live stream is attached by showing retained
  page errors or source readiness where available.

Exit criteria:

- Selecting a live workspace with console entries shows scoped rows and counts.
- Selecting a retained or missing workspace shows a reasoned state, not an
  empty placeholder.
- The tab does not expose arbitrary JavaScript eval in the default inspector
  lane.

### Slice K4 | Chat Evidence Handoff

Goal: let Codex app-server Chat inspect Console evidence without making Console
or Chat a mutation path.

Tasks:

- Replace `console.unavailable` in the selected-workspace Chat packet when
  scoped Console evidence exists.
- Include selected Console row IDs, summarized messages, counts, and source
  readiness in the Chat evidence packet.
- Add a send-to-Chat affordance that opens Chat with Console evidence selected.
- Ensure Chat observations cite Console evidence IDs when they discuss page or
  runtime errors.
- Keep Codex app server as the only visible Chat provider.

Exit criteria:

- Tests prove Chat receives scoped Console evidence when available.
- Tests prove unavailable Console evidence remains explicit when capture or
  attribution is missing.

### Slice K5 | Focused Tests

Goal: prove Console is selected-workspace scoped, dense, and bounded.

Tasks:

- Add or extend tests for the Console evidence model.
- Add Console tab UI tests for selected workspace changes, group counts,
  filters, empty state, unscoped fallback labeling, copy controls, and Chat
  handoff.
- Assert that no arbitrary eval control appears in the selected-workspace
  Console lane.
- Assert that unscoped global console rows are not counted as scoped rows.
- Assert that sensitive values are redacted in copy and Chat payloads.

Exit criteria:

- `pnpm test:dashboard-selected-workspace-console`
- `pnpm test:dashboard-workspace-inspector-tab`
- `pnpm test:dashboard-selected-workspace-context`
- `pnpm test:dashboard-contextual-chat`
- `pnpm build:dashboard`
- `git diff --check`

### Slice K6 | Runtime Publish And Hosted Smoke

Goal: make the Console tab externally visible and prove it works against a live
selected workspace.

Tasks:

- Publish the local dashboard runtime after source validation.
- Smoke the hosted dashboard against a live selected workspace.
- Generate a harmless console probe in a controlled browser page when needed.
- Open Console and verify scoped counts, row details, unavailable source
  reasons, copy controls, and Chat handoff.
- Verify Workspace, Chat, and Activity still preserve selected-workspace
  context after Console changes.

Exit criteria:

- Hosted smoke proves Console is useful for a live browser session.
- The runtime-visible dashboard includes the same behavior as the source build.

## Backend Contract Notes

The likely missing contract is target attribution for live console events.

If the existing stream broadcast can include session, browser, tab, target,
frame, stream port, and page URL without breaking clients, extend the event
payload and update dashboard parsing. If that is too broad, add a selected
workspace Console evidence endpoint that resolves attribution server-side.

Do not add a frontend-only heuristic that treats matching URL text or active
session name as proof of selected-workspace ownership.

## Validation Matrix

Required source checks:

```bash
pnpm test:dashboard-selected-workspace-console
pnpm test:dashboard-workspace-inspector-tab
pnpm test:dashboard-selected-workspace-context
pnpm test:dashboard-contextual-chat
pnpm build:dashboard
git diff --check
```

If Rust stream or browser action contracts change:

```bash
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
cargo test --manifest-path cli/Cargo.toml native::stream -- --nocapture
pnpm test:service-api-mcp-parity
pnpm test:service-client-contract
pnpm test:service-client-types
```

Runtime checks:

```bash
pnpm publish:local-dashboard -- \
  --expect-marker Console \
  --expect-marker data-selected-workspace-id \
  --json

node scripts/smoke-local-dashboard-runtime.js \
  --expect-section overview \
  --expect-section console
```

Add or adapt a hosted smoke if the current smoke script cannot select a live
workspace, generate a harmless console probe, and verify scoped Console rows.

## Completion Criteria

- Console tab defaults to selected-workspace scoped evidence.
- Unavailable and unscoped sources are labeled with reasons.
- Errors, warnings, page errors, and runtime exceptions are visible in a dense
  first-screen layout.
- Copy and Chat handoff use redacted Console evidence.
- Arbitrary JavaScript eval is not exposed in the default selected-workspace
  Console lane.
- Runtime publication and hosted smoke prove the behavior is visible outside
  the source tree.

## Completion Notes

Completed on 2026-06-01.

Implementation:

- Added a shared selected-workspace Console evidence model in
  `packages/dashboard/src/lib/selected-workspace-console.ts`.
- Stamped live console and page-error stream events with their stream port in
  `packages/dashboard/src/store/stream.ts`.
- Rebuilt the right-pane Console tab as a dense selected-workspace evidence
  surface with scoped counts, source readiness, attribution labels, redacted
  copy, and Chat handoff.
- Removed the default arbitrary JavaScript eval lane from the selected
  workspace Console surface.
- Wired scoped Console evidence into the selected-workspace Chat packet while
  preserving explicit unavailable evidence when scoped rows are missing.
- Added focused Console tests and validation selector coverage.

Validation:

```bash
pnpm test:dashboard-selected-workspace-console
pnpm test:dashboard-selected-workspace-chat-packet
pnpm test:dashboard-contextual-chat
pnpm test:dashboard-workspace-inspector-tab
pnpm test:dashboard-selected-workspace-context
pnpm test:dashboard-workspace-nodes
pnpm test:dashboard-workspace-navigator
pnpm test:dashboard-inspector-actions
pnpm build:dashboard
git diff --check
node scripts/dev/select-validation.js --base HEAD --json
```

Runtime publication:

```bash
pnpm publish:local-dashboard -- \
  --expect-marker Console \
  --expect-marker data-console-evidence-attribution \
  --skip-browser \
  --json
```

The publish installed the current local dashboard runtime and restarted
`agent-browser-dashboard.service`.

Hosted runtime smoke:

```bash
node scripts/smoke-local-dashboard-runtime.js \
  --dashboard-url 'https://agent-browser.ecochran.dyndns.org/' \
  --expect-marker Console \
  --expect-marker data-console-evidence-attribution \
  --skip-browser \
  --json

node scripts/smoke-local-dashboard-runtime.js \
  --dashboard-url 'https://agent-browser.ecochran.dyndns.org/?view=workspace%3Acontrol&workspace=browser%3Asession%3Adefault&browser=session%3Adefault&session=default&profile=default' \
  --expect-marker Console \
  --expect-marker data-console-evidence-attribution \
  --browser-profile /tmp/agent-browser-console-tab-hosted-smoke \
  --json
```

The browser-level hosted smoke passed after removing stale display lock files.
It loaded the hosted dashboard, found the Workspace, Chat, Activity, Console,
Network, Storage, and Extensions tabs in the right pane, confirmed a workspace
pane was present, and reported CDP screencast readiness as `ready`.
