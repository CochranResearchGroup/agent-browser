# Plan 0101 | Route-Bound Open And Actions Deepening

Date: 2026-08-09

Plan version: 2

State: READY FOR CLOSED-WORLD CYCLE 2

Review state: Cycle 1 accepted findings remediated; Cycle 2 is limited to
`P0101-A1-01` through `P0101-A1-06` and critical regressions introduced by
this revision

Lane: P101

Depends On:

- `docs/dev/notes/2026-04-22-agent-browser-service-roadmap.md`
- `docs/dev/plans/0045-2026-06-23-remote-view-architecture-consolidation-plan.md`
- `docs/dev/plans/0069-2026-07-06-shared-profile-routing-and-handoff-deepening-plan.md`
- `docs/dev/plans/0095-2026-08-07-remote-control-duplicate-pressure-readiness-repair-plan.md`
- `docs/dev/plans/0096-2026-08-07-durable-remote-view-handoff-plan.md`
- `docs/dev/plans/0097-2026-08-08-cli-command-timeout-layering-repair-plan.md`
- `docs/dev/plans/0098-2026-08-09-service-request-normalization-deepening-plan.md`
- `docs/dev/plans/0099-2026-08-09-dashboard-workspace-job-control-deepening-plan.md`
- `docs/dev/plans/0100-2026-08-09-service-status-projection-authority-deepening-plan.md`

## Planning Delegation Receipt

- Disposition: `spawned`
- Bounded lane: candidate 4 deep analysis and implementation-ready planning
  for route-bound open orchestration and the `actions.rs` monolith
- Runtime handle: `/root/plan_remote_view_actions`
- Parent orchestrator: `/root`
- Write scope: this plan only
- Source edits, other-plan edits, commits, runtime mutation, and live effects:
  none
- Terminal status: completed for plan authoring
- Evidence returned: current CodeGraph health and structural reads, the
  oversized-file exception for `actions.rs`, bounded direct source inspection,
  current file and symbol counts, Graphiti discovery, roadmap and policy reads,
  prior-plan reconciliation, deletion-test analysis, phased extraction design,
  acceptance criteria, and a risk and validation matrix
- Reconciliation authority: the parent orchestrator owns plan-audit finding
  disposition, execution authority, integration, and final completion judgment
- Cycle 1 revision status: completed in the same bounded plan-author lane;
  accepted findings are adjudicated below and no source, audit-note, runtime,
  or commit effect was taken

## Cycle 1 Adjudication

Reviewed plan SHA-256:
`816cee3ce59b5ffda31dc04286dcd76644ebd2acbcdb6906b8c07364f49e4f8a`

Review artifact:
`docs/dev/notes/0101-2026-08-09-route-bound-open-actions-plan-audit.md`

| Finding | Orchestrator disposition | Version 2 resolution |
| --- | --- | --- |
| `P0101-A1-01` | `blocking`, accepted | Keep `remote_view.rs` as the only Rust module root; add directory children from it; place the decision-free temporary production effect adapter in `actions.rs`; delete it in Slice C. |
| `P0101-A1-02` | `blocking`, accepted | Add the complete transaction, compensation, cancellation, supervisor, join, idempotence, and typed-outcome ledger below. |
| `P0101-A1-03` | `blocking`, accepted | Freeze one typed `DirectOpen` versus `DurableResolution` invocation enum, one coordinator, typed outcomes and blocker, and a closed fallback eligibility ledger. |
| `P0101-A1-04` | `blocking`, accepted | Replace the umbrella D, E, and F slices with numbered one-responsibility execution packets and packet-level receipts, validation, rollback, and commits. |
| `P0101-A1-05` | promoted from `needs_evidence` to `blocking`, accepted | P0101 source movement is a hard stop until P0098, P0099, and P0100 are landed, validated, and bound to exact commits and path ownership. |
| `P0101-A1-06` | promoted from `needs_evidence` to `blocking`, accepted | Freeze the tracked stable-ID inventory, Rust `syn` generator/checker, JavaScript and pnpm entry points, reviewed allowlist, 615-definition completeness proof, and fail-closed fixture as P0 hard stops. |

Cycle 2 is exactly one `closed_world` review. It may verify only these six
finding ids and critical regressions caused by their remediation. It must use
this plan version and revised SHA as the frozen artifact, carry the same ids,
and may not restart broad architecture discovery. There is no Cycle 3. A
residual proven blocker stops or splits only its affected packet; every other
residual is logged with evidence and disposition and the campaign moves on.

## Objective

Deepen route-bound open into one module whose interface owns the complete
operator-visible acquisition transaction. The module must normalize intent,
plan without mutation, reserve ownership, prepare the display, launch or reuse
one browser, acquire and prove one target, check the operator route, finalize
or roll back, persist the durable handoff, and return one authoritative result.

That extraction is the first implementation slice, but it is not the end of
this plan. Continue with a phased domain extraction campaign until
`cli/src/native/actions.rs` owns only command dispatch and truly shared daemon
coordination. The plan cannot close while `actions.rs` remains the place where
profile, route, tab, proof, workflow, persistence, or browser-action semantics
are implemented.

Preserve all public CLI, HTTP, MCP, generated-client, service-state, dashboard,
and installed-runtime contracts unless a separately reviewed contract change
is required to preserve behavior. This is a structural deepening campaign, not
an opportunity to substitute a smaller behavior set.

## Authority And Reconciliation

Authority for this packet, in descending order:

1. `AGENTS.md` and applicable policy under `docs/dev/policies/`;
2. current public schemas, generated clients, action metadata, and source tests;
3. Plans 0095, 0096, and 0097 for current remote-control safety contracts;
4. Plan 0045 for the durable target architecture;
5. Plan 0069 for the implemented shared-profile and route-handoff behavior;
6. the service roadmap and top-level `ROADMAP.md` for product direction;
7. advisory Graphiti memory, only after source verification.

Plan 0069 says further Slice C extraction should be driven by a concrete live
failure or a second adapter rather than moving dispatcher-owned effects for
their own sake. This plan intentionally reopens that recommendation under new,
explicit maintainer authority. The direct source audit now shows concrete
architectural friction: route-bound open is still sequenced in a 341-line
action handler, the handoff module exposes 75 public items, and the action file
contains 615 production functions across unrelated domains. The goal is not to
move async effects blindly. It is to put their sequencing behind one deep
module interface and then finish the broader domain extraction.

`CONTEXT.md`, `docs/adr/`, `docs/agents/codex-code-discovery.md`, and
`docs/agents/codex-stack.md` are absent from the current tree. No ADR conflict
was found. This plan uses the existing domain language: remote-view intent,
route-bound handoff, acquisition lease, retained browser, service tab handle,
operator-visible proof, durable handoff, provider fallback, Service State, and
daemon command.

## Discovery Evidence

### Graph-backed discovery

Graphiti runtime doctor was healthy. A focused read of group
`agent_browser_main` returned source-linked facts for Plan 0069, same-owner
route reuse, route-bound readiness, and shared-profile handoff behavior. Those
facts were used only as routing leads and were verified against current plans
and source.

### CodeGraph coverage and limitation

CodeGraph is healthy with:

- 419 indexed files;
- 14,341 nodes;
- 43,350 edges;
- one intentionally skipped file:
  `cli/src/native/actions.rs`, 1,466,172 bytes.

Indexed structural reads covered `remote_view.rs`,
`remote_view_handoff.rs`, `remote_view_lease.rs`,
`remote_view_finalization.rs`, `remote_view_proof.rs`, the service model, and
the live smoke. `actions.rs` was inspected directly in bounded ranges because
it exceeds the one MiB index cap. The cap was not raised because this plan can
reason about the hand-authored file safely through bounded direct reads, and no
index mutation is needed for planning.

### Current size and responsibility inventory

Current authoritative measurements:

| Evidence | Current value |
| --- | ---: |
| `actions.rs` bytes | 1,466,172 |
| `actions.rs` lines | 37,746 |
| Production portion | lines 1 through 23,488 |
| In-file test portion | lines 23,489 through 37,746 |
| Production function and method definitions | 615 |
| In-file tests | 260 |
| Total function and method definitions | 877 |
| Direct `use super::` import groups | 41 |
| Remote-view-named production helpers or handlers | 40 |
| Remote-view-named tests in `actions.rs` | 42 |
| `remote_view_handoff.rs` lines | 4,079 |
| `remote_view_handoff.rs` public structs, enums, and functions | 75 |
| `remote_view_handoff.rs` tests | 44 |

The 615 production definitions currently divide approximately as follows:

| Current responsibility family | Definition count | Observed examples |
| --- | ---: | --- |
| Browser command handlers | 162 | navigation, interaction, input, storage, tracing, auth, network |
| Service action handlers | 65 | status, resources, profiles, monitors, incidents, jobs, route actions |
| Service domain and projection helpers | 47 | readiness, repair, filtering, trace and lifecycle interpretation |
| Remote-view orchestration and helpers | 40 | route open, target reuse, display access, proof, checkout, fallback |
| Daemon, launch, recovery, and shared local helpers | 301 | `DaemonState`, launch planning, event handling, response shaping |

These groups are evidence for campaign sizing, not the target module layout.
The target follows ownership and seams, not prefixes or file length.

### Route-bound open call shape

`handle_remote_view_open`, currently at lines 13,255 through 13,595, owns this
ordered transaction:

1. normalize the request into `RemoteViewOpenIntent`;
2. derive handoff, browser, and session identity;
3. load Service State and optionally create a managed one-time profile;
4. merge request-scoped route-pool evidence;
5. build the acquisition plan and handoff commands;
6. return the dry-run projection or persist route-pool evidence;
7. reserve the acquisition lease;
8. grant and probe display access;
9. decide retained-browser reuse or launch;
10. acquire or create the target and wait for URL readiness;
11. focus the target and prove a visible browser window;
12. probe public operator access;
13. build and gate pre-checkout operator-visible proof;
14. check out the route;
15. rebuild and gate final proof against the final route binding;
16. finalize ownership, persist the durable handoff, and serialize the result;
17. on each failure, restore the lease, roll back state, and close only the
    action-created tab or browser allowed by the cleanup policy.

`remote_view_handoff.rs` owns substantial pieces of that behavior, but the
caller must still select and order many of its 75 public items. Its public
input structs expose command JSON, typed intent and plan values, repository
identity, lease identity, timestamps, launch and tab results, two proof
records, checkout, display-access evidence, and cleanup details.

