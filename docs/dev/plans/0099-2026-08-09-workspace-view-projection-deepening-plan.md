# Plan 0099 | Workspace View Projection Deepening

State: IMPLEMENTATION READY | CYCLE 2 RESIDUAL RESOLVED
Roadmap: P99
Plan version: 2
Date: 2026-08-09
Candidate: Architecture review candidate 2, workspace view derivation
Review state: closed-world Cycle 2 complete; bounded orchestrator correction recorded
Related authorities:

- `docs/dev/plans/0012-2026-05-31-workspace-inspection-pane-app-intelligence-roadmap.md`
- `docs/dev/plans/0045-2026-06-23-remote-view-architecture-consolidation-plan.md`

## Objective

Create one deep, in-process workspace view projection module whose small
interface determines the effective browser, stream choices, selected stream,
route URL, tab, view and control capability, readiness, recovery, and tile
ordering used by every dashboard caller.

The module must preserve canonical service authority, explicit operator stream
preference, route and provider fallback, foreign CDP read-only lifecycle
semantics, selected-target recovery, and the existing asynchronous viewport
controller contract. The refactor is complete only when competing stream score,
readiness parsing, browser merge, and primary-stream selection paths are
deleted rather than layered under another helper.

## Vocabulary And Authority

No `CONTEXT.md` or `docs/adr/` tree exists in the current worktree. This plan
therefore uses the established domain language in P12, P45, and the current
dashboard source:

- selected workspace;
- canonical inventory;
- service-owned browser;
- detected non-owned browser;
- view stream;
- remote route;
- operator-visible proof;
- viewport readiness;
- explicit operator preference.

Architecture terms in this plan have their codebase-design meanings:

- the **module** is the complete workspace view projection implementation;
- its **interface** is the single projection function and the facts a caller
  must supply or may consume;
- its external **seam** is the projection function imported by dashboard
  callers;
- private source normalization, authority merge, ranking, URL selection, tab
  recovery, and readiness derivation are **internal seams**;
- **depth** is earned when all of those decisions can be exercised through the
  one interface;
- **leverage** comes from one decision path serving workspace nodes, selected
  context, the viewport, tile mode, and the Service inspector;
- **locality** means a stream-selection or readiness correction changes one
  module and one behavior-focused test matrix.

Graphiti discovery against `agent_browser_main` was healthy but returned no
direct P99 or workspace-projection decision. It did surface prior route-bound,
foreign-CDP, and stale-target plans as advisory leads. P12, P45, current source,
and current tests are authoritative for this plan.

## Current Evidence

CodeGraph was healthy at planning time with 419 indexed files, 14,341 nodes,
and 43,350 edges. There were no pending-file warnings for the dashboard files
used here. CodeGraph intentionally skips the oversized Rust
`cli/src/native/actions.rs`; that file is outside this dashboard-only plan.

The current dashboard view path is spread across these files:

- `packages/dashboard/src/lib/service-workspaces.ts`, 3,144 lines;
- `packages/dashboard/src/lib/service-view-streams.ts`, 259 lines;
- `packages/dashboard/src/lib/workspace-view-stream-selection.ts`, 104 lines;
- `packages/dashboard/src/lib/workspace-browser-selection.ts`, 116 lines;
- `packages/dashboard/src/lib/selected-workspace-context.ts`, 481 lines;
- `packages/dashboard/src/lib/workspace-viewport-state.ts`, 406 lines;
- `packages/dashboard/src/lib/workspace-viewport-controller.ts`, 166 lines;
- `packages/dashboard/src/components/workspace-remote-viewport.tsx`, 2,913
  lines;
- `packages/dashboard/src/components/service-panel.tsx`, 8,692 lines.

Observed competing answers:

1. `service-workspaces.ts` has a private `workspaceViewStreamScore` that gives
   URL presence, non-CDP providers, route evidence, display isolation,
   provider mode, control, and readiness independent weights.
2. `workspace-view-stream-selection.ts` exports another
   `workspaceViewStreamScore`. It uses `canOpenViewStream` as an 80-point gate,
   then applies similar but not identical route, display, provider, control,
   and readiness weights.
3. `service-panel.tsx` bypasses both scores. Its
   `browserPrimaryViewStream` picks the first embeddable stream, then the first
   stream, even when that differs from workspace-node and viewport selection.
4. `service-workspaces.ts` and `service-view-streams.ts` each implement a
   private recursive `readinessState`. Their callers differ on whether
   attachability, display-content proof, or open capability supplies the
   effective readiness label.
5. `workspace-viewport-state.ts` separately compacts structured readiness and
   maps it to blocking, action-required, checking, or ready viewport outcomes.
   That richer transient interpretation is useful, but it is assembled by the
   viewport after another module already selected a stream.
6. `workspace-remote-viewport.tsx` locally reconstructs a daemon browser, a
   selected-context browser, service and daemon session identity, source merge,
   primary stream, tile order, route sharing, selected tab, stale-tab recovery,
   route URL, view/control capability, and readiness input.
7. `selected-workspace-context.ts` exposes the canonical node stream, while the
   viewport fetches service status again and may choose another stream after
   merging service, selected-context, and daemon candidates.
8. `serviceBrowserForWorkspaceSelection` and
   `serviceViewStreamForSelectedWorkspaceContext` are shallow caller-facing
   helpers. Their outputs still require each caller to know how to merge,
   rank, choose, and interpret a view.

The current tests encode important behavior, but the behavior is divided among
direct tests of shallow helpers and source-text regular expressions:

- `scripts/test-dashboard-view-streams.js`, 1,082 lines;
- `scripts/test-dashboard-workspace-nodes.js`, 1,760 lines;
- `scripts/test-workspace-viewport-controller.js`, 188 lines;
- Service inspector and browser-row assertions in
  `scripts/test-dashboard-browser-table.js` and
  `scripts/test-dashboard-inspector-actions.js`.

This split allows each isolated helper to pass while two rendering callers can
still disagree on the chosen stream or readiness.

