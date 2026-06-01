# Activity Tab Selected Workspace Timeline Plan

Date: 2026-06-01
State: OPEN
Lane: P12-I
Parent Roadmap: `docs/dev/plans/0012-2026-05-31-workspace-inspection-pane-app-intelligence-roadmap.md`
Depends On:
- `docs/dev/plans/0013-2026-05-31-selected-workspace-context-plan.md`
- `docs/dev/plans/0019-2026-06-01-workspace-tab-dense-inspector-plan.md`
- `docs/dev/plans/0020-2026-06-01-chat-tab-selected-workspace-evidence-plan.md`

## Purpose

Ship the next right-pane tab well: Activity.

The Activity tab should answer what happened to the selected workspace without
making the operator hunt through global service state, placeholder panels, raw
JSON, or unrelated records. It should be a dense selected-workspace timeline
that explains lifecycle, stream, job, incident, operator, and App Intelligence
events with enough evidence to act or hand off.

## Product Principle

Activity is the audit trail for the selected workspace.

It should show factual, source-labeled events and make it obvious whether the
workspace is live, idle, blocked, recovered, retained, viewed, controlled, or
acted on. It should also provide the evidence bridge that Chat can consume,
but it should not become a generic chat transcript, global event feed, or raw
debug log.

Do:

- scope every default row to the selected workspace
- group events by lifecycle, stream, jobs, incidents, operator actions, and
  App Intelligence observations
- show counts, latest state, freshness, and source for each group
- expose related IDs such as browser, session, profile, tab, target, stream,
  job, incident, and request IDs
- show unavailable or unscoped event sources explicitly
- provide copy and send-to-Chat affordances for selected event groups
- keep the first screen compact and information dense

Do not:

- show unrelated global Activity as if it belongs to the selected workspace
- expose mutating retry, cancel, repair, or prune actions unless a service
  contract explicitly advertises them
- bury the event evidence behind bulky cards
- require raw JSON inspection for routine diagnosis
- include secrets, raw storage values, cookies, auth headers, dashboard auth
  artifacts, screenshots, or private page content in copy or Chat packets

## User Questions

The Activity tab must answer these questions quickly:

- what selected workspace the timeline is scoped to
- when the browser launched, attached, recovered, focused, streamed, or closed
- whether any service jobs are running, queued, completed, failed, or blocked
- whether incidents or monitor findings explain a bad state
- whether an operator or agent recently viewed, controlled, or acted on it
- whether Chat or App Intelligence produced observations tied to the workspace
- which event sources are unavailable or not yet scoped
- what evidence can be copied or sent to Chat

## Scope

This plan productizes Activity only.

Initial event groups:

- Lifecycle: launch, attach, focus, recovery, stale-target recovery, health,
  close, retain, and prune-related evidence when available
- Stream: stream route, provider, viewer count, controller status, connection,
  disconnection, takeover, last frame, and embeddability evidence
- Jobs: selected service request jobs and outcomes tied to browser, session,
  tab, target, profile, stream, or workspace IDs
- Incidents: selected monitor findings and incident summaries tied to the same
  IDs
- Operator actions: dashboard focus, view, control, open external stream, copy,
  and other audited UI actions when already recorded
- App Intelligence: Codex app-server inspection runs and validated observation
  summaries when already available

Console, Network, Storage, and Extensions remain out of scope except as
unavailable event sources with reasons. Their full evidence surfaces belong to
later tab plans.

## Layout Contract

Use a compact timeline layout with high information density.

### Header Strip

Always visible:

- selected workspace label, state, and health
- scoped event count
- latest event age
- active filters
- related job and incident counts
- source readiness summary

### Group Summary Row

Show a dense row of group chips:

- Lifecycle
- Stream
- Jobs
- Incidents
- Operator
- App Intelligence
- Console unavailable
- Network unavailable
- Storage unavailable
- Extensions unavailable

Each chip shows count, status, and freshness. Unavailable groups are disabled
with a short reason.

### Filters

Expose compact controls:

- group filter
- severity or state filter
- time window
- source filter
- include unavailable sources

The default view should show all scoped Activity groups sorted newest first.

### Timeline Rows

Each row should show:

- timestamp and relative age
- group and severity
- concise event title
- state transition or outcome when known
- source label
- related IDs in compact badges
- one-line evidence summary
- disabled action reasons when an action is not supported

Detailed raw data can live behind a disclosure on each row.

### Actions

Primary controls:

- copy selected activity slice
- copy row evidence
- send selected activity evidence to Chat
- jump to related workspace, job, incident, tab, or stream when already routable

Mutating actions such as retry job, cancel job, repair browser, close browser,
or prune retained records must stay absent or disabled with service-contract
reasons unless this slice wires a documented service request contract.

## Implementation Slices

### Slice I1 | Activity Evidence Model

Goal: create a selected-workspace Activity evidence model that all Activity UI
and Chat handoff paths can share.

