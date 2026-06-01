# Workspace Inspector Tabs Productization Plan

Date: 2026-06-01
State: OPEN
Lane: P12-F
Parent Roadmap: `docs/dev/plans/0012-2026-05-31-workspace-inspection-pane-app-intelligence-roadmap.md`
Depends On:
- `docs/dev/plans/0013-2026-05-31-selected-workspace-context-plan.md`
- `docs/dev/plans/0014-2026-05-31-contextual-chat-codex-app-server-plan.md`
- `docs/dev/plans/0015-2026-05-31-codex-app-server-supervisor-plan.md`
- `docs/dev/plans/0016-2026-05-31-effective-stealth-remote-default-launch-plan.md`

## Purpose

Make the right inspector pane tabs useful for a selected workspace.

The current dashboard can select a workspace, render a live viewport for CDP
or remote-view streams, and run a Codex app-server contextual Chat inspection.
The remaining gap is that most right-pane tabs still read like placeholders or
generic global panels. This plan turns the tabs into selected-workspace
inspection surfaces that show concrete facts, scoped evidence, and safe
operator actions.

## Product Contract

When a user selects a workspace, the right pane must answer:

- what is selected
- whether it is live, retained, blocked, stale, viewable, or controllable
- what page, process, stream, jobs, incidents, logs, requests, storage, and
  extensions are relevant
- what safe action can be taken from each tab
- what Chat can infer from the same evidence

The selected workspace is the shared context for every tab. Tabs must not drift
into unrelated global state unless they explicitly label that state as
unscoped fallback.

## Provider Boundary

Chat remains Codex app server only.

Do not expose OpenAI, AI Gateway, model selection, Codex exec, OpenClaw,
AuraCall, or any other provider in the selected-workspace Chat surface. Existing
legacy chat plumbing can remain internally available where already supported,
but it must not appear as a selectable provider for contextual workspace Chat.

## Target Tabs

### Workspace

Goal: selected-workspace operational summary.

Show:

- workspace label, source, browser ID, session ID, profile ID, active tab ID,
  and target ID
- service, agent, task, owner, lease, and job links when known
- state, health, lifecycle, browser host, browser build, profile source, and
  selected URL/title
- PID, memory, CPU time, uptime, CDP port, stream port, last frame age, and
  stream readiness when available
- stream provider, route, control input provider, embeddability, viewer state,
  and control readiness
- active retained or attention reason with source evidence

Actions:

- focus selected tab through `view_focus`
- refresh or reconnect viewport when supported
- open external stream when the provider exposes one
- close active browser when service contracts allow it
- retry or repair known browser failures when service contracts allow it
- copy a compact diagnostic bundle

### Chat

Goal: interpret selected-workspace evidence through Codex app server.

Show:

- Codex app server provider badge
- selected workspace freshness and evidence groups included in the packet
- latest structured observation, blockers, risks, suggested next inspections,
  unsupported actions, and event log
- validation state for the run ledger

Actions:

- run read-only inspection
- ask a follow-up about the selected workspace
- include or exclude safe evidence groups
- copy observation or handoff summary
- attach validated observation back to Activity only if a service contract
  exists for that event write

### Activity

Goal: explain what happened to the selected workspace.

Show:

- browser launch, attach, close, health, recovery, and reconciliation events
- tab lifecycle and focus events
- service request jobs, queue state, outcomes, and cancellations
- stream connect, disconnect, viewer, controller, and takeover events
- incidents and monitor findings related to selected browser, tab, profile,
  job, or session

Actions:

- filter by lifecycle, jobs, stream, incidents, agent actions, and operator
  actions
- jump to related browser, profile, tab, session, job, or incident
- retry failed jobs and cancel queued/running jobs only through advertised
  service contracts
- copy or export a scoped activity slice

### Console

Goal: show page/runtime errors for the selected workspace.

Show:

- console logs grouped by level
- page errors with source URL, line, column, and stack when available
- browser/runtime errors
- security, mixed-content, CSP, and extension-origin entries
- timestamp, target, and tab association

Actions:

- filter by level, source, and time
- clear local view state without mutating browser history
- copy selected error bundle
- send selected errors to Chat as an evidence group

### Network

Goal: show selected-page request health.

Show:

- recent requests with method, URL, status, resource type, timing, and size
- failed, blocked, redirected, CORS, mixed-content, WebSocket, and EventSource
  requests
- request/response headers with sensitive-value redaction
- safe text or JSON previews when already available through service APIs
- origin and target-service grouping

Actions:

- filter failed, XHR/fetch, document, WebSocket, image, and third-party
  requests
- copy redacted request details or curl
- export HAR when supported
- send selected request group to Chat for diagnosis

### Storage

Goal: summarize selected-origin/profile state without leaking secrets.

Show:

- current origin, site, profile path, browser family, keyring posture, and
  profile freshness hints
- cookie metadata only by default: domain, path, expiration, SameSite, Secure,
  HttpOnly, and size
- localStorage/sessionStorage keys and value-size summaries, not raw values
- IndexedDB, cache storage, service worker, permissions, and auth-readiness
  summaries when available

Actions:

- search storage keys
- delete selected cookies or keys only through service-mediated actions
- clear origin storage only through an explicit service contract
- export redacted storage evidence
- send redacted storage facts to Chat

### Extensions

Goal: show extension influence on the selected workspace.

Show:

- installed extensions, versions, IDs, names, and enabled state
- permissions and host permissions
- background page or service-worker status
- extension console errors
- attributable extension-origin network or storage effects when available

Actions:

