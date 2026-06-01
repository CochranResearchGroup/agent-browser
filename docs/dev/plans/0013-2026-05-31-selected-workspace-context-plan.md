# Selected Workspace Context Plan

Date: 2026-05-31
State: DONE
Lane: P12-A
Parent Roadmap: `docs/dev/plans/0012-2026-05-31-workspace-inspection-pane-app-intelligence-roadmap.md`

## Purpose

Implement the first slice of the Workspace Inspection Pane and App Intelligence
roadmap: a shared selected-workspace context that every right-pane tab can
consume.

The current dashboard has the right tab labels but does not have a single
workspace evidence model. `WorkspaceSelectionPanel` is local to
`packages/dashboard/src/app/page.tsx` and mostly reads daemon session state.
`WorkspaceRemoteViewport` independently resolves the same URL selection and
service state. Console, Network, Storage, Extensions, Activity, and Chat still
mostly use the active daemon session rather than the selected workspace.

This plan creates the shared context contract and wires the Workspace tab first
without overbuilding Chat or the evidence providers in this slice.

## Source Findings

- `packages/dashboard/src/lib/workspace-url-selection.ts` is already the URL
  authority for `workspace`, `browser`, `session`, `tab`, `profile`, and `job`
  query values.
- `packages/dashboard/src/lib/service-workspaces.ts` already derives
  `WorkspaceNode` records with useful runtime, ownership, primary tab,
  stream, related IDs, actions, diagnostics, and counts.
- `packages/dashboard/src/components/workspace-navigator.tsx` already fetches
  service status, derives workspace nodes, and updates URL selection when a
  workspace is chosen.
- `packages/dashboard/src/components/workspace-remote-viewport.tsx`
  independently re-reads URL selection, service status, daemon sessions, tabs,
  and view streams to render the selected workspace viewport.
- `packages/dashboard/src/app/page.tsx` defines `WorkspaceSelectionPanel`
  inline. It resolves only a daemon session name and shows a small set of
  details, so it misses service browser health, PID, CPU, memory, CDP port,
  profile, ownership, incidents, jobs, and retained/attention reasons.
- `console-panel.tsx`, `network-panel.tsx`, `storage-panel.tsx`,
  `extensions-panel.tsx`, and `chat-panel.tsx` currently key off
  `activeSessionNameAtom`; they are not selected-workspace aware yet.
- Graphiti discovery for `agent_browser_main` was healthy and reinforced the
  repo rule that dashboard expansion must consume authoritative service state
  rather than invent frontend-only semantics.

## Non-Goals

- Do not implement the full App Intelligence bridge in this slice.
- Do not redesign Chat beyond accepting or displaying selected-workspace
  context affordances if needed for wiring.
- Do not add new service mutating actions.
- Do not add Network, Storage, Console, or Extension backend capture contracts.
- Do not replace the Service selected-record inspector.
- Do not change Guacamole/RDP or CDP stream rendering behavior except where the
  shared context removes duplicated selection logic.
- Do not publish a formal release. Local runtime publication is only for
  operator-visible validation.

## Product Contract

Add a selected-workspace context model that answers:

- what URL selection is active
- which `WorkspaceNode` best represents that selection
- which daemon session, service browser, service sessions, tabs, jobs,
  incidents, and profile allocation are related
- what the primary tab and primary stream are
- whether the workspace is live, retained, attention-worthy, blocked,
  viewable, or controllable
- which runtime indicators are known: PID, running state, CPU seconds, RSS
  bytes, CDP port, stream port, uptime when available, and last frame age when
  available
- which actions are service-backed and currently enabled
- which evidence rows are safe to share with Chat/App Intelligence later

Proposed TypeScript shape:

