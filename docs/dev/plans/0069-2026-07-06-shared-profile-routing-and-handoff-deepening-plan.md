# Plan 0069: Shared Profile Routing And Handoff Deepening

State: OPEN

Created: 2026-07-06

Lane: P44 follow-up

Source artifacts:

- `/tmp/architecture-review-agent-browser-2026-07-06T15-20-00.html`
- `docs/dev/notes/2026-07-06-last30days-profile-routing-failure.md`
- `docs/dev/plans/0037-2026-06-19-runtime-profile-sharing-plan.md`
- `docs/dev/plans/0067-2026-07-05-rdp-reattachment-stress-hardening-plan.md`
- `docs/dev/plans/0068-2026-07-06-operator-handoff-and-one-time-profile-hardening-plan.md`

## Goal

Make profile reuse boring and deterministic:

- `agent-browser open` must honor explicit runtime profile, browser build, and
  browser host routing facts before any launch selector or service default can
  choose a different profile.
- Operators and software clients must be able to open a new tab against a
  runtime profile that is already in use by a retained browser, instead of
  treating the profile lock as a reason to refuse the work.
- The route-bound handoff path should deepen into one module that owns planning,
  shared-profile acquisition, operator-visible proof, finalization, and recovery
  rather than leaving callers to understand profile locks, route hints, retained
  browser reuse, selected tabs, and Guacamole/RDP proof separately.

## Product Invariants

1. A runtime profile directory remains exclusive to one Chrome process group by
   default.
2. Profile sharing means retained-browser tab or window sharing through
   service-owned route hints, service tab handles, viewer leases, and queued
   control.
3. A client that asks for `runtimeProfile=last30days-facebook` must either:
   - reuse or launch that profile; or
   - fail closed with requested and planned profile evidence.
4. A profile lock on the requested profile is not automatically fatal when a
   compatible retained service browser owns the profile. The default response
   should be to route a new tab through that owner.
5. A profile lock on some other profile is never evidence that the requested
   profile is logged out or unavailable.

## Current Failure Shape

The `last30days` X check used plain `open`:

```bash
agent-browser --json \
  --session x-login-check \
  --runtime-profile last30days-facebook \
  --browser-host remote_headed \
  --browser-build stealthcdp_chromium \
  open https://x.com/home
```

The requested profile database still contained X auth indicators, but the live
probe attempted to launch `stealthcdp-default` and failed on that profile lock.
That made a routing failure look like an X authentication failure.

The likely class is broader than `last30days`: plain `open` parses to a minimal
`navigate` command while launch identity and posture are mostly carried by
daemon environment and later selectors. Service-request and remote-view paths
already have stronger explicit-profile protections, but plain `open` can still
leave too much profile intent implicit.

## Architecture Review Recommendations Applied

### Route-Bound Handoff Module

Implement this first. The external interface should accept one handoff request
and return one authoritative result:

- selected profile identity;
- selected or reused browser/session route;
- selected tab or opened tab;
- selected route/display when remote view is requested;
- operator-visible proof when an operator route is involved;
- typed blocker with requested versus planned evidence when it cannot proceed.

Internal implementation can keep the current route acquisition, proof,
attachability, lease, and finalization helpers, but callers should not need to
coordinate those facts.

### Workspace Inventory Module

Use this plan to remove dashboard/client ambiguity around profile sharing. The
inventory projection should make a shared-profile browser row say whether the
next operation is:

- open a new tab in this retained browser;
- focus or reuse an existing compatible tab;
- wait for a lease;
- take over or reconnect a viewer;
- launch a new browser because no compatible holder exists.

The viewport and workspace rail should consume this projected actionability
instead of inferring profile locks, route ownership, or viewer state from raw
stream shape.

### Contract Catalog Module

Only make contract changes needed for this lane. If a new public request or
response field is required, update the schema once and regenerate Rust metadata
and TypeScript helpers from that authority. Do not add another manually synced
field family without a test proving drift is caught.