- reload, enable, or disable extensions only through explicit service contract
  support
- inspect background worker or extension page when available
- copy manifest and permission evidence
- send extension errors to Chat

## Implementation Slices

### Slice 1 | Shared Inspector Evidence Model

Create a normalized selected-workspace inspector model consumed by every tab.

Tasks:

- Extend the existing selected-workspace context with typed tab evidence:
  workspace, activity, console, network, storage, and extensions.
- Keep values derived from service state, daemon stream state, and existing
  browser APIs.
- Add explicit freshness and source labels for every evidence group.
- Add redaction helpers for headers, cookies, storage values, URLs with
  credentials, and private auth artifacts.

Exit criteria:

- Unit tests prove a selected workspace produces a stable evidence packet with
  redacted sensitive fields.

### Slice 2 | Workspace Tab Details And Actions

Replace the Workspace tab placeholder with a selected-workspace detail surface.

Tasks:

- Show process, runtime, stream, ownership, tab, and readiness facts.
- Add action buttons only when service contracts or stream metadata say the
  action is available.
- Route actions through existing service request paths.
- Show disabled action reasons when actions are unavailable.

Exit criteria:

- Dashboard tests prove selecting an active browser fills Workspace details,
  and selecting retained/dead records shows source-backed reasons instead of an
  empty pane.

### Slice 3 | Evidence Tabs

Make Activity, Console, Network, Storage, and Extensions selected-workspace
scoped.

Tasks:

- Filter each tab by selected browser/session/tab/profile/job where possible.
- Show an explicit empty state when no scoped evidence exists.
- Preserve raw inspectability behind disclosures.
- Add copy/export affordances with redaction.
- Add “send to Chat” affordances for selected evidence groups.

Exit criteria:

- Tests prove each tab changes when the selected workspace changes, and no tab
  silently shows unrelated global evidence as if it were scoped.

### Slice 4 | Chat Evidence Integration

Wire selected tab evidence into Codex app-server Chat.

Tasks:

- Let the operator include/exclude Workspace, Activity, Console, Network,
  Storage, and Extensions evidence groups.
- Keep the provider surface Codex app server only.
- Record which evidence groups were included in every inspection run.
- Validate structured observations against the existing schema and evidence
  references.
- Render failures as structured, actionable messages.

Exit criteria:

- Dashboard tests prove the Chat tab exposes only Codex app server, includes
  selected evidence groups, and renders structured observations or failures.

### Slice 5 | Runtime Publish And Hosted UX Smoke

Prove the installed dashboard reflects the inspector changes.

Tasks:

- Publish the local dashboard runtime with markers for Workspace details and
  Codex contextual Chat.
- Run an authenticated hosted dashboard smoke with an isolated browser profile.
- Select a live workspace and prove the right pane shows non-empty Workspace
  details plus at least one scoped evidence tab.
- Run a Chat inspection and prove the structured observation cites selected
  evidence groups.

Exit criteria:

- Hosted UX smoke proves a user can inspect a live workspace from the right
  pane without relying on local source-only behavior.

## Contracts And Safety

- Every mutating action must flow through `POST /api/service/request`, an
  existing browser API, MCP-equivalent service contract, or a newly documented
  service request action.
- If a needed action has no backend contract, show it as unavailable with a
  source-backed reason and add it to a future action-contract plan.
- Do not expose raw cookies, storage values, auth headers, dashboard auth
  cookies, private page content, screenshots, passwords, tokens, or browser auth
  artifacts to Chat by default.
- Do not let Codex app server execute actions in this plan. It can inspect,
  summarize, and recommend next inspections only.
- Preserve selected-record inspector behavior in the Service tab.

## Validation Matrix

Required local checks:

```bash
pnpm test:dashboard-selected-workspace-context
pnpm test:dashboard-selected-workspace-chat-packet
pnpm test:dashboard-contextual-chat
pnpm test:dashboard-workspace-navigator
pnpm test:dashboard-view-streams
pnpm test:dashboard-inspector-actions
pnpm build:dashboard
git diff --check
```

Add focused tests as new surfaces are implemented:

```bash
pnpm test:dashboard-workspace-inspector-tabs
pnpm test:dashboard-workspace-evidence-redaction
```

Required runtime checks:

```bash
pnpm publish:local-dashboard -- \
  --expect-marker data-codex-app-server-contextual-chat \
  --expect-marker Workspace \
  --json

node scripts/smoke-local-dashboard-runtime.js \
  --dashboard-url https://agent-browser.ecochran.dyndns.org/ \
  --workspace-session <live-session> \
  --browser-profile /tmp/agent-browser-dashboard-inspector-tabs-smoke \
  --expect-marker data-codex-app-server-contextual-chat \
  --expect-marker Workspace \
  --json
```

Run Rust checks only if service contracts, stream server, or backend evidence
providers change:

```bash
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
cargo test --manifest-path cli/Cargo.toml native::stream -- --nocapture
cargo test --manifest-path cli/Cargo.toml native::actions -- --nocapture --test-threads=1
```

## Completion Criteria

- Selecting any workspace fills the Workspace tab with inspectable detail or a
  source-backed unavailable reason.
- Activity, Console, Network, Storage, and Extensions are visibly scoped to the
  selected workspace.
- Chat exposes only Codex app server and can include selected evidence groups.
- Sensitive values are redacted before copy/export/Chat paths.
- Unsupported actions are visible as unavailable with backend-contract reasons.
- Local dashboard tests pass.
- The installed dashboard is republished.
- Hosted authenticated smoke proves the right pane is useful on a live
  workspace.
