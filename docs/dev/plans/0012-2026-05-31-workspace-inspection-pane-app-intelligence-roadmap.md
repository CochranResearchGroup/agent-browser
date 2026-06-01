# Workspace Inspection Pane And App Intelligence Roadmap

Date: 2026-05-31
State: OPEN
Lane: P12
Owner: dashboard/service-control-plane

## Purpose

Turn the right inspection pane tabs into a selected-workspace command surface
that answers:

- What browser or workspace is selected?
- Is it live, viewable, controllable, blocked, stale, or retained?
- What page, runtime state, events, errors, requests, storage, and extensions
matter right now?
- What can the operator safely do from here?
- What should App Intelligence infer or recommend from the current workspace
evidence?

The current stable CDP stream example gives the first concrete anchor: a
selected `cdp_screencast` workspace can render live browser pixels, expose
workspace readiness, and hold enough service-state identity to drive useful
inspection. The next step is to make the right pane tabs consume and manipulate
that selected workspace rather than acting as generic global panels.

## Product Principles

- The selected workspace is the shared context for every right-pane tab.
- The dashboard consumes service-owned runtime, tab, stream, job, incident, and
  browser state. It must not invent frontend-only service semantics.
- Chat is the interpretive layer. It should use Workspace, Activity, Console,
  Network, Storage, Extensions, page snapshot, and App Intelligence signals as
  context.
- Non-chat tabs should stay factual and action-oriented. They provide evidence,
  filters, and scoped actions.
- Every mutating action must flow through an existing or planned service
  contract, not direct ad hoc dashboard mutation.
- Raw IDs, endpoints, and JSON stay inspectable, but default views should lead
  with human-readable operational summaries.

## Desired Tab Model

### Workspace

Primary question: what is selected, and can it be viewed or controlled?

Data:

- workspace, browser, session, profile, active tab, target ID, and owner labels
- state, health, lifecycle, host mode, browser build, and provider
- PID, memory, CPU, uptime, CDP port, stream port, and last frame age
- stream provider, stream route, control input provider, viewer count, and
  readiness reason
- active URL, title, origin, lifecycle, and focus status
- current service, agent, task, job, incident, and profile lease ownership
- retained or attention reason with source evidence

Manipulations:

- focus selected tab
- refresh viewport stream
- open external stream when supported
- close live browser when service contract permits
- retry or repair known service failures
- prune retained browser records when retained and non-live
- copy compact diagnostic bundle

### Chat

Primary question: what does the selected workspace mean, and what should happen
next?

Data:

- selected workspace summary from the Workspace tab
- page snapshot, screenshot, active URL, visible title, and selected target
- recent Activity events, service jobs, and incidents
- Console errors and warnings
- failed or suspicious Network requests
- Storage summaries with secret-safe redaction
- extension state and extension-origin errors
- App Intelligence observations, confidence, evidence, risks, and suggested
  actions

Manipulations:

- ask about the current page or workflow state
- run a bounded read-only App Intelligence inspection
- summarize why the workspace is blocked or idle
- generate next-step recommendations
- produce a handoff note from current workspace evidence
- request scoped browser actions through service-mediated tools
- attach AI observations back to Activity as durable evidence

Chat must not be just a generic assistant. It should be a contextual workspace
copilot that can say, for example: this page appears logged out, these requests
failed, the owning job is blocked, the browser is live, and the safest next
action is to refresh the session or ask for credentials.

### Activity

Primary question: what happened to this workspace?

Data:

- browser launch, attach, close, health, and recovery events
- tab lifecycle and focus events
- service request jobs and outcomes
- stream connect, disconnect, viewer, controller, and takeover events
- incidents and monitor findings tied to this browser, profile, tab, or job
- dashboard operator actions

Manipulations:

- filter by lifecycle, jobs, stream, incidents, agent actions, and operator
  actions
- retry failed job when the service contract allows it
- cancel running or queued job when supported
- jump to related browser, profile, tab, session, job, or incident
- copy or export an activity slice

### Console

Primary question: what did the page or runtime report?

Data:

- console logs grouped by level
- page errors with source URL, line, column, and stack when available
- browser runtime errors
- security, mixed-content, CSP, and extension-origin console entries
- timestamp and tab association

Manipulations:

- filter by level and source
- clear or pause live collection for the selected workspace
- copy selected error bundle
- send selected errors to Chat for explanation

### Network

Primary question: is the page talking to the right services?

Data:

- recent requests with method, URL, status, resource type, timing, and size
- failed, blocked, redirected, CORS, mixed-content, WebSocket, and EventSource
  requests
- request and response headers with sensitive-value redaction
- safe response previews for text and JSON
- origin and target-service grouping

Manipulations:

- filter failed, XHR/fetch, document, WebSocket, image, and third-party
  requests
- copy as curl with secret redaction
- export HAR when supported
- replay safe GET requests when service policy permits
- send selected request group to Chat for diagnosis