## Slice A: Plain `open` Routing Identity

Status: implemented for plain `open`, `goto`, and `navigate`

Goal: make plain `open` carry explicit launch identity and posture into the
auto-launch path so service defaults cannot silently replace them.

Work:

- Extend command parsing so plain `open`, `goto`, and `navigate` preserve these
  global CLI flags on the command payload when supplied:
  - `runtimeProfile`;
  - `profileId`;
  - `profile`;
  - `browserBuild`;
  - `browserHost`;
  - `viewStreamProvider`;
  - `controlInputProvider`;
  - `displayIsolation`.
- Ensure `launch_command_with_effective_service_defaults` and
  `apply_auto_launch_command_hints` treat these fields as explicit caller
  authority.
- On launch failure, include compact diagnostic fields:
  - `requestedRuntimeProfile`;
  - `plannedRuntimeProfile`;
  - `requestedUserDataDir`;
  - `plannedUserDataDir`;
  - selector source that changed or preserved the plan.
- Add parser and no-launch launch-planning tests for the exact shape from
  `docs/dev/notes/2026-07-06-last30days-profile-routing-failure.md`.

Acceptance:

- A plain `open` command with `--runtime-profile last30days-facebook` cannot
  plan `stealthcdp-default`. Covered by
  `test_open_preserves_runtime_profile_when_default_profile_is_locked_shape`.
- A profile-lock error for another profile cannot be reported as the requested
  profile's failure.
- Existing `remote-view open` parser behavior remains unchanged. The plain
  navigation parser now preserves explicit global `--runtime-profile`,
  `--browser-build`, `--browser-host`, `--view-stream-provider`,
  `--control-input-provider`, and `--display-isolation` flags on the
  `navigate` command payload.

## Slice B: Shared-Profile Tab Acquisition Is The Default

Status: implemented and live-smoked for ordinary `open`, `goto`, `navigate`,
and HTTP/MCP `service_request` tab acquisition

Goal: when the requested runtime profile is already owned by a compatible
retained browser, open or reuse a tab through that browser instead of trying to
launch another Chrome process or refusing the operation.

Work:

- Add a shared-profile acquisition decision module behind the handoff interface.
- Inputs:
  - requested runtime profile or profile id;
  - requested browser build, host, stream provider, control provider, display
    isolation;
  - target URL and target service/account/task labels;
  - current service browser, session, tab, route, and lease state.
- Output:
  - `reuse_existing_browser`;
  - `open_shared_profile_tab`;
  - `reuse_compatible_tab`;
  - `wait_for_profile_holder`;
  - `launch_new_browser`;
  - `reject_duplicate_process`;
  - typed blocker when no safe owner exists.
- Route ordinary `open` and service-request tab actions through this decision
  before direct launch.
- Preserve the exclusive-process invariant: never launch a second Chrome
  process on the same profile unless `allowDuplicateProfileLane=true` or an
  equivalent reviewed isolation flag is explicitly set.

Acceptance:

- A retained `last30days-facebook` remote-headed browser can receive a new X tab
  while a Facebook tab remains open. Plain navigation auto-launch now selects a
  compatible same-profile retained browser with a CDP endpoint, attaches to it,
  creates a fresh active tab, and lets the existing navigation handler load the
  requested URL. Covered by
  `test_shared_profile_attach_target_selects_compatible_retained_browser`.
- HTTP and MCP `service_request` `tab_new` commands now ask the shared
  access-plan route-hint helper for a compatible retained same-profile owner
  before relay. The adapters route to the owner session using the synthesized
  `browserId` and `sessionName` hints instead of treating the requested
  profile's active owner as a duplicate-process blocker. Covered by
  `service_request_route_hints_reuse_compatible_live_browser`,
  `service_request_command_applies_shared_profile_route_hints`, and
  `service_request_tool_session_uses_browser_id_route_hint`.
- Public service tab responses already expose compact
  `sharedAcquisition` evidence for route-hinted tab opens, including reused
  browser/session and tab-open status.