### Deletion test

Deleting the current `remote_view_handoff` module would force its response,
lease, proof, persistence, and cleanup behavior back into `actions.rs`. The
module therefore already has real depth and should not be discarded.

Deleting one or more current public handoff helpers, however, usually moves a
small branch back to the caller while leaving the caller responsible for the
same full ordering. That portion of the module is shallow. Its interface is too
close in complexity to its implementation.

Deleting the proposed route-bound open module would force a caller to recreate
all 17 ordered steps, the failure matrix, cleanup ownership, and durable result
assembly. That is the desired deletion-test outcome: a small interface hides
the whole transaction, giving leverage to command, resolver, HTTP, MCP, and
test callers while concentrating locality in one module.

Deleting the current `actions.rs` would scatter hundreds of unrelated domain
rules. That is not healthy depth because its interface is the entire daemon
action vocabulary and its implementation changes for almost every feature.
The campaign must replace it with several deep modules, not rename the file or
split it mechanically.

## Target Architecture

### External route-bound open seam

Keep `cli/src/native/remote_view.rs` as the one and only Rust module root. It
declares directory children such as `mod open;` from files under
`cli/src/native/remote_view/`. `remote_view.rs` and
`remote_view/mod.rs` must never coexist in any worktree state, commit, or
generated patch. This plan does not perform a later root rename; the existing
root remains the compatibility and typed-concept root throughout the campaign.

The external seam exposes one route-bound open operation to command dispatch
and durable handoff resolution. Its conceptual shape is:

```text
RouteBoundOpenCoordinator
  open(RouteBoundOpenInvocation, RouteBoundOpenRuntime)
    -> RouteBoundOpenOutcome
```

The invocation and outcome are frozen as typed enums:

```text
RouteBoundOpenInvocation
  DirectOpen { request, handoff_id }
  DurableResolution { handoff_id, allow_reopen_closed, service_job_id }

RouteBoundOpenOutcome
  NotFound
  ExplicitlyClosed
  Reopened { opened }
  Opened { opened }
  RolledBack { blocker, compensation }
  ProviderFallback { fallback }
```

`DirectOpen` contains the normalized request and explicit caller identity.
`DurableResolution` contains only opaque durable-resolution authority. The
coordinator loads the handoff, handles `NotFound` and `ExplicitlyClosed`,
derives the normalized reacquisition request, and then executes the same open
implementation used by `DirectOpen`. `allow_reopen_closed=true` produces
`Reopened`, never an ordinary `Opened` outcome. `RouteBoundOpenOutcome`
converts to the existing JSON only at the command edge.

Browser ownership failure is typed at the runtime seam as
`RouteBoundRuntimeIssue::RequestedProfileInUseByPid { profile_id, pid,
owner_browser_id, owner_session_id }`. The compatibility serializer may emit
the existing `already in use by PID` text, but neither the coordinator nor
fallback eligibility may parse an error string.

The module tree should converge on:

```text
remote_view.rs            sole module root and typed public concepts
remote_view/
  open.rs                the only route-bound open coordinator
  intent.rs              request normalization and ambiguity checks
  planner.rs             no-mutation acquisition decisions
  acquisition.rs         lease reservation, finalization, and rollback
  target.rs              retained-tab reuse, creation, readiness, cleanup
  operator_route.rs      display access and bounded operator-route observations
  proof.rs               typed operator-visible proof and readiness gate
  handoff.rs             durable handoff persistence and resolution intent
  response.rs            compatibility serialization at the command edge
```

This does not create nine externally visible modules. `remote_view.rs`
re-exports only the typed concepts and one coordinator operation that callers
need. Every directory child remains private or narrowly `pub(crate)`.

### Dependency classification and adapters

- Intent normalization, plan selection, route-binding merge, proof
  classification, cleanup selection, and response construction are
  in-process. They require no adapter.
- The JSON Service State repository and temporary test repository are
  local-substitutable. The deep module should use the existing repository
  interface and test through a temporary repository, not invent a second
  public persistence port.
- Browser launch or reuse, target acquisition, focus, and transaction-owned
  compensation vary between the live daemon and deterministic tests. This is a
  real internal seam with `ActionsRouteBoundOpenRuntime` during Slices A and B,
  the daemon-runtime/browser-lifecycle adapter from Slice C, and a scripted
  in-memory adapter.
- Display access, visible-window observation, and public operator-route
  readiness vary by host. This is a real internal seam with a bounded local
  adapter, an explicit unavailable adapter, and an in-memory test adapter.
- The dashboard ingress is remote but owned. Its readiness check stays behind
  the operator-route seam. Production may use the bounded HTTP adapter while
  tests use the in-memory adapter.

Internal adapter interfaces must remain private or `pub(crate)` and must not
become part of the public CLI, HTTP, MCP, or generated-client contract. Tests
cross the route-bound open interface. Adapter-specific tests cover only the
real variations at those internal seams.

#### Temporary `ActionsRouteBoundOpenRuntime` topology

Slice A defines the narrow `pub(crate) RouteBoundOpenRuntime` effect trait in
`remote_view/open.rs`. `actions.rs` defines
`ActionsRouteBoundOpenRuntime<'a> { state: &'a mut DaemonState }` and implements
that route-module-owned trait. The adapter may call current private action
handlers or `BrowserManager` methods, but it must not select routes, decide
reuse, classify readiness, choose cleanup, interpret proof, or assemble domain
JSON. It returns typed raw observations or effect results to the coordinator.

The exact temporary method ledger is:

| Method | Current effect delegated | Decision prohibition |
| --- | --- | --- |
| `observe_browser()` | active process, session, runtime profile, active target, and page snapshot from `DaemonState` and `BrowserManager` | no compatibility or reuse decision |
| `launch_browser(command)` | `handle_launch` | no launch-command shaping or profile fallback |
| `refresh_targets()` | drain current CDP events and return page and active-target observations | no target selection |
| `switch_target(target_id)` | exact `BrowserManager::tab_switch_target_id` | no fallback target choice |
| `navigate_target(url)` | `BrowserManager::navigate` with the coordinator-supplied URL and wait posture | no readiness classification |
| `open_target(command)` | `handle_tab_new` | no retained-target decision |
| `focus_target(command)` | `handle_view_focus` | no proof or checkout decision |
| `close_created_target(target_id)` | transaction-created target close through the exact tab-close path | never closes a reused target |
| `close_created_browser(browser_identity)` | bounded transaction compensation through `handle_close` | never runs for a reused or previously established browser |
| `checkout_route(command)` | `handle_service_remote_view_route_checkout` | no route selection or proof interpretation |
| `ensure_display_access(binding)` | current bounded display probe and privileged-helper grant | no route-readiness decision |
| `observe_visible_window(binding)` | current bounded visible-window probe | returns evidence, not proof state |
| `observe_operator_access(binding)` | current bounded owned-ingress HTTP observation | no fallback eligibility decision |

The trait has no generic `execute(Value)` escape hatch. Every method accepts a
typed request owned by the route module and returns a typed result. Slice C
implements the same trait in the daemon runtime and browser lifecycle modules,
migrates the call site, and deletes `ActionsRouteBoundOpenRuntime`, its impl,
and every adapter-only import from `actions.rs` in checkpoint `P0101-C02`.

### Dependency direction

The required dependency direction is:

```text
actions command dispatch
  -> ActionsRouteBoundOpenRuntime effect implementation, Slices A and B only
  -> remote_view::open
       -> intent and planner
       -> acquisition
       -> RouteBoundOpenRuntime trait declared by the route module
       -> target
       -> proof
       -> handoff persistence
       -> response compatibility adapter
```

The temporary adapter is dependency inversion, not a reverse import:
`actions.rs` imports and implements a trait owned by `remote_view::open`; the
route module never imports `actions`. After checkpoint `P0101-C02`, the daemon
runtime and browser lifecycle modules implement the trait and dispatch imports
only the coordinator.

The following directions are forbidden:

- `remote_view::* -> actions::*`;
- target, proof, acquisition, or handoff modules calling command dispatch;
- `actions.rs` reading or mutating route-pool, display, acquisition-lease, or
  durable-handoff fields directly after the route-bound slice;
- response JSON deciding domain state that was not already represented in a
  typed outcome;
- dashboard code promoting provider URLs or host observations into ownership.

### Final responsibility of `actions.rs`

At closeout, `actions.rs` may own only:

- command id and action extraction;
- policy and confirmation gates shared by all commands;
- shared stale-browser pre-dispatch guard and backend compatibility guard;
- dispatch from an action name to one deep module interface;
- response timing and the stable success or error envelope;
- compatibility re-exports during the migration, deleted at final closeout.

It must not own:

- request normalization for a domain;
- profile or browser-build selection;
- route, display, lease, target, proof, or provider-fallback interpretation;
- Service State projection, repair, persistence, or query filtering;
- UI action, probe, network capture, file transfer, or diagnostics recipes;
- browser-action implementation details;
- X11, privileged-helper, filesystem, HTTP, or raw CDP calls;
- typed domain structs or enums that belong to extracted modules;
- domain-behavior tests.

### Wider domain extraction campaign

After route-bound open, extract by concepts that already have cohesive
behavior. Do not create one file per action or thin pass-through modules.

1. **Daemon runtime module**
   owns `DaemonState`, event application, current-browser connection recovery,
   stream-client synchronization, cancellation attachment, and lifecycle
   observation. Its small interface exposes the state transitions needed by
   dispatch and domain adapters.
2. **Browser lifecycle module**
   owns launch planning, profile and capability selection, retained-browser
   attachment, duplicate-profile protection, launch or close behavior,
   runtime handoff, and recovery. It preserves the process-exit-only cleanup
   rule from Plan 0097.
3. **Service workflow modules**
   deepen probe, UI action, network capture, file transfer, and diagnostics
   recipes. Each module owns validation, execution, summary, and error
   vocabulary behind one workflow interface.
4. **Browser operation modules**
   group navigation and evaluation, page inspection, interaction and input,
   waits and locators, browser storage and auth, capture and recording, and
   request routing by cohesive behavior. Existing deep modules such as
   `interaction`, `snapshot`, `cookies`, `storage`, `network`, `recording`, and
   `auth` should absorb orchestration where their interfaces already have
   depth. Avoid wrapper-only duplicates.
