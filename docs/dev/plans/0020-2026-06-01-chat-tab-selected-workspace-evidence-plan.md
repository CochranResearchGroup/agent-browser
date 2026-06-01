# Chat Tab Selected Workspace Evidence Plan

Date: 2026-06-01
State: COMPLETE
Lane: P12-H
Parent Roadmap: `docs/dev/plans/0012-2026-05-31-workspace-inspection-pane-app-intelligence-roadmap.md`
Depends On:
- `docs/dev/plans/0014-2026-05-31-contextual-chat-codex-app-server-plan.md`
- `docs/dev/plans/0019-2026-06-01-workspace-tab-dense-inspector-plan.md`

## Purpose

Ship the next right-pane tab well: Chat.

The Workspace tab now proves selected-workspace identity, runtime facts,
stream readiness, action availability, and diagnostic evidence. The Chat tab
should consume that same selected-workspace evidence and turn it into a
read-only Codex app-server inspection surface that is specific, bounded,
auditable, and useful.

## Product Principle

Chat is not a generic model playground in this dashboard lane.

It is a selected-workspace inspection console backed by the Codex app server.
The tab should help an operator understand the selected browser, viewport,
stream, page, ownership, and blockers without leaving the dashboard or copying
raw diagnostics by hand.

Do:

- keep Codex app server as the only exposed provider
- show which selected-workspace evidence groups are included
- keep the first screen dense and operational
- make inspection results structured and replayable
- cite evidence IDs or evidence groups in every observation
- render failures as actionable inspection failures, not chat errors
- keep unsupported or unavailable evidence explicit

Do not:

- expose OpenAI, AI Gateway, model selectors, Codex exec, OpenClaw, AuraCall,
  or generic provider configuration in this selected-workspace Chat surface
- let Chat execute browser, service, file, deploy, storage, or incident
  actions
- send raw cookies, storage values, auth headers, dashboard auth cookies,
  screenshots, passwords, tokens, private page content, or browser auth
  artifacts by default
- use bulky transcript cards that push provider, evidence, and run state below
  the fold
- show free-form assistant prose without validated structure

## User Questions

The Chat tab must answer these questions quickly:

- what selected workspace is Chat inspecting
- which evidence groups are included or unavailable
- whether Codex app server is ready
- whether the latest inspection succeeded, failed validation, or is still
  running
- what Codex observed about viewability, controllability, page state, stream
  readiness, ownership, and blockers
- which evidence supports each observation
- what read-only checks should happen next

## Scope

This plan productizes Chat only. It can improve the shared selected-workspace
Chat packet when needed, but it should not implement Activity, Console,
Network, Storage, or Extensions as full evidence tabs.

Initial evidence groups:

- Workspace: identity, state, runtime, page, stream, ownership, diagnostics,
  actions, and selected unavailable reasons from Plan 0019
- Activity summary: counts and related jobs/incidents already present on the
  selected-workspace context
- Stream readiness: viewport provider, route summary, control input, CDP/stream
  ports, embeddability, and live viewport readiness status when available

Other evidence groups can appear as unavailable with source-backed reasons,
but should not pretend to contain scoped detail until those tabs are
implemented.

## Layout Contract

Use a compact inspection layout.

### Header Strip

Always visible:

- selected workspace label and state
- Codex app server provider badge
- readiness: ready, starting, unavailable, or failed
- evidence freshness age
- latest run status

### Evidence Selector

Compact toggles or checkboxes:

- Workspace
- Activity summary
- Stream readiness
- Console unavailable
- Network unavailable
- Storage unavailable
- Extensions unavailable

Unavailable groups should be visible but disabled with a reason.

### Prompt And Actions

Primary controls:

- Inspect selected workspace
- Ask follow-up
- Copy observation
- Copy evidence packet

Do not expose mutating browser or service actions.

### Observation Surface

Render the latest validated observation with compact sections:

- Summary
- Detected state
- Blockers
- Risks
- Suggested next read-only inspections
- Evidence references
- Run ledger: run ID, thread ID, turn ID, started, completed, validation

The raw event log can be collapsed below the observation.

## Implementation Slices

### Slice H1 | Evidence Packet Tightening

Goal: make the selected-workspace Chat packet match the dense Workspace tab.

Tasks:

- Include Plan 0019 Workspace facts in the packet with evidence IDs.
- Add evidence-group metadata: included, unavailable, freshness, and source.
- Preserve redaction guarantees for URLs, headers, cookies, storage, tokens,
  auth state, and private artifacts.
- Ensure unavailable Console, Network, Storage, and Extensions groups are
  labeled as unavailable rather than omitted silently.

Exit criteria:

- Packet tests prove Workspace, Activity summary, and Stream readiness evidence
  are included when available, and unavailable groups carry reasons.

### Slice H2 | Chat Tab Dense UI

Goal: replace generic chat affordances with a selected-workspace inspection
surface.

Tasks:

- Render the header strip, evidence selector, prompt controls, observation
  surface, and collapsed event log.