## Deletion Test

Deleting the proposed projection module after migration would force at least
four callers to recreate all of the following:

- service, selected-context, and daemon source precedence;
- source identity and deduplication;
- canonical versus fallback stream authority;
- persisted and route-provided operator preference;
- automatic ranking within an authority tier;
- local versus public route URL selection;
- selected-tab lookup and stale-target recovery;
- view-only versus controllable capability;
- service proof, structured readiness, and transient viewport readiness;
- repair action and tile ordering.

That complexity would reappear across the callers. The new module therefore
passes the deletion test and earns depth.

By contrast, deleting `workspace-view-stream-selection.ts` or
`workspace-browser-selection.ts` today mostly moves their short algorithms
back into `workspace-remote-viewport.tsx`. They do not concentrate the full
decision and remain shallow. The plan deletes them after migration.

## Frozen Semantics

The implementation must freeze these rules before moving call sites.

### Canonical authority

1. A canonical service browser and its canonical inventory classification are
   authoritative for ownership, browser health, lifecycle actionability, and
   whether a view is service-owned.
2. A route record may fill URL and route-descriptor fields omitted from a
   workspace node and may contribute provider or transport readiness evidence.
   It may not upgrade ownership, inventory class, lifecycle actionability,
   route-bound ownership, or operator-visible proof.
3. A selected workspace node may supply a manual-runtime RDP view when no
   service-owned browser exists. This remains detected non-owned or
   manual-runtime inventory.
4. A daemon projection may add an alternate view source only when it shares a
   daemon-session identity with the canonical browser, or when there is no
   canonical service browser.
5. Automatic fallback must not let an incidental daemon CDP stream override a
   canonical service-owned route stream. If the canonical route stream is
   blocked, automatic selection presents that blocker and its repair path.
6. An explicit operator preference may select another currently available
   source, including CDP, without changing lifecycle ownership or canonical
   inventory classification.

### Source-authority ledger

Presentation stream selection must not be an input to lifecycle, inventory,
proof, route-ownership, or lifecycle-action classification. Before the
projection runs, `service-workspaces.ts` must classify raw canonical Service
Status records into an explicit ledger:

```typescript
export type WorkspaceViewAuthorityLedger = Readonly<
  Record<string, WorkspaceViewAuthorityEntry>
>;

export type WorkspaceViewAuthorityEntry = {
  subjectKey: string;
  authoritySource: "canonical-inventory" | "service-status-compatibility" | "daemon-detection";
  browserId: string | null;
  workspaceId: string | null;
  inventoryClass: WorkspaceInventoryClass;
  inventoryPlacement: WorkspaceInventoryPlacement;
  lifecycle: {
    state: WorkspaceNodeState;
    live: boolean;
    retained: boolean;
    health: string | null;
  };
  routeBoundOwnership: WorkspaceRouteBoundOwnership | null;
  operatorVisibleProof: {
    state: string;
    reason: string | null;
    routeId: string | null;
    displayAllocationId: string | null;
  } | null;
  lifecycleActions: readonly WorkspaceNodeAction[];
  presentationActionCeilings: {
    view: { allowed: boolean; reason: string | null };
    control: { allowed: boolean; reason: string | null };
    stream: { allowed: boolean; reason: string | null };
    screenshot: { allowed: boolean; reason: string | null };
  };
  diagnostics: readonly WorkspaceOwnershipDiagnostic[];
};
```

The projection receives this ledger as a required input for every canonical
service browser and as an explicit detected or manual authority entry for every
daemon-only or selected-context subject. An absent ledger entry fails closed to
a diagnostic, action-disabled projection. The module may not infer a ledger
entry from stream URL, provider, `readOnly`, control input, embed capability,
readiness, or route shape.

The compatibility classifier is upstream of the projection. It reads raw
Service Status browser, session, allocation, route-bound ownership,
operator-visible proof, incident, job, and explicit inventory fields. It may
pass through a canonical inventory record when the service supplies one. Until
that record exists for every row, it may retain the current compatibility
classification only after removing all dependencies on selected presentation
stream capability. The compatibility classifier must never read the
projection result.

Projection output preserves, byte-for-byte where structurally possible, the
ledger's inventory class, placement, lifecycle, ownership, proof, lifecycle
actions, presentation action ceilings, and diagnostics. Presentation facts may
only reduce View, Control, Stream, or Screenshot availability. Their final
enabled state is the intersection of the ledger ceiling and projected
capability. A projection may disable a presentation action because a URL,
provider, readiness, or control input is unavailable; it may never enable a
presentation action whose ceiling is disabled, add or enable a lifecycle action,
or upgrade a diagnostic, retained, detected, or view-only class.

The exact migration call order is:

1. Read one raw canonical Service Status snapshot.
2. Classify its raw lifecycle, inventory, proof, route-ownership, action, and
   diagnostic facts into `WorkspaceViewAuthorityLedger` without calling the
   projection and without selecting a presentation stream.
3. Call `projectWorkspaceViews` with the raw source snapshot, the ledger, and
   the caller intent.
4. Create workspace nodes by copying authority fields from the ledger and
   presentation fields from the projection.
5. Derive presentation labels and View or Control disabled reasons from the
   projection, intersected with ledger presentation action ceilings. Copy
   lifecycle actions without projection changes.
6. Build selected workspace context from those nodes and the same projection
   result.

As this order lands, delete the current `workspaceState` and
`browserInventoryClass` dependencies on `viewStream.embeddable`,
`viewStream.controllable`, or the selected stream. Do not retain the old path
as an earlier or later fallback. Any genuinely lifecycle-specific behavior
from those functions moves into the upstream compatibility classifier and is
tested through raw authority fixtures, not through stream selection.

### Preference and identity

1. The `view-provider` route value remains the highest explicit preference for
   the selected browser only. It is never broadcast across tile candidates.
2. The persisted browser-specific preference remains second for the selected
   browser and is the only preference input used for each tile candidate. It
   continues to
   use `agent-browser.workspace-view-stream-preferences.v1`.
