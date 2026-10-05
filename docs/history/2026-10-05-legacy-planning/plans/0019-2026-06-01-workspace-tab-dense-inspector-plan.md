# Workspace Tab Dense Inspector Plan

Date: 2026-06-01
State: COMPLETE
Lane: P12-G
Parent Roadmap: `docs/dev/plans/0012-2026-05-31-workspace-inspection-pane-app-intelligence-roadmap.md`
Umbrella Plan: `docs/dev/plans/0018-2026-06-01-workspace-inspector-tabs-productization-plan.md`
Depends On:
- `docs/dev/plans/0013-2026-05-31-selected-workspace-context-plan.md`
- `docs/dev/plans/0016-2026-05-31-effective-stealth-remote-default-launch-plan.md`

## Purpose

Ship one right-pane tab well: Workspace.

Plan 0018 is the broad product roadmap for all inspector tabs. This plan is the
next executable slice. It narrows the work to a high-density Workspace tab that
lets an operator quickly understand what browser is selected, whether it is
alive, why it can or cannot be viewed or controlled, and which service-backed
actions are available.

The Workspace tab becomes the proof point for the selected-workspace evidence
model that later Chat, Activity, Console, Network, Storage, and Extensions tabs
can reuse.

## Product Principle

The Workspace tab must be dense, legible, and actionable.

Do:

- prioritize the facts an operator needs to make the next decision
- keep related facts in compact rows, tables, and key-value grids
- use small status indicators with source-backed reasons
- expose actions only when they have a backend contract or stream capability
- show unavailable actions with a short reason when the missing action matters
- keep raw IDs, long URLs, and diagnostic payloads inspectable but collapsed

Do not:

- fill the pane with bulky cards or large vertical panels
- repeat the same state in multiple sections
- show cryptic IDs without labels, copy affordances, or context
- show decorative status chips that do not change operator behavior
- show inactionable data just because it is available
- leave active, retained, blocked, or missing selections with an empty right
  pane

## User Questions

The tab must answer these questions in the first screen without scrolling on a
normal desktop viewport:

- what workspace or browser is selected
- is it live, retained, stale, blocked, viewable, and controllable
- what page or target is active
- which process owns it, including PID, memory, CPU, and uptime when available
- which CDP and stream endpoints exist, and whether the viewport should work
- who or what owns it, such as service, agent, task, lease, session, or job
- what is the single most important reason it needs attention, if any
- what can I do now

Long diagnostics can require disclosure expansion. The first screen cannot.

## Source Findings

- Plan 0013 created the selected-workspace context direction and identified
  `packages/dashboard/src/lib/service-workspaces.ts` as the main source for
  `WorkspaceNode` runtime, ownership, primary tab, stream, related IDs,
  actions, diagnostics, and counts.
- Plan 0018 defines the full inspector-tabs product roadmap but is too broad
  for the next implementation slice.
- Current user feedback says Needs Attention and retained records are not
  useful when they look like dead browsers without inspectable detail.
- Current user feedback says an active selected browser must visibly prove
  there is a browser, show small PID/memory/CPU indicators, and render or
  diagnose viewport readiness.
- Graphiti discovery for `agent_browser_main` was healthy and reinforced the
  prior repo decision that dashboard expansion should consume authoritative
  service-owned state rather than invent frontend-only state.

## Non-Goals

- Do not implement the other inspector tabs in this slice.
- Do not redesign Chat or add new App Intelligence behavior.
- Do not add Console, Network, Storage, or Extensions capture backends.
- Do not add a new service action unless the Workspace tab cannot truthfully
  represent an existing capability without it.
- Do not let frontend heuristics pretend an action is supported when no
  service contract or stream capability advertises it.
- Do not publish a formal release. Local runtime publication is only for
  operator-visible validation.

## Layout Contract

Use a compact inspector layout, not a card stack.

### Header Strip

One line, always visible:

- selected label
- source, such as service browser, daemon session, profile, retained, missing
- state and health
- active URL host or title, truncated with tooltip or copy
- last refreshed age
- tiny indicators for PID, memory, CPU, CDP, and stream when known

The header strip should make the difference between live, stale, retained,
blocked, and missing obvious without requiring a section scan.

### Action Row

Directly under the header:

- Focus
- View
- Control
- Open externally
- Reconnect or refresh stream when supported
- Close or stop only when service contracts allow it
- Repair only when service contracts advertise a specific repair path
- Copy diagnostics

Disabled actions must explain the reason in a tooltip or inline compact note.
Do not show destructive actions as primary.

### Dense Fact Grid

Use a two-column or three-column responsive grid of labeled facts:

- identity: workspace, browser, session, profile, target, tab
- runtime: PID, running, RSS, CPU, uptime, browser host, browser build
- page: title, URL, lifecycle, focus, target ID
- stream: provider, route, port, last frame, viewers, control input, ready
- ownership: service, agent, task, lease, job
- attention: retained reason, blocked reason, incident, diagnostic

Facts should use short labels and stable formatting. Unknown values should
read `unknown`, `not reported`, or `not applicable` based on source semantics.
Avoid blank cells.

### Evidence Disclosure

Collapsed by default:

- raw related IDs
- raw stream URL
- raw service diagnostics
- related jobs and incidents
- source timestamps
- copyable redacted JSON diagnostic bundle

This disclosure exists for inspection and support, not as the main UI.

## Information Priority

Render facts in this order:

1. State that changes operator action: blocked, stale, missing, retained,
   unreachable, viewable, controllable.
2. Viewport readiness: stream provider, stream port, last frame age,
   embeddable route, control readiness.
3. Process health: PID, RSS, CPU, uptime, browser running.
4. Page identity: title, URL, target ID, active tab.
5. Ownership and lifecycle: service, agent, task, lease, job, profile source.
6. Lower-priority diagnostics and raw IDs.

If space is constrained, drop raw IDs before dropping state, stream, process,
or action facts.

## Implementation Slices

### Slice G1 | Workspace Data Audit

Goal: identify the exact fields already available and the gaps that need
backend support.

Tasks:

- Audit `WorkspaceNode`, selected-workspace context, service status, daemon
  sessions, stream state, jobs, incidents, and diagnostics.
- Produce a field map for the Workspace tab with source, freshness, and
  availability.
- Classify unavailable fields as frontend wiring gap, backend reporting gap,
  or not applicable.
- Decide whether PID, memory, CPU, uptime, stream port, CDP port, and last
  frame age can be shown from current state.

Exit criteria:

- A source-backed field map exists in code comments, tests, or the plan
  implementation notes.
- No UI work starts by guessing which values are real.

### Slice G2 | Dense Workspace Component

Goal: replace the placeholder Workspace tab with the compact inspector layout.

Tasks:

- Create or update a dedicated Workspace inspector component.
- Render the header strip, action row, dense fact grid, and evidence
  disclosure.
- Keep the layout visually compact on desktop and still readable on mobile.
- Use existing design system primitives, icons, and tooltips.
- Avoid nested cards and large padded panels.
- Add copy controls for useful IDs and diagnostic bundles.

Exit criteria:

- Selecting the stable `default` active browser shows visible browser identity,
  process indicators, stream/CDP readiness, current page, and available
  actions.
- Selecting a retained, blocked, or missing workspace shows a useful reason and
  does not look like an empty placeholder.

### Slice G3 | Action Availability And Reasons

Goal: make actions trustworthy.

Tasks:

- Derive enabled actions from `WorkspaceNode.actions`, stream metadata, and
  service request capabilities.
- Route existing actions through existing dashboard action paths.
- Show disabled reasons for relevant unavailable actions.
- Keep destructive actions visually secondary and confirmation-gated through
  existing non-native dialog patterns when applicable.
- Ensure no native browser dialogs are introduced.

Exit criteria:

- The UI never presents Focus, View, Control, Open externally, Close, Stop,
  Repair, or Reconnect as available unless the selected workspace supports it.
- The user can tell why a desired action is missing or disabled.

### Slice G4 | Focused Tests

Goal: prove the Workspace tab works before expanding other tabs.

Tasks:

- Add a focused dashboard test for the Workspace inspector layout.
- Cover active browser, retained record, missing selection, and stream
  unavailable cases.
- Assert dense facts are present without relying on full-page snapshots.
- Assert action availability and disabled reasons.
- Assert raw diagnostics are collapsed by default but inspectable.

Exit criteria:

- `pnpm test:dashboard-workspace-inspector-tab` or an equivalent focused test
  passes.
- Existing selected workspace, viewport, and inspector action tests still pass.

### Slice G5 | Publish And Hosted Smoke

Goal: make the work immediately visible in the live dashboard.

Tasks:

- Build and publish the dashboard to the local installed runtime.
- Run a hosted authenticated smoke against
  `https://agent-browser.ecochran.dyndns.org/`.
- Select the known active `default` workspace.
- Verify the right pane shows dense Workspace detail and that the viewport
  readiness facts match the live stream behavior.
- Select at least one retained or missing record and verify the tab shows a
  useful reason instead of dead space.

Exit criteria:

- The user can refresh the hosted dashboard and see the Workspace tab changes
  without a source-only build.
- Smoke evidence records the URL, selected workspace, visible facts, and any
  remaining unavailable reasons.