5. **Service command modules**
   own profile, session, browser, tab, monitor, provider, incident, job,
   resource, trace, and retained-state command behavior. Reuse existing
   `service_*` modules as the implementation authority. Command adapters should
   load inputs, call one module interface, and map the typed result.
6. **Dispatch closeout**
   delete compatibility wrappers and move domain tests beside the interfaces
   they exercise. Keep only routing, shared gate, cancellation, timing, and
   envelope tests in `actions.rs`.

Plans 0098, 0099, and 0100 own service-request normalization, workspace-view
projection, and Service Status Projection. Their landed modules count toward
this campaign only after Plan 0101 verifies their commits, validation receipts,
dependency direction, and removal of residual copies from shared paths. Plan
0101 remains responsible for the final monolith gate even though those three
candidates must land first.

## Preserved Contracts And Safety Invariants

### Public and compatibility contracts

- Keep the `remote_view_open` action name and its CLI, HTTP, MCP, and generated
  client shapes.
- Preserve `routeBoundHandoff`, `sharedAcquisition`, `operatorVisible`,
  `preCheckoutOperatorVisible`, `acquisitionLease`, cleanup summaries,
  verification fields, and typed error codes.
- Keep command ids, daemon queue ordering, service job attribution, and caller
  trace fields unchanged.
- Preserve existing compatibility aliases, including the subcommand-local
  `provider=rdp_gateway` alias, without allowing it into cloud-provider
  selection.
- Keep unknown or stale contract versions fail-closed. Do not use the refactor
  to add a silent fallback.

### Durable handoffs and provider fallback

- Successful opens continue to return the authenticated opaque
  `/remote-view/<handoff-id>` URL.
- Raw Guacamole or other provider URLs remain ephemeral evidence and never
  become the durable public identity.
- Resolution prefers the same logical browser and tab, strips stale route and
  display selectors before reacquisition, and preserves profile, host, build,
  stream, control, and display-isolation posture.
- Explicit close remains terminal unless an explicit Reopen action is present.
- The retained RDP provider fallback remains best-effort only when the original
  browser cannot be reacquired safely but the retained route is still usable.
  It must preserve the browser, report `operatorVisible.state=best_effort`, and
  never create a duplicate profile lane or claim normal managed-browser
  control.
- Authenticated access and operator-visible proof remain required. URL presence
  alone is not readiness.

Fallback eligibility is a closed conjunction. Every row must be true against
one immutable `DurableResolutionSnapshot` loaded before reacquisition:

| Required evidence | Exact eligibility rule |
| --- | --- |
| Invocation | `DurableResolution`, never `DirectOpen` |
| Prior provider | retained handoff has `view_stream_provider=rdp_gateway` |
| Snapshot identity | snapshot contains the requested handoff, its last route id, browser id, session id, profile id, desired URL, and original posture; the same snapshot is used for the open attempt and fallback decision |
| Snapshot timing | snapshot has one `loaded_at` timestamp recorded in the outcome; no post-error state reload may broaden eligibility |
| Explicit close | handoff is not explicitly closed, or the invocation carried `allow_reopen_closed=true` and returns `Reopened` on acquisition success |
| Exact ownership cause | adapter returned `RequestedProfileInUseByPid` for the normalized requested profile and the retained owner identity; no generic launch, proof, target, timeout, or route blocker qualifies |
| Retained route | the snapshot route id equals the handoff last route id, uses RDP gateway, is nonterminal, and exposes the retained provider route from authoritative route evidence rather than a caller URL |
| Authentication | the resolver request already passed dashboard authentication and the bounded owned-ingress observation does not report auth failure |
| Operator evidence | retained route has current usable provider evidence and the fallback outcome explicitly reports `operatorVisible.state=best_effort`; it never reports `ready` without full proof |
| Browser preservation | retained browser and profile owner are preserved; no close, launch, target creation, route replacement, or lifecycle adoption is attempted by fallback |
| Duplicate-lane gate | coordinator proves no duplicate profile lane was created or authorized |

Failure of any row returns the original typed blocker or rolled-back outcome.
The compatibility serializer preserves existing fields and text but cannot
change eligibility.

### Runtime and browser safety

- One runtime profile directory remains exclusive to one Chrome process group
  unless an explicit reviewed duplicate-lane flag authorizes otherwise.
- Same-owner retained-browser and retained-tab reuse remains preferred when all
  identity and readiness facts agree.
- Route, display, browser, target, stream, lease, and proof identity must agree
  before finalization.
- Every failure after reservation passes through idempotent rollback and emits
  cleanup evidence.
- Failed-transaction compensation before finalization may close only the tab or
  browser created by that transaction. Reused retained browsers, reused tabs,
  unrelated tabs, and previously established browsers always survive.
- Plan 0097 daemon cleanup of an established reachable `BrowserManager` is a
  different ownership class. Only observed process exit authorizes that daemon
  cleanup. Timeout, cancellation, renderer failure, target failure, route
  failure, and Guacamole disconnect do not authorize it.
- No extraction may start a background task that outlives the job without an
  existing supervised owner and cancellation contract.

### Timeout and cancellation contracts

- Preserve the positive `jobTimeoutMs` control-plane deadline and its ordering
  beneath the caller's outer deadline.
- Preserve renderer-side termination for JavaScript evaluation. Cancelling the
  worker future is not accepted as proof that renderer work stopped.
- Preserve bounded target initialization and retained-target fallback.
- Preserve queue release after timeout or cancellation only after the
  route-bound supervisor joins compensation, without discarding a reachable
  BrowserManager or launching a replacement browser.
- Preserve response-before-health-probe ordering and the Linux same-inode
  executable identity fast path.
- Preserve the status-specific ten-second dashboard backend timeout and the
  five-second coalescing cache. This plan does not move those controls into the
  generic command budget.

### Route-bound transaction and compensation ledger

`RouteBoundOpenCoordinator` is a cooperative-cancellation transaction. The
control-plane worker must not race and drop this command future. For
`remote_view_open` and durable resolution it starts one
`RouteBoundTransactionSupervisor`, supplies the existing cancellation token
and deadline, and awaits the coordinator. Timeout or cancel signals the token;
the coordinator stops starting new forward effects, runs compensation, and
returns a typed outcome. The worker releases the serialized queue only after
the supervisor joins that outcome.

Forward effects and compensation effects each use existing bounded operation
deadlines. The compensation phase has a named
`ROUTE_BOUND_COMPENSATION_JOIN_TIMEOUT_MS` of 15,000 ms. Reaching that watchdog
does not drop the coordinator or release the queue: the worker records
`rollback_join_blocked`, stays unavailable for the next command, and continues
joining the one supervised future until it returns. Each individual cleanup
effect must itself terminate within its declared bound and return a typed
failure, so the normal path returns within the join bound. No detached cleanup,
spawned close, or orphan rollback task is permitted.

| Phase | Persisted mutation | Runtime effect and owner | Cancellation checkpoint | Compensation | Idempotence key | Required final state | Typed outcome on failure or cancel |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `normalized` | none | intent and invocation parsing, coordinator | before planning | none | invocation id | no state change | typed blocker, `NotFound`, or `ExplicitlyClosed` |
| `planned` | none | immutable plan, coordinator | after plan | none | invocation id plus plan digest | no state change | `RolledBack` with `no_mutation` compensation |
| `route_pool_persisted` | request-scoped entries with prior snapshots | repository, coordinator acquisition module | immediately after repository commit | restore inserted or changed entries from captured snapshots | invocation id plus route-pool entry id | exact pre-invocation route-pool state | `RolledBack` |
| `reserved` | pending lease, route-pool checkout, display allocation, and route records | repository, acquisition module | immediately after reservation | idempotent acquisition rollback | lease id | lease `rolled_back`; prior route, pool, and display state restored | `RolledBack` |
| `display_prepared` | no new Service State ownership | bounded display-access grant and probe, runtime adapter | after bounded grant/probe | acquisition rollback; current helper has no safe revoke, so retained access grant is recorded as non-ownership diagnostic evidence | lease id plus display name | Service State rolled back; no live browser row; grant retention explicit | `RolledBack` |
| `browser_reused` | no new browser ownership | exact retained-browser observation, runtime adapter | after observation | acquisition rollback only; never close browser | lease id plus retained browser id | reused browser and all prior tabs unchanged | `RolledBack` |
| `browser_created` | pending browser association under lease | launch, runtime adapter | immediately after launch result | bounded `close_created_browser`, await result, then acquisition rollback | lease id plus created browser identity | created browser closed or cleanup failure recorded; prior browsers unchanged | `RolledBack` with cleanup result |
| `target_reused` | pending selected-target evidence | switch or observe exact retained target, runtime adapter | after switch and each readiness observation | acquisition rollback only; never close reused target or browser | lease id plus retained target id | reused target and browser remain live | `RolledBack` |
| `target_created` | pending selected-target and service-tab evidence | open and navigate transaction target, runtime adapter | after create, navigation, and each wait iteration | bounded `close_created_target`, await result; close created browser too only when this transaction also created it; then acquisition rollback | lease id plus created target id | created target closed or cleanup failure recorded; reused state survives | `RolledBack` with cleanup result |
| `focused` | none | exact target focus and maximize, runtime adapter | after focus | same ownership-based target/browser compensation, then acquisition rollback; no focus reversal | lease id plus target id | no finalized ownership; reused state survives | `RolledBack` |
| `precheckout_proved` | proof evidence remains pending | visible-window and operator-access observations plus typed proof, coordinator | after each bounded observation and proof gate | ownership-based compensation, then acquisition rollback | lease id plus proof digest | no live controllable row | `RolledBack` |
| `checked_out` | route, pool, display, stream, and lease checkout state | route checkout, runtime adapter plus acquisition module | immediately after checkout | route checkout rollback plus ownership-based browser/target compensation | lease id plus route allocation id | prior checkout restored; lease `rolled_back` | `RolledBack` |
| `final_proved` | final proof remains pending | final binding and proof, coordinator | after final proof | same checked-out compensation | lease id plus final proof digest | no finalized ownership unless proof is ready | `RolledBack` |
| `finalized_and_persisted` | lease finalization, route ownership, inventory evidence, and durable handoff when requested in one repository transaction | acquisition and handoff modules | cancellation observed before commit aborts; after commit it cannot trigger transaction compensation | none; later lifecycle uses explicit service actions | lease id plus handoff id or direct-open invocation id | finalized ownership and durable handoff agree atomically | `Opened` or `Reopened` |
| `provider_fallback` | no new lifecycle ownership | closed fallback ledger evaluation, coordinator | before serialization | none; fallback performs no launch, close, or route mutation | handoff id plus snapshot digest | retained browser and route unchanged | `ProviderFallback` |

