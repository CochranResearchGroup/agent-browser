# Remote View Architecture Consolidation Plan

Date: 2026-06-23
State: PLANNED
Lane: P45
Depends On:
- `docs/dev/plans/0041-2026-06-22-foreign-cdp-browser-discovery-and-control-plan.md`
- `docs/dev/plans/0043-2026-06-22-route-handoff-confusion-audit-plan.md`
- `docs/dev/plans/0044-2026-06-22-rdp-browser-deterministic-refactor-plan.md`
- `docs/dev/notes/2026-06-22-facebook-remote-view-open-friction.md`
- `docs/dev/notes/2026-06-22-rdp-browser-determinism-audit.md`

## Purpose

Refactor the remote-view code so it is easy to explain, easy to test, and hard
to wire incorrectly.

P44 is repairing the observed behavior: terminal-only routes, stale target
URLs, weak proof, route-pool recovery, and dashboard grouping. P45 should make
those repairs durable by consolidating the route-bound browser flow into one
small architecture with explicit ownership boundaries.

The target explanation should fit in one paragraph:

Agent Browser accepts an operator-visible browser intent, plans route-bound
acquisition without mutation, reserves a route/display/browser lease, opens or
reuses one tab, proves the selected browser is visible through the selected
operator route, then publishes one canonical inventory record. Foreign CDP
browsers are discovered separately as non-owned addressable inventory and are
never treated as service-owned route-bound browsers unless explicitly adopted.

## Audit Finding

The recent failures were not caused by one missing check. They came from
several code paths each holding a partial interpretation of the same thing:

- `remote_view_open` orchestration, proof, lease rollback, launch command
  shaping, and route checkout logic still live together in `actions.rs`.
- `remote_view.rs` now owns useful intent and plan types, but it is still a
  facade over data and decisions that other modules can reinterpret.
- Route-pool entries, remote-view routes, display allocations, browsers, tabs,
  streams, jobs, incidents, and Guacamole records are all individually valid,
  but there is not one canonical aggregate that represents an operator-visible
  route-bound browser.
- Dashboard grouping now has better vocabulary, but the left rail, selected
  context, viewport, and stream helpers can still rederive actionability from
  row shape, URL params, or stream presence.
- Foreign CDP browser discovery and service-owned route acquisition still share
  enough presentation surface that a non-owned browser, stale daemon stream, or
  terminal-only display can look like a live control target.

P45 should remove the overlap rather than add more local conditionals.

## Design Rule

Every important question should have exactly one answering module:

- What did the caller ask for?
  - `remote_view::intent`
- What should happen before mutation?
  - `remote_view::planner`
- What service state is reserved, finalized, or rolled back?
  - `remote_view::lease`
- Is the operator-visible browser actually visible and correct?
  - `remote_view::proof`
- What record should the dashboard show?
  - `remote_view::inventory` in Rust, consumed by dashboard TypeScript
- What is foreign and addressable but not owned?
  - `foreign_cdp`
- What is stale history or diagnostics?
  - incident and activity log surfaces, not the live control rail

If a second module needs the same answer, it should consume the canonical
record instead of reconstructing it.

The refactor should optimize for local explainability over clever reuse. A new
engineer should be able to trace a route-bound open by reading these five
modules in order: intent, planner, lease, proof, inventory. Any branch that
cannot be named in one of those modules is probably a fallback path that should
be deleted, moved behind an explicit repair action, or converted into a typed
blocker.

P45 should also keep one compatibility rule in force during migration: the
installed binary, source checkout binary, HTTP service, MCP server, generated
client, and dashboard must agree on the same contracts. A feature is not
considered refactored until the older binary can either consume the new record
through a documented compatibility shim or reject it with a typed version or
capability error. Silent fallback to stale row shape, stale URL params, or a
daemon CDP stream is a refactor failure.

## Refactor Shape

The refactor should produce a boring call graph:

```text
remote_view::open
  -> intent::normalize
  -> capability::read_envelope
  -> planner::plan
  -> lease::reserve
  -> route_desktop::prepare
  -> browser_session::launch_or_attach
  -> tab::acquire
  -> tab::prove_selected_target
  -> proof::build
  -> lease::finalize_or_rollback
  -> inventory::publish
```

`actions.rs` should not know how route-pool fallback works, how proof states
are selected, how stale target recovery is classified, or which dashboard class
is live. Its job should be command dispatch, repository loading, calling the
coordinator, and serializing the response.

Each extracted module should be small enough to explain in one sentence:

- `intent` says what the caller asked for.
- `capability` says whether the installed runtime can speak the needed
  contracts.
- `planner` says what will happen without mutating state.
- `lease` says what is owned, pending, finalized, released, or rolled back.
- `route_desktop` says whether the selected RDP desktop is browser-ready.
- `browser_session` says whether one compatible browser process exists.
- `tab` says which target is selected and why.
- `target_readiness` says whether the selected target reached the requested
  navigation state.