- Closing or releasing the X tab does not close the Facebook tab or the browser.
  Live service-request proof passed with `pnpm test:service-request-live` using
  the debug binary via `AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD`; the smoke opened
  two same-profile service tabs through one retained browser, released one
  physical target, and then successfully evaluated the surviving tab handle.
- If the profile owner is incompatible, the response says why and names the
  owner. The current partial implementation leaves incompatible owners on the
  existing duplicate-lane rejection path, covered by
  `test_shared_profile_attach_target_ignores_incompatible_retained_browser`.
- Duplicate-process launch remains rejected by default.

Remaining: none for Slice B.

## Slice C: Route-Bound Handoff Deepening

Status: partially implemented

Goal: collapse scattered route-bound handoff knowledge into one module
interface.

Work:

- Introduce or reshape the handoff module around one public operation:
  `plan_and_acquire_handoff` or an equivalent repo-style name.
- Move call-site sequencing behind the module:
  - explicit profile preservation;
  - shared-profile acquisition;
  - route acquisition;
  - tab acquisition;
  - operator-visible proof;
  - finalization;
  - rollback and retained-browser recovery.
- Keep `actions.rs` as command dispatch and repository plumbing.
- Keep existing typed helpers as internal implementation until their seams are
  justified by more than one adapter.
- Add tests that prove deleting the module would force callers to understand the
  route, display, profile, tab, proof, and lease ordering again.

Acceptance:

- `remote-view open`, plain `open` with remote-headed posture, HTTP
  `service_request`, and MCP `service_request` use the same acquisition result
  for shared-profile and route-bound decisions.
- A successful response has one authoritative profile, browser, session, tab,
  route, display, and operator-visible proof record. `remote_view_open` now
  publishes this as `routeBoundHandoff` for dry-run plans and successful opens.
- Failure responses never mix requested profile evidence with another planned
  profile's lock or page state. Operator-visible proof failures now include a
  focused `routeBoundHandoff` failure diagnostic for the failing route binding;
  final post-checkout proof failures keep pre-checkout evidence under the
  separate `preCheckoutOperatorVisible` label.

Progress:

- Added `cli/src/native/remote_view_handoff.rs` as the named module for the
  route-bound handoff proof record.
- Wired `remote_view_open` dry-run and success responses to the handoff record
  so clients can read one profile/browser/session/tab/route/display/proof
  surface instead of reconstructing it from sibling fields.
- Added focused coverage for the handoff record's authoritative profile
  selection and for the dry-run response shape.
- Moved `remote_view_open` planned and opened response assembly behind
  `remote_view_handoff::{planned_route_bound_handoff_response,
  opened_route_bound_handoff_response}`. `actions.rs` still dispatches the
  command and performs repository/browser calls, but it no longer assembles the
  authoritative profile, browser, session, route, display, tab,
  operator-visible proof, and verification response surfaces field by field.
- Moved route-bound cleanup/rollback summary formatting behind
  `remote_view_handoff::route_bound_handoff_cleanup_summary`, so failure
  responses report cleanup and lease rollback from the same handoff module that
  owns planned/opened response records. The repository rollback mutation itself
  remains in `actions.rs` pending the next sequencing pass.
- Moved route-bound acquisition completion, lease restoration, rollback
  mutation, and post-cleanup rollback update behind
  `remote_view_handoff::{complete_route_bound_handoff_acquisition,
  restore_route_bound_handoff_lease_if_missing,
  rollback_route_bound_handoff_acquisition,
  update_route_bound_handoff_acquisition_cleanup}`. The action dispatcher keeps
  timestamp generation and browser/repository orchestration, but the handoff
  module now owns the lease finalization and rollback state mutations.
- Moved begin-acquisition lease reservation behind
  `remote_view_handoff::begin_route_bound_handoff_acquisition`. The action
  dispatcher still supplies the observation timestamp and provider-derived
  default control-input adapter, while the handoff module owns pending
  route-pool, display-allocation, route, and lease repository mutations.