3. An explicit preference wins while its stream remains available.
4. A missing or stale preference falls back deterministically and does not
   delete the underlying preference during this plan.
5. Existing `id:<id>` and provider, route, index keys remain recognized so
   stored preferences survive migration. If a more stable alias is added, the
   projection must accept both old and new keys before writing a new versioned
   storage contract.
6. Automatic ranking happens only inside the highest available authority tier.
   Within that tier it preserves ready, operator-visible RDP and private pooled
   route preference, control capability, route identity, display allocation,
   and provider-mode ordering encoded by current fixtures.
7. In tile mode, every browser resolves only its own persisted preference key.
   A selected-browser route provider may affect the optional selected
   projection, but it must not affect `tiles`, tile route-sharing counts, or
   tile order.

### Foreign CDP

1. Detected or `foreign_cdp` daemon sessions project as `cdp_snapshot` with
   read-only lifecycle ownership.
2. Snapshot, inspect, and screenshot remain available when current policy and
   readiness allow them.
3. A Borrow grant may temporarily enable pointer, keyboard, and wheel input for
   the selected target. It must not change inventory class, lifecycle
   ownership, close capability, or adoption state.
4. No source merge may turn a foreign stream into a service-owned controllable
   browser.

### Route and provider fallback

1. A local dashboard prefers `localEmbedUrl`, then other embeddable route
   fields.
2. A public dashboard must not embed a loopback-only route when a public or
   dashboard embed URL exists.
3. Explicit provider preference applies only to available choices.
4. `providerFallback` and retained RDP routes remain best-effort evidence, not
   proof of normal managed-browser control.
5. Route sharing is computed from the effective route identity after stream
   selection.

### Readiness

Readiness precedence must be explicit and source-preserving:

1. canonical inventory and operator-visible proof gate service ownership and
   actionability;
2. blocking display-content and structured remote-readiness evidence gate
   opening;
3. provider URL and embed capability gate rendering;
4. control input and read-only state gate control;
5. attachability recommends reattach or route-switch recovery but does not
   upgrade a current stream to ready;
6. viewport preflight, frame ownership, focus, takeover, and stale-target state
   produce viewer-local transient readiness after the effective stream is
   selected.

The output must retain the decisive source, reason, and suggested action so a
caller never has to parse raw readiness again.

### Async viewport lifecycle

`workspace-viewport-controller.ts` remains a separate deep module. It owns
target-token correlation and rejects stale preflight, frame, and recovery
events. The projection consumes its current state as transient input. It does
not absorb fetch effects, controller dispatch, Borrow mutation, focus requests,
or frame events.

## Proposed Deep Module

Add:

`packages/dashboard/src/lib/workspace-view-projection.ts`

The dependency is in-process pure computation. No adapter or injected port is
justified. One adapter would create a hypothetical seam, and there is no remote
or substitutable dependency in the decision path.

### External seam

Expose one function:

```typescript
export function projectWorkspaceViews(
  input: WorkspaceViewProjectionInput,
): WorkspaceViewProjection;
```

The stable input is grouped into three records so callers do not learn private
ordering details:

```typescript
export type WorkspaceViewProjectionInput = {
  sources: WorkspaceViewSources;
  authorityLedger: WorkspaceViewAuthorityLedger;
  intent: WorkspaceViewIntent;
  transient?: WorkspaceViewTransientState | null;
};

export type WorkspaceViewSources = {
  serviceBrowsers?: WorkspaceViewBrowserSource[];
  serviceTabs?: WorkspaceViewTabSource[];
  remoteViewRoutes?: Record<string, ServiceViewStream>;
  daemonSessions?: WorkspaceViewDaemonSource[];
  selectedContext?: WorkspaceViewSelectedContextSource | null;
};

export type WorkspaceViewIntent = {
  selection?: DashboardWorkspaceUrlSelection | null;
  mode: "view" | "control" | "tile" | "inspect";
  dashboardHref?: string | null;
  preferences?: WorkspaceViewPreferenceScope;
  tileLimit?: number;
};

export type WorkspaceViewPreferenceScope = {
  selected?: {
    subjectKey: string;
    provider?: string | null;
    streamKey?: string | null;
  } | null;
  byBrowserId?: Readonly<
    Record<string, { streamKey?: string | null }>
  >;
};
```

The route-derived `view-provider` value is represented only by
`preferences.selected.provider` and must include the resolved selected
`subjectKey`. Persisted browser preferences populate `byBrowserId`. When
building tiles, the module ignores the entire `selected` preference record and
uses only each tile browser's entry in `byBrowserId`.

`WorkspaceViewTransientState` carries current preflight, frame issue, focus,
takeover, recovery, and optional foreign Borrow facts. It contains observations
only and cannot alter source ownership.

Return one immutable projection:

```typescript
export type WorkspaceViewProjection = {
  selected: ProjectedWorkspaceView | null;
  candidates: readonly ProjectedWorkspaceView[];
  tiles: readonly ProjectedWorkspaceView[];
};
```

Each `ProjectedWorkspaceView` contains:

- stable browser, workspace, session, and source identity;
- the exact canonical inventory class, placement, lifecycle, ownership,
  proof, lifecycle actions, presentation action ceilings, diagnostics, and
  authority source copied from its ledger entry;
- stream choices with stable preference keys and provenance;
- effective stream and the reason it won;
- selected tab and stale-target recovery evidence;
- frame and external URLs already resolved for the dashboard origin;
- route identity and shared-route state;
- view, control, embed, and foreign-lifecycle capability;
- normalized readiness, decisive evidence, recovery action, and presentation
  labels;
- viewport target facts needed by `workspace-viewport-controller.ts`.

The projection also reports `authoritySubjectKey` and
`authorityPreservation: "preserved" | "missing"`. A canonical subject with
`missing` authority is diagnostic and action-disabled. No output mode exists
for inferred or upgraded authority.