- `proof` says whether the selected target is operator-visible.
- `inventory` says what the UI may render and which actions are allowed.
- `foreign_cdp` says what is reachable but not owned.

No module should return loosely shaped JSON as its primary internal API. JSON is
the wire format at the edges. Inside the Rust path, use typed structs and enums,
then convert to JSON at service, MCP, HTTP, CLI, and dashboard compatibility
boundaries.

The desired dependency direction is one-way:

```text
intent -> planner -> lease -> route_desktop/browser_session/tab -> proof -> inventory
```

`foreign_cdp` feeds inventory, but it must not feed lease or proof unless a
future explicit adoption flow creates service ownership first. Dashboard code
may render inventory and request actions; it must not reconstruct lease,
route-binding, proof, or ownership answers from URL params or stream URLs.

## Audit To Refactor Map

The audit found route confusion because several surfaces can currently answer
the same question differently. P45 should make each audit finding disappear by
moving the answer to one module and adding a guard that prevents reintroduction.

- Direct remote-headed launch can look like an operator route handoff.
  Canonical owner: `remote_view::intent`.
  Refactor outcome: route-bound opens reject or classify direct launch surfaces
  instead of falling through to them.
  Guard: parser and service-request fixtures for route-bound intent.
- Stale route-pool, display, browser, and target records can outrank fresh
  request evidence.
  Canonical owner: `remote_view::planner`.
  Refactor outcome: planning is no-mutation, request-scoped, and names every
  reuse or rejection reason.
  Guard: planner fixtures for stale retained state and request-scoped route
  pools.
- Partial acquisition leaves rows that the dashboard treats as live.
  Canonical owner: `remote_view::lease`.
  Refactor outcome: route, display, browser, and tab state advance together or
  roll back together.
  Guard: lease transition tests and forced-proof failure smoke.
- A terminal-only or terminal-topmost display can be mistaken for a browser.
  Canonical owner: `remote_view::proof`.
  Refactor outcome: success requires selected target, browser window, route,
  display, stream, and Guacamole proof.
  Guard: proof fixtures for terminal-only, terminal-topmost, wrong-tab,
  stale-route, and unavailable route states.
- The left rail can synthesize ownership from session shape, target params, or
  stream URLs.
  Canonical owner: `remote_view::inventory`.
  Refactor outcome: the dashboard renders canonical inventory classes and
  actionability. It does not infer ownership locally.
  Guard: dashboard node, navigator, viewport, and inspector-action fixtures.
- AuraCall and im-receipts CDP browsers are reachable but missing or
  misclassified.
  Canonical owner: `foreign_cdp`.
  Refactor outcome: foreign browsers are discovered as non-owned addressable
  inventory with read-only capabilities.
  Guard: foreign CDP discovery fixtures for fixed ports, `DevToolsActivePort`,
  and single-string cmdlines.
- A route-bound browser can be visible while the selected CDP target is still
  blank or stale.
  Canonical owner: `remote_view::tab` and `remote_view::proof`.
  Refactor outcome: tab acquisition does not mean success until the selected
  `targetId` has fresh URL/title evidence for the requested navigation.
  Guard: tab fixtures for `about:blank` after open, delayed navigation
  readiness, stale target ID recovery, and duplicate same-origin targets.
- Stale installed helper or daemon binary can masquerade as route failure.
  Canonical owner: capability envelope.
  Refactor outcome: live control fails closed when binary, schema, helper,
  route-pool, or inventory contracts are stale.
  Guard: compatibility fixtures and install-doctor readbacks.

## Deterministic Route-Bound Algorithm

The refactored happy path should be a small state machine, not a collection of
recovery heuristics. The coordinator in `remote_view::open` should do only this:

1. Normalize the caller request into `RemoteViewOpenIntent`.
2. Run fast capability and route preflight.
3. Build `RemoteViewAcquisitionPlan` without mutating service state.
4. Reserve a pending `RouteBoundBrowserLease`.
5. Prepare the route desktop and display access for the selected display.
6. Launch or attach exactly one compatible service-owned browser on that
   display.
7. Acquire the intended target tab according to the tab policy.
8. Wait for selected-target readiness against the requested URL.
9. Build `OperatorVisibleProof`.
10. Finalize the lease only when `operatorVisible.state=ready`.
11. Publish one `WorkspaceInventoryRecord` for the finalized result.

Every failure after reservation must move through the lease rollback path and
return a typed blocker. Every failure before reservation must return a typed
plan or preflight blocker. No failure path should publish a service-owned live
control row unless it has a finalized lease and ready proof.

### State Machine Contract

The coordinator should use one typed state machine. These are the only normal
states:

- `requested`: normalized intent exists, no service mutation.
- `planned`: route, display, browser, and tab decisions are known, no service
  mutation.