The `finalized_and_persisted` repository mutation removes the current possible
gap between lease finalization and durable-handoff persistence. If atomic
mutation cannot be implemented with the existing repository interface, Slice A
stops before source movement and revises the acquisition persistence design;
it may not preserve a partially finalized success path.

## Architecture Measurements And Enforcement

### Tracked baseline inventory and architecture checker

The tracked completeness ledger is
`docs/dev/architecture/actions-responsibility-inventory.v1.json`. Its root
schema is `actions-responsibility-inventory.v1` and contains:

- `schemaVersion` and generator version;
- baseline commit, `actions.rs` SHA-256, byte count, line count, production
  definition count `615`, and in-file test count `260`;
- the stable identity convention;
- one record for every production definition;
- associated test ids and their target interface;
- the reviewed dispatcher/shared-coordination allowlist;
- packet assignment, target module, movement status, wrapper owner and deletion
  packet, and final disposition.

Stable ids are independent of source line:

```text
ari:<item-kind>:<qualified-impl-or-trait-owner>:<name>:<normalized-signature-sha256-prefix>
```

Free functions use qualified owner `native::actions`; inherent methods use the
fully qualified impl type; trait methods include the trait and impl type. The
signature normalizer removes whitespace, comments, attributes unrelated to the
call contract, parameter binding names, and source locations while preserving
item kind, visibility, asyncness, unsafety, ABI, receiver shape, generic and
where constraints, parameter types, and return type. The first 16 hexadecimal
characters of its SHA-256 digest are used in the id; the complete digest is
stored in the record. A digest collision or duplicate stable id is a hard
failure.

Add a repo-owned Rust parser and checker at
`scripts/architecture/actions-inventory/`. Its independent Cargo manifest uses
`syn` with full AST and visit support. It has deterministic `generate`,
`check`, and `self-test` modes and never queries CodeGraph. Add
`scripts/check-actions-architecture.js` as the stable wrapper and this package
entry:

```json
"test:actions-architecture": "node scripts/check-actions-architecture.js --check"
```

The JavaScript wrapper invokes the Rust checker with the pinned repo manifest,
source path, and tracked inventory. Clean local and CI runs therefore use the
same parser and inputs. It fails closed on unparseable source, missing or extra
definitions, unknown status or packet values, an unclassified definition, an
unreviewed retained definition, expired wrappers, forbidden dependencies,
reverse imports, budget failures, hash drift without a packet receipt, and
inventory ids that cannot be regenerated.

The reviewed allowlist is stored in the same JSON under
`dispatcherAllowlist`. Each entry contains a stable id, allowed responsibility,
rationale, reviewer, and plan version. Only command routing, shared policy and
confirmation gates, shared backend and stale-browser guards, timing, and stable
response envelopes may be approved.

The fail-closed fixture is
`scripts/architecture/actions-inventory/fixtures/unclassified-action.rs`. It
contains one valid production definition absent from its fixture inventory.
`self-test` must prove this fixture fails with `unclassified_definition` while
the matching classified fixture passes.

P0 generates the baseline, manually classifies all 615 production definitions,
binds the exact source hash, reviews the allowlist, and demonstrates clean
`generate`, `check`, and `self-test` receipts. Prefix-based counts and
line-derived ids are forbidden.

### Final measurable gates

The final architecture gate requires all of these conditions:

1. `actions.rs` has at most 2,500 total lines.
2. It contains at most 35 production function or method definitions.
3. It contains at most 20 routing, gate, timing, or response-envelope tests.
4. Every remaining production definition is present on the explicit
   dispatcher/shared-coordination allowlist.
5. No remaining definition is marked `compatibility wrapper` in the
   responsibility inventory.
6. No direct `reqwest`, `std::process::Command`, repository mutation, X11,
   privileged-helper, raw CDP command, route-pool, acquisition-lease, durable
   handoff, or Service State projection logic remains in `actions.rs`.
7. `actions.rs` does not define domain structs or enums. `DaemonState` and its
   domain methods live in the daemon runtime module.
8. Each dispatch branch calls one deep module interface and contains no domain
   decision branch. A temporary compatibility branch may exist only inside its
   owning migration slice and must be gone at closeout.
9. Extracted domain modules do not import `actions`.
10. Domain tests live beside and cross the extracted interface. The interface
    is the test surface; tests do not reach through it to assert private
    implementation state.

The line and definition budgets are intentionally secondary to the
responsibility allowlist. They prevent a renamed monolith or a huge in-file
test body, but a small `actions.rs` that delegates to shallow pass-through
modules still fails.

Add a fast `pnpm test:actions-architecture` gate that checks the inventory,
budgets, forbidden dependencies, reverse imports, and wrapper count. The gate
must use stable parsing where available and fail closed when it cannot classify
a production definition.

### Per-slice movement rule

Every slice must:

- select one coherent responsibility from the inventory;
- freeze observable behavior at its current interface;
- move the implementation and its tests together;
- delete the old implementation in the same slice;
- leave only a temporary compatibility wrapper when an unmigrated caller truly
  requires it;
- record the wrapper owner and deletion slice;
- reduce the unclassified or compatibility-wrapper count;
- keep source runnable and contracts compatible.

Moving code into a file while keeping decision logic or duplicate tests in
`actions.rs` does not count as progress.

## Execution Plan

### Preflight P0 | Freeze identity and baseline

This is a no-movement preflight. No source extraction may precede Slice A.

P0101 source work is blocked until P0098, P0099, and P0100 implementation is
landed and validated. A plan or audit artifact is not a landed predecessor.

1. Record branch, worktree status, current HEAD, `actions.rs` hash and counts,
   CodeGraph status, and every dirty or untracked path.
2. Record one predecessor and write-ownership matrix with, for each of P0098,
   P0099, and P0100: landed commit id, validation receipt, shared paths changed,
   owning plan and packet, and whether the path is now clean.
3. Record the last-known-green base commit after all three predecessors land.
   Run `pnpm validation:select -- --base <last-green-base>` and preserve the
   selected gate list for the whole campaign.
4. For every shared path, record the reconciliation action: no overlap,
   fast-forward adoption, rebase, manual semantic reconciliation, or blocked.
   `actions.rs`, native module roots, package scripts, service contracts,
   dashboard projection files, generated clients, ROADMAP, and shared docs are
   mandatory matrix rows when touched by a predecessor.
5. Hard stop if a shared path is dirty under another packet, a predecessor
   commit or receipt is missing, the current checkout does not contain the
   recorded commit, the last-green base is not an ancestor, or semantic
   reconciliation is unresolved.
6. Add the Rust `syn` generator/checker, JavaScript wrapper, pnpm entry, tracked
   inventory, reviewed allowlist, and fail-closed fixtures as the isolated
   `P0101-P00` architecture-harness checkpoint.
7. Prove exactly 615 baseline production definitions are classified, every id
   regenerates from the baseline hash, no stable-id collision exists, the clean
   checker passes, and the intentionally unclassified fixture fails with the
   expected code.
8. Generate the first expected red final-budget report. Red size and movement
   budgets are expected; any unclassified, parser, identity, allowlist, or
   predecessor failure is a hard stop.
9. Freeze route-bound success, failure, cancellation, compensation, durable
   resolution, and provider-fallback fixtures before moving behavior.

Preflight exit requires one durable P0 receipt containing all predecessor
commit ids, validation receipt paths, last-green base, worktree and dirty-path
readback, shared-path ownership and reconciliation, baseline source hash,
inventory hash, 615-of-615 classification count, allowlist review, checker
self-test output, and checkpoint `P0101-P00`. Missing evidence blocks Slice A.

### Slice A | Extract route-bound open first

This is the first source extraction.

1. Keep `remote_view.rs` as the sole root, declare `mod open;` and its other
   directory children there, and add the typed invocation, outcome, blocker,
   compensation, and runtime-effect vocabulary.
2. Define the exact `RouteBoundOpenRuntime` trait in the route module, implement
   the frozen 13-method ledger in temporary
   `ActionsRouteBoundOpenRuntime`, and add the scripted in-memory adapter. Do
   not expose each transaction step to callers.
3. Move the complete `handle_remote_view_open` sequence behind the coordinator,
   including dry-run, profile preparation, request-scoped route-pool handling,
   lease reservation, display access, launch or reuse, target acquisition,
   focus, proof, checkout, final proof, persistence, response assembly, and
   rollback.
4. Move all `remote_view_open_*` helpers and their 42 named action tests to the
   owning route-view modules or delete them when the coordinator makes them
   redundant.
5. Replace the control-plane drop-on-timeout/cancel behavior for route-bound
   invocations with the named cooperative supervisor, cancellation signal,
   bounded compensation join, and queue-release contract in the transaction
   ledger. Do not alter the established BrowserManager cleanup rule for other
   commands.
6. Make command dispatch call one route-bound open operation.
7. Make durable handoff resolution use the `DurableResolution` invocation of
   that same operation. Preserve typed not-found, explicit-close, reopen, the
   closed provider-fallback eligibility ledger, and compatibility serialization.
8. Keep `remote_view_handoff.rs` as a compatibility shim only for unmigrated
   callers. Record every exported item and delete or privatize it in Slice B.
9. Add interface-level fixtures for planned, opened, rolled-back,
   explicitly-closed, recovered, and provider-fallback outcomes.

Slice A acceptance:

- `actions.rs` has no route-open sequencing or `remote_view_open_*` helper;
- its `remote_view_open` branch calls one coordinator operation;
- a caller cannot construct a partial open by invoking public plan, proof,
  cleanup, and finalization helpers in arbitrary order;
- the route-bound interface tests prove the full success and failure matrix;
- exact public JSON fixtures remain unchanged unless a separately audited bug
  repair explicitly updates the contract;
- timeout and cancel after every mutation phase join typed compensation before
  queue release, and the forced watchdog fixture proves no orphan task;
- transaction-created tab and browser cleanup is bounded and awaited, while
  reused and established browsers and tabs survive;