Callers may render or dispatch existing actions from these facts. They may not
rescore streams, parse readiness, restore route URLs, or infer ownership.

### Internal seams

Keep these implementation details private to the module:

1. source normalization;
2. session-identity matching;
3. canonical and fallback source merge;
4. stream identity and legacy preference aliases;
5. authority-tier selection and automatic ranking;
6. route descriptor and dashboard-origin URL resolution;
7. tab selection and stale-target recovery;
8. structured readiness normalization;
9. transient viewport readiness composition;
10. candidate and shared-route tile ordering.

Begin with one file so locality is obvious. If the implementation becomes hard
to navigate, split private implementation into a
`workspace-view-projection/` directory with one `index.ts` external seam and
unexported internal modules. Do not create caller-visible ports, strategy
objects, or one-function pass-through modules.

### Import direction

The module may import the transport-shaped `ServiceViewStream` type and URL
selection primitives. It must not import `service-workspaces.ts` or
`selected-workspace-context.ts`, because both become callers and that would
create a cycle.

Define the narrow source and authority-ledger types at the projection seam.
Move or re-export the existing authority string unions and record shapes so
`service-workspaces.ts` imports them from the projection module or a lower
type-only file; the projection must never import them from its caller. The type
names in the ledger sketch above describe the required structural values, not
permission for a reverse import. Current service browser, workspace node,
selected-context, tab, and daemon-session records should satisfy or be
translated to those structural types without JSON round trips.

## Caller Migration

### `use-workspace-view-preferences.ts`

- Add one dashboard-level preference controller used once by
  `DashboardExperience`. It owns reading and writing
  `agent-browser.workspace-view-stream-preferences.v1`, reading the selected
  route's `view-provider`, and publishing a monotonically changing revision
  whenever either source changes.
- Its immutable snapshot contains the selected provider value and the complete
  browser-keyed persisted preference map. It does not resolve a selected
  subject; the selected-context builder resolves that identity from the same
  source snapshot and constructs `preferences.selected.subjectKey`.
- Its write operation updates local storage and its in-memory snapshot in the
  same event, so same-document writes rebuild the projection without depending
  on the browser's cross-document `storage` event.
- `DashboardExperience` passes this snapshot into
  `useSelectedWorkspaceContext` and passes the controller's write operation to
  `WorkspaceRemoteViewport`. No child keeps an independent preference copy.

### `service-workspaces.ts`

- At the start of `deriveWorkspaceNodes`, classify the raw canonical snapshot
  into `WorkspaceViewAuthorityLedger` without selecting, scoring, or opening a
  presentation stream.
- Project browser views once after the ledger exists and index both authority
  entries and projected views by canonical subject key and browser ID.
- Use the projection's effective stream, readiness, route summary, and
  capability when creating browser workspace nodes.
- Copy lifecycle, inventory class, inventory placement, route-bound ownership,
  operator-visible proof, diagnostics, and action ceilings only from the
  ledger. Intersect projected View and Control capability with those action
  ceilings.
- Change `createBrowserWorkspaceNode`, `workspaceState`,
  `browserInventoryClass`, inventory placement, and browser action derivation
  in the same slice so none uses the selected presentation stream to create or
  upgrade an authority fact.
- Delete the private `workspaceViewStreamScore`, private recursive
  `readinessState`, and competing stream readiness label once the caller uses
  projection output.
- Delete the old stream-dependent state and class branches rather than keeping
  them as compatibility fallback.

### `use-selected-workspace-context.ts`

- Add `remoteViewRoutes?: Record<string, WorkspaceServiceViewStream>` to the
  hook's `ServiceStatusData.service_state` shape.
- Treat the parsed `/status` response as one immutable source snapshot for the
  selected-context memo. Pass `service_state.remoteViewRoutes` with browsers,
  sessions, tabs, jobs, incidents, allocations, manual browsers, and browser
  authority into selected workspace context construction.
- Accept the immutable dashboard-level preference snapshot as an explicit
  hook input. Include its revision in memo dependencies. Resolve the selected
  subject from the raw source snapshot, then construct the selected-subject
  preference plus per-browser tile preferences and build the authority ledger
  and complete projection from that same snapshot. Do not issue a second route
  fetch or look up a route later in the viewport.
- Include the route map in memo dependencies through the existing
  `serviceStatus` snapshot identity; do not split it into independently timed
  state.
- Extend `UseSelectedWorkspaceContextResult` with the complete immutable
  `WorkspaceViewProjection`, its source snapshot identity, and the existing
  refresh operation. `SelectedWorkspaceContext` carries the selected projected
  view for P12 consumers. The viewport receives the same full projection for
  candidates and tiles and does not need the raw route map or a second status
  snapshot.

### `selected-workspace-context.ts`

- Keep `SelectedWorkspaceContext` as the P12 shared context contract.
- Add `projectedView` and its source snapshot timestamp or identity. A node
  stream containing only `routeId` must be completed from the same snapshot's
  `remoteViewRoutes[routeId]` before the context is returned.
- Replace redundant `viewable`, `controllable`, and stream interpretation
  fields only after every P12 caller is migrated.
- Preserve current context fields during the compatibility slice so Chat,
  Console, Workspace evidence, and inspector callers do not break.
- Derive compatibility fields from the projection, not independently from the
  node stream.
- Do not retain `serviceViewStreamForSelectedWorkspaceContext` as a second
  route-restoration path.

### `workspace-remote-viewport.tsx`

- Replace local daemon browser construction, selected-context stream
  restoration, browser merge, primary stream, stream choices, tile ordering,
  route sharing, selected tab, URL resolution, and readiness assembly with one
  `projectWorkspaceViews` call.
- Receive the complete projection and preference write operation from
  `DashboardExperience`. Render `projection.selected` and `projection.tiles`
  from that one construction. Do not maintain a separately refreshed Service
  Status value for workspace projection, look up `selectedContextRouteId`, or
  rebuild the selected route descriptor. A user refresh invokes the shared
  selected-context refresh operation and keeps explicit viewer-local refresh
  state only.