- `reserved`: route-pool entry, route record, and display allocation are
  pending under one lease.
- `display_ready`: route desktop, X11 socket, and display access are ready for
  browser launch.
- `browser_attached`: exactly one compatible service-owned process is launched
  or attached.
- `tab_acquired`: one current target ID is selected for the requested URL.
- `target_ready`: the selected target has fresh URL/title evidence compatible
  with the requested navigation.
- `proof_ready`: `OperatorVisibleProof.state=ready`.
- `finalized`: lease, route, browser, tab, stream, and inventory agree.
- `rolled_back`: pending mutations were undone after a failure.
- `failed_diagnostic`: no live control ownership was published.

Allowed transitions should be explicit and tested. Anything that wants to skip
from `planned` to `finalized`, or from `browser_attached` to a live dashboard
row without proof, is a bug. Anything that fails after `reserved` must pass
through `rolled_back` or leave a typed lease-repair record that the live left
rail cannot render as a control target.

## Confusion Gates

The refactor is not complete until these gates are enforced in code and tests:

- **Provider gate**: `rdp_gateway` is accepted only as a view-stream provider
  or subcommand-local compatibility alias. It never reaches cloud provider
  selection.
- **Ownership gate**: only finalized route-bound leases can create
  `service_owned_controllable_browser` inventory.
- **Proof gate**: `operatorVisible.state=ready` is the only success state for
  operator handoff. `browser_window_visible` alone is not enough.
- **Route freshness gate**: request-scoped route-pool evidence outranks stale
  daemon-retained route-pool state. Retained state is reusable only when lease,
  route, display, browser, profile, session, and proof agree.
- **Dashboard gate**: the live left rail consumes canonical inventory classes.
  It cannot show `needs attention`, inactive, resolved incident, terminal-only,
  or retained-history rows as live controls.
- **Foreign CDP gate**: reachable non-owned CDP browsers can expose inspect,
  stream, and screenshot actions, but mutation requires explicit borrow and
  lifecycle ownership requires explicit adoption.
- **Binary harmony gate**: stale installed binaries, stale helper contracts,
  stale route desktop templates, and unknown inventory contracts produce typed
  diagnostics, not inferred live rows.
- **Target-readiness gate**: a selected `targetId` whose URL is still
  `about:blank`, stale, missing, or incompatible with the requested URL cannot
  finalize a route-bound lease.

These gates should live in fixtures with product-language names, for example
`terminal_only_route_is_log_only`, `foreign_cdp_is_read_only`, and
`stale_helper_blocks_route_bound_open`.

## Current Evidence To Carry Into The Refactor

The latest live audit narrowed the Facebook route-bound failure to a smaller
and more useful boundary:

- Request-scoped route-pool evidence now selects route A correctly:
  connection `3`, route `guacamole:3`, display `:11`, and route user
  `agent-browser-rdp-a`.
- Route B was not inherently unavailable; local runtime config had a stale
  display name. Updating `AGENT_BROWSER_RDP_ROUTE_B_DISPLAY_NAME` from `:14`
  to `:12` made the route-pool readiness smoke report both route A and route B
  display sockets as ready.
- Display access can be granted for both route displays, but the installed
  root-owned helper is still stale. It lacks `status-json` and still writes a
  terminal-first route desktop template, so helper freshness must remain a
  capability blocker until refreshed from an interactive sudo boundary.
- With fresh route-pool evidence, the live one-liner no longer leaves a stale
  CDP stream or terminal-only live row. It reaches route A and then fails
  closed with rollback.
- The remaining live proof failure is `wrong_tab`: route, display, Guacamole,
  stream, and browser visibility are ready, but the selected CDP target for the
  requested Facebook open still reports `about:blank`.

The refactor must preserve those distinctions. Route readiness, helper
capability, selected-target navigation, and dashboard inventory are separate
answers owned by separate modules. A fix in one layer must not be allowed to
paper over a stale answer in another.

## Binary And Runtime Harmony Contract

The audit showed that a current source checkout and a stale installed runtime
can produce the same user-visible symptom: the dashboard attaches to a terminal
or CDP-shaped stream while the intended route-bound browser path is absent or
only partly understood. P45 should make that impossible to confuse.

Every operator-visible surface should read a small capability envelope before
rendering live control actions:

- executable path and version;
- source checkout version, when available;
- daemon binary path and version;
- service schema version;
- dashboard inventory contract version;
- installed privileged-helper version and route-desktop template state;
- Guacamole route-pool contract version;
- route preflight freshness timestamp.

Compatibility rules:

- Same-version or explicitly compatible versions may render canonical
  inventory.
- Older but recognized versions may render only documented fallback fields and
  must not synthesize service-owned route-bound rows from CDP streams.
- Unknown newer versions must fail closed for live control and show a typed
  compatibility diagnostic.
- A stale privileged helper, stale route desktop template, stale route-pool
  schema, or stale daemon binary is a diagnostic blocker, not a live browser
  row.

