# Superuser Codex Operator Agent Plan

Date: 2026-06-01
State: OPEN
Lane: P12-J
Parent Roadmap: `docs/dev/plans/0012-2026-05-31-workspace-inspection-pane-app-intelligence-roadmap.md`
Depends On:
- `docs/dev/plans/0020-2026-06-01-chat-tab-selected-workspace-evidence-plan.md`
- `docs/dev/plans/0021-2026-06-01-activity-tab-selected-workspace-timeline-plan.md`

## Purpose

Promote App Intelligence from a read-only selected-workspace inspector into a
superuser-only Codex operator agent that is as capable as an interactive Codex
session for agent-browser operations.

The current Plan 0020 Chat implementation intentionally runs Codex app-server
as a read-only adapter. That was the right first safety boundary, but it is not
the intended final product. Superusers should be able to ask the agent to
inspect, operate, debug, and control browser workspaces through the dashboard,
with the same practical expertise an operator has in this repo and runtime.

## Product Principle

There are two different modes:

- Inspect: read-only, available to authenticated dashboard users when allowed.
- Operate: powerful, tool-using, superuser-only, audited, and explicit about
  which workspace, browser, tab, profile, and service action it is touching.

Operate should feel like a resident agent-browser expert inside the dashboard.
It should understand the selected workspace, the live viewport, the service
control plane, the dashboard UX, agentic website operation, DOM discovery, CDP
debugging, and safe service-mediated browser automation.

## Current State

The current app-server path is deliberately constrained:

- `cli/src/native/stream/app_intelligence.rs` rejects mutating request fields.
- `cli/src/native/stream/app_intelligence_supervisor.rs` starts Codex with
  read-only instructions, no network, no tools, no commands, and no service
  mutation.
- The observation schema only allows summaries, blockers, risks, suggested
  next inspections, unsupported actions, and confidence.
- The dashboard exposes a Codex app server read-only Chat surface.

The auth layer already returns a dashboard identity with a `role` field, but
role semantics need hardening before Operate ships. The bootstrap users are
named `admin` and `codex`; `admin` is labeled default superuser, while `codex`
is labeled observer, yet the current helper assigns the same `superuser` role
to both. This plan must make superuser gating explicit and testable.

## Non-Negotiable Requirements

- Operate mode is available only to users whose authenticated dashboard
  identity has role `superuser`.
- Non-superusers and unauthenticated requests receive no operator-agent route,
  no operator-agent status, no tools list, and no actionable affordance.
- The existing read-only inspection mode remains available as a separate,
  bounded surface.
- Every operator-agent action is tied to an authenticated user, selected
  workspace, target browser or tab, prompt, tool call, result, and timestamp.
- Browser and service mutations flow through existing service contracts or
  newly documented contracts. The operator agent must not invent ad hoc
  dashboard-only mutation paths.
- Destructive or high-risk actions require explicit confirmation unless a
  service contract marks them safe for unattended execution.
- Secrets, cookies, raw storage values, auth headers, dashboard auth cookies,
  screenshots containing private data, and browser auth artifacts are not sent
  by default. The agent can request scoped evidence through redacted providers.
- The operator agent must never silently switch target workspaces. If it needs
  to operate a different workspace, it must say so and update selection through
  the dashboard/service contract.

## Capability Target

The superuser operator agent should be agent-browser smart.

It should know how to:

- explain service-owned browser/session/profile/tab state
- distinguish live, retained, blocked, stale, view-only, and controllable
  workspaces
- diagnose CDP, stream, Guacamole, RDP gateway, route, frame, and control-input
  readiness
- switch the active viewed browser in the dashboard
- open a new browser or workspace using service-mediated launch contracts
- navigate the selected browser or a newly created browser to a URL
- inspect DOM structure, forms, clickable controls, accessible names, text,
  visibility, and layout
- click, type, scroll, focus, select, submit, wait, screenshot, snapshot, and
  evaluate safe DOM queries through browser-control tools
- use console, network, storage, and extension evidence when those providers
  exist
- operate the dashboard UX itself when asked, including tab switching,
  workspace selection, viewport mode changes, and inspector pane actions
- debug why a page, workspace, stream, or browser automation task is stuck
- produce concise status, handoff, and audit summaries

## UX Contract

Add a visible mode split in Chat or App Intelligence:

- Inspect: read-only observation, as shipped in Plan 0020.
- Operate: superuser-only agent with tools.

When a superuser opens Operate:

- show selected workspace and target scope first
- show “superuser operator” status and authenticated username
- show available tool groups and disabled reasons
- show pending confirmation requests inline
- show tool call timeline with inputs and outputs redacted where needed
- show final result with changed state, evidence, and any residual risk