- Remove `selectedContextRouteId`, `selectedContextRoute`, and the viewport-local
  `remoteViewRoutes[routeId]` lookup in the same slice.
- Continue to own viewer effects: refresh requests through the shared hook,
  route-listener subscription, preference writes through the shared
  controller, preflight fetch, focus and takeover requests, foreign Borrow
  actions, frame interaction, fullscreen, and rendering. It does not own a
  second preference store or status polling loop.
- Continue to dispatch projection target facts into
  `workspace-viewport-controller.ts`.
- Continue to write recovered selected-tab identity through the existing URL
  selection helper, but use recovery evidence from the projection.
- Model `view-provider` as a selected-subject preference and persisted keys as
  `byBrowserId`. The hook rebuilds the complete projection when the preference
  revision changes. Tile projection receives only the latter.

### `service-panel.tsx`

- Replace `browserPrimaryViewStream` with projection output for browser rows,
  the detail inspector, readiness strip, view, and control actions.
- Keep the current `inspect` intent free of selected-workspace preference unless
  the browser-specific stored preference is supplied explicitly.
- Ensure the Service inspector and workspace viewport report the same effective
  stream and readiness for the same source snapshot and intent.

### Shallow module retirement

- Delete `workspace-view-stream-selection.ts` after its last caller migrates.
- Delete `workspace-browser-selection.ts` after selection and context-source
  projection move behind the new seam.
- Reduce `service-view-streams.ts` to transport types and genuinely shared
  rendering labels only. Move readiness parsing, scoring, and choice semantics
  behind the projection seam.
- Retain `workspace-viewport-controller.ts` as the async lifecycle module.
- Retain only the readiness-compaction behavior needed by launcher eligibility
  outside workspace viewing. Move it to a correctly named shared readiness
  evidence module if leaving it in `workspace-viewport-state.ts` would falsely
  imply that launcher eligibility depends on viewport projection.

No compatibility wrapper may survive the final slice. A temporary wrapper must
be marked `TODO(P99-delete)` and delegate without changing semantics.

## Implementation Slices

### Slice 0 | Freeze projection behavior

Add a new behavior-focused fixture matrix before moving source.

Required cases:

- ready canonical RDP plus daemon CDP defaults to canonical RDP;
- blocked canonical RDP plus ready daemon CDP still presents the canonical
  blocker automatically;
- a ready-looking stream with URL, control input, and ready provider evidence
  crossed with canonical `service-owned-diagnostic-browser`, non-ready
  operator proof, or disabled ledger actions remains diagnostic and has View,
  Control, Close, Kill, and Add tab disabled as specified by the ledger;
- a missing authority-ledger entry fails closed even when the stream looks
  ready;
- explicit CDP preference selects available CDP without changing ownership;
- absent service stream permits a matched daemon fallback;
- selected manual-runtime RDP without a service browser remains viewable;
- foreign CDP remains lifecycle read-only before, during, and after Borrow;
- duplicate service and daemon streams retain the service copy;
- missing explicit preference falls back deterministically;
- old stored preference keys continue to resolve;
- private pooled display outranks shared/config routes inside one authority
  tier;
- local and public dashboards resolve the correct route URL;
- a selected-context node whose stream contains only `routeId` receives local,
  public, descriptor, and structured readiness fields from
  `remoteViewRoutes[routeId]` in the same hook snapshot; the returned context
  and viewport projection are equal without a viewport-local route lookup;
- blocking readiness arrays, nested readiness components, display-content
  blockers, and ready proof normalize consistently;
- attachability supplies recovery without upgrading readiness;
- stale selected target recovers to the best live nonblank tab;
- tile order, two-tile limit, and shared-route marking remain stable;
- a combined two-browser tile fixture supplies a selected-browser
  `view-provider` plus distinct persisted keys for both browsers and proves the
  route preference affects no tile, each tile uses only its browser key, and
  route-sharing counts and order remain unchanged;
- an integration fixture constructs the preference controller, selected
  context, and viewport projection together; changing the selected browser to
  an available alternate stream rebuilds the viewport effective stream while
  context and viewport retain identical route evidence, then changing one
  browser's persisted key changes only that browser's tile;
- Service inspector and workspace viewport intents select the same result from
  the same snapshot.

Exit gate: new tests fail for at least the known competing-answer cases before
the implementation is added.

### Slice 1 | Add the deep projection module

- Add the one external interface, required authority-ledger input, explicit
  preference scope, and private internal seams.
- Make all normalization pure and deterministic.
- Return provenance and selection reasons with every chosen stream and
  readiness result.
- Fail closed for a missing authority entry and preserve every authority field
  without upgrade.
- Add legacy preference aliases without changing the current local storage
  version.
- Add temporary `TODO(P99-delete)` wrappers only where required to keep the
  dashboard buildable between commits.

Exit gate: the projection fixture matrix passes through the external interface;
no test imports a private helper.

### Slice 2 | Migrate canonical node and selected context

- Add the dashboard-level preference controller and place its single instance
  in `DashboardExperience` before constructing selected workspace context.
- In `service-workspaces.ts`, classify the authority ledger from raw Service
  Status records, then project service browser views once during workspace-node
  derivation, then build nodes from ledger authority plus projected
  presentation.
- Remove score and readiness duplication from `service-workspaces.ts`.
- Remove every selected-stream dependency from lifecycle state, inventory
  class, inventory placement, proof, and lifecycle action classification.
- In `use-selected-workspace-context.ts`, add `remoteViewRoutes` to the fetched
  status shape and pass it from the same immutable status snapshot into context
  construction.
- Populate compatibility fields in `SelectedWorkspaceContext` from projection
  output and carry the complete projected selected view.
- Remove selected-context route restoration from
  `workspace-browser-selection.ts`; it must not survive as a wrapper.
- Prove workspace nodes, selected context, and canonical inventory placement
  remain unchanged except for fixtures that intentionally eliminate a
  competing answer.