The goal is not to force all components to update at once. The goal is to make
drift obvious, typed, and non-dangerous.

## Architectural Invariants

These invariants are the guardrails for the refactor. If a slice cannot keep
one of them true, it should stop and add a fixture before moving code.

- `remote-view open` is the only happy-path coordinator for operator-visible
  route-bound browser opens.
- A service-owned route-bound browser exists only when intent, plan, lease, tab,
  proof, and inventory agree on browser, profile, route, display, target, and
  stream IDs.
- `operatorVisible.state=ready` is the only successful operator handoff state.
- `rdp_gateway` is a view-stream provider value, not a cloud browser provider.
- Full `doctor remote-view` is a diagnostic and repair tool, not the critical
  path for a normal open.
- The dashboard live rail renders canonical inventory classes. It does not
  rediscover ownership from stream URLs, session names, target params, or row
  shape.
- Terminal-only displays, resolved incidents, inactive retained rows, and
  "needs attention" rows without user action are log or diagnostic records, not
  live control entries.
- Foreign CDP browsers are addressable inventory until explicitly adopted.
  Inspecting or streaming them must not imply lifecycle ownership.
- Route-pool, display, browser, and tab mutation happens through a lease state
  machine with idempotent rollback.
- Every route-bound failure returns a typed blocker that says whether repair is
  automatic, explicit, or manual.

## Target Module Layout

### Rust

Create a real `cli/src/native/remote_view/` module tree. Keep
`cli/src/native/remote_view.rs` as a compatibility shim only during migration.

- `intent.rs`
  - `RemoteViewOpenIntent`
  - parser and service-request normalization
  - provider alias handling
  - validation of ambiguous caller fields

- `planner.rs`
  - `RemoteViewAcquisitionPlan`
  - no-mutation route, display, browser, and tab decisions
  - strict reuse eligibility
  - typed blockers and suggested commands

- `lease.rs`
  - `RouteBoundBrowserLease`
  - pending, finalized, failed, rolled-back, and released states
  - route-pool, route, display, browser, and tab snapshots for rollback
  - reconciliation and stale-lease repair helpers

- `tab.rs`
  - route-bound tab acquisition policy
  - selected target tracking
  - delayed URL/title readiness for the selected `targetId`
  - stale target recovery and duplicate compatible-target cleanup
  - no direct display, route, or dashboard decisions

- `proof.rs`
  - `OperatorVisibleProof`
  - CDP target proof
  - X11 display/window proof
  - route and Guacamole proof
  - terminal-only, terminal-topmost, wrong-tab, stale-route, and missing-CDP
    failure vocabulary

- `open.rs`
  - the only route-bound open coordinator
  - calls intent, planner, lease, launch, tab acquisition, proof, finalize
  - owns rollback on every failure
  - returns the service response shape

- `inventory.rs`
  - canonical `WorkspaceInventoryRecord`
  - converts service state into live-control, detected-non-owned, diagnostic,
    retained-history, and log-only records
  - emits actionability and disabled-action reasons

- `foreign_cdp.rs`
  - process and DevTools discovery for non-owned browsers
  - ownership classification
  - read-only addressability and explicit adoption preflight

### Dashboard

The dashboard should become a renderer of canonical inventory, not another
ownership engine.

- Keep `packages/dashboard/src/lib/service-workspaces.ts` as the TypeScript
  compatibility layer while adding tests around the Rust inventory contract.
- Split local presentation helpers after the Rust contract is stable:
  - `workspace-inventory.ts`
  - `workspace-actions.ts`
  - `workspace-stream-selection.ts`
  - `workspace-url-recovery.ts`
- Make `workspace-navigator.tsx` render only live control groups:
  - Agent-browser owned
  - Detected non-owned browsers
- Move attention, stale retained rows, viewer clients, and resolved incidents
  to diagnostic panels, logs, or service incident views.
- Make `workspace-remote-viewport.tsx` reject or repair stale route and target
  params by calling one canonical recovery helper.

## Module Interface Contracts

Define the Rust module APIs before moving implementation. The first PR for P45
should be allowed to add wrapper types and tests without moving most code.

### Intent API

Input:

- CLI command JSON;
- HTTP/MCP service request payload;
- generated-client request shape;
- optional access-plan defaults.

Output:

- `RemoteViewOpenIntent`;
- `RemoteViewIntentDiagnostic` for aliases, defaults, and rejected ambiguity.

Must not decide:

- route selection;
- browser reuse;
- dashboard actionability.

### Planner API

Input:

- `RemoteViewOpenIntent`;
- immutable service-state snapshot;
- request-scoped route-pool evidence;
- capability envelope.

Output:

- `RemoteViewAcquisitionPlan`;
- `RemoteViewPlanBlocker` with reason code, owner, freshness, and suggested
  action.

Must not mutate:

- route-pool entries;
- display allocations;
- browser rows;
- incidents.

### Lease API

Input:

- `RemoteViewAcquisitionPlan`;
- mutable service-state repository transaction.

Output:

- `RouteBoundBrowserLease` in `reserved`, `finalized`, `rolled_back`,
  `released`, or `failed_diagnostic` state;
- idempotent rollback evidence.

Must not decide:

- whether the route is good enough;
- whether the selected tab is the correct URL;
- whether dashboard rows are live.

### Proof API

Input:

- finalized or pending lease snapshot;
- browser process and CDP evidence;
- selected tab evidence;
- display and route evidence;
- Guacamole readiness evidence.

Output:

- `OperatorVisibleProof`;
- component proofs for target, browser, display, route, stream, and Guacamole.

Must not mutate:

- service state;
- route-pool state;
- dashboard rows.

### Tab API

Input:

- route-bound lease snapshot;
- browser session handle;
- requested URL;
- tab acquisition policy;
- existing live target list.

Output:

- selected target ID;
- selected target URL and title readback;
- acquisition decision;
- duplicate cleanup evidence;
- target-readiness state.

Must not decide:

- route selection;
- display readiness;
- Guacamole readiness;
- dashboard actionability.

### Inventory API

Input:

- service-state snapshot;
- finalized leases;
- proof records;
- incidents and jobs;
- detected foreign CDP records;
- capability envelope.

Output:

- `WorkspaceInventoryRecord[]`;
- action descriptors with enabled, disabled, reason, and service action.

Must not perform:

- browser launch;
- route repair;
- foreign CDP mutation;
- target recovery side effects.

### Foreign CDP API

Input:

- process inventory;
- DevTools endpoint probes;
- service-owned profile and browser registry;
- optional explicit adoption records.

Output:

- `DetectedForeignCdpBrowser`;
- read-only action capabilities;
- borrow/adopt preflight blockers.

Must not create:

- service-owned browser rows;
- route-bound leases;
- lifecycle mutation actions without explicit adoption.

## Canonical Data Model

### `RouteBoundBrowserLease`

One record should bind these IDs together:

- `leaseId`
- `intentId`
- `browserId`
- `sessionId`
- `profileId`
- `routePoolEntryId`
- `remoteViewRouteId`
- `displayAllocationId`
- `displayName`
- `processId`
- `cdpEndpoint`
- `selectedTargetId`
- `selectedTargetUrl`
- `selectedTargetTitle`
- `selectedTargetReadiness`
- `viewStreamProvider`
- `controlInputProvider`
- `operatorVisibleProof`
- `state`

No code should treat a browser as route-bound unless this binding exists or can
be reconstructed and reconciled by the lease module.

### `OperatorVisibleProof`

Proof should be a typed struct first and JSON second. The top-level state should
be one of:

- `ready`
- `not_checked`
- `terminal_only`
- `terminal_topmost`
- `wrong_tab`
- `guacamole_route_unavailable`
- `cdp_target_unavailable`
- `stale_route_record`
- `route_mismatch`
- `display_mismatch`
- `browser_not_visible`
- `target_not_ready`
- `proof_failed`

Every non-ready state must include:

- component that failed;
- user-facing reason;
- machine-readable reason code;
- whether repair is automatic, explicit, or manual;
- suggested command or service action when known.

### `WorkspaceInventoryRecord`

Inventory should classify records into exactly one class:

- `service_owned_controllable_browser`
- `service_owned_view_only_browser`
- `service_owned_diagnostic_browser`
- `detected_non_owned_browser`
- `viewer_client`
- `retained_history`
- `log_only_incident`
- `profile_action`

The live left rail should include only:

- `service_owned_controllable_browser`
- `service_owned_view_only_browser`
- `detected_non_owned_browser`

Diagnostic browsers with no user action, inactive history, resolved incidents,
and terminal-only route records should not appear as live left-rail entries.

## Source Movement Strategy

Move code by ownership boundary, not by file size.

1. Add empty modules and typed wrappers that call the current functions.
2. Add product-language fixtures for the desired behavior before moving logic.
3. Move one decision at a time into its owner module.
4. Leave a compatibility wrapper with a `TODO(P45-delete)` marker only when a
   public call site still depends on the old function name.
5. Delete the wrapper in the slice that migrates the last call site.
6. Run the targeted validation for that module before starting the next move.

The first extraction target should be `handle_remote_view_open` in
`cli/src/native/actions.rs`. It should shrink in this order:

- move request normalization to `remote_view/intent.rs`;
- move plan construction and route-pool precedence to
  `remote_view/planner.rs`;
- move pending, finalize, rollback, and repair helpers to
  `remote_view/lease.rs`;
- move target acquisition and target readiness to `remote_view/tab.rs`;
- move display, route, Guacamole, and operator-visible proof construction to
  `remote_view/proof.rs`;