When a non-superuser opens the same UI:

- do not render Operate controls
- do not expose tool names, hidden routes, or implementation hints
- show Inspect only, or a concise “superuser required” state where needed

## Backend Contract

Add a new operator surface instead of expanding the read-only endpoint:

- `GET /api/app-intelligence/operator/status`
- `POST /api/app-intelligence/operator/turn`
- `POST /api/app-intelligence/operator/confirm`
- optional event stream or polling endpoint for tool progress

All operator endpoints must:

- authenticate dashboard headers or session cookie
- require `role == "superuser"`
- reject unauthenticated and non-superuser requests before parsing tool
  payloads
- record the authenticated username in the operator ledger
- use CSRF-aware same-origin request handling consistent with dashboard auth
- preserve the existing read-only `/api/app-intelligence/inspect-workspace`
  semantics

## Role Hardening

Before enabling Operate:

- make role construction explicit: `admin` is `superuser`, `codex` is
  `observer` unless configured otherwise
- add a helper such as `require_superuser(headers)` in dashboard auth
- add tests proving observer and unauthenticated identities cannot access
  operator routes
- expose only the authenticated user's display label and role to the dashboard
  UI
- preserve existing bootstrap credentials without silently escalating observer
  accounts

If existing user-scoped auth stores already have `codex` as `superuser`, the
migration must be explicit and documented. Do not silently downgrade a human
configured account without a backup or operator note.

## Agent Instructions

Operate mode should start Codex app-server with a system prompt that teaches it
the product and runtime:

- You are Agent Browser Operator, a superuser-only browser operations agent.
- Your job is to operate agent-browser, its dashboard, and selected browser
  workspaces for a human superuser.
- You understand agent-browser service state, runtime profiles, CDP, streams,
  Guacamole/RDP, dashboard workspace selection, inspector tabs, and service
  request contracts.
- You can use provided tools to inspect DOM, operate pages, navigate browsers,
  switch viewed workspaces, launch browsers, and request service actions.
- Prefer service-owned state over frontend guesses.
- Before mutating, identify target workspace, browser, tab, profile, and
  service contract.
- Use confirmations for destructive or broad actions.
- Keep tool use minimal and purposeful.
- Report what changed and cite tool call evidence.

The prompt should reference repo-local operating guidance and current runtime
facts through structured context packets, not by giving unrestricted file-system
access to private state.

## Tool Groups

### Dashboard Tools

- get selected workspace
- set selected workspace
- switch inspector tab
- switch viewport mode
- open external stream
- copy or produce diagnostic bundle

### Browser Tools

- list browsers, sessions, profiles, tabs, and streams
- focus browser or tab
- open new browser or workspace
- navigate selected tab
- new tab
- close tab or browser with confirmation when destructive
- wait for load, selector, frame, stream, or network idle

### DOM Tools

- snapshot accessible tree
- query elements by role, label, text, selector, and coordinates
- click, type, press, scroll, select, drag when supported
- evaluate safe read-only DOM expressions
- screenshot selected viewport or element with privacy controls

### Debug Evidence Tools

- console summary
- network summary
- storage summary with key/value redaction
- extension summary
- stream readiness
- service jobs and incidents
- Activity event lookup

### Service Tools

- submit existing service request actions
- inspect service job status
- cancel/retry only where a service contract says it is supported
- repair/reconnect stream only through documented contracts
- launch with profile, browser build, display isolation, and lease policy

## Confirmation Policy

Require explicit superuser confirmation for:

- closing or killing browsers
- pruning retained records
- clearing storage or cookies
- deleting profile data
- broad multi-workspace actions
- actions against authenticated profiles
- navigation to sensitive or destructive admin pages when detected
- any action where the target workspace is ambiguous

Confirmation records must include action, target, risk summary, requesting
prompt, username, timestamp, and result.

## Audit And Activity

Every operator run should write:

- run ledger
- prompt and selected context hash
- authenticated username and role
- tool calls and redacted arguments
- confirmations requested and answered
- service request IDs and job IDs
- before/after selected workspace facts
- final answer

Activity should show operator-agent events scoped to the selected workspace.
Chat should be able to include those Activity rows as evidence.

## Implementation Slices

### Slice J1 | Superuser Gate And Role Semantics

Goal: make authorization safe before tools exist.

Tasks:

- Add explicit auth roles for `superuser` and `observer`.
- Add a superuser authorization helper.
- Add operator status route returning available only for superusers.
- Ensure non-superusers cannot discover operator route capabilities.
- Add auth tests for admin, observer, missing cookie, and invalid session.