Exit gate: workspace-node, hook status-shape, and selected-context tests pass;
the crossed authority fixture remains diagnostic; the route-ID-only context
and viewport agree; and no daemon or presentation stream can create or upgrade
canonical ownership.

### Slice 3 | Migrate viewport and tile mode

- Replace local view derivation in `workspace-remote-viewport.tsx` with one
  projection call.
- Preserve route listeners, preference persistence, URL replacement after
  stale-tab recovery, preflight effects, controller event correlation, foreign
  Borrow effects, and rendering.
- Remove migrated local helpers in the same slice.

Exit gate: view, control, tile, stale-target, public-route, foreign-CDP, and
controller regression tests pass.

### Slice 4 | Migrate Service inspector

- Use projection output for row actionability, detail readiness, view open, tab
  focus, and control open.
- Remove `browserPrimaryViewStream` and any local readiness reconstruction.
- Update structural rendering assertions to check observable labels and enabled
  actions rather than the deleted helper name.

Exit gate: browser table and inspector actions agree with projection fixtures.

### Slice 5 | Replace tests and delete compatibility

- Add `scripts/test-dashboard-workspace-view-projection.js` and
  `pnpm test:dashboard-workspace-view-projection`.
- Move merge, preference, authority, URL, tab, capability, and readiness cases
  out of direct tests of shallow modules and into projection-interface tests.
- Keep provider rendering primitive tests only where those primitives remain a
  real shared seam.
- Keep controller tests focused on stale async event rejection.
- Keep UI tests focused on rendered behavior and dispatched existing actions.
- Delete source-text assertions that require old helper names.
- Delete shallow modules and every `TODO(P99-delete)` wrapper after the final
  caller migrates.

Exit gate: the old files and duplicate symbols are absent, while all behavior
tests pass.

### Slice 6 | Documentation and durable closeout

- Update this plan with checkpoints, exact commits, validation receipts,
  accepted audit findings, and residual backlog.
- Update P12 Slice A architecture direction so the shared selected workspace
  context consumes P99 projection output.
- Update P45 Dashboard guidance and Slice F so canonical inventory flows
  through P99 rather than competing dashboard stream helpers.
- Add a dated `docs/dev/notes/` migration note only if the execution exposes a
  reusable semantic mismatch or compatibility lesson not already captured
  here.
- If visible labels, preference behavior, route behavior, or operator workflow
  change beyond the frozen semantics, update README, the agent skill, and the
  docs site in the same slice. If behavior remains an internal deepening,
  record explicitly that no user-facing documentation contract changed.
- Do not write Graphiti memory unless the orchestrator determines the closed,
  source-backed result is durable enough and the active authority permits a
  memory write.

Exit gate: P12, P45, P99, tests, and current source describe one projection
path.

## Test Replacement Strategy

The interface is the test surface. New tests assert complete observable
projections, not private functions.

The upstream compatibility authority classifier remains part of the existing
`deriveWorkspaceNodes` seam. Test the crossed raw-status authority case through
that interface, then pass its ledger entry through `projectWorkspaceViews` and
assert exact preservation plus presentation-action intersection. Do not import
the private classifier. Test same-snapshot selected context through
`buildSelectedWorkspaceContext` with the hook-shaped status sources, and keep a
narrow wiring guard that `useSelectedWorkspaceContext` passes
`remoteViewRoutes`.

Delete or migrate tests that import:

- `mergeWorkspaceViewStreams`;
- `selectWorkspaceViewStream`;
- `workspaceViewStreamScore`;
- `workspaceViewStreamKey` as an independent algorithm;
- `serviceBrowserForWorkspaceSelection`;
- `serviceViewStreamForSelectedWorkspaceContext`;
- `deriveWorkspaceViewportUxState` or
  `deriveWorkspaceViewportReadiness` when the behavior now belongs to the
  projection.

Keep `workspace-viewport-controller` tests because its target-token and stale
event behavior remains a separate real seam. Keep launcher eligibility tests
for launcher-specific interpretation; do not force launcher intent through the
workspace view interface.

Required targeted validation after the final migration:

```bash
pnpm test:dashboard-workspace-view-projection
pnpm test:dashboard-view-streams
pnpm test:dashboard-workspace-nodes
pnpm test:dashboard-selected-workspace-context
pnpm test:dashboard-workspace-navigator
pnpm test:dashboard-workspace-inspector-tab
pnpm test:dashboard-inspector-actions
pnpm test:dashboard-browser-table
pnpm test:dashboard-browser-row-actions-render
pnpm test:workspace-viewport-controller
pnpm test:cross-seam-interlocks
pnpm build:dashboard
pnpm validation:select -- --base <last-green-commit>
```

The executor must run every check selected by the final
`validation:select` result for all touched files since the last known green
commit. No Rust gate is implied unless execution changes Rust or a cross-language
contract.

## Acceptance Criteria

1. One `projectWorkspaceViews` interface is the test surface for workspace view
   source merge, selection, URL, tab, capability, readiness, recovery, and tile
   projection.
2. Raw canonical Service Status records are classified into an explicit
   source-authority ledger before projection; lifecycle, inventory, proof,
   route ownership, diagnostics, and lifecycle actions do not depend on a
   selected presentation stream.
3. Projection output exactly preserves its ledger authority and may only
   reduce action availability. Ready-looking presentation facts cannot upgrade
   diagnostic authority or enable a ledger-disabled action.
4. `service-workspaces.ts`, `use-selected-workspace-context.ts`,
   `selected-workspace-context.ts`, `workspace-remote-viewport.tsx`, and
   `service-panel.tsx` consume the same-snapshot ledger or projection output
   instead of reconstructing answers.
5. The two current `workspaceViewStreamScore` implementations are gone.
6. Competing private `readinessState` implementations are gone from
   `service-workspaces.ts` and `service-view-streams.ts`.
7. `browserPrimaryViewStream`, viewport-local browser merge, and direct
   selected-context route restoration are gone.