Tasks:

- Add a typed selected-workspace activity packet or evidence provider.
- Normalize lifecycle, stream, job, incident, operator, and App Intelligence
  facts into timeline rows.
- Attach source, freshness, severity, related IDs, and evidence IDs to every
  row.
- Mark unscoped or unavailable sources explicitly.
- Reuse existing redaction helpers for copy and Chat paths.

Exit criteria:

- Tests prove selected workspace IDs produce scoped Activity rows and
  unavailable sources carry reasons.

### Slice I2 | Dense Activity Tab UI

Goal: replace the placeholder Activity tab with a compact operational
timeline.

Tasks:

- Render header strip, group summary row, filters, timeline rows, row detail
  disclosures, and copy or Chat handoff controls.
- Preserve selected-workspace context when switching tabs.
- Keep rows compact enough that several events are visible in the first pane.
- Show a source-backed empty state when no scoped Activity exists.
- Avoid generic global Activity unless it is clearly labeled unscoped fallback.

Exit criteria:

- Selecting an active live workspace shows scoped Activity facts without empty
  placeholder tabs or unrelated global rows.

### Slice I3 | Chat Handoff

Goal: let Activity contribute useful evidence to the Codex app-server Chat
lane without making Chat a mutation path.

Tasks:

- Add a redacted Activity evidence group to the selected-workspace Chat packet.
- Include selected Activity row IDs and summaries in inspection prompts.
- Add a send-to-Chat affordance that opens Chat with Activity evidence selected.
- Ensure Chat records which Activity evidence IDs were included.
- Keep Codex app server as the only visible Chat provider.

Exit criteria:

- Tests prove Activity evidence can be included in Chat and that unsupported
  event sources remain unavailable with reasons.

### Slice I4 | Focused Tests

Goal: prove the Activity tab is selected-workspace scoped, dense, and bounded.

Tasks:

- Add or extend focused dashboard tests for Activity evidence construction.
- Add Activity tab UI tests for selected workspace changes, group counts,
  unavailable sources, filters, copy controls, and Chat handoff.
- Assert no mutating service action appears unless a matching service contract
  exists.
- Assert the tab does not silently show unrelated global Activity as scoped.

Exit criteria:

- `pnpm test:dashboard-selected-workspace-activity`
- `pnpm test:dashboard-workspace-inspector-tab`
- `pnpm test:dashboard-selected-workspace-context`
- `pnpm test:dashboard-contextual-chat`
- `pnpm build:dashboard`
- `git diff --check`

### Slice I5 | Runtime Publish And Hosted Smoke

Goal: make the Activity tab externally visible and prove it works against a
live selected workspace.

Tasks:

- Publish the local dashboard runtime after source validation.
- Smoke the hosted dashboard against a live selected workspace.
- Open the Activity tab and verify scoped event groups, timeline rows,
  unavailable source reasons, copy controls, and Chat handoff affordance.
- Verify the Workspace and Chat tabs still render their selected-workspace
  context after Activity changes.

Exit criteria:

- Hosted smoke proves the Activity tab is useful for a live browser without
  relying on source-only behavior.

## Validation Matrix

Required source checks:

```bash
pnpm test:dashboard-selected-workspace-activity
pnpm test:dashboard-workspace-inspector-tab
pnpm test:dashboard-selected-workspace-context
pnpm test:dashboard-contextual-chat
pnpm build:dashboard
git diff --check
```

Required runtime checks:

```bash
pnpm publish:local-dashboard -- \
  --expect-marker data-workspace-activity-timeline \
  --expect-marker data-codex-app-server-contextual-chat \
  --expect-marker Workspace \
  --browser-profile /tmp/agent-browser-activity-tab-publish-smoke \
  --json

node scripts/smoke-local-dashboard-runtime.js \
  --dashboard-url https://agent-browser.ecochran.dyndns.org/ \
  --workspace-session <live-session> \
  --browser-profile /tmp/agent-browser-activity-tab-hosted-smoke \
  --expect-marker data-workspace-activity-timeline \
  --expect-marker data-codex-app-server-contextual-chat \
  --expect-marker Workspace \
  --json
```

Run Rust checks only if this slice changes service contracts, backend event
resources, stream state, or daemon Activity emission:

```bash
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
cargo test --manifest-path cli/Cargo.toml native::actions -- --nocapture --test-threads=1
```

## Completion Criteria

- Activity is the only newly productized inspector tab in this slice.
- Selected workspace Activity rows are scoped, source-labeled, and dense.
- Lifecycle, stream, jobs, incidents, operator, and App Intelligence groups are
  visible with counts and freshness.
- Unavailable Console, Network, Storage, and Extensions event sources show
  reasons.
- Copy and Chat handoff paths use redacted evidence.
- No mutating action appears without a documented service contract.
- Source checks pass.
- The installed dashboard is republished.
- Hosted smoke proves the Activity tab works against a live selected workspace.