- provider fallback passes only when every row of the closed ledger is true and
  uses the immutable resolution snapshot;
- the responsibility inventory marks the whole route-bound-open family moved
  or deleted, with no unowned wrapper.

### Slice B | Deepen the handoff implementation and delete its shallow surface

1. Partition the 4,079-line handoff implementation by internal ownership under
   `remote_view/`.
2. Make planning, acquisition, target, proof, cleanup, persistence, and
   response helpers private to the coordinator unless a second production
   caller proves a real seam.
3. Keep durable handoff resolution as a second caller of the coordinator, not
   a second implementation of acquisition.
4. Reduce the route-view crate-visible interface to typed domain concepts, one
   open operation, and the focused read operations truly used outside the
   module.
5. Delete `remote_view_handoff.rs` or reduce it to a short documented re-export
   shim, then delete that shim before final campaign closeout.

Slice B acceptance:

- deleting `remote_view::open` would force callers to recreate the entire
  transaction;
- no action caller coordinates individual lease, proof, cleanup, or response
  helpers;
- internal module tests survive implementation movement because they assert
  through the external route-bound interface;
- no loose JSON is the primary internal interface for intent, state,
  acquisition, proof, or outcome.

### Slice C | Extract daemon runtime and browser lifecycle

1. Move `DaemonState`, event subscription and application, stream-client
   synchronization, input-state reset, cancellation attachment, and connection
   recovery into a daemon runtime module.
2. Preserve `actions::DaemonState` as a temporary re-export for daemon,
   control-plane, HTTP, parity, and end-to-end callers; migrate those callers
   and delete the re-export in this slice or the immediately following slice.
3. Move launch planning, profile and capability selection, retained-browser
   attachment, duplicate-profile policy, runtime handoff, browser recovery,
   and close behavior into the browser lifecycle module.
4. Add the permanent daemon-runtime/browser-lifecycle implementation of
   `RouteBoundOpenRuntime`, migrate the coordinator call site, and delete
   `ActionsRouteBoundOpenRuntime`, its trait impl, and its adapter-only imports
   in checkpoint `P0101-C02`.
5. Keep command dispatch ignorant of profile locks, executable selection,
   display isolation, stream construction, and process ownership.

Slice C acceptance:

- `actions.rs` no longer defines `DaemonState`, browser lifecycle types, or
  launch-policy decisions;
- only the daemon runtime module mutates shared daemon runtime fields;
- timeout and cancellation regressions prove reachable-browser preservation,
  queue release, renderer termination, and no replacement launch;
- direct users import the new authority, not the compatibility re-export.
- `P0101-C02` proves the temporary actions adapter has zero remaining stable
  inventory ids and no route module imports `actions`.

### Packet contract for D, E, and F

The former umbrella slices are replaced by the numbered packets below. Each
packet is one independently buildable, auditable, testable, and revertible
commit. Baseline line spans are selection anchors only; P0 expands every row
to the exact stable inventory ids and associated test ids before execution.
An id outside the selected family is a hard stop requiring a new packet row,
not permission to widen the current packet.

For every packet:

- `actions.rs` owns the temporary call adapter or wrapper only during that
  packet and deletes it before the packet commit unless the table names a later
  deletion checkpoint;
- expected line and definition deltas are measured against the P0 baseline;
  the packet cannot report a smaller delta without identifying deletions or
  revised stable ids in its receipt;
- all associated behavior tests move beside the target interface; wrapper-only
  tests are deleted after parity is proven;
- focused validation means the named filter or pnpm gate plus
  `pnpm test:actions-architecture`, formatting, strict Clippy, and patch checks;
- canonical validation means `cd cli && cargo test`; packets touching
  dashboard, generated clients, docs, contracts, or HTTP/MCP also run the
  corresponding selected pnpm build and contract gates;
- rollback is `revert <packet-commit>` with no later packet allowed to depend
  on an unvalidated commit;
- the packet receipt records exact inventory ids, test ids moved and deleted,
  before and after counts, commit id, focused and canonical results, wrapper
  count, and rollback readiness.

### D packets | Deep service workflows

| Packet | Stable-id generation selection and expected delta | Exact source family to target module | Temporary owner and deletion | Tests and validation | Commit and rollback boundary |
| --- | --- | --- | --- | --- | --- |
| `P0101-D01` | baseline span 9043–9340, diagnostics family, minus 298 lines and 6 definitions | service diagnostics, caps, summaries, and bounded diagnostic helpers to `service_diagnostics.rs` | actions call adapter, delete D01 | move associated `service_diagnostics` tests; focused `service_diagnostics` and service contract metadata; canonical Rust | commit D01 only; revert D01 |
| `P0101-D02` | 6854–7325, probe family, minus 472 lines and 7 definitions | bounded evaluate, detector execution, identity normalization, freshness, and fingerprinting to `service_probe.rs` | actions call adapter, delete D02 | move `service_probe` and bounded-evaluate tests; focused probe, evaluation timeout, cancellation; canonical Rust | commit D02 only; revert D02 |
| `P0101-D03` | 7326–7741, UI-action family, minus 416 lines and 8 definitions | UI recipe validation, execution, find, dialog, summary, caller, and page readback to `service_ui_action.rs` | actions call adapter, delete D03 | move `service_ui_action` tests; focused UI action and service request parity; canonical Rust | commit D03 only; revert D03 |
| `P0101-D04` | 7742–8257, network-capture family, minus 516 lines and 8 definitions | capture validation, trigger, match, event, body, and allowed-header behavior to `service_network_capture.rs` | actions call adapter, delete D04 | move `service_network_capture` tests; focused network capture and cancellation; canonical Rust | commit D04 only; revert D04 |
| `P0101-D05` | 8258–9042, file-transfer family, minus 785 lines and 19 definitions | upload, download, path authorization, safe-name, fetch capture, result, and MIME behavior to `service_file_transfer.rs` | actions call adapter, delete D05 | move file-transfer and allowed-path tests; focused upload/download/path gates; canonical Rust | commit D05 only; revert D05 |

Each D module owns validation, execution order, cancellation, result summary,
and failure vocabulary behind one interface. The daemon browser adapter is an
internal seam; command dispatch cannot repeat recipe rules.

### E packets | Browser operation families