Exit criteria:

- Tests prove Operate is invisible and inaccessible unless role is superuser.

### Slice J2 | Operator Agent Skeleton

Goal: create a separate operator app-server path without browser mutation yet.

Tasks:

- Add operator ledger model and run root.
- Start Codex app-server with operator-specific instructions.
- Pass selected-workspace context, dashboard identity, and available tool
  manifest.
- Return structured operator responses and tool-plan proposals.
- Preserve Plan 0020 read-only Inspect unchanged.

Exit criteria:

- A superuser can ask for an operational plan and receive a structured response
  that knows the selected workspace and available tools.

### Slice J3 | Read Tools And Dashboard Selection Tools

Goal: let the operator agent inspect current state and operate the dashboard
selection safely.

Tasks:

- Implement read tools for service state, selected workspace, Activity,
  stream readiness, and viewport status.
- Implement dashboard selection tools for switching active viewed workspace,
  switching inspector tabs, and changing viewport mode.
- Audit every tool call.

Exit criteria:

- A superuser can ask the agent to switch the active viewed browser and explain
  what changed.

### Slice J4 | Browser And DOM Control Tools

Goal: let the operator agent control the selected browser through service and
CDP contracts.

Tasks:

- Add focus, navigate, new tab, wait, DOM snapshot, query, click, type, press,
  scroll, and screenshot tools.
- Scope every browser tool to selected browser/tab unless the prompt
  explicitly asks to launch or switch.
- Add target ambiguity checks.
- Add privacy controls for screenshots and page content.

Exit criteria:

- A superuser can ask the agent to operate the selected browser through a
  simple website workflow and see audited tool calls.

### Slice J5 | Launch And Service Action Tools

Goal: let the operator agent create and manage browser workspaces through
service contracts.

Tasks:

- Expose service-mediated launch with profile, browser build, URL, display
  isolation, and lease policy.
- Expose safe repair/reconnect/focus actions.
- Add confirmation-gated close, kill, prune, retry, cancel, and storage actions
  only where service contracts support them.

Exit criteria:

- A superuser can ask the agent to open a new browser, navigate to a site, and
  switch the dashboard viewport to that browser.

### Slice J6 | UX Productization And Runtime Smoke

Goal: make Operate externally useful and visibly safe.

Tasks:

- Add the Inspect/Operate mode split.
- Add superuser-only controls, tool timeline, confirmation UI, and audit
  summary.
- Add hosted smokes for admin allowed and observer denied.
- Add a live browser-operation smoke that opens a browser, navigates, switches
  the viewport, and performs a DOM interaction.

Exit criteria:

- Hosted runtime proves superusers can operate browser workspaces through the
  agent and non-superusers cannot access the operator.

## Validation Matrix

Required source checks will expand as slices land. Initial checks:

```bash
pnpm test:dashboard-contextual-chat
pnpm test:dashboard-selected-workspace-chat-packet
pnpm test:dashboard-workspace-inspector-tab
pnpm build:dashboard
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
cargo test --manifest-path cli/Cargo.toml app_intelligence -- --nocapture
git diff --check
```

Required runtime checks once tools are present:

```bash
pnpm publish:local-dashboard -- \
  --expect-marker data-codex-app-server-contextual-chat \
  --expect-marker data-superuser-operator-agent \
  --browser-profile /tmp/agent-browser-operator-agent-publish-smoke \
  --json

node scripts/smoke-local-dashboard-runtime.js \
  --dashboard-url https://agent-browser.ecochran.dyndns.org/ \
  --workspace-session <live-session> \
  --browser-profile /tmp/agent-browser-operator-agent-hosted-smoke \
  --expect-marker data-superuser-operator-agent \
  --json
```

Add dedicated smokes for:

- admin can see and use Operate
- observer cannot see Operate controls
- unauthenticated requests cannot discover operator tools
- operator can switch viewed workspace
- operator can launch a browser and navigate to a URL
- operator can perform a DOM discovery and click/type workflow
- destructive action requires confirmation

## Completion Criteria

- Operate mode is superuser-only in backend routes and frontend UX.
- Read-only Inspect remains separate and unchanged for Plan 0020 behavior.
- The app-server agent has agent-browser-specific operating instructions.
- The agent has audited tools for dashboard selection, browser operation, DOM
  discovery, debugging evidence, and service-mediated actions.
- Every mutation is scoped, logged, and service-contract backed.
- Destructive or broad actions require confirmation.
- Hosted runtime proves a superuser can operate the active browser and switch
  the viewed workspace.
- Hosted runtime proves non-superusers cannot access operator capabilities.