- Moved retained-browser failure cleanup decision and result vocabulary behind
  `remote_view_handoff::{route_bound_handoff_failure_cleanup_plan,
  route_bound_handoff_skipped_failure_cleanup,
  route_bound_handoff_failure_cleanup_result}`. `actions.rs` still executes the
  selected async browser command, but the handoff module now decides whether a
  failure should close only the opened tab on a reused browser, close a newly
  launched browser, or skip cleanup when no opened-tab index is available.
- Moved route-bound failure rollback and browser-cleanup summary sequencing
  behind `remote_view_handoff::{rollback_route_bound_handoff_failure,
  complete_route_bound_handoff_failure_cleanup}`. `actions.rs` still performs
  the async browser side effect, but tab, focus, proof, checkout, and final
  proof failures now enter a single handoff rollback/cleanup path instead of
  open-coding lease restoration, rollback mutation, cleanup attachment, and
  summary formatting in each branch.
- Moved rollback cleanup payload vocabulary for tab, focus, proof, checkout,
  operator-visible, and final operator-visible failures behind
  `remote_view_handoff::{route_bound_handoff_pending_rollback_cleanup,
  route_bound_handoff_operator_visible_failure_cleanup,
  route_bound_handoff_final_operator_visible_failure_cleanup}`. `actions.rs`
  still executes rollback and browser cleanup, but it no longer hand-builds the
  failure cleanup JSON shapes.
- Moved final route-binding derivation after route checkout behind
  `remote_view_handoff::final_route_bound_handoff_route_binding`. The handoff
  module now owns the merge from planned binding, checkout route readback, and
  route-pool readback into the authoritative final binding used by
  `operatorVisible`, `routeBoundHandoff`, and opened responses.
- Moved route-bound browser-build proof finalization behind
  `remote_view_handoff::route_bound_handoff_browser_build_proof`. The handoff
  module now owns selected-build, executable-path, and mismatch evidence before
  it is attached to `routeBoundHandoff` and opened responses.
- Moved the reused-browser launch-result evidence for already checked-out
  route-bound browsers behind
  `remote_view_handoff::route_bound_handoff_reused_browser_launch_result`.
  `actions.rs` still decides whether to reuse the current browser, but the
  handoff module now owns the `"reused"` launch result shape with browser,
  session, route, display, and reason evidence.
- Moved route-bound launch, tab, focus, and route-checkout command
  construction behind
  `remote_view_handoff::{route_bound_handoff_launch_command,
  route_bound_handoff_tab_command, route_bound_handoff_focus_command,
  route_bound_handoff_checkout_command}`. `actions.rs` still executes the
  browser and service commands, but the handoff module now owns the command
  artifacts that carry route, display, provider, session, and tab target
  evidence through the plan/open sequence.
- Moved visible-window proof enrichment of the route-checkout command behind
  `remote_view_handoff::route_bound_handoff_checkout_command_with_visible_window_proof`.
  `actions.rs` still executes checkout, but the handoff module now owns the
  readiness and display-content payload attached to checkout after visible
  browser-window proof.
- Added `remote_view_handoff::route_bound_handoff_plan`, returning a
  `RouteBoundHandoffPlan` that groups the normalized route binding with launch,
  tab, and checkout command artifacts. `actions.rs` now receives one handoff
  plan after acquisition-plan selection instead of normalizing the route
  binding and constructing sibling command values locally.
- Added handoff-plan acquisition helpers
  `remote_view_handoff::{begin_route_bound_handoff_plan_acquisition,
  complete_route_bound_handoff_plan_acquisition}`. `actions.rs` now supplies
  the observation timestamp and execution context while the handoff module owns
  default control-input selection, begin-acquisition mutation, missing-lease
  restoration, and acquisition completion for the plan path.