| Packet | Stable-id generation selection and expected delta | Exact source family to target module | Temporary owner and deletion | Tests and validation | Commit and rollback boundary |
| --- | --- | --- | --- | --- | --- |
| `P0101-E01-01` | 6469–6654, navigation family, minus 186 lines and 4 definitions | navigation and service-tab persistence to `cli/src/native/browser_navigation.rs` | actions adapter, delete E01-01 | move navigation tests; focused navigation persistence; canonical Rust | commit E01-01; revert E01-01 |
| `P0101-E01-02` | 6655–6735, URL and inspector family, minus 81 lines and 4 definitions | URL, CDP URL, inspector, and external URL opening to `cli/src/native/browser_inspection.rs` | actions adapter, delete E01-02 | move URL/inspector tests; focused inspection; canonical Rust | commit E01-02; revert E01-02 |
| `P0101-E01-03` | 6736–6817, page-read family, minus 82 lines and 2 definitions | title and content reads to `cli/src/native/browser_page_read.rs` | actions adapter, delete E01-03 | move title/content tests; focused page reads; canonical Rust | commit E01-03; revert E01-03 |
| `P0101-E01-04` | 6818–6853, evaluation family, minus 36 lines and 2 definitions | renderer-deadline evaluation to `cli/src/native/browser_evaluation.rs` | actions adapter, delete E01-04 | move evaluation tests; focused renderer timeout and cancellation; canonical Rust | commit E01-04; revert E01-04 |
| `P0101-E02` | 9666–9870, primary capture family, minus 205 lines and 2 definitions | snapshot and screenshot orchestration to `cli/src/native/page_capture.rs`, which delegates primitive capture to existing `snapshot.rs` and `screenshot.rs` | actions adapter, delete E02 | move snapshot/screenshot tests; focused capture; canonical Rust | commit E02; revert E02 |
| `P0101-E03` | 9871–10527, primary interaction family, minus 657 lines and 18 definitions | click, fill, type, press, hover, scroll, select, check, and query orchestration to existing `cli/src/native/interaction.rs` | actions adapter, delete E03 | move interaction tests; focused interaction and input state; canonical Rust | commit E03; revert E03 |
| `P0101-E04-01` | 10528–10607, history family, minus 80 lines and 3 definitions | back, forward, and reload to `cli/src/native/browser_navigation.rs` | actions adapter, delete E04-01 | move history tests; focused navigation; canonical Rust | commit E04-01; revert E04-01 |
| `P0101-E04-02` | 10608–10726, wait family, minus 119 lines and 5 definitions | selector, URL, text, function, and polling waits to `cli/src/native/browser_wait.rs` | actions adapter, delete E04-02 | move wait tests; focused wait timeout and cancellation; canonical Rust | commit E04-02; revert E04-02 |
| `P0101-E05-01` | 10727–10778, cookie family, minus 52 lines and 3 definitions | cookie get/set/clear to existing `cli/src/native/cookies.rs` | actions adapter, delete E05-01 | move cookie tests; focused cookies; canonical Rust | commit E05-01; revert E05-01 |
| `P0101-E05-02` | 10779–10837, storage family, minus 59 lines and 3 definitions | storage get/set/clear to existing `cli/src/native/storage.rs` | actions adapter, delete E05-02 | move storage tests; focused storage; canonical Rust | commit E05-02; revert E05-02 |
| `P0101-E05-03` | 10838–10848, content-mutation family, minus 11 lines and 1 definition | set-content to `cli/src/native/browser_page_content.rs` | actions adapter, delete E05-03 | move set-content tests; focused page content; canonical Rust | commit E05-03; revert E05-03 |
| `P0101-E05-04` | 10849–10875, network-posture family, minus 27 lines and 2 definitions | headers and offline posture to `cli/src/native/network.rs` | actions adapter, delete E05-04 | move header/offline tests; focused network posture; canonical Rust | commit E05-04; revert E05-04 |
| `P0101-E05-05` | 10876–10890, console-diagnostic family, minus 15 lines and 2 definitions | console and page errors to `cli/src/native/browser_console.rs` | actions adapter, delete E05-05 | move console/error tests; focused diagnostics; canonical Rust | commit E05-05; revert E05-05 |
| `P0101-E05-06` | 10891–10930, saved-state family, minus 40 lines and 2 definitions | state save/load to existing `cli/src/native/state.rs` | actions adapter, delete E05-06 | move saved-state tests; focused state; canonical Rust | commit E05-06; revert E05-06 |
| `P0101-E05-07` | 10931–11038, diff family, minus 108 lines and 2 definitions | snapshot and URL diff to existing `cli/src/native/diff.rs` | actions adapter, delete E05-07 | move diff tests; focused diff; canonical Rust | commit E05-07; revert E05-07 |
| `P0101-E06` | 11039–11083, credential family, minus 45 lines and 5 definitions | credential set/get/delete/list and auth-show commands to `cli/src/native/auth.rs` | actions adapter, delete E06 | move credential tests; focused auth; canonical Rust | commit E06; revert E06 |
| `P0101-E07` | 11084–11174, compatibility input family, minus 91 lines and 2 definitions | top-level mouse and keyboard compatibility commands to `browser_input.rs` | actions adapter, delete E07 | move compatibility input tests; focused input; canonical Rust | commit E07; revert E07 |
| `P0101-E08` | 11175–11264, basic tab family, minus 90 lines and 3 definitions | tab list, browser PID, and tab-new command edge to `browser_tabs.rs` | actions adapter, delete E08 | move tab-list/new tests excluding route-open target tests; focused tab; canonical Rust | commit E08; revert E08 |
| `P0101-E09-01` | 16288–16346, emulation family, minus 59 lines and 3 definitions | viewport, user-agent, and media to `cli/src/native/browser_emulation.rs` | actions adapter, delete E09-01 | move emulation tests; focused emulation; canonical Rust | commit E09-01; revert E09-01 |
| `P0101-E09-02` | 16347–16506, download-start family, minus 160 lines and 1 definition | download initiation to `cli/src/native/browser_download.rs` | actions adapter, delete E09-02 | move download-start tests; focused download posture; canonical Rust | commit E09-02; revert E09-02 |
| `P0101-E10-01` | 16507–16543, tracing-profiler family, minus 37 lines and 4 definitions | trace and profiler lifecycle to `cli/src/native/tracing.rs` | actions adapter, delete E10-01 | move trace/profiler tests; focused tracing and cancellation; canonical Rust | commit E10-01; revert E10-01 |
| `P0101-E10-02` | 16544–16731, recording family, minus 188 lines and 3 definitions | recording lifecycle to existing `cli/src/native/recording.rs` | actions adapter, delete E10-02 | move recording tests; focused recording and cancellation; canonical Rust | commit E10-02; revert E10-02 |
| `P0101-E10-03` | 16732–16782, PDF family, minus 51 lines and 1 definition | PDF capture to `cli/src/native/page_pdf.rs` | actions adapter, delete E10-03 | move PDF tests; focused PDF capture; canonical Rust | commit E10-03; revert E10-03 |
| `P0101-E11-01` | 16783–16932, direct page-interaction family, minus 150 lines and 7 definitions | focus, clear, select-all, scroll-into-view, dispatch, highlight, and tap to `cli/src/native/page_interaction.rs` | actions adapter, delete E11-01 | move direct-interaction tests; focused interaction; canonical Rust | commit E11-01; revert E11-01 |
| `P0101-E11-02` | 16933–17044, element-value family, minus 112 lines and 6 definitions | bounding box, inner text/HTML, input value, set value, and count to `cli/src/native/element.rs` | actions adapter, delete E11-02 | move element-value tests; focused element reads/writes; canonical Rust | commit E11-02; revert E11-02 |
| `P0101-E11-03` | 17045–17070, computed-style family, minus 26 lines and 1 definition | computed-style reads to `cli/src/native/browser_styles.rs` | actions adapter, delete E11-03 | move style tests; focused styles; canonical Rust | commit E11-03; revert E11-03 |
| `P0101-E11-04` | 17071–17129, browser-context posture family, minus 59 lines and 5 definitions | fronting, timezone, locale, geolocation, and permissions to `cli/src/native/browser_context.rs` | actions adapter, delete E11-04 | move context tests; focused permissions and emulation; canonical Rust | commit E11-04; revert E11-04 |
| `P0101-E11-05` | 17130–17162, dialog family, minus 33 lines and 1 definition | dialog handling to `cli/src/native/browser_dialog.rs` | actions adapter, delete E11-05 | move dialog tests; focused dialog; canonical Rust | commit E11-05; revert E11-05 |
| `P0101-E11-06` | 17163–17207, upload family, minus 45 lines and 1 definition | upload command to `cli/src/native/browser_upload.rs` | actions adapter, delete E11-06 | move upload tests; focused file authorization; canonical Rust | commit E11-06; revert E11-06 |
| `P0101-E11-07` | 17208–17300, script-style injection family, minus 93 lines and 3 definitions | page script, init-script, and style injection to `cli/src/native/page_injection.rs` | actions adapter, delete E11-07 | move injection tests; focused scripts/styles; canonical Rust | commit E11-07; revert E11-07 |
| `P0101-E11-08` | 17301–17360, clipboard family, minus 60 lines and 1 definition | clipboard command to existing `cli/src/native/clipboard.rs` | actions adapter, delete E11-08 | move clipboard tests; focused clipboard; canonical Rust | commit E11-08; revert E11-08 |
| `P0101-E11-09` | 17361–17385, wheel family, minus 25 lines and 1 definition | wheel input to `cli/src/native/browser_input.rs` | actions adapter, delete E11-09 | move wheel tests; focused input; canonical Rust | commit E11-09; revert E11-09 |
| `P0101-E12-01` | 17386–17432, device-posture family, minus 47 lines and 1 definition | device emulation to `cli/src/native/browser_emulation.rs` | actions adapter, delete E12-01 | move device tests; focused device posture; canonical Rust | commit E12-01; revert E12-01 |
| `P0101-E12-02` | 17433–17582, stream-runtime family, minus 150 lines and 16 definitions | stream, engine, provider, and extension markers plus stream lifecycle/status to `cli/src/native/stream_runtime.rs` | actions adapter, delete E12-02 | move stream lifecycle tests; focused stream and runtime markers; canonical Rust | commit E12-02; revert E12-02 |
| `P0101-E13` | 20719–20796, screencast family, minus 78 lines and 2 definitions | screencast start/stop to `stream_screencast.rs` | actions adapter, delete E13 | move screencast tests; focused screencast; canonical Rust | commit E13; revert E13 |
| `P0101-E14` | 20797–21008, page-load and frame family, minus 212 lines and 6 definitions | wait-for-URL/load/function and frame/main-frame behavior to `browser_frame.rs` | actions adapter, delete E14 | move frame and load-wait tests; focused frames and waits; canonical Rust | commit E14; revert E14 |
| `P0101-E15` | 21009–21562, semantic locator family, minus 554 lines and 17 definitions | subactions, roles, semantic locators, nth, find, eval handle, drag, expose, pause, multiselect to `browser_locator.rs` | actions adapter, delete E15 | move locator and subaction tests; focused locator/drag; canonical Rust | commit E15; revert E15 |
| `P0101-E16-01` | 21563–21645, response-body family, minus 83 lines and 1 definition | response-body retrieval to `cli/src/native/network_response.rs` | actions adapter, delete E16-01 | move response-body tests; focused response retrieval; canonical Rust | commit E16-01; revert E16-01 |
| `P0101-E16-02` | 21646–21706, download-wait family, minus 61 lines and 1 definition | download completion wait to `cli/src/native/browser_download.rs` | actions adapter, delete E16-02 | move download-wait tests; focused download timeout/cancellation; canonical Rust | commit E16-02; revert E16-02 |
| `P0101-E16-03` | 21707–21787, new-window family, minus 81 lines and 1 definition | new-window creation to `cli/src/native/browser_tabs.rs` | actions adapter, delete E16-03 | move new-window tests; focused tabs; canonical Rust | commit E16-03; revert E16-03 |
| `P0101-E16-04` | 21788–21851, screenshot-diff family, minus 64 lines and 1 definition | screenshot diff to `cli/src/native/diff.rs` | actions adapter, delete E16-04 | move screenshot-diff tests; focused diff; canonical Rust | commit E16-04; revert E16-04 |
| `P0101-E16-05` | 21852–21888, video family, minus 37 lines and 2 definitions | video lifecycle to `cli/src/native/browser_video.rs` | actions adapter, delete E16-05 | move video tests; focused video lifecycle; canonical Rust | commit E16-05; revert E16-05 |
| `P0101-E17-01` | 21889–22259, HAR family, minus 371 lines and 14 definitions | HAR lifecycle, conversion, paths, timing, and browser metadata to `cli/src/native/network_archive.rs` | actions adapter, delete E17-01 | move HAR tests; focused HAR; canonical Rust | commit E17-01; revert E17-01 |
| `P0101-E17-02` | 22260–22478, Fetch-interception family, minus 219 lines and 3 definitions | paused-request resolution and Fetch pattern construction to `cli/src/native/network.rs` | actions adapter, delete E17-02 | move Fetch tests; focused request interception; canonical Rust | commit E17-02; revert E17-02 |
| `P0101-E18-01` | 22479–22580, request-route mutation family, minus 102 lines and 2 definitions | route and unroute to `cli/src/native/network.rs` | actions adapter, delete E18-01 | move route mutation tests; focused network routing; canonical Rust | commit E18-01; revert E18-01 |
| `P0101-E18-02` | 22581–22681, request-query family, minus 101 lines and 2 definitions | request listing and detail to `cli/src/native/network_requests.rs` | actions adapter, delete E18-02 | move request-query tests; focused request inventory; canonical Rust | commit E18-02; revert E18-02 |
| `P0101-E18-03` | 22682–22713, HTTP-credential family, minus 32 lines and 1 definition | HTTP credentials to `cli/src/native/auth.rs` | actions adapter, delete E18-03 | move HTTP credential tests; focused auth; canonical Rust | commit E18-03; revert E18-03 |
| `P0101-E19` | 22714–23048, auth workflow family, minus 335 lines and 5 definitions | selector waits, auth save/login, confirm, and deny to `auth_workflow.rs` | actions adapter, delete E19 | move login and confirmation tests; focused auth; canonical Rust | commit E19; revert E19 |
| `P0101-E20` | 23049–23194, mobile gesture family, minus 146 lines and 2 definitions | swipe and device list to `cli/src/native/webdriver/mobile_gestures.rs` | actions adapter, delete E20 | move mobile tests; focused WebDriver/iOS/Safari where applicable; canonical Rust | commit E20; revert E20 |
| `P0101-E21` | 23195–23472, low-level input family, minus 278 lines and 12 definitions | mouse masks and raw mouse, keyboard, touch, key, text, and pointer events to `browser_input.rs` | actions adapter, delete E21 | move raw-input tests; focused input; canonical Rust | commit E21; revert E21 |