```ts
export type SelectedWorkspaceContext = {
  selection: DashboardWorkspaceUrlSelection;
  node: WorkspaceNode | null;
  source: "service-browser" | "service-session" | "daemon-session" | "profile" | "none";
  label: string;
  state: WorkspaceNodeState | "none" | "missing";
  live: boolean;
  retained: boolean;
  viewable: boolean;
  controllable: boolean;
  browser: WorkspaceServiceBrowser | null;
  daemonSession: SessionInfo | null;
  serviceSessions: WorkspaceServiceSession[];
  tabs: WorkspaceServiceTab[];
  primaryTab: WorkspaceNodePrimaryTab | null;
  profileAllocation: WorkspaceServiceProfileAllocation | null;
  jobs: WorkspaceServiceJob[];
  incidents: WorkspaceServiceIncident[];
  stream: WorkspaceNodeViewStream | null;
  runtime: {
    pid: number | null;
    running: boolean | null;
    rssBytes: number | null;
    cpuSeconds: number | null;
    cdpPort: number | null;
    streamPort: number | null;
    lastFrameAt: number | null;
  };
  ownership: WorkspaceNodeOwnership;
  actions: WorkspaceNodeAction[];
  diagnostics: WorkspaceOwnershipDiagnostic[];
  evidence: SelectedWorkspaceEvidence;
  refreshedAt: number;
};
```

The exact field names can change during implementation, but the model should
remain stable enough for later Chat/App Intelligence context packets.

## Implementation Slices

### Slice A1 | Context Module And Selection Resolution

Goal: create a reusable selected-workspace context library without changing the
visible UI.

Tasks:

- Add `packages/dashboard/src/lib/selected-workspace-context.ts`.
- Reuse `DashboardWorkspaceUrlSelection` and `deriveWorkspaceNodes`.
- Implement selection matching by priority:
  - direct `browserId`
  - `workspaceId` with `browser:` prefix
  - direct `sessionId` or `workspaceId` with `daemon-session:` prefix
  - direct `profileId`
  - direct `tabId`
  - `jobId` related IDs
- Return a missing context with an explicit reason when a URL selection no
  longer maps to service or daemon state.
- Add helpers for display labels, runtime indicator formatting, diagnostic
  bundle creation, and Chat-safe evidence extraction.

Exit criteria:

- Unit/static tests can build a selected context from fixture service state and
  daemon sessions.
- Missing, stale, service-browser, daemon-session, retained, and profile-only
  selections are distinguishable.

### Slice A2 | Shared Hook And Data Fetch Boundary

Goal: give dashboard components one hook for selected workspace context.

Tasks:

- Add `useSelectedWorkspaceContext` in either a new dashboard hook file or the
  context module.
- Centralize service status fetch cadence for right-pane workspace context, or
  explicitly document why `WorkspaceNavigator` and right pane still fetch
  separately in this slice.
- Listen to `DASHBOARD_WORKSPACE_SELECTION_EVENT` and `popstate`.
- Include daemon session and tab atoms so daemon-only sessions still resolve.
- Avoid polling from inactive/unmounted right-pane tabs where possible.

Exit criteria:

- `WorkspaceSelectionPanel` and `WorkspaceRemoteViewport` can consume the same
  context object.
- Selection changes update the Workspace tab without requiring a full route
  reload.

### Slice A3 | Workspace Tab Replacement

Goal: replace the placeholder Workspace tab with an operational selected
workspace summary.

Tasks:

- Move `WorkspaceSelectionPanel` out of `page.tsx` into a dedicated component,
  for example `packages/dashboard/src/components/workspace-selection-panel.tsx`.
- Render sections:
  - Identity: workspace, browser, session, profile, source, owner
  - Runtime: state, health, PID, CPU, memory, running, CDP port, stream port
  - Page: active title, URL, target ID, lifecycle, focus status
  - Stream: provider, readiness, route, input, view/control status
  - Activity: related jobs, incidents, diagnostics, retained/attention reason
- Add compact action buttons only for actions already present on
  `WorkspaceNode.actions`, such as focus, view, control, external open, close,
  kill, repair, copy link, or add tab.
- Add `Copy diagnostics` to copy a redacted selected-workspace diagnostic
  bundle.
- Keep long raw endpoints and IDs behind a compact Evidence disclosure.

Exit criteria:

- Selecting the current stable `default` CDP workspace shows live runtime and
  stream facts in the Workspace tab.
- A retained or missing workspace selection shows why it cannot be inspected or
  controlled.
- No native browser dialogs are used.

### Slice A4 | Right-Pane Tab Context Handoff