- Added a route-bound `sharedAcquisition` record to `remote_view_open` planned
  and opened responses via
  `remote_view_handoff::route_bound_handoff_shared_acquisition`. It uses the
  same top-level acquisition-result name consumed by access-plan and
  service-request tab responses, while preserving route/display evidence under
  `routeBoundHandoff`.
- Moved `tab_new` shared acquisition evidence and route-bound
  `remote_view_open` shared acquisition evidence through the same
  `remote_view_handoff::shared_profile_acquisition_result` builder. HTTP and
  MCP `service_request` `tab_new` responses and route-bound `remote_view_open`
  responses now share one acquisition-result JSON constructor instead of
  maintaining parallel response shapes.
- Routed plain remote-headed `open`/`navigate` shared-profile auto-launch
  through the same named acquisition-result builder. When auto-launch attaches
  to a compatible retained same-profile browser and opens a tab, the following
  navigation response now includes `sharedAcquisition` with the selected owner
  browser/session, requested/planned profile evidence, duplicate-process
  policy, and `routeHintSource: shared_profile_auto_launch`.
- Extended `summarizeServiceSharedProfileAcquisition()` so software clients can
  summarize route-bound `remote_view_open` responses from `data.intent`,
  `data.sharedAcquisition`, nested `data.tab`, and nested `serviceTabHandle`
  without parsing the route-bound response by hand.
- Restored `remote-view open` parser preservation of global
  `--browser-build`; the broad `remote_view_open_` test filter now covers that
  `--browser-build stealthcdp_chromium` survives both global and subcommand
  placement.
- Moved pre-launch display-access and post-launch browser-launch failure
  cleanup payloads behind
  `remote_view_handoff::{route_bound_handoff_pre_launch_failure_cleanup,
  route_bound_handoff_launch_failure_cleanup}`. `actions.rs` still performs
  rollback and launch execution, but the handoff module now owns the
  skipped-before-launch and skipped-after-launch cleanup shapes.
- Moved operator-visible proof failure diagnostic construction behind
  `remote_view_handoff::{route_bound_handoff_operator_visible_failure,
  route_bound_handoff_final_operator_visible_failure}`. `actions.rs` still
  detects the failure state and executes rollback/cleanup, but the handoff
  module now builds the paired error text and cleanup payload that preserve
  `routeBoundHandoff` and `preCheckoutOperatorVisible` labels.
- Moved route-bound failure recovery staging behind
  `remote_view_handoff::{begin_route_bound_handoff_failure_recovery,
  route_bound_handoff_immediate_failure}`. `actions.rs` still executes the
  selected async tab/browser close command, but the handoff module now owns
  rollback-before-cleanup ordering, cleanup-plan selection, skipped-cleanup
  detection, and immediate display/launch failure summaries.
- Moved successful route-bound open finalization behind
  `remote_view_handoff::complete_route_bound_handoff_open`. `actions.rs` still
  performs the final operator-visible check and command dispatch, but the
  handoff module now owns lease completion, browser-build proof derivation,
  lease serialization, and opened response assembly.
- Moved operator-visible readiness gating behind
  `remote_view_handoff::{route_bound_handoff_operator_visible_failure_if_not_ready,
  route_bound_handoff_final_operator_visible_failure_if_not_ready}`. `actions.rs`
  still computes operator-visible proof, but the handoff module now decides
  whether pre-checkout and final proof states should produce rollback
  diagnostics.
- Moved opened-response final route-binding derivation inside
  `remote_view_handoff::complete_route_bound_handoff_open`. `actions.rs` still
  derives a final binding for final operator-visible proof, but the handoff
  completion path now interprets checkout readback itself before assembling the
  authoritative opened response.
- Moved post-checkout proof sequencing behind
  `remote_view_handoff::route_bound_handoff_post_checkout_proof`. `actions.rs`
  still supplies the operator-visible proof calculation and rollback execution,
  but the handoff module now derives the final route binding, invokes the proof
  calculation, and applies the final proof readiness gate as one step.