Prefer deepening existing modules over wrapper files. A packet that merely
moves a handler without concentrating its invariants fails its deletion test.

### F packets | Service State and remote-view commands

| Packet | Stable-id generation selection and expected delta | Exact source family to target module | Temporary owner and deletion | Tests and validation | Commit and rollback boundary |
| --- | --- | --- | --- | --- | --- |
| `P0101-F01` | 14369–14910, route preflight family, minus 542 lines and 11 definitions | route preflight, helper status, display access, and desktop observations to `remote_view/preflight.rs` | actions adapter, delete F01 | move route-preflight tests; focused preflight timing and route gates; canonical Rust | commit F01; revert F01 |
| `P0101-F02` | 14911–15868, reattach and route lifecycle family, minus 958 lines and 14 definitions | browser reattach, parking selection, checkout, and release to `remote_view/route_lifecycle.rs` | actions adapter, delete F02 | move reattach/checkout/release tests; focused route cleanup and route confusion; canonical Rust | commit F02; revert F02 |
| `P0101-F03` | 15869–16287, viewer lease family, minus 419 lines and 8 definitions | viewer request, controller takeover, heartbeat, release, timestamp, and stream upsert to `remote_view/viewer_lease.rs` | actions adapter, delete F03 | move viewer lease tests; focused takeover and heartbeat; canonical Rust | commit F03; revert F03 |
| `P0101-F04` | 17583–17652, status action family, minus 70 lines and 1 definition | adopt landed P0100 projector at command seam; no parallel projector | actions adapter, delete F04 | move status action tests to P0100 interface; run P0100 architecture and status gates; canonical Rust plus dashboard | commit F04; revert F04 |
| `P0101-F05` | 17653–17762, resources and GC family, minus 110 lines and 8 definitions | resources, monitor summary, GC, process statistics to `service_resources.rs` | actions adapter, delete F05 | move resources/GC tests; focused resources; canonical Rust | commit F05; revert F05 |
| `P0101-F06` | 17763–18198, access and capability family, minus 436 lines and 10 definitions | access plan and browser capability preflight/preference to `service_access.rs` and capability registry authority | actions adapter, delete F06 | move access/capability tests; focused service client and registry parity; canonical Rust | commit F06; revert F06 |
| `P0101-F07` | 18199–18342, core inventory query family, minus 144 lines and 6 definitions | profile, seeding-handoff, session, browser, and tab inventory reads to `cli/src/native/service_inventory.rs` | actions adapter, delete F07 | move inventory query tests; focused MCP resources and clients; canonical Rust | commit F07; revert F07 |
| `P0101-F08` | 18343–18436, configured entity query family, minus 94 lines and 4 definitions | monitor, site-policy, provider, and challenge inventory reads to `cli/src/native/service_configuration_inventory.rs` | actions adapter, delete F08 | move query tests; focused MCP resource reads; canonical Rust | commit F08; revert F08 |
| `P0101-F09` | 18437–18718, reconcile and browser repair family, minus 282 lines and 6 definitions | reconcile, route-pool refresh, browser close/repair, and health labeling to `service_health.rs` | actions adapter, delete F09 | move reconcile/browser repair tests; focused service health and no-replacement; canonical Rust | commit F09; revert F09 |
| `P0101-F10` | 18719–18881 and 19258–19839, retained-state family, minus 745 lines and 22 definitions | retained prune/repair, stale-reason, eligibility, and retained observation helpers to `service_retained_state.rs` | actions adapter, delete F10 | move retained repair/prune tests; focused retained resource repair; canonical Rust | commit F10; revert F10 |
| `P0101-F11` | 18882–19257, route-pool repair family, minus 376 lines and 4 definitions | route-pool reconciliation and repair to `remote_view/route_pool_repair.rs` | actions adapter, delete F11 | move route-pool repair tests; focused route repair and no-duplicate ownership; canonical Rust | commit F11; revert F11 |
| `P0101-F12` | 19840–19853, job-cancel family, minus 14 lines and 1 definition | job cancellation to `cli/src/native/service_jobs.rs` | actions adapter, delete F12 | move job-cancel tests; focused service-job cancellation; canonical Rust | commit F12; revert F12 |
| `P0101-F13` | 19854–19914, profile-mutation family, minus 61 lines and 4 definitions | profile upsert, freshness, seeding-handoff, and delete to `cli/src/native/service_config.rs` | actions adapter, delete F13 | move profile mutation tests; focused service config and store tests; canonical Rust | commit F13; revert F13 |
| `P0101-F14` | 19915–19937, session-mutation family, minus 23 lines and 2 definitions | session upsert and delete to `cli/src/native/service_lifecycle.rs` | actions adapter, delete F14 | move session mutation tests; focused lifecycle and store tests; canonical Rust | commit F14; revert F14 |
| `P0101-F15` | 19938–19960, site-policy mutation family, minus 23 lines and 2 definitions | site-policy upsert and delete to `cli/src/native/service_config.rs` | actions adapter, delete F15 | move site-policy mutation tests; focused policy contract and store tests; canonical Rust | commit F15; revert F15 |
| `P0101-F16` | 19961–20046, monitor-mutation family, minus 86 lines and 6 definitions | monitor upsert, delete, state, reset, triage, and due execution to `cli/src/native/service_monitors.rs` | actions adapter, delete F16 | move monitor mutation tests; focused monitor scheduler and store tests; canonical Rust | commit F16; revert F16 |
| `P0101-F17` | 20047–20069, provider-mutation family, minus 23 lines and 2 definitions | provider upsert and delete to `cli/src/native/providers.rs` | actions adapter, delete F17 | move provider mutation tests; focused provider and store tests; canonical Rust | commit F17; revert F17 |
| `P0101-F18` | 20070–20096, capability-registry mutation family, minus 27 lines and 2 definitions | capability registry upsert and config-id validation to `cli/src/native/service_access.rs` | actions adapter, delete F18 | move capability mutation tests; focused registry parity and store tests; canonical Rust | commit F18; revert F18 |
| `P0101-F19` | 20097–20135, browser-retry family, minus 39 lines and 1 definition | service browser retry to `cli/src/native/service_health.rs` | actions adapter, delete F19 | move browser-retry tests; focused service health and no-replacement tests; canonical Rust | commit F19; revert F19 |
| `P0101-F20` | 20136–20160, remedy-application family, minus 25 lines and 1 definition | remedy application to `cli/src/native/service_incidents.rs` | actions adapter, delete F20 | move remedy tests; focused incident remedies and contracts; canonical Rust | commit F20; revert F20 |
| `P0101-F21` | 20161–20213, incident-mutation family, minus 53 lines and 4 definitions | incident acknowledge/resolve and operator/note normalization to `cli/src/native/service_incidents.rs` | actions adapter, delete F21 | move incident mutation tests; focused incident contracts; canonical Rust | commit F21; revert F21 |
| `P0101-F22` | 20214–20266, event-query family, minus 53 lines and 1 definition | service-event reads to `cli/src/native/service_activity.rs` | actions adapter, delete F22 | move event-query tests; focused activity and MCP resource reads; canonical Rust | commit F22; revert F22 |
| `P0101-F23` | 20267–20324, incident-query family, minus 58 lines and 1 definition | incident reads to `cli/src/native/service_incidents.rs` | actions adapter, delete F23 | move incident-query tests; focused incident resources and filters; canonical Rust | commit F23; revert F23 |
| `P0101-F24` | 20325–20401, job-query family, minus 77 lines and 1 definition | service-job reads to `cli/src/native/service_jobs.rs` | actions adapter, delete F24 | move job-query tests; focused job resources and filters; canonical Rust | commit F24; revert F24 |
| `P0101-F25` | 20402–20416, incident-activity query family, minus 15 lines and 1 definition | incident activity reads to `cli/src/native/service_activity.rs` | actions adapter, delete F25 | move incident-activity tests; focused activity resources; canonical Rust | commit F25; revert F25 |
| `P0101-F26` | 20417–20718, trace-query family, minus 302 lines and 18 definitions | service trace assembly, labels, timestamps, filters, and joins to `cli/src/native/service_trace.rs` | actions adapter, delete F26 | move trace and filter tests; focused trace and MCP resource reads; canonical Rust | commit F26; revert F26 |

F04 consumes the P0100 landed interface. P0098 and P0099 are predecessor hard
stops but do not authorize duplicate work in these packets. After F26, Service
State reads and mutations are absent from `actions.rs`; clients and dashboard
consume the same canonical results, and predecessor architecture gates remain
green.

### Slice G | Dispatch closeout and compatibility deletion

1. Delete every remaining compatibility wrapper and update direct imports.
2. Move domain tests out of `actions.rs`; retain only dispatcher, shared gate,
   backend compatibility, timing, cancellation handoff, and envelope tests.
3. Make the architecture gate green against the final responsibility
   inventory.
4. Update inline documentation and developer architecture notes. Update CLI
   help, README, skill guidance, docs site, schemas, generated clients, and
   public examples only if an audited public behavior or contract changed.
5. Update ROADMAP and the plan closeout with exact before and after counts,
   validation evidence, remaining nonblocking backlog, and installed-runtime
   state.

Slice G acceptance:

- all ten final measurable gates pass;
- `actions.rs` is command dispatch and truly shared coordination only;
- no source file is a renamed replacement monolith;
- Plan 0101 remains open if any compatibility wrapper, responsibility gap, or
  typed-domain leak remains.

### Slice H | Installed and operator-visible regression proof

This slice occurs only after source validation and effect-boundary review.

1. Build the reviewed release-mode candidate.
2. Inspect current retained browsers, PIDs, profiles, targets, routes, displays,
   leases, and service status before installation.
3. Install through the normal user-scoped checkpoint flow only when the packet
   authorizes that effect.