Goal: prepare non-workspace tabs for evidence-provider conversion without
  implementing their full redesign.

Tasks:

- Pass selected-workspace context or a context ID to Chat, Activity, Console,
  Network, Storage, and Extensions panels.
- Add minimal empty-state copy when a tab is still using active daemon session
  fallback instead of selected workspace data.
- Add stable data attributes or test markers proving each tab received the
  selected context.
- Keep existing panel behavior otherwise unchanged.

Exit criteria:

- Each right-pane tab can identify the selected workspace in testable DOM
  state, even if its detailed content remains a later slice.
- Chat has access to a selected-workspace summary packet but does not yet call
  App Intelligence.

### Slice A5 | Tests, Live Publish, And Handoff

Goal: prove the context model works in source and in the installed dashboard.

Tasks:

- Add or extend dashboard static tests for selected-workspace context.
- Add DOM smoke assertions for the Workspace tab sections and selected-context
  markers.
- Run the selected validation gates.
- Publish to the local runtime before asking for external operator review.
- Smoke the external dashboard against the stable `default` CDP workspace.

Exit criteria:

- Source tests pass.
- Live dashboard chunks include the selected-workspace marker.
- External DOM smoke proves the Workspace tab shows selected-context runtime
  data and the viewport remains live.

## File-Level Plan

Expected new files:

- `packages/dashboard/src/lib/selected-workspace-context.ts`
- `packages/dashboard/src/components/workspace-selection-panel.tsx`
- optionally `scripts/test-dashboard-selected-workspace-context.js`

Expected edited files:

- `packages/dashboard/src/app/page.tsx`
- `packages/dashboard/src/components/workspace-remote-viewport.tsx`
- `packages/dashboard/src/components/chat-panel.tsx`
- `packages/dashboard/src/components/activity-feed.tsx`
- `packages/dashboard/src/components/console-panel.tsx`
- `packages/dashboard/src/components/network-panel.tsx`
- `packages/dashboard/src/components/storage-panel.tsx`
- `packages/dashboard/src/components/extensions-panel.tsx`
- `packages/dashboard/src/app/globals.css`
- `scripts/test-dashboard-view-streams.js` or a new focused test script
- `package.json` if a new test script is added

Docs are not required for Slice A unless visible workflow language changes
substantially. If the tab behavior is documented during implementation, update
`README.md`, `docs/src/app/`, and `skills/agent-browser/SKILL.md` together.

## Data And Action Rules

- Prefer `WorkspaceNode` and service status over daemon-only session atoms
  when both exist.
- Use daemon-only session atoms as fallback for sessions not reconciled into
  service state.
- Treat terminal health values such as `process_exited`, `cdp_disconnected`,
  `closed`, `not_started`, and `unreachable` as non-live unless service state
  provides a repair action.
- Do not show Close or Kill for retained records unless the service action is
  meaningful.
- Do not show storage, request, cookie, header, or screenshot secrets in the
  diagnostic bundle.
- Use service request actions for mutation. Frontend-only action buttons may
  navigate, copy, switch tabs, or open existing stream URLs, but must not
  mutate service state.

## Validation Matrix

Required source validation:

- `pnpm test:dashboard-view-streams`
- `pnpm test:dashboard-workspace-nodes`
- `pnpm test:dashboard-inspector-actions`
- `pnpm build:dashboard`
- `git diff --check`

If a new selected-context test script is added:

- `pnpm test:dashboard-selected-workspace-context`

Required live validation before operator closeout:

```bash
pnpm publish:local-dashboard -- --expect-marker <selected-workspace-marker> --json
```

Then run a live DOM smoke against:

```text
https://agent-browser.ecochran.dyndns.org/?workspace=browser%3Asession%3Adefault&session=default&view=workspace%3Acontrol&browser=session%3Adefault&profile=default
```

The smoke should prove:

- Workspace tab renders the selected context for `default`.
- Runtime indicators include at least PID and stream port when the browser is
  live.
- CDP canvas still renders live frames.
- No nested dashboard login iframe appears.
- Right-pane non-workspace tabs expose a selected-context marker.

## Risks And Mitigations