- Moved checkout failure diagnostic preparation behind
  `remote_view_handoff::route_bound_handoff_checkout_failure`. `actions.rs`
  still executes the async checkout command and rollback cleanup, but the
  handoff module now owns the checkout-failure phase and rollback cleanup
  payload.
- Moved simple route-bound rollback failure descriptors for tab open, focus,
  and visible-window proof behind
  `remote_view_handoff::{route_bound_handoff_tab_open_failure,
  route_bound_handoff_focus_failure,
  route_bound_handoff_visible_window_proof_failure}`. `actions.rs` still
  executes the async browser commands and rollback cleanup, but the handoff
  module now owns those failure phases and rollback cleanup payloads.
- Moved operator-visible proof record assembly behind
  `remote_view_handoff::route_bound_handoff_operator_visible`. `actions.rs`
  still computes the visible-window proof and live tab readback, but the
  handoff module now owns the route, display, browser, tab, stream, Guacamole,
  and URL-readiness response vocabulary used by dry-run, pre-checkout, and
  final proof paths.
- Moved route-bound failure cleanup task construction behind
  `remote_view_handoff::{route_bound_handoff_failure_cleanup_task,
  route_bound_handoff_failure_cleanup_task_result}`. `actions.rs` still
  dispatches the async tab-close or browser-close side effect, but the handoff
  module now owns the cleanup task vocabulary, close-tab command payload, close
  browser command marker, skipped-cleanup payload, and cleanup result mapping.
- Extended `remote_view_handoff::begin_route_bound_handoff_failure_recovery`
  so the recovery result carries the selected cleanup task. `actions.rs` still
  performs the async cleanup side effect, but it no longer interprets
  `cleanup_plan` or `skipped_cleanup` to decide what cleanup operation the
  recovery path selected.
- Audited the remaining `remote_view_open` route-bound orchestration after the
  recovery extraction. The remaining action-local work is now command
  dispatch, live browser side effects, timestamp supply, and
  repository/service plumbing. Removed stale action-local rollback/update
  wrappers so tests call the handoff rollback API directly instead of keeping
  duplicate dispatcher vocabulary.

Remaining:

- Slice C is ready for live proof. Any further extraction should be driven by
  a concrete live-proof failure or a second adapter needing the same sequence,
  not by moving dispatcher-owned async side effects into the handoff module.
- Run Slice F live proof against the installed binary/runtime after the
  remaining handoff sequencing is converged.

## Slice D: Workspace Inventory Actionability

Status: implemented for workspace inventory actionability

Goal: make the dashboard and clients show the correct shared-profile operation
instead of presenting profile-in-use as a dead end.

Work:

- Extend workspace node projection with shared-profile actionability:
  - `openSharedProfileTab`;
  - `reuseCompatibleTab`;
  - `waitForProfileHolder`;
  - `rejectDuplicateProcess`;
  - `takeOverViewer`;
  - `routeSwitch`.
- Show profile owner, profile class, active tabs, route/display ownership, and
  recommended action in the selected workspace context.
- Keep raw lock and route diagnostics visible, but make the recommended action
  come from the inventory projection.
- Add dashboard tests for:
  - Facebook and X tabs in one profile/browser;
  - profile owner incompatible with requested browser build;
  - profile owner compatible but viewer lease taken over elsewhere;
  - duplicate process rejected but new tab enabled.

Acceptance:

- The dashboard no longer tells an operator that a profile already in use is
  inherently unavailable when a retained-browser tab can be opened. Workspace
  browser rows now carry `profileActionability.recommendedAction` and expose
  `openSharedProfileTab` plus an enabled `add-tab` action when the live
  service-owned retained browser is the compatible profile owner.
- Workspace rows distinguish "profile locked by our retained browser, route via
  it" from "profile locked by unknown process, inspect or close owner".
  Profile-only conflict rows now carry `waitForProfileHolder` or
  `rejectDuplicateProcess` actionability with holder/session evidence instead
  of relying on raw lock state.