4. Re-run `agent-browser install doctor` and `agent-browser doctor remote-view`.
5. Run one neutral-fixture route-bound open and durable-handoff resolution.
6. Prove route, display, browser, target, stream, lease, proof, and durable
   handoff identity agree.
7. Prove provider fallback with a no-duplicate fixture or retained-state unit
   adapter. Do not deliberately disrupt a private authenticated browser merely
   to force fallback.
8. Compare retained-browser PIDs and tab sets before and after. Any unexplained
   replacement, close, or duplicate profile lane is blocking.

## Test And Validation Matrix

### Route-bound open interface

Required focused behavior:

- dry-run is no-mutation and returns the existing planned response;
- ready open finalizes exactly one acquisition lease;
- same-owner route, browser, and target reuse remains deterministic;
- incompatible or duplicate profile ownership fails closed;
- request-scoped route evidence cannot overwrite a live active checkout with
  stale inactive evidence;
- display-access failure rolls back without launching;
- launch, target, focus, visible-window, operator-access, checkout, and final
  proof failures all roll back with the correct cleanup owner;
- a blank or stale selected target cannot finalize;
- repeated opens converge to one active intended target;
- explicit close remains terminal;
- durable resolution strips stale route identity and reacquires current state;
- retained RDP provider fallback is explicit best-effort and never duplicates
  the browser;
- cancellation and timeout preserve the browser and release the queue.

Focused commands should include the current repo gates, adjusted only when
test names move:

```bash
cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture
cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture
cargo test --manifest-path cli/Cargo.toml remote_view_acquisition -- --nocapture
cargo test --manifest-path cli/Cargo.toml remote_view_target_readiness -- --nocapture
cargo test --manifest-path cli/Cargo.toml cancellation -- --nocapture
pnpm test:route-confusion-gates
pnpm test:route-handoff-audit
pnpm test:service-client
```

### Per-slice structural validation

```bash
pnpm test:actions-architecture
pnpm validation:select -- --base <last-green-ref>
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
git diff --check
```

Run the focused tests for every touched module and every moved public or
crate-visible symbol. CodeGraph may remain stale for `actions.rs` until the file
falls beneath the index cap; use the responsibility inventory and direct reads
until then. Once it falls below the cap, run `codegraph sync` only if normal
watcher behavior does not index it, then verify dependency direction through
CodeGraph impact and caller reads.

### Campaign closeout validation

```bash
pnpm test:actions-architecture
pnpm test:service-client
pnpm test:dashboard-workspace-nodes
pnpm test:dashboard-inspector-actions
pnpm test:route-confusion-gates
pnpm test:route-handoff-audit
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
cd cli && cargo test
```

Run additional contract, dashboard, service workflow, and generated-client
gates selected from the complete slice diff. Run the ignored native end-to-end
suite serially when browser-operation families or `DaemonState` movement could
affect real Chrome behavior:

```bash
cd cli && cargo test e2e -- --ignored --test-threads=1
```

The tester must report discovered, run, passed, failed, and ignored counts,
not only an exit code.

### Live proof

When Slice H is authorized, use the neutral fixture and current live smoke
rather than a private site:

```bash
pnpm test:remote-view-open-fixture-live
pnpm test:rdp-guac-cold-restart-readback-live
agent-browser install doctor
agent-browser doctor remote-view
```

Live proof is required for installed-runtime closeout, but it does not replace
the no-launch and Rust suites. A live route failure caused by unavailable host
infrastructure is not evidence that source architecture passed.

## Five-Role Execution And Bounded Review

The orchestrator runs this candidate through the user-requested five roles:

1. deep-analysis and plan author;
2. independent plan auditor;
3. executor;
4. independent work auditor;
5. tester.

The current plan-author role is complete under runtime handle
`/root/plan_remote_view_actions`. Later role receipts must record runtime
handle, frozen objective, write scope, start and finish status, evidence,
accepted or rejected findings, and orchestrator reconciliation.

The executor and tester receipts must include one result row for every P0,
slice, and numbered D, E, and F packet, including skipped or blocked packets.
The independent work auditor reviews every packet commit and its rollback
boundary, then reviews the cumulative architecture, compatibility, dependency,
wrapper, line, definition, and validation gates at each checkpoint and at
campaign closeout. The five roles remain candidate-level roles; packetization
does not create a new role cycle per packet.

### Two-cycle audit bound

Each audit stage has at most two cycles across the whole candidate, not two
cycles per wording revision or per reviewer.

- Cycle 1 is a fresh-context `drift_discovery` pass against the frozen
  objective, acceptance criteria, safety invariants, non-goals, and target
  commit or plan version.
- The orchestrator classifies every finding as `blocking`,
  `nonblocking_backlog`, `rejected`, or `needs_evidence` and authorizes one
  consolidated remediation pass for accepted blockers.
- Cycle 2 is a `closed_world` verification limited to accepted blocking
  finding ids and critical regressions introduced by their fixes.
- No third broad review starts. Any residual complaint is logged with finding
  id, criterion, evidence, consequence, confidence, and disposition. A proven
  unresolved safety or contract blocker stops that execution packet. A
  nonblocking, speculative, duplicate, or scope-expanding complaint enters the
  backlog and the campaign moves on.

The review cap prevents infinite cycles. It does not permit the orchestrator to
downgrade a proven unresolved acceptance or safety failure merely to finish.
When such a blocker remains after cycle 2, split or repair the bounded packet,
record the evidence, and continue the larger campaign without restarting broad
discovery for already accepted findings.

## Rollback Strategy

- Keep each migration slice independently buildable and reviewable.
- Preserve wire and persistence compatibility during movement. Do not migrate
  persisted Service State merely to move code.
- Delete old code only after its interface-level fixture is green in the new
  module.
- Do not retain a runtime toggle that silently selects old and new semantics.
  Rollback is source control reversion of the bounded slice, not a permanent
  dual path.
- If a slice changes stored data despite the non-goal, stop and open a
  separately reviewed backup and recovery packet before applying it.
- Before installed validation, record exact retained browser and route state.
  If installation causes unexplained browser replacement or contract drift,
  restore the prior validated executable and dashboard payload, preserve
  state artifacts for diagnosis, and do not rerun the effect automatically.
- Route-bound failures after lease reservation must use the same idempotent
  rollback path during both migration and rollback validation.

## Risks And Controls

| Risk | Consequence | Control |
| --- | --- | --- |
| Mechanical file splitting creates shallow modules | Complexity moves without depth | deletion test, one-interface rule, responsibility inventory |
| Coordinator gains a huge adapter interface | Caller complexity moves to a trait | private internal seams, two-adapter justification, interface audit |
| Duplicate route-open paths survive | contracts drift and cleanup diverges | one coordinator for dispatch and resolver, wrapper ledger, final zero-wrapper gate |
| `DaemonState` movement changes visibility or lifecycle | runtime races or cleanup regression | compatibility re-export, focused control-plane tests, serial end-to-end suite |
| Renderer cancellation regresses | queue appears free while page JavaScript continues | renderer deadline and immediate recovery regression from Plan 0097 |
| Timeout cleanup closes a retained browser | session loss and replacement process | ownership-class fixtures: transaction-created state may be compensated; reused or established state survives; Plan 0097 daemon cleanup remains process-exit-only |
| Provider fallback becomes normal success | false control claim or duplicate browser | explicit best-effort type, proof state, no-duplicate assertion |
| Durable URL leaks ephemeral or private identity | security and stale-link failure | opaque handoff fixture and authenticated resolver test |
| Parallel candidate plans edit overlapping files | lost work or duplicate semantics | preflight ownership reconciliation and narrow commits |
| Architecture budget rewards moving to a renamed monolith | no improvement in locality | responsibility allowlist, reverse-import gate, per-module deletion tests |
| Large test movement weakens coverage | false green refactor | move tests with behavior, interface fixtures first, report exact counts |
| Live proof disturbs authenticated browsers | operator session loss | neutral fixture, effect-boundary review, before and after PID and tab inventory |

## Explicit Non-Goals

- Do not change the public action vocabulary or remove supported browser
  behavior to meet the size budget.
- Do not redesign Guacamole, XRDP, dashboard authentication, or provider
  routing.
- Do not replace the JSON Service State store or migrate its schema solely for
  this refactor.
- Do not merge foreign CDP browsers into service-owned lifecycle.
- Do not introduce speculative ports with only one adapter.
- Do not create one shallow module per action.
- Do not move domain logic into the dashboard, HTTP, MCP, CLI parser, generated
  client, or compatibility shim.
- Do not weaken profile exclusivity, route proof, cleanup ownership, timeout,
  cancellation, or explicit-close gates.
- Do not treat a lower line count, successful compile, or passing focused test
  subset as campaign completion.
- Do not formalize a release, tag, or public upstream pull request.

## Completion Criteria

Plan 0101 is complete only when current evidence proves all of the following:

1. Route-bound open is one deep module interface used by command dispatch and
   durable handoff resolution.
2. The full planned, success, failure, rollback, resolution, and provider
   fallback matrix passes through that interface.
3. Public contracts and installed-runtime behavior remain compatible and
   current, or every reviewed contract change is documented across all
   required surfaces.
4. Renderer timeout, cancellation, supervised compensation join, queue
   release, transaction-created cleanup, reused-state preservation, and Plan
   0097 process-exit-only established-BrowserManager cleanup invariants pass.
5. Durable handoff identity, authentication, explicit-close behavior, and
   provider fallback pass.
6. The responsibility inventory accounts for every baseline production
   definition and has no unclassified or compatibility-wrapper entry.
7. All ten final architecture gates pass, including the 2,500-line and
   35-definition budgets.
8. No extracted module imports `actions`, and no typed domain logic leaks back
   into dispatch.
9. Focused, structural, canonical Rust, selected dashboard and client, and
   required serial end-to-end validations pass with exact counts.
10. When installed proof is authorized, the validated executable, dashboard,
    service state, retained browser inventory, doctors, and neutral live smoke
    agree without unexplained browser or tab mutation.
11. Plan and work audits stop after their two bounded cycles, with every
    residual finding recorded and adjudicated.
12. ROADMAP, plan closeout, developer architecture documentation, and current
    source agree on ownership and remaining work.

Route-bound open extraction alone does not satisfy completion. Plan 0101 stays
open while `actions.rs` remains a monolith, while a renamed replacement
monolith exists, or while a caller still coordinates typed domain steps that
belong behind a deep module interface.