- move response inventory projection to `remote_view/inventory.rs`;
- leave `actions.rs` with command dispatch and repository transaction
  boundaries only.

For dashboard code, the first extraction target should be the current
workspace-node derivation path. Move toward one local adapter that consumes
canonical inventory and one live projection used by the navigator. Delete any
component-local filter that can recreate attention, retained, or terminal-only
rows in the live rail.

For foreign CDP discovery, keep the current discovery improvements, but move
them behind a separate `foreign_cdp` boundary before expanding mutating
capability. The only initial actions should remain inspect, stream, and
screenshot.

## Implementation Slices

### Slice 0: Stabilize The Compatibility Boundary

Goal: prevent installed-binary and source-checkout drift from looking like a
runtime route failure.

Deliverables:

- Add a no-launch version and capability contract that the dashboard and client
  can read before using route-bound inventory fields.
- Make source checkout smokes report the exact binary path and version used.
- Make installed-binary mismatch a typed diagnostic with a suggested install or
  rebuild command.
- Add installed privileged-helper content or version readback to the same
  contract, including the route desktop template state.
- Make the daemon publish the contract it is actually using, not the contract
  implied by the caller's shell environment.
- Add one fixture where the dashboard receives a newer inventory record from
  the service and an older client fallback cannot reinterpret it as a live CDP
  stream.

Acceptance:

- A stale binary can no longer produce a terminal-only or CDP-only live rail row
  by failing to understand the newer route-bound contract.
- Live validation logs identify whether failures came from the installed
  binary, source checkout binary, service daemon, helper, Guacamole, or RDP
  route.
- The route-bound one-liner and dashboard use the same capability vocabulary.

Validation:

```bash
agent-browser install doctor
pnpm test:service-client
pnpm test:dashboard-workspace-nodes
```

### Slice 1: Define The State Machine Before Moving Code

Goal: make the refactor mechanical by writing down the only allowed lifecycle.

Deliverables:

- Add a route-bound acquisition state diagram to this plan or a companion
  architecture note.
- Define the allowed transitions:
  - `requested`;
  - `planned`;
  - `reserved`;
  - `display_ready`;
  - `browser_attached`;
  - `tab_acquired`;
  - `proof_ready`;
  - `finalized`;
  - `rolled_back`;
  - `failed_diagnostic`.
- Define which module owns each transition.
- Define which persisted state may exist after each transition.
- Define rollback behavior for every transition after `reserved`.

Acceptance:

- `actions.rs` can be reduced to a coordinator because every branch has a
  named state-machine transition.
- No transition can publish a live dashboard row before `proof_ready`.
- Failure after `reserved` always has deterministic cleanup evidence.

Validation:

```bash
cargo test --manifest-path cli/Cargo.toml remote_view_state_machine -- --nocapture
pnpm test:route-confusion-gates
```

### Slice A: Freeze The Current Contract

Goal: prevent the refactor from changing behavior accidentally.

Deliverables:

- Add contract fixtures for current P44 success and failure response shapes.
- Add a route-bound open fixture containing:
  - ready Facebook-style open;
  - terminal-only route;
  - terminal-topmost route;
  - wrong tab;
  - stale route record;
  - Guacamole unavailable;
  - binary/helper capability mismatch;
  - stale daemon retained route-pool entry overridden by request-scoped
    preflight;
  - detected non-owned CDP browser;
  - resolved incident.
- Add dashboard snapshot fixtures for the two live rail groups.
- Add binary compatibility fixtures for old service status rows, current
  canonical inventory rows, and rejected unknown inventory classes.

Acceptance:

- The refactor can be done behind tests that already encode the desired user
  behavior.
- Fixture names use product language, not implementation accidents.
- Current failing cases are represented as explicit desired outcomes: typed
  blocker, diagnostic/log-only row, or detected non-owned browser.

Validation:

```bash
pnpm test:route-handoff-audit
pnpm test:dashboard-workspace-nodes
pnpm test:dashboard-view-streams
pnpm test:service-client
```

### Slice B: Move Intent And Planner Behind Stable APIs

Goal: make request semantics independent from `actions.rs`.

Deliverables:

- Move intent normalization to `remote_view/intent.rs`.
- Move no-mutation acquisition planning to `remote_view/planner.rs`.
- Replace loose JSON reads in call sites with typed intent and plan methods.
- Keep old public function names as wrappers until call sites are migrated.

Acceptance:

- CLI, HTTP, MCP, generated client, and dashboard launch requests all normalize
  through one intent path.
- `rdp_gateway` cannot leak into the cloud browser provider lane.
- Request-scoped route-pool data cannot be silently replaced by stale daemon
  retained route-pool data.
- Planner output explains selected route, skipped routes, blockers, and repair
  suggestions without mutating service state.

Validation:

```bash
cargo test --manifest-path cli/Cargo.toml remote_view_intent -- --nocapture
cargo test --manifest-path cli/Cargo.toml remote_view_acquisition_plan -- --nocapture
pnpm test:service-request-client
```

### Slice C: Extract Lease State Machine

Goal: make route, display, browser, and tab ownership atomic.

Deliverables:

- Move pending/finalize/rollback logic to `remote_view/lease.rs`.
- Represent lease state with an enum instead of stringly typed scattered state.
- Make rollback idempotent.
- Make stale checked-out route repair use the same lease reconciliation helpers.

Acceptance:

- A failed open cannot leave a live left-rail route unless it has a finalized
  lease and ready proof.
- Repair actions report which lease, route, display, and pool entry were
  repaired.
- Manual service-state JSON edits are not needed for audited route-pool drift.

Validation:

```bash
cargo test --manifest-path cli/Cargo.toml remote_view_acquisition_lease -- --nocapture
cargo test --manifest-path cli/Cargo.toml service_route_pool_repair -- --nocapture
pnpm test:rdp-guac-route-cleanup-live
```

### Slice D: Extract Operator Proof

Goal: make success mean what the operator actually sees.

Deliverables:

- Move proof construction and state selection to `remote_view/proof.rs`.
- Replace ad hoc JSON proof checks with typed proof methods.
- Add component proofs for target, browser, display, route, Guacamole, and
  stream.
- Make terminal-only and terminal-topmost states fail before route checkout is
  reported as successful.

Acceptance:

- `operatorVisible.state=ready` is the only success state for operator handoff.
- The proof vocabulary is identical across CLI, service response, generated
  client summary, dashboard row readiness, and tests.
- A Unix terminal viewport cannot be represented as a successful browser.

Validation:

```bash
cargo test --manifest-path cli/Cargo.toml operator_visible -- --nocapture
pnpm test:service-request-client
pnpm test:dashboard-view-streams
```

### Slice E: Extract Tab Acquisition And Target Readiness

Goal: stop route-bound opens from treating "target created" as "requested page
is visible."

Deliverables:

- Move target selection, duplicate compatible-target handling, and stale target
  recovery to `remote_view/tab.rs`.
- Add a typed selected-target readiness result with URL, title, target ID,
  attempts, elapsed time, and final state.
- Wait for the selected `targetId` to reach a URL compatible with the requested
  navigation before proof can become `ready`.
- Preserve `wrong_tab` and `target_not_ready` proof states as typed blockers
  with cleanup evidence instead of publishing a live row.

Acceptance:

- A selected target that remains `about:blank` cannot finalize a lease.
- Delayed navigation can converge without opening duplicate tabs.
- Repeat opens reuse or replace compatible targets according to policy, and the
  lease stores the current selected target.
- Dashboard stale `tab=target:*` URLs recover to the current selected target or
  render an explicit stale-target diagnostic.

Validation:

```bash
cargo test --manifest-path cli/Cargo.toml tab_handle -- --nocapture
cargo test --manifest-path cli/Cargo.toml remote_view_target_readiness -- --nocapture
pnpm test:dashboard-workspace-inspector-tab
pnpm test:dashboard-view-streams
```

### Slice F: Create Canonical Inventory

Goal: stop the dashboard from reconstructing ownership and actionability.

Deliverables:

- Add Rust inventory projection from service state and detected browser state.
- Emit canonical inventory records through the service status or a new focused
  workspace inventory endpoint.
- Update `service-workspaces.ts` to consume canonical classes first and retain
  local fallback only for old service versions.
- Remove the live left-rail attention group.
- Keep incident and retained-history browsing in non-live diagnostic views.
- Add a dashboard compatibility guard that prevents old or unknown inventory
  contracts from being inflated into route-bound live rows.

Acceptance:

- The left rail cannot show a terminal-only route as a live control target.
- The left rail cannot show PID-only "needs attention" rows with no user
  action.
- Daemon CDP streams cannot override a service-owned route stream.
- Foreign CDP browsers appear only in the detected non-owned group.

Validation:

```bash
pnpm test:dashboard-workspace-nodes
pnpm test:dashboard-workspace-navigator
pnpm test:dashboard-inspector-actions
pnpm build:dashboard
```

### Slice G: Isolate Foreign CDP Discovery

Goal: make non-owned browsers useful without confusing lifecycle ownership.

Deliverables:

- Move foreign process and CDP detection to `foreign_cdp.rs` or an equivalent
  focused module.
- Classify discovered browsers by:
  - process owner;
  - executable;
  - parent process;
  - profile path;
  - DevTools endpoint;
  - service state match;
  - explicit adoption record.
- Allow read-only inspect, stream, and screenshot when policy allows.
- Require explicit borrow/adopt before mutation, tab creation, close, kill, or
  profile release.

Acceptance:

- AuraCall and im-receipts browsers appear as detected non-owned browsers when
  their CDP endpoints are reachable.