8. `workspace-view-stream-selection.ts` and
   `workspace-browser-selection.ts` are deleted, with no compatibility wrapper
   left behind.
9. Automatic selection never lets daemon CDP override a canonical
   service-owned route stream.
10. Explicit operator preference still selects an available alternate stream
    and survives source reordering and migration from current stored keys.
    `view-provider` is selected-browser-only, while tiles use only each
    browser's persisted preference key. A same-document preference write
    rebuilds selected context and viewport from the same controller revision;
    no child preference copy or stale precomputed projection survives.
11. A selected manual-runtime RDP route without a service browser remains
   viewable when its canonical selected-workspace evidence permits it.
12. Foreign CDP remains detected non-owned and lifecycle read-only; Borrow
    changes only the allowed transient input operations.
13. Local and public dashboard origins resolve route URLs without embedding an
    avoidable loopback URL on public ingress.
14. A route-ID-only selected-context stream is completed from
    `remoteViewRoutes` in the same hook snapshot, and context and viewport
    expose the same descriptor and readiness without a viewport-local route
    lookup.
15. Canonical operator proof, structured readiness, display blockers,
    attachability recovery, and transient viewport state produce one
    source-backed readiness result.
16. Stale target recovery, tile ordering, shared-route marking, and controller
    stale-event rejection remain covered.
17. New behavior tests cross only the projection interface, and obsolete
    shallow-module tests are removed rather than duplicated.
18. P12, P45, and P99 are updated with the final architecture and current
    validation evidence.
19. Every targeted and selected validation command passes, or a concrete
    unrelated baseline failure is recorded with evidence and adjudicated by
    the orchestrator.

## Guard Searches

The final worker should run literal guard searches in addition to tests:

```bash
rg -n "function workspaceViewStreamScore|function readinessState|function browserPrimaryViewStream|TODO\(P99-delete\)" packages/dashboard/src
rg -n "workspace-view-stream-selection|workspace-browser-selection" packages/dashboard/src scripts package.json
rg -n "selectedContextRouteId|selectedContextRoute|remoteViewRoutes\?\.\[selectedContextRouteId\]" packages/dashboard/src/components/workspace-remote-viewport.tsx
rg -n "remoteViewRoutes" packages/dashboard/src/hooks/use-selected-workspace-context.ts packages/dashboard/src/lib/selected-workspace-context.ts
```

The first command must return no P99-owned duplicates or wrappers. The second
must return no imports, tests, or scripts for deleted modules. Any intentionally
retained generic readiness parser must have a non-viewport name, a distinct
caller contract, and an explicit disposition in the P99 closeout.
The third command must return no viewport-local selected-context route lookup.
The fourth must prove the hook and context builder carry the route map from the
same status snapshot; tests remain the behavioral proof that the field is used
correctly.

## Risks And Controls

### New monolith risk

Moving helpers into one large file without shrinking the interface would not
create depth. Control this by keeping one external projection function,
structural source inputs, immutable output, and private internal seams. Split
private implementation only for locality, never to expose more caller choices.

### Import-cycle risk

`service-workspaces.ts` and `selected-workspace-context.ts` must be callers, not
dependencies, of the projection. Keep shared transport types below the seam and
use structural inputs.

### Canonical-authority regression

Merging a ready daemon stream with a blocked service route can accidentally
hide the canonical blocker. Building the ledger after projection would make
presentation capability an ownership engine; building it before projection
while retaining the old stream-dependent class path would layer competing
answers. Freeze crossed authority tests first, enforce the documented call
order, and delete old class and state dependencies in Slice 2.

### Same-snapshot route risk

A route-ID-only selected context loses public URL or structured readiness when
the hook omits `remoteViewRoutes`, while a viewport-local lookup can silently
use a newer snapshot. Carry routes into context construction from the same
parsed status object and render the completed projected view without a second
selected-route lookup.

### Preference migration risk

Current fallback keys include a sorted index and may shift as choices change.
Accept legacy aliases, keep the current storage key, and add a new persisted
version only through an explicit compatibility slice. Keep route-derived
provider preference in the selected-subject record and keep tile preference in
the per-browser map so a scalar preference cannot broadcast across tiles.

### Readiness flattening risk

A single label can erase the evidence source or suggest a false repair. Return
structured decisive evidence and recovery, then derive labels from that result.

### Async race risk

Projection purity does not replace target-token event correlation. Preserve the
viewport controller and its stale-event tests unchanged until the projection
callers are stable.

### Source-text test risk

Several dashboard tests assert helper names and source layout. Replace P99-owned
assertions with observable projection or rendered-action behavior in the same
slice that removes the helper.

### Installed-runtime drift

This plan does not change the service status wire contract. If execution
discovers that a wire change is required, stop that slice, revise P99, and add
service/client compatibility validation before proceeding.

## Non-Goals

- no Rust `actions.rs` refactor;
- no remote-view open, lease, proof, or inventory wire-contract redesign;
- no new service request action;
- no dashboard polling or fetch-cache redesign;
- no Guacamole, RDP, CDP, or provider implementation change;
- no new browser lifecycle ownership for foreign CDP;
- no local storage contract version change unless current keys cannot be
  preserved;
- no visual redesign of the workspace viewport or Service inspector;
- no formal release, installed-runtime replacement, live browser mutation, or
  tenant operation.

## Bounded Review And Delegation Workflow

The orchestrator owns finding disposition and integration. Review is bounded at
two cycles for plan quality and two cycles for implementation quality.

### Plan review

1. One fresh plan auditor performs `drift_discovery` against this objective,
   frozen semantics, evidence, interface, slices, and acceptance criteria.
2. The orchestrator classifies each finding as `blocking`,
   `nonblocking_backlog`, `rejected`, or `needs_evidence`.
3. The author or orchestrator performs one consolidated remediation pass for
   accepted blocking findings.
4. The same or a fresh auditor performs one `closed_world` verification limited
   to accepted blocking finding IDs and critical regressions introduced by the
   revision.
5. Remaining nonblocking concerns are logged in P99 and implementation starts.
   No third broad plan audit is allowed.