- Workspace rows also distinguish viewer-control and route-switch cases. A
  browser row with an active viewer controller lease now recommends
  `takeOverViewer`, and a row whose stream attachability asks for route
  switching now recommends `routeSwitch` instead of exposing `add-tab` as the
  wrong next operation.

Progress:

- Added `WorkspaceProfileActionability` to the dashboard workspace inventory
  projection.
- Exposed shared-profile actionability in workspace row search and selected
  row detail.
- Enabled service-owned browser row `add-tab` only when the projection says the
  retained profile owner can accept another tab.
- Wired the service-owned browser row `add-tab` action to HTTP
  `service_request` `tab_new` with the retained owner `browserId`,
  `sessionName`, runtime profile, and actionability evidence. After the request
  returns, the dashboard refreshes service state and selects the returned
  browser/tab identity.
- Added focused workspace-node coverage for viewer-controller lease takeover
  and route-switch actionability. Those rows keep `add-tab` disabled and show
  the lease or attachability reason as the recommended next operation.
- Updated README, dashboard docs, and the agent-browser skill with the
  retained-owner versus duplicate-process distinction.

Remaining:

- None for Slice D no-launch workspace inventory actionability. Live
  route-switch behavior remains part of Slice F end-to-end proof.

## Slice E: Contract Catalog And Client Ergonomics

Status: implemented for shared-profile helper summaries

Goal: make software clients choose the shared-profile path without knowing
agent-browser internals.

Work:

- If needed, add a compact `sharedProfileAcquisition` record to service access
  plan and service request responses.
- Generate service request and observability client helpers from schemas.
- Add a helper summary for downstream clients:
  - requested profile;
  - planned profile;
  - acquisition mode;
  - retained browser/session route hints;
  - tab handle;
  - duplicate-process policy.
- Update docs and `skills/agent-browser/SKILL.md` so clients prefer
  access-plan or service-request shared-profile acquisition over direct
  plain-open probes for authenticated profile checks.

Acceptance:

- A client can request "open X using last30days-facebook" and receive a
  tab-handle or route-hint plan without parsing raw service state.
- Generated client tests cover shared-profile helper summaries.

Progress:

- Access-plan `decision.profileReuse.sharedAcquisition` and service-request
  `tab_new` response `data.sharedAcquisition` already expose the route-hinted
  retained-browser acquisition record.
- Added generated client type coverage for
  `ServiceSharedProfileAcquisitionSummary`.
- Added `summarizeServiceSharedProfileAcquisition()` to
  `@agent-browser/client/service-request`. It accepts either an access-plan
  response or a tab response and returns requested profile, planned profile,
  runtime profile, profile id, acquisition mode, retained browser/session route
  hints, tab/target ids, service tab handle, route-hint requirement,
  duplicate-process policy, and a compact log summary.
- Added focused service-request client tests for shared-profile helper
  summaries from both an access-plan response and a tab response.
- Updated README, docs site service-mode guidance, and
  `skills/agent-browser/SKILL.md` so software clients use the helper instead
  of parsing nested `decision.profileReuse` or action-specific response data.

Remaining:

- None for Slice E.

## Slice F: Validation And Live Proof

Status: complete

No-launch validation:

- `cargo test --manifest-path cli/Cargo.toml open_preserves_runtime_profile -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml shared_profile -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `pnpm test:service-client`
- `pnpm test:dashboard-workspace-nodes`
- `pnpm test:dashboard-profile-allocation`
- `pnpm test:route-confusion-gates`
- `git diff --check`

Progress:

- Added live-discovered shared-profile regression coverage for plain `open`
  requests against a profile already owned by the same service session. The
  attach target selector now accepts `open` alongside `navigate` and `tab_new`,
  and prefers the current session's live owner record before falling back to
  other compatible retained browsers. This fixes the observed Guacamole route
  viewer refusal where `rdp-guac-route-a-viewer` already owned
  `/home/ecochran76/.agent-browser/guacamole-route-viewers/a`.
- Added remote-view acquisition coverage for repeat opens against a same-owner
  checked-out route and for the stale-route shape where the route is marked
  `orphaned` by reconciliation while the browser, session, display allocation,
  and route id still agree. Same-owner explicit repeats now reuse the route
  instead of failing with `route_pool_entry_unavailable`.
- Repaired live route-display proof by overriding stale local route display
  env (`:13` / `:14`) with the current inspected route displays (`:10` /
  `:11`) for the proof run. Route-pool readiness was green with abstract X11
  sockets `@/tmp/.X11-unix/X10` and `@/tmp/.X11-unix/X11`.
- Full live fixture proof passed with:
  `/tmp/agent-browser-remote-view-open-live-2026-07-06T22-14-26-356Z`.
  The proof used `./cli/target/debug/agent-browser` against the installed
  service/runtime environment, route `guacamole:4`, display allocation
  `remote-view-display:10`, display `:10`, fixture URL
  `http://127.0.0.1:38187/`, one active intended target
  `1C6AF67A7E98EE208D14AADBAA9F2773`, handoff classification
  `route_bound_ready`, visual state `browser_window_visible`, and OCR text
  containing `REMOTE VIEW OPEN FIXTURE 55948`.

Validation run during live-proof completion:

- `cargo test --manifest-path cli/Cargo.toml open_preserves_runtime_profile -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml shared_profile -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml shared_profile_attach_target -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml acquisition_plan_reuses_same_owner -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo build --manifest-path cli/Cargo.toml`
- `pnpm test:service-client`
- `pnpm test:dashboard-workspace-nodes`
- `pnpm test:dashboard-profile-allocation`
- `pnpm test:route-confusion-gates`
- `AGENT_BROWSER_RDP_ROUTE_A_DISPLAY_NAME=:10 AGENT_BROWSER_RDP_ROUTE_B_DISPLAY_NAME=:11 AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD=./cli/target/debug/agent-browser pnpm test:rdp-guac-route-pool-readiness`
- `AGENT_BROWSER_RDP_ROUTE_A_DISPLAY_NAME=:10 AGENT_BROWSER_RDP_ROUTE_B_DISPLAY_NAME=:11 AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD=./cli/target/debug/agent-browser pnpm test:remote-view-open-fixture-live`
- `git diff --check`

Remaining:

- None for Plan 0069.

Live validation:

- Create or reuse a non-secret synthetic shared profile.
- Launch one retained remote-headed browser for that profile.
- Open tab A against `https://www.facebook.com/` or a neutral fixture standing
  in for Facebook.
- Open tab B against `https://x.com/home` or a neutral fixture standing in for
  X.
- Prove both tabs are present and attributable to the same profile and browser
  process.
- Prove route/display/operator-visible evidence remains correct for the active
  remote view.
- Prove closing tab B does not close tab A or the retained browser.
- Prove a direct duplicate process launch still fails closed by default.

Operator closeout proof:

- Include the final artifact directory.
- Include requested and planned profile ids for both operations.
- Include browser id, session name, tab ids, target ids, route id, display
  allocation id, and operator-visible state.
- Include a negative duplicate-process proof.

## Non-Goals

- Do not make two independent Chrome process groups share one authenticated
  profile directory.
- Do not make X and Facebook domain-specific selectors part of the generic
  acquisition module.
- Do not broaden this slice into a release.
- Do not require live private cookies or cookie values in tests or fixtures.

## Closeout Criteria

- The routing bug note is updated or linked with the implemented remediation.
- The architecture review top recommendation has an implemented first slice:
  route-bound handoff owns shared-profile and proof sequencing.
- Operators can open a new tab against a retained in-use profile by default.
- Plain `open` no longer loses explicit runtime profile, browser host, or
  browser build intent.
- Docs, generated clients, and dashboard actionability agree on the
  shared-profile acquisition contract.