- Non-owned rows are visually and behaviorally distinct from service-owned
  route-bound browsers.
- No foreign browser lifecycle mutation happens from a live rail click.

Validation:

```bash
pnpm test:dashboard-workspace-nodes
pnpm test:dashboard-inspector-actions
cargo test --manifest-path cli/Cargo.toml foreign_cdp -- --nocapture
```

Optional live validation when those apps are running:

```bash
agent-browser service status --json
```

Then verify the dashboard shows reachable AuraCall and im-receipts browsers in
the detected non-owned group only.

### Slice H: Delete Compatibility Fallbacks

Goal: remove the old overlapping interpretations after the new contract is
proven.

Deliverables:

- Delete direct remote-headed fallback from route-bound open.
- Delete route-binding fallback paths that can reuse stale display allocation
  without lease and proof agreement.
- Delete dashboard URL and stream inference that conflicts with canonical
  inventory.
- Delete PID-only "needs attention" live-rail rows and route them to logs or
  diagnostics.
- Delete terminal-only and resolved-incident live-rail entries.
- Convert old JSON helper functions into typed compatibility shims or remove
  them.
- Update docs, skill guidance, README, and help output to show one route-bound
  one-liner.

Acceptance:

- There is one route-bound open coordinator.
- There is one inventory classifier.
- There is one operator-visible proof vocabulary.
- Repair paths are explicit actions, not hidden fallback behavior.

Validation:

```bash
pnpm test:route-confusion-gates
pnpm test:route-handoff-audit
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
pnpm validation:select -- --base HEAD
```

### Slice I: Live End-To-End Closeout

Goal: prove the refactor fixes the user-visible failure mode.

Deliverables:

- Cold start the route stack.
- Run the one-liner against Facebook with the `last30days-facebook` profile.
- Open the dashboard workspace route.
- Open the direct Guacamole route.
- Inspect service status and inventory output.
- Record a dated validation note.

Acceptance:

- The one-liner returns `operatorVisible.state=ready`.
- Dashboard and direct Guacamole show the same browser, not a terminal.
- The left rail shows the browser under Agent-browser owned.
- Foreign browsers, if present, appear under Detected non-owned browsers.
- No useless "needs attention" PID row appears in the live rail.
- Route, display, browser, target, stream, and Guacamole IDs agree in service
  readback.

Validation:

```bash
agent-browser --json remote-view open https://www.facebook.com/ \
  --runtime-profile last30days-facebook \
  --browser-build stealthcdp_chromium \
  --view-stream-provider rdp_gateway

pnpm audit:route-handoff -- --json
pnpm test:rdp-guac-cold-restart-readback-live
```

## Migration Order

1. Stabilize the compatibility boundary so stale binaries cannot masquerade as
   route failures.
2. Define the state machine and fixture vocabulary before moving code.
3. Freeze fixtures before moving code.
4. Move intent and planner first because they should not mutate state.
5. Move lease and rollback next because they are the core safety boundary.
6. Move tab acquisition and target readiness after lease so proof never has to
   guess whether a selected target really navigated.
7. Move proof after tab readiness so proof can rely on stable
   route/display/browser/target IDs.
8. Add canonical inventory after proof so dashboard classes reflect real
   operator visibility.
9. Isolate foreign CDP discovery after inventory classes exist.
10. Delete old fallback paths only after live route-bound opens and dashboard
   grouping pass.

## Worktree Safety Notes

This plan intentionally follows P44 rather than replacing it. P44 remains the
behavioral repair lane. P45 is the structural consolidation lane that should
absorb the successful P44 fixes into named modules and delete the fallback
paths that made the failures hard to reason about.

Do not start P45 by moving thousands of lines at once. Start by adding stable
contract fixtures and module wrappers, then move one answering responsibility at
a time. Each slice should leave the binary runnable and the dashboard contract
compatible.

## Non-Goals

- Do not redesign Guacamole itself.
- Do not require agents to understand Guacamole connection internals.
- Do not merge foreign CDP browsers into service-owned lifecycle by default.
- Do not keep terminal-only route rows in the live rail for visibility. They
  belong in diagnostics.
- Do not make full `doctor remote-view` part of the happy path.

## Closeout Criteria

P45 is complete when a maintainer can explain the code with these six boxes:

1. Intent
2. Plan
3. Lease
4. Tab
5. Proof
6. Inventory

Each box must have one primary module, focused tests, and no competing dashboard
or service fallback that reinterprets its answer.

The final code should make the expected Facebook handoff a real one-liner:

```bash
agent-browser --json remote-view open https://www.facebook.com/ \
  --runtime-profile last30days-facebook \
  --browser-build stealthcdp_chromium \
  --view-stream-provider rdp_gateway
```

Success means a browser is visible to the operator through the selected
Guacamole/RDP route. Anything else is a typed blocker, a diagnostic record, or
an explicit repair action.