### Implementation review

1. One implementation worker executes the audited plan.
2. One fresh work auditor performs `drift_discovery` against the frozen P99
   acceptance criteria and target commit or worktree state.
3. The orchestrator adjudicates findings and authorizes one consolidated
   remediation pass for accepted blockers.
4. One `closed_world` verification checks only accepted finding IDs and
   critical regressions from remediation.
5. Remaining nonblocking concerns are logged in P99. No third broad work audit
   is allowed.
6. One independent test worker runs the full targeted and selected validation
   packet against the reconciled work. The test worker reports commands, exit
   status, exact failures, and target identity. It does not expand architecture
   scope.

An accepted blocking finding that still fails the second closed-world review
must be split into a bounded repair plan or reported as a blocker. It must not
restart broad discovery.

## Cycle 1 Plan Audit Adjudication

Audit authority:
`docs/dev/notes/0099-2026-08-09-workspace-view-projection-plan-audit.md`

| Finding | Auditor recommendation | Orchestrator disposition | Plan version 2 remediation | Cycle 2 check |
| --- | --- | --- | --- | --- |
| `P0099-A1-01` | `blocking` | `blocking`, accepted | Added the required source-authority ledger, fail-closed missing-authority behavior, exact raw-status to ledger to projection to node call order, authority-preserving output rule, deletion of stream-dependent class and state paths, crossed diagnostic-authority fixtures, acceptance criteria, and risk control. | Verify ledger input is explicit, no authority fact can be manufactured or upgraded by presentation, old derivations are scheduled for deletion rather than layering, and the crossed fixture is mandatory. |
| `P0099-A1-02` | `blocking` | `blocking`, accepted | Added `use-selected-workspace-context.ts` and its status shape to migration scope, same-snapshot `remoteViewRoutes` plumbing, complete projected view on selected context, deletion of viewport-local route lookup, route-ID-only descriptor and readiness fixture, acceptance criterion, guard search, and risk control. | Verify the hook, context, and viewport use one route-bearing snapshot and the viewport-local selected-route lookup is explicitly removed. |
| `P0099-A1-03` | `needs_evidence` | `blocking design clarification`, accepted | Froze current semantics: route `view-provider` applies only to the selected subject; tile views use only browser-specific persisted keys. Replaced scalar preference input with explicit selected and per-browser scopes and added the combined two-browser tile fixture. | Verify no scalar provider preference can broadcast across tiles and the combined fixture covers choices, sharing counts, and order. |

Cycle 1 has no nonblocking backlog or rejected finding. Plan version 2 is the
single allowed remediation pass. Cycle 2 must use `closed_world` mode and check
only `P0099-A1-01`, `P0099-A1-02`, `P0099-A1-03`, and critical regressions
introduced by this revision. It must not reopen broad discovery.

## Cycle 2 Residual And Bounded Orchestrator Resolution

Cycle 2 passed `P0099-A1-01` and `P0099-A1-02`. It retained one blocking
integration contradiction under `P0099-A1-03`: selected context precomputed a
complete projection without receiving the preference state still owned by the
viewport.

The orchestrator resolved that closed finding without a third audit:

1. One `use-workspace-view-preferences.ts` controller is instantiated by
   `DashboardExperience`, above both selected-context construction and viewport
   rendering.
2. The controller snapshot owns the selected URL provider, complete persisted
   browser-keyed preferences, and a same-document revision. Its write operation
   updates storage and the snapshot atomically.
3. `useSelectedWorkspaceContext` receives that snapshot, resolves the selected
   subject from the same raw status snapshot, and builds the complete selected,
   candidate, and tile projection once.
4. The parent passes that projection and the controller write operation to the
   viewport. The viewport has no second status snapshot or preference store for
   projection and cannot rebuild route evidence.
5. The crossed integration fixture proves an alternate selected preference
   changes the effective viewport stream without changing route evidence, and
   a browser-keyed preference changes only its matching tile.

This is the bounded repair for the terminal Cycle 2 finding, not another
review cycle. The finding and consequence remain recorded in the audit note.
Root verification against the current `DashboardExperience` caller topology
confirms the controller can be constructed above the hook and both desktop and
mobile viewport call sites. Plan 0099 may proceed to implementation with no
further plan audit.

## Planning Delegation Receipt

- delegation decision: `spawned`;
- orchestrator: `/root`;
- planning worker handle: `/root/plan_workspace_view`;
- bounded lane: deep analysis and implementation-ready plan for workspace view
  projection only;
- write scope: this P99 file only;
- prohibited scope: source edits, other plan edits, commits, live state;
- runtime status at receipt close: `completed` when the worker returns its
  terminal announce after writing this artifact;
- evidence returned: CodeGraph structural readback, current plan authorities,
  current dashboard source, current test surfaces, file sizes, deletion test,
  proposed external seam, migration slices, validation packet, and risks;
- primary reconciliation: Cycle 1 findings `P0099-A1-01`, `P0099-A1-02`, and
  `P0099-A1-03` accepted; Cycle 2 passed the first two and the terminal
  preference-construction contradiction is closed by the bounded orchestrator
  resolution above, with no Cycle 3;
- optimization: balanced quality and wall-clock time, with independent plan,
  implementation, review, and test roles requested by the operator;
- timeout and stop condition: stop after this plan is written; do not edit
  implementation or extend discovery into `actions.rs`;
- transcript path or session ID: not exposed by the collaboration runtime;
  the durable runtime handle is `/root/plan_workspace_view`.

## Done Definition

P99 is complete only when:

- every acceptance criterion has current source or validation evidence;
- the projection passes the deletion test in the resulting architecture;
- old shallow modules, duplicate algorithms, and temporary wrappers are gone;
- canonical authority and operator preference are both proven through the new
  interface;
- work audit has completed its bounded two-cycle process;
- the independent test worker has returned a passing or fully adjudicated
  validation receipt;
- P12, P45, and P99 agree on the final workspace view path.