- Keep Codex app server as the only visible provider.
- Remove or hide generic model/provider selector text from this lane.
- Keep the first screen useful before any run has completed by showing the
  selected workspace and included evidence groups.
- Preserve existing manual follow-up input, but scope it to selected-workspace
  evidence.

Exit criteria:

- A selected workspace shows provider, readiness, evidence groups, and an
  inspection action without requiring scrolling through chat history.

### Slice H3 | Structured Observation Handling

Goal: make inspection results reliable and auditable.

Tasks:

- Validate Codex app-server responses against the existing observation schema.
- Render validation failures as structured inspection failures.
- Keep event logs and run metadata available in a collapsed section.
- Prevent unvalidated prose from being displayed as an observation.
- Add copy controls for the validated observation and redacted evidence packet.

Exit criteria:

- Tests prove valid observations render, invalid observations render as
  inspection failures, and every observation cites evidence groups.

### Slice H4 | Focused Tests

Goal: prove Chat is selected-workspace aware and provider bounded.

Tasks:

- Add or extend focused tests for the Chat packet and Chat panel.
- Cover Codex app-server-only provider display.
- Cover included and unavailable evidence groups.
- Cover read-only inspection controls.
- Cover structured observation, validation failure, and event-log rendering.
- Assert no model selector, AI Gateway provider selector, OpenAI label, Codex
  exec action, or mutating service action appears in the selected-workspace
  Chat lane.

Exit criteria:

- `pnpm test:dashboard-selected-workspace-chat-packet`
- `pnpm test:dashboard-contextual-chat`
- a new focused Chat UI test, if needed

### Slice H5 | Runtime Publish And Hosted Smoke

Goal: make the Chat tab changes externally visible immediately.

Tasks:

- Publish the local dashboard runtime after source validation.
- Smoke the hosted dashboard against a live selected workspace.
- Open the Chat tab and verify Codex app server provider, evidence groups,
  read-only inspection action, and structured run output.
- Also verify that Workspace tab still renders correctly after Chat changes.

Exit criteria:

- Hosted smoke proves the selected-workspace Chat tab is useful without a
  source-only build.

## Validation Matrix

Required source checks:

```bash
pnpm test:dashboard-selected-workspace-chat-packet
pnpm test:dashboard-contextual-chat
pnpm test:dashboard-workspace-inspector-tab
pnpm test:dashboard-selected-workspace-context
pnpm build:dashboard
git diff --check
```

Add a focused Chat UI test if existing tests cannot prove provider boundaries,
evidence toggles, and structured observation rendering.

Required runtime checks:

```bash
pnpm publish:local-dashboard -- \
  --expect-marker data-codex-app-server-contextual-chat \
  --expect-marker Workspace \
  --browser-profile /tmp/agent-browser-chat-tab-publish-smoke \
  --json

node scripts/smoke-local-dashboard-runtime.js \
  --dashboard-url https://agent-browser.ecochran.dyndns.org/ \
  --workspace-session <live-session> \
  --browser-profile /tmp/agent-browser-chat-tab-hosted-smoke \
  --expect-marker data-codex-app-server-contextual-chat \
  --expect-marker Workspace \
  --json
```

## Completion Criteria

- Chat is the only newly productized inspector tab in this slice.
- Codex app server is the only visible provider.
- Selected-workspace evidence groups are visible, selectable, and source
  labeled.
- Unavailable groups show reasons.
- Structured observations render with evidence references and run ledger.
- Validation failures are useful and distinguishable from transport failures.
- No mutating browser, service, file, deploy, incident, storage, or auth action
  can be executed from Chat.
- Source checks pass.
- The installed dashboard is republished.
- Hosted smoke proves the Chat tab works against a live selected workspace.

## Completion Evidence

Completed on 2026-06-01.

Source checks:

```bash
pnpm test:dashboard-selected-workspace-chat-packet
pnpm test:dashboard-contextual-chat
pnpm test:dashboard-workspace-inspector-tab
pnpm test:dashboard-selected-workspace-context
pnpm build:dashboard
git diff --check
```

Runtime checks:

```bash
pnpm publish:local-dashboard -- \
  --expect-marker data-codex-app-server-contextual-chat \
  --expect-marker Workspace \
  --browser-profile /tmp/agent-browser-chat-tab-publish-smoke \
  --json

node scripts/smoke-local-dashboard-runtime.js \
  --dashboard-url https://agent-browser.ecochran.dyndns.org/ \
  --workspace-session default-posture-smoke \
  --browser-profile /tmp/agent-browser-chat-tab-hosted-smoke-2 \
  --expect-marker data-codex-app-server-contextual-chat \
  --expect-marker Workspace \
  --json
```

The hosted smoke proved the selected `default-posture-smoke` workspace rendered
a live viewport, dense Workspace facts, Codex app server read-only Chat, the
Workspace, Activity summary, Stream readiness, and unavailable evidence groups,
plus a structured Codex observation with event-log and thread/turn ledger
metadata.