### Storage

Primary question: what state is this site holding in the selected profile?

Data:

- current origin, site, profile path, browser family, and keyring posture
- cookies with domain, expiration, SameSite, Secure, HttpOnly, and size
- localStorage and sessionStorage keys
- IndexedDB, cache storage, service worker, and permissions summaries
- auth freshness hints when monitors or App Intelligence provide them

Manipulations:

- search storage keys
- delete selected cookies or keys
- clear origin storage through a service-mediated action
- export selected storage evidence with secret redaction
- ask Chat to identify session/auth state from redacted storage facts

### Extensions

Primary question: what browser extensions affect this workspace?

Data:

- installed extensions, versions, IDs, names, and enabled state
- permissions and host permissions
- background page or service worker status
- extension console errors
- extension-driven network or storage effects when attributable

Manipulations:

- reload extension when supported
- enable or disable extension only through explicit service contract support
- inspect background worker or extension page when available
- copy manifest and permission evidence
- send extension errors to Chat for diagnosis

## Architecture Direction

### Shared Selection Context

Create a selected-workspace evidence context consumed by all right-pane tabs.
It should normalize:

- selected workspace/browser/session/profile/tab IDs
- primary view stream and control input evidence
- service-owned runtime and ownership facts
- active tab identity and page snapshot handles
- related jobs, incidents, events, and monitor findings

This context should be derived from service state and daemon/session stream
state, not local-only component state.

### Evidence Providers

Each non-chat tab becomes an evidence provider with a small stable contract:

- status summary
- evidence rows
- available actions
- last updated timestamp
- structured facts suitable for Chat/App Intelligence context

Initial providers:

- workspace runtime evidence
- activity evidence
- console evidence
- network evidence
- storage evidence
- extension evidence
- page snapshot and screenshot evidence

### App Intelligence Bridge

Add an App Intelligence bridge behind Chat that accepts a selected workspace
context packet and returns structured observations:

- summary
- blockers
- risks
- detected workflow state
- available safe actions
- recommended next actions
- evidence references
- confidence and freshness

The first implementation can be read-only. Mutating recommendations should be
presented as proposed actions that route through service request contracts.

### Activity As Audit Trail

AI observations that affect operator decisions should be attachable to Activity
as events. The event should record:

- workspace and tab identity
- evidence packet version
- model or provider label
- observation summary
- accepted or rejected action, when applicable

## Roadmap Slices

### Slice A | Workspace Context And Pane Contract

Goal: make every right-pane tab consume the same selected workspace context.

Tasks:

- Define `SelectedWorkspaceContext` for browser, session, profile, tab, stream,
  ownership, readiness, and related service records.
- Wire `WorkspaceSelectionPanel`, `WorkspaceRemoteViewport`, and side-panel
  tabs to the same context source.
- Add compact runtime indicators for PID, CPU, memory, uptime, stream port, CDP
  port, and last frame age where available.
- Add a selected-workspace diagnostic bundle helper.

Exit criteria:

- Selecting a workspace updates every right-pane tab context.
- Workspace tab can explain live, idle, retained, attention, blocked, and
  missing-stream states with source evidence.
- No tab falls back to unrelated global session data when a workspace is
  selected.

Validation:

- `pnpm test:dashboard-view-streams`
- `pnpm test:dashboard-workspace-nodes`
- `pnpm test:dashboard-inspector-actions`
- `pnpm build:dashboard`
- live publish with `pnpm publish:local-dashboard -- --expect-marker
  <workspace-context-marker>` before external operator QA

### Slice B | Activity And Console Evidence

Goal: make Activity and Console selected-workspace evidence providers.

Tasks:

- Filter Activity by selected browser, session, tab, job, incident, profile,
  and stream route.
- Add event grouping for lifecycle, jobs, incidents, stream, and operator
  actions.
- Bind Console to the selected tab/session stream rather than only the current
  daemon global.
- Add copy/send-to-chat affordances for selected event or error groups.

Exit criteria:

- Activity answers what happened to the selected workspace.
- Console answers what the selected page reported.
- Chat can receive selected Activity or Console evidence packets.

Validation:

- focused dashboard activity/console tests, adding fixtures if needed
- `pnpm test:dashboard-view-streams`
- `pnpm build:dashboard`

### Slice C | Network And Storage Evidence

Goal: expose page communication and state for the selected workspace.

Tasks:

- Scope Network to the selected browser/tab and show failed, blocked,
  redirected, WebSocket, and third-party requests.
- Add safe request/response detail with redaction.
- Scope Storage to the selected origin/profile and summarize cookies,
  localStorage, sessionStorage, IndexedDB, cache, permissions, and service
  workers.
- Add service-mediated clear/delete actions only where supported.
- Add Chat handoff actions for request groups and storage summaries.

Exit criteria:

- Network identifies why a page may be failing to load or authenticate.
- Storage identifies which origin/profile state is relevant without exposing
  secrets by default.