- Risk: duplicating service-status fetches causes stale or noisy UI.
  Mitigation: centralize the hook when practical, otherwise document the
  temporary duplicate fetch boundary and keep refresh cadence modest.
- Risk: selected context chooses the wrong record when service and daemon IDs
  overlap.
  Mitigation: make matching priority explicit and add fixture tests for
  service-browser, daemon-session, profile-only, tab, and stale selections.
- Risk: moving `WorkspaceSelectionPanel` creates layout regressions.
  Mitigation: keep the first UI dense and scoped; validate desktop and mobile
  after source tests pass.
- Risk: Chat starts consuming too much evidence too early.
  Mitigation: Slice A only passes a compact summary packet and leaves full App
  Intelligence to Slice E.
- Risk: live publication restarts the dashboard service and stops the default
  browser during validation.
  Mitigation: relaunch `default` with `agent-browser open about:blank --session
  default --json` before final external smoke, as done in prior CDP stream
  validation.

## Completion Criteria

Slice A is complete when:

- The selected-workspace context module exists and is tested.
- The Workspace tab uses it as its source of truth.
- The viewport and right-pane tabs agree on the selected workspace.
- The current stable `default` CDP workspace shows live runtime, stream, page,
  ownership, and related-record summaries.
- The installed dashboard service has been updated through
  `pnpm publish:local-dashboard`.
- The final handoff includes source validation and live external smoke
  evidence.

## Recommended Implementation Order

1. Add context module and fixture tests.
2. Move and replace `WorkspaceSelectionPanel`.
3. Wire `WorkspaceRemoteViewport` to consume the shared context where this does
   not destabilize stream rendering.
4. Add selected-context markers to the remaining right-pane tabs.
5. Run source validation.
6. Publish and run live smoke on the external dashboard.

## Implementation Closeout

Completed: 2026-05-31

Implemented:

- Added `packages/dashboard/src/lib/selected-workspace-context.ts` as the
  shared selected-workspace evidence model.
- Added `packages/dashboard/src/hooks/use-selected-workspace-context.ts` for
  right-pane service status, daemon session, tab, and URL selection handoff.
- Replaced the inline daemon-only Workspace tab with
  `packages/dashboard/src/components/workspace-selection-panel.tsx`.
- Added selected-context markers to Chat, Activity, Console, Network, Storage,
  Extensions, and the Workspace remote viewport.
- Added `scripts/test-dashboard-selected-workspace-context.js` and the matching
  `pnpm test:dashboard-selected-workspace-context` script.

Source validation passed:

- `pnpm test:dashboard-selected-workspace-context`
- `pnpm test:dashboard-view-streams`
- `pnpm test:dashboard-workspace-navigator`
- `pnpm test:dashboard-workspace-nodes`
- `pnpm test:dashboard-inspector-actions`
- `pnpm build:dashboard`
- `git diff --check`

Runtime publication passed:

```bash
pnpm publish:local-dashboard -- --dashboard-url https://agent-browser.ecochran.dyndns.org/ --expect-marker data-selected-workspace-context --json
```

Result:

- Installed binary: `/home/ecochran76/.local/bin/agent-browser`
- Backup: `/home/ecochran76/.local/bin/agent-browser.pre-local-dashboard-20260531145437`
- User service restarted from PID `84389` to PID `89806`
- External bundle marker `data-selected-workspace-context` was found in a
  served JavaScript chunk.

External selected-workspace DOM smoke passed against:

```text
https://agent-browser.ecochran.dyndns.org/?workspace=browser%3Asession%3Adefault&session=default&view=workspace%3Acontrol&browser=session%3Adefault&profile=default
```

Observed:

- Workspace panel marker: `ready`
- Workspace id: `browser:session:default`
- Workspace state: `controllable`
- Viewport selected workspace id/state matched the Workspace panel.
- Chat, Activity, Console, Network, Storage, and Extensions all exposed
  selected-context markers for `browser:session:default`.
- Workspace text included PID `6938`, CDP port `36789`, stream port `36511`,
  and `cdp_screencast`.
- CDP canvas rendered at `800x513`.
- No nested dashboard login was present inside the viewport.