## Validation Matrix

Required source checks:

```bash
pnpm test:dashboard-selected-workspace-context
pnpm test:dashboard-workspace-navigator
pnpm test:dashboard-view-streams
pnpm test:dashboard-inspector-actions
pnpm test:dashboard-workspace-inspector-tab
pnpm build:dashboard
git diff --check
```

If the focused test name changes, update this plan with the actual script name
before closing it.

Required runtime checks:

```bash
pnpm publish:local-dashboard -- \
  --expect-marker Workspace \
  --json

node scripts/smoke-local-dashboard-runtime.js \
  --dashboard-url https://agent-browser.ecochran.dyndns.org/ \
  --workspace-session default \
  --browser-profile /tmp/agent-browser-workspace-tab-smoke \
  --expect-marker Workspace \
  --json
```

Run Rust checks only if backend service state, service contracts, stream
metadata, or daemon reporting changes:

```bash
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
cargo test --manifest-path cli/Cargo.toml native::stream -- --nocapture
cargo test --manifest-path cli/Cargo.toml native::actions -- --nocapture --test-threads=1
```

## Completion Criteria

- Workspace tab is the only productized inspector tab in this slice.
- The first screen is information dense and does not depend on bulky panels.
- Active selections show identity, runtime, page, stream, ownership, and action
  readiness.
- Retained, blocked, stale, and missing selections show source-backed reasons.
- PID, memory, CPU, uptime, CDP port, stream port, and last frame age appear
  when available and are clearly marked when unavailable.
- Actions are available only when backed by service or stream capability.
- Raw diagnostics are collapsed, copyable, and redacted where needed.
- Focused dashboard tests pass.
- The installed dashboard is republished and a hosted smoke proves the change
  is externally visible.

## Completion Evidence

Completed on 2026-06-01.

Source changes:

- `packages/dashboard/src/components/workspace-selection-panel.tsx` now renders
  a compact header strip, runtime indicators, action row, dense fact grid, and
  collapsed evidence disclosure.
- `packages/dashboard/src/app/globals.css` now uses dense Workspace inspector
  layout rules instead of a bulky vertical section stack.
- `docs/dev/notes/2026-06-01-workspace-tab-field-map.md` records the field
  source audit and explicitly marks process uptime and last-frame age as not
  reported unless service state supplies them.
- `scripts/test-dashboard-workspace-inspector-tab.js` covers active,
  retained, missing, and stream-unavailable selected-workspace cases.
- `scripts/publish-local-dashboard-runtime.js` now accepts `--browser-profile`
  so publish smokes can avoid the shared default profile lock.
- `scripts/smoke-local-dashboard-runtime.js` now accepts `--skip-chat` and
  validates dense Workspace details directly for Workspace-only smoke runs.

Validation:

```bash
pnpm test:dashboard-workspace-inspector-tab
pnpm test:dashboard-selected-workspace-context
pnpm test:dashboard-selected-workspace-chat-packet
pnpm test:dashboard-workspace-navigator
pnpm test:dashboard-view-streams
pnpm test:dashboard-inspector-actions
pnpm build:dashboard
git diff --check
```

Runtime publication:

```bash
pnpm publish:local-dashboard -- \
  --expect-marker Workspace \
  --browser-profile /tmp/agent-browser-workspace-tab-publish-smoke \
  --json
```

Result: success. The dashboard service restarted from PID `65610` to `75446`,
and the served bundle contained the `Workspace` marker.

Hosted smoke:

```bash
node scripts/smoke-local-dashboard-runtime.js \
  --dashboard-url https://agent-browser.ecochran.dyndns.org/ \
  --workspace-session default \
  --browser-profile /tmp/agent-browser-workspace-tab-hosted-smoke \
  --expect-marker Workspace \
  --skip-chat \
  --json
```

Result: success. The selected `default` workspace was retained/not_started and
the right pane showed dense Workspace facts, PID/RSS/CPU/CDP/stream indicators,
Copy diagnostics, and CDP canvas readiness.

```bash
node scripts/smoke-local-dashboard-runtime.js \
  --dashboard-url https://agent-browser.ecochran.dyndns.org/ \
  --workspace-session dashboard-viewer-plan0016 \
  --browser-profile /tmp/agent-browser-workspace-tab-active-hosted-smoke \
  --expect-marker Workspace \
  --skip-chat \
  --json
```

Result: success. The selected `dashboard-viewer-plan0016` workspace resolved to
`browser:session:dashboard-viewer-plan0016` with state `controllable`, PID
`93182`, CDP port `38151`, stream diagnostics, Copy diagnostics, and ready CDP
canvas rendering.