- Mutations are explicitly service-backed or absent.

Validation:

- focused network-panel and storage-panel tests
- service contract tests if new clear/delete actions are added
- `pnpm build:dashboard`

### Slice D | Extensions Evidence

Goal: make Extensions useful for runtime diagnosis without overbuilding it.

Tasks:

- Show extension ID, name, version, enabled state, permissions, and background
  status for the selected browser.
- Surface extension-origin console errors where available.
- Add manifest and permission evidence disclosure.
- Defer enable, disable, and reload unless service contracts already support
  them or this slice adds those contracts.

Exit criteria:

- Extensions explains what extension code may affect this workspace.
- Operators can copy extension evidence and send it to Chat.

Validation:

- focused extension-panel tests
- `pnpm build:dashboard`

### Slice E | Contextual Chat And App Intelligence Read-Only Bridge

Goal: make Chat understand the selected workspace and run read-only App
Intelligence inspections.

Tasks:

- Build a selected-workspace context packet from evidence providers.
- Add Chat context controls for include page snapshot, screenshot, activity,
  console, network, storage, and extensions.
- Add App Intelligence read-only inspection endpoint or service request action.
- Return structured observations, blockers, risks, confidence, evidence
  references, and next-action suggestions.
- Render observations as first-class Chat artifacts, not just prose.

Exit criteria:

- Chat can answer "what is this browser doing?", "why is it blocked?", and
  "what should I inspect next?" from selected workspace evidence.
- App Intelligence can inspect without mutating the browser.
- Observations cite which evidence tabs contributed to the answer.

Validation:

- Chat transport tests for selected-workspace context inclusion
- App Intelligence contract tests or fixtures
- `pnpm build:dashboard`
- live external smoke with a selected workspace and Chat context enabled

### Slice F | Proposed Actions And Audit Trail

Goal: let App Intelligence propose actions while preserving operator control.

Tasks:

- Define proposed-action schema for focus, reload, snapshot, extract, copy,
  retry, repair, clear storage, and close/prune classes.
- Map each proposed action to an existing service request or mark it
  unsupported.
- Require explicit operator confirmation for destructive actions.
- Add accepted/rejected AI observation events to Activity.
- Add evidence packet IDs or hashes so recommendations are traceable.

Exit criteria:

- Chat can propose service-backed actions without executing them implicitly.
- Accepted actions flow through service contracts and appear in Activity.
- Unsupported or destructive actions are clearly gated.

Validation:

- service request action parity tests when contracts change
- dashboard proposed-action tests
- `pnpm test:dashboard-inspector-actions`
- `pnpm build:dashboard`

### Slice G | Live Runtime QA And Documentation

Goal: make the new inspection model operator-visible and documented.

Tasks:

- Publish to the user-scoped runtime after each operator-visible slice.
- Add docs for selected-workspace inspection, Chat/App Intelligence context,
  and evidence-provider redaction.
- Update `README.md`, `docs/src/app/`, and `skills/agent-browser/SKILL.md`
  when the user-facing workflow changes.
- Record live smoke evidence against `https://agent-browser.ecochran.dyndns.org/`
  for the stable CDP workspace example.

Exit criteria:

- External dashboard shows the shipped behavior immediately after closeout.
- Docs explain what each tab is for and what Chat can use as context.
- Operators can distinguish evidence, AI interpretation, and service actions.

Validation:

- `pnpm publish:local-dashboard -- --expect-marker <changed-ui-marker>`
- live DOM smoke for selected workspace, right-pane tab selection, and Chat
  context availability
- docs build when docs changed

## Backend Contract Backlog

The roadmap should prefer existing contracts, but these backend/service
contracts may be required:

- selected-workspace context resource
- process metrics resource for PID, CPU, memory, and uptime
- selected-tab console resource with event timestamps
- selected-tab network event resource and HAR export action
- selected-origin storage summary and clear/delete actions
- extension inventory and background status resource
- App Intelligence read-only inspect action
- App Intelligence observation event action
- proposed-action schema and execution mapping

## Non-Goals

- Do not turn Chat into an unscoped global assistant.
- Do not expose raw secrets from cookies, storage, headers, or screenshots.
- Do not add destructive storage, browser, profile, or retained-record actions
  without service support and explicit confirmation.
- Do not make frontend-only guesses about service ownership or runtime health.
- Do not block factual tab improvements on the full App Intelligence bridge.
- Do not replace the existing Service selected-record inspector; this roadmap
  is for the workspace right-pane tabs.

## Recommended First Slice

Start with Slice A, then Slice E.

Slice A gives every tab a reliable selected-workspace context and fixes the
current placeholder feel. Slice E then makes Chat valuable quickly by letting
App Intelligence consume that context read-only. Activity, Console, Network,
Storage, and Extensions can then become progressively richer evidence
providers without changing the core interaction model.
