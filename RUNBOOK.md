# Runbook

This file records dated execution turns for repo governance, planning, release,
and operational handoff work. Detailed command output belongs in validation
notes or artifacts, not in this log.

## Turn 110 | 2026-07-19

Scope: complete P76 source, contract, installed-runtime, and closeout gates.

Actions:

- Completed bounded clipboard-write capture, daemon-owned dependent batches,
  per-command timing fields, browser accessibility-tree role lookup, and
  bounded closed-tab status projection with full diagnostic retrieval.
- Updated every required CLI help, README, skill, docs-site, schema, contract,
  HTTP, MCP, generated-client, and inline documentation surface.
- Added and passed a real Chrome accessibility fixture for dynamically mounted
  `aria-labelledby` content and supported shadow-root lookup.
- Corrected two stale close-action tests to match intentional removal of
  `NotStarted` browser placeholders and empty released sessions.
- Used the installed smoke to find and repair missing
  `closedTabProjection` metadata on the CLI-local no-launch status path.
- Published the local dashboard runtime, retired stale daemon sessions, and
  verified a converged installed runtime with `agent-browser install doctor`.
- Queued one compact Graphiti closeout episode in `agent_browser_main` from the
  completed plan and redacted incident note after provider readiness passed.

Validation:

- `scripts/ci/rust-tests.sh`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- focused clipboard, CDP lifecycle, dependent batch, service projection, and
  real Chrome accessibility tests
- `pnpm --config.verify-deps-before-run=false test:service-client`
- `pnpm --config.verify-deps-before-run=false test:service-api-mcp-parity`
- repeated successful docs and dashboard production builds
- installed live capture, timeout recovery, dependent batch, status projection,
  and doctor readbacks with a temporary profile

Result:

- P76 is closed. All six slices are implemented and documented.
- The unresolved clipboard promise returned within the bounded deadline, a
  following evaluation succeeded on the same target, and opt-in write capture
  restored the patched method.
- Installed ordinary and full status modes returned their respective
  projection metadata. Final install doctor reported no issues and zero stale
  runtimes.
- The privacy-safe closeout evidence is recorded in Plan 0076 and the incident
  note. The temporary validation profile was removed.

## Turn 109 | 2026-07-19

Scope: open and execute P76 clipboard target recovery and interaction
performance remediation.

Actions:

- Reviewed the retained clipboard incident against the current clipboard,
  CDP timeout, evaluation, locator, batch, and service-status implementations.
- Opened Plan 0076 with six bounded slices covering evidence correction,
  cancellation-safe clipboard timeout and recovery, clipboard-write capture,
  timing and dependent batching, accessible locator repair, and closed-tab
  status projection.
- Made the CDP command lifecycle the deep module for deadline enforcement,
  pending-command cleanup, late responses, and timeout classification.
- Recorded review mediation so empty clipboard text remains successful, target
  recovery must be proved, locator coverage reproduces accessible-name
  behavior, and service-status compaction remains a projection rather than a
  mutation of persisted lifecycle authority.
- Completed Slice A by correcting causal language in the incident note,
  labeling historical observations, replacing the insufficient portal-only
  locator regression, and adding a privacy-safe validation artifact template.
- Completed Slice B source work with a cancellation-safe per-command CDP
  deadline, Chrome renderer timeout, execution termination fallback, normal
  evaluation health probe, successful empty-text output, stable failure codes,
  and explicit replacement-tab guidance.
- Updated CLI help, README, docs command and streaming pages, MCP tool
  description, and repo plus installed skill guidance for the bounded read
  contract.

Validation:

- `git diff --check`
- `cargo test --manifest-path cli/Cargo.toml clipboard -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml native::cdp::client::tests -- --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm --config.verify-deps-before-run=false test:service-api-mcp-parity`
- `pnpm --config.verify-deps-before-run=false --dir docs build`
- `pnpm validation:select -- --base HEAD` was blocked before selection by
  pnpm 11 ignored-build enforcement. The underlying selector script completed
  directly without approving dependency build scripts.

Result:

- P76 is open and in progress. Slices A and B source work are complete. Slice C
  clipboard-write capture is the current execution boundary; Slice B installed
  retained-browser proof remains a final closeout gate.

## Turn 108 | 2026-07-06

Scope: close P69 Slice F live proof and fix live-discovered shared-profile and
route repeat-open failures.

Actions:

- Reproduced the in-use profile refusal through
  `scripts/open-rdp-guac-route-displays.js`: plain `open` against
  `/home/ecochran76/.agent-browser/guacamole-route-viewers/a` failed even
  though service state showed `session:rdp-guac-route-a-viewer` already owned
  the profile and exposed a CDP endpoint.
- Updated shared-profile auto-launch target selection so `open` participates in
  retained-browser attach/reuse and so the current session's live service
  browser can be selected when daemon metadata drift leaves `state.browser`
  empty.
- Reproduced the P69 route repeat bug in the full fixture smoke: first
  `remote_view open` checked out `guacamole-rdp-a`, while repeat open failed
  with `route_pool_entry_unavailable`.
- Updated remote-view acquisition to treat same-owner `checked_out` and
  reconciliation-stale `orphaned` route records as reusable when browser id,
  session id, route id, and display allocation still agree.
- Overrode stale route-display env for the live proof with inspected route
  displays `:10` and `:11`.

Validation:

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

Result:

- Full fixture live proof passed with artifact
  `/tmp/agent-browser-remote-view-open-live-2026-07-06T22-14-26-356Z`.
  It proved route `guacamole:4`, display allocation `remote-view-display:10`,
  display `:10`, `route_bound_ready`, `browser_window_visible`, one active
  intended target, and OCR text containing `REMOTE VIEW OPEN FIXTURE 55948`.
  P69 validation is complete.

## Turn 107 | 2026-07-06

Scope: audit P69 Slice C residual `remote_view_open` orchestration and remove
stale dispatcher rollback wrappers before live proof.

Actions:

- Audited the remaining `remote_view_open` route-bound sequence after the
  handoff recovery extraction.
- Confirmed the remaining action-local responsibilities are command dispatch,
  live browser side effects, timestamp supply, and repository/service plumbing.
- Removed stale `remote_view_open_rollback_acquisition_lease`.
- Removed stale `remote_view_open_update_acquisition_lease_cleanup`.
- Updated the acquisition-rollback test to call
  `remote_view_handoff::rollback_route_bound_handoff_acquisition` directly with
  an explicit observed timestamp.
- Updated P69 to mark Slice C ready for live proof rather than continuing
  unbounded micro-extractions.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Stale dispatcher rollback vocabulary is gone. P69 remains open for Slice F
  installed-runtime/live proof.

## Turn 106 | 2026-07-06

Scope: continue P69 Slice C by moving failure recovery cleanup-task selection
into the handoff recovery result.

Actions:

- Extended `remote_view_handoff::RouteBoundHandoffFailureRecovery` with
  `cleanup_task`.
- Updated `remote_view_handoff::begin_route_bound_handoff_failure_recovery` so
  it returns the selected cleanup task alongside rollback and cleanup-plan
  evidence.
- Rewired `remote_view_open_rollback_failure_after_cleanup` so `actions.rs`
  executes the handoff-selected cleanup task directly instead of interpreting
  `cleanup_plan` and `skipped_cleanup`.
- Updated the action cleanup test to exercise the task form.
- Extended handoff-module recovery coverage to assert the selected skipped
  cleanup task.
- Updated P69 to record this recovery-result extraction.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Failure recovery now returns the cleanup task selected by the handoff module.
  P69 remains open for final sequencing assessment and Slice F live proof.

## Turn 105 | 2026-07-06

Scope: continue P69 Slice C by moving route-bound failure cleanup task
vocabulary into the handoff module.

Actions:

- Added `remote_view_handoff::RouteBoundHandoffFailureCleanupTask`.
- Added `remote_view_handoff::route_bound_handoff_failure_cleanup_task`.
- Added
  `remote_view_handoff::route_bound_handoff_failure_cleanup_task_result`.
- Rewired `remote_view_open_cleanup_after_failure` so `actions.rs` still
  dispatches async tab-close or browser-close side effects, but no longer owns
  the close-tab command payload, close-browser task marker, skipped-cleanup
  payload, or cleanup result mapping.
- Added handoff-module coverage for close-tab, close-browser, and skipped
  cleanup task shapes.
- Updated P69 to record this cleanup task extraction.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Failure cleanup task construction and cleanup result mapping are now owned by
  `remote_view_handoff`. P69 remains open for broader sequencing consolidation
  and Slice F live proof.

## Turn 104 | 2026-07-06

Scope: continue P69 Slice C by moving operator-visible proof record assembly
into the handoff module.

Actions:

- Added `remote_view_handoff::route_bound_handoff_operator_visible`.
- Moved route, display, browser, tab, stream, Guacamole, and URL-readiness
  response vocabulary out of `actions.rs`.
- Rewired `remote_view_open` dry-run, pre-checkout proof, final proof, and
  related route-bound tests to use the handoff-owned proof builder.
- Added handoff-module coverage for the operator-visible proof record shape.
- Updated P69 to record the proof assembly extraction.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Operator-visible route-bound proof vocabulary is now owned by
  `remote_view_handoff`. P69 remains open for broader sequencing consolidation
  and Slice F live proof.

## Turn 103 | 2026-07-06

Scope: continue P69 Slice C by moving the remaining simple rollback failure
descriptors into the handoff module.

Actions:

- Added `remote_view_handoff::route_bound_handoff_tab_open_failure`.
- Added `remote_view_handoff::route_bound_handoff_focus_failure`.
- Added `remote_view_handoff::route_bound_handoff_visible_window_proof_failure`.
- Rewired the `remote_view_open` tab, focus, and visible-window proof failure
  branches so `actions.rs` still executes async browser commands and rollback
  cleanup, but no longer owns those failure phase strings or rollback cleanup
  payloads.
- Added handoff-module coverage for the simple rollback failure descriptor
  shapes.
- Updated P69 to record this descriptor consolidation.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Tab-open, focus, visible-window proof, and checkout failure descriptors now
  share the handoff module shape. P69 remains open for broader sequencing
  consolidation and Slice F live proof.

## Turn 102 | 2026-07-06

Scope: continue P69 Slice C by moving checkout failure diagnostic preparation
into the handoff module.

Actions:

- Added `remote_view_handoff::RouteBoundHandoffRollbackFailure`.
- Added `remote_view_handoff::route_bound_handoff_checkout_failure`.
- Rewired the `remote_view_open` checkout failure branch so `actions.rs` still
  executes the async checkout command and rollback cleanup, but no longer owns
  the checkout failure phase string or rollback cleanup payload.
- Added handoff-module coverage for the checkout failure phase and cleanup
  payload.
- Updated P69 to record this checkout failure diagnostic extraction.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Checkout failure phase and rollback cleanup payload construction now live in
  the handoff module. P69 remains open for broader sequencing consolidation and
  Slice F live proof.

## Turn 101 | 2026-07-06

Scope: continue P69 Slice C by moving post-checkout proof sequencing into the
handoff module.

Actions:

- Added `remote_view_handoff::RouteBoundHandoffPostCheckoutProof`.
- Added `remote_view_handoff::RouteBoundHandoffPostCheckoutProofInput`.
- Added `remote_view_handoff::route_bound_handoff_post_checkout_proof`.
- Rewired `remote_view_open` so `actions.rs` supplies the final
  operator-visible proof calculation and executes rollback when needed, while
  the handoff module derives the final route binding, invokes the proof
  calculation, and applies the final proof readiness gate.
- Added handoff-module tests for ready and not-ready post-checkout proof
  results.
- Updated P69 to record this post-checkout proof sequencing extraction.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Final route-binding derivation, final proof calculation invocation, and final
  proof readiness gating now run as one handoff step after checkout. P69 remains
  open for broader sequencing consolidation and Slice F live proof.

## Turn 100 | 2026-07-06

Scope: continue P69 Slice C by moving opened-response final route-binding
derivation into the handoff completion path.

Actions:

- Changed `remote_view_handoff::CompleteRouteBoundHandoffOpenInput` so callers
  no longer pass a precomputed final route binding.
- Updated `remote_view_handoff::complete_route_bound_handoff_open` to derive
  the final route binding from checkout readback before completing the lease and
  assembling the opened response.
- Rewired `remote_view_open` to pass only the planned route binding and checkout
  readback into the completion helper.
- Strengthened handoff-module coverage so the completion helper proves it uses
  checkout readback by returning `route-final` in the opened response.
- Updated P69 to record this final route-binding ownership move.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Opened-response final route-binding derivation now lives in the handoff
  completion path. P69 remains open for broader sequencing consolidation and
  Slice F live proof.

## Turn 99 | 2026-07-06

Scope: continue P69 Slice C by moving operator-visible readiness gating into
the handoff module.

Actions:

- Added
  `remote_view_handoff::route_bound_handoff_operator_visible_failure_if_not_ready`.
- Added
  `remote_view_handoff::route_bound_handoff_final_operator_visible_failure_if_not_ready`.
- Rewired `remote_view_open` so `actions.rs` still computes operator-visible
  proof, but no longer interprets pre-checkout or final proof `state` values to
  decide whether rollback diagnostics are required.
- Added handoff-module tests for ready and not-ready pre-checkout proof gates
  and final proof context preservation.
- Updated P69 to record this readiness-gating extraction.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Operator-visible readiness decisions now live in the handoff module. P69
  remains open for broader sequencing consolidation and Slice F live proof.

## Turn 98 | 2026-07-06

Scope: continue P69 Slice C by moving successful route-bound open finalization
into the handoff module.

Actions:

- Added `remote_view_handoff::CompleteRouteBoundHandoffOpenInput`.
- Added `remote_view_handoff::complete_route_bound_handoff_open`.
- Rewired `remote_view_open` so `actions.rs` still performs command dispatch
  and final operator-visible proof, but no longer completes the route-bound
  lease, derives browser-build proof, serializes the lease, or assembles the
  opened response locally.
- Added handoff-module coverage proving the helper finalizes the lease and
  returns the opened `routeBoundHandoff` response surface.
- Updated P69 to record this successful-open finalization extraction.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Successful route-bound open finalization now lives in the handoff module.
  P69 remains open for broader sequencing consolidation and Slice F live proof.

## Turn 97 | 2026-07-06

Scope: continue P69 Slice C by moving route-bound failure recovery staging into
the handoff module.

Actions:

- Added `remote_view_handoff::RouteBoundHandoffFailureRecoveryInput` and
  `RouteBoundHandoffFailureRecovery`.
- Added
  `remote_view_handoff::begin_route_bound_handoff_failure_recovery` to perform
  rollback-before-cleanup sequencing and return the cleanup plan plus any
  skipped-cleanup payload.
- Added `remote_view_handoff::RouteBoundHandoffImmediateFailureInput` and
  `route_bound_handoff_immediate_failure` for pre-browser display/launch
  failures that only need rollback plus summary formatting.
- Rewired `remote_view_open` so `actions.rs` still executes async tab/browser
  close commands, but no longer derives cleanup plans from launch/tab evidence
  or builds immediate failure rollback summaries locally.
- Added handoff-module tests for failure recovery staging and immediate
  failures.
- Updated P69 to record this recovery-staging extraction.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Rollback-before-cleanup ordering, cleanup-plan selection, skipped-cleanup
  detection, and immediate display/launch failure summaries now live in the
  handoff module. P69 remains open for broader sequencing consolidation and
  Slice F live proof.

## Turn 96 | 2026-07-06

Scope: continue P69 Slice C by moving operator-visible failure diagnostics into
the handoff module.

Actions:

- Added `remote_view_handoff::RouteBoundHandoffProofFailure`.
- Added
  `remote_view_handoff::route_bound_handoff_operator_visible_failure`.
- Added
  `remote_view_handoff::route_bound_handoff_final_operator_visible_failure`.
- Rewired `remote_view_open` operator-visible and final operator-visible
  failure branches to use those helpers for paired error text and rollback
  cleanup payloads.
- Added handoff-module tests proving the diagnostics preserve
  `routeBoundHandoff` and `preCheckoutOperatorVisible` labels.
- Updated P69 to record this failure-diagnostic extraction.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Operator-visible proof failure error text and cleanup payload construction now
  lives in the handoff module. P69 remains open for full sequencing extraction
  and Slice F live proof.

## Turn 95 | 2026-07-06

Scope: continue P69 Slice C by moving pre-launch and launch-failure cleanup
payloads into the handoff module.

Actions:

- Added
  `remote_view_handoff::route_bound_handoff_pre_launch_failure_cleanup`.
- Added
  `remote_view_handoff::route_bound_handoff_launch_failure_cleanup`.
- Rewired the `remote_view_open` display-access failure branch and browser
  launch failure branch to use those handoff helpers instead of hand-built JSON.
- Added handoff-module coverage for the skipped-before-launch and
  skipped-after-launch cleanup payload shapes.
- Updated P69 to record this cleanup-payload extraction.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Pre-launch and launch-failure cleanup JSON shapes now live in the handoff
  module. P69 remains open for deeper end-to-end orchestration and Slice F live
  proof.

## Turn 94 | 2026-07-06

Scope: continue P69 Slice C by moving reused-browser launch evidence into the
handoff module.

Actions:

- Added
  `remote_view_handoff::route_bound_handoff_reused_browser_launch_result`.
- Rewired `remote_view_open` to use that helper when the selected route is
  already checked out to the current browser/session.
- Removed the inline reused-launch JSON shape from `actions.rs`.
- Added handoff-module coverage for browser, session, route, display, and
  reason evidence in the reused-launch result.
- Updated P69 to record this launch-result vocabulary extraction.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Reused route-bound browser launch evidence now lives in the handoff module.
  P69 remains open for deeper end-to-end orchestration and Slice F live proof.

## Turn 93 | 2026-07-06

Scope: continue P69 Slice C by moving visible-window checkout command
finalization into the handoff module.

Actions:

- Added
  `remote_view_handoff::route_bound_handoff_checkout_command_with_visible_window_proof`.
- Rewired `remote_view_open` to finalize the route-checkout command through
  the handoff helper after visible-window proof.
- Removed the action-local checkout command mutation that attached readiness
  and display-content proof.
- Added handoff-module tests for checkout command finalization with and
  without display content.
- Updated P69 to record this checkout finalization extraction.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Visible-window proof enrichment of checkout commands now lives in the
  handoff module. P69 remains open for deeper end-to-end orchestration and
  Slice F live proof.

## Turn 92 | 2026-07-06

Scope: continue P69 Slice C by moving failure rollback cleanup payload
vocabulary into the handoff module.

Actions:

- Added handoff helpers for generic pending rollback cleanup, operator-visible
  failure cleanup, and final operator-visible failure cleanup payloads.
- Rewired `remote_view_open` tab, focus, visible-window proof, checkout,
  operator-visible, and final operator-visible failure branches to use the
  handoff cleanup payload helpers.
- Kept rollback execution and async browser cleanup in `actions.rs`.
- Added handoff-module tests for simple rollback cleanup and the two
  operator-visible proof cleanup surfaces.
- Updated P69 to record the failure-cleanup vocabulary extraction.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Failure cleanup JSON shapes for the route-bound handoff path now live in the
  handoff module. P69 remains open for deeper end-to-end orchestration and
  Slice F live proof.

## Turn 91 | 2026-07-06

Scope: continue P69 Slice C by moving plan-path acquisition begin/complete
adapters into the handoff module.

Actions:

- Added `begin_route_bound_handoff_plan_acquisition` to own the plan-path
  begin-acquisition call plus default control-input selection.
- Added `complete_route_bound_handoff_plan_acquisition` to restore a missing
  lease when needed and complete the acquisition in one handoff API.
- Rewired `remote_view_open` to call those handoff helpers while keeping
  timestamp generation in `actions.rs`.
- Removed the action-local begin, complete, and restore acquisition wrappers.
- Updated the lease rollback test to exercise the handoff begin helper.
- Updated P69 to record the acquisition helper consolidation.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- The route-bound plan path now enters and completes acquisition through named
  handoff-module APIs. P69 remains open for deeper end-to-end orchestration and
  Slice F live proof.

## Turn 90 | 2026-07-06

Scope: continue P69 Slice C by grouping route-bound plan artifacts behind the
handoff module.

Actions:

- Added `remote_view_handoff::RouteBoundHandoffPlan` and
  `route_bound_handoff_plan` to group the normalized route binding with launch,
  tab, and route-checkout command artifacts.
- Rewired `remote_view_open` to consume one handoff plan after acquisition-plan
  selection instead of normalizing the route binding and constructing command
  values locally.
- Removed the action-local route-binding normalization helper.
- Updated stale acquisition-pending readiness coverage to exercise the handoff
  plan path and added a handoff-module test for grouped plan artifacts.
- Updated P69 to record this plan-artifact consolidation.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Route-bound planning now has a named handoff-module API that returns the
  normalized route binding and command artifacts together. P69 remains open for
  deeper acquisition/finalization sequencing and Slice F live proof.

## Turn 89 | 2026-07-06

Scope: continue P69 Slice C by moving route-bound command artifact
construction into the handoff module.

Actions:

- Added handoff-owned builders for route-bound launch, tab, focus, and
  route-checkout commands.
- Rewired `remote_view_open` to use the handoff command builders while keeping
  browser/service command execution in `actions.rs`.
- Removed the action-local route-bound command builders for launch, tab,
  focus, and checkout.
- Moved focus-command coverage into the handoff module and added coverage for
  launch/checkout route fields and tab default URL behavior.
- Updated P69 to record this command-artifact extraction.

Validation:

- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Route-bound command artifacts now have named handoff-module APIs. P69 remains
  open for deeper plan/acquire/finalize orchestration and Slice F live proof.

## Turn 88 | 2026-07-06

Scope: continue P69 Slice C by moving route-bound browser-build proof
finalization into the handoff module.

Actions:

- Added `remote_view_handoff::route_bound_handoff_browser_build_proof` to own
  selected browser build, executable path, applied capability, and mismatch
  evidence for route-bound opened responses.
- Rewired `remote_view_open` to call the handoff helper before opened-response
  assembly.
- Removed the action-local browser-build proof helper and moved its mismatch
  regression coverage into the handoff module.
- Updated P69 to record this proof-finalization extraction.

Validation:

- `cargo test --manifest-path cli/Cargo.toml browser_build_proof -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- Browser-build proof finalization now has a named handoff-module API. P69
  remains open for deeper plan/acquire orchestration and Slice F live proof.

## Turn 87 | 2026-07-06

Scope: continue P69 Slice C by moving route-bound final route-binding
derivation into the handoff module.

Actions:

- Added `remote_view_handoff::final_route_bound_handoff_route_binding` to own
  the merge of planned route binding, route checkout readback, and route-pool
  checkout readback.
- Rewired `remote_view_open` to use that handoff helper before final
  operator-visible proof and opened-response assembly.
- Removed the action-local final binding merge helper.
- Added handoff-module coverage for route and route-pool checkout readback
  overriding the planned binding, and kept the action-level stale-route proof
  coverage green.
- Updated P69 to record the finalization extraction.

Validation:

- `cargo fmt --manifest-path cli/Cargo.toml`
- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_final_route_binding -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `git diff --check -- cli/src/native/actions.rs cli/src/native/remote_view_handoff.rs docs/dev/plans/0069-2026-07-06-shared-profile-routing-and-handoff-deepening-plan.md RUNBOOK.md`

Result:

- Final route binding derivation now has a named handoff-module API. P69
  remains open for deeper plan/acquire orchestration and Slice F live proof.

## Turn 86 | 2026-07-06

Scope: continue P69 Slice C by moving route-bound failure rollback sequencing
behind the handoff module.

Actions:

- Added `remote_view_handoff::rollback_route_bound_handoff_failure` to restore
  a missing acquisition lease and roll route, display, route-pool, and browser
  display-allocation state back through one handoff API.
- Added `remote_view_handoff::complete_route_bound_handoff_failure_cleanup` to
  attach browser cleanup evidence to the rollback and produce the cleanup
  summary string.
- Rewired `remote_view_open` tab, focus, proof, checkout, and final-proof
  failure branches through one cleanup adapter. `actions.rs` still performs the
  async browser cleanup command, but no longer open-codes lease restoration,
  rollback mutation, cleanup attachment, and summary formatting in every branch.
- Added a repository-backed handoff-module test for restoring a missing lease,
  rolling pending state back to previous values, recording browser cleanup, and
  returning a parseable cleanup summary.
- Updated P69 to record this Slice C sequencing progress.

Validation:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_cleanup -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `git diff --check -- cli/src/native/actions.rs cli/src/native/remote_view_handoff.rs docs/dev/plans/0069-2026-07-06-shared-profile-routing-and-handoff-deepening-plan.md RUNBOOK.md`

Result:

- Route-bound failure rollback and cleanup summary sequencing now has a named
  handoff-module interface. P69 remains open for deeper plan/acquire/finalize
  orchestration and Slice F live proof.

## Turn 85 | 2026-07-06

Scope: continue P69 Slice C by routing plain remote-headed `open` through the
shared acquisition-result surface.

Actions:

- Added a short-lived daemon response slot for launch-time shared-profile
  acquisition evidence.
- Reused `remote_view_handoff::shared_profile_acquisition_result` for plain
  `open`/`navigate` auto-launch when it attaches to a compatible retained
  same-profile browser and opens a tab there.
- Taught the subsequent navigation response to include `sharedAcquisition`
  with the selected retained owner browser/session, requested/planned profile,
  duplicate-process policy, and `routeHintSource: shared_profile_auto_launch`.
- Added focused Rust coverage for the plain-open owner evidence shape.
- Updated P69 to remove the plain remote-headed `open` acquisition-result gap.

Validation:

- `~/.local/bin/graphiti-runtime doctor`
- `cargo fmt --manifest-path cli/Cargo.toml`
- `cargo test --manifest-path cli/Cargo.toml shared_profile_auto_launch_acquisition -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml shared_profile -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `node --check packages/client/src/service-request.js`
- `node --check scripts/test-service-request-client.js`
- `pnpm test:service-client`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `git diff --check -- cli/src/native/actions.rs docs/dev/plans/0069-2026-07-06-shared-profile-routing-and-handoff-deepening-plan.md RUNBOOK.md`

Result:

- Plain remote-headed `open` now participates in the same named
  `sharedAcquisition` response vocabulary as `remote_view_open` and HTTP/MCP
  `service_request` tab acquisition. P69 remains open for the full
  plan/acquire/finalize/rollback sequencing move and Slice F live proof.

## Turn 67 | 2026-07-06

Scope: execute P69 Slice A and the ordinary-open part of Slice B.

Actions:

- Added global `--browser-build` parsing and `clean_args` handling.
- Preserved explicit global launch-routing flags on plain `open`, `goto`, and
  `navigate` command payloads.
- Added a shared-profile auto-launch acquisition path that attaches to a
  compatible retained same-profile browser with a CDP endpoint, creates a fresh
  tab, and then lets the existing navigation handler load the requested URL.
- Updated the P69 plan, the `last30days` routing-failure note, CLI help,
  README, docs site commands page, and `skills/agent-browser/SKILL.md`.

Validation run:

- `cargo test --manifest-path cli/Cargo.toml test_parse_global_browser_build_flag -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml test_navigate_preserves_explicit_global_launch_routing_flags -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml open_preserves_runtime_profile -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml shared_profile -- --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `git diff --check -- cli/src/flags.rs cli/src/commands.rs cli/src/native/actions.rs cli/src/output.rs README.md docs/src/app/commands/page.mdx skills/agent-browser/SKILL.md`

Result:

- Slice A is implemented for plain navigation commands.
- Slice B is partially implemented for ordinary `open`, `goto`, and `navigate`.
  HTTP/MCP `service_request` parity, public acquisition response fields,
  dashboard/client actionability, and live two-tab proof remain open P69 work.

## Turn 66 | 2026-07-06

Scope: write P69 for shared-profile routing and handoff deepening.

Actions:

- Reviewed the architecture review report, the `last30days` profile-routing
  failure note, the runtime-profile sharing plan, and P67/P68 profile identity
  follow-ups.
- Added
  `docs/dev/plans/0069-2026-07-06-shared-profile-routing-and-handoff-deepening-plan.md`
  to make plain `open` preserve explicit runtime identity, route compatible
  in-use profiles through retained-browser tab acquisition, deepen the
  route-bound handoff module, and align workspace inventory plus generated
  client contracts.
- Added the P69 roadmap entry so the new lane is discoverable from
  `ROADMAP.md`.

Validation run:

- Read-only policy, Graphiti, CodeGraph, roadmap, runbook, and source-note
  inspection only.

Result:

- P69 is open and ready for Slice A implementation.

## Turn 65 | 2026-06-27

Scope: implement P46 S10 harness and stop at the S10 retry lock.

Actions:

- Added S10 scenario metadata for a service-owned route-bound browser beside a
  zero-lease foreign CDP browser.
- Added a live S10 runner path that launches a foreign Chromium profile outside
  `~/.agent-browser`, captures authenticated dashboard inventory, and evaluates
  selected workspace action and route/display isolation.
- Ran two live S10 attempts from the installed binary lane. Both failed before
  S10 evaluation on dashboard inventory endpoint/auth issues, and both reset
  cleanly with zero active incidents.
- Repaired the harness to read `/api/sessions` and
  `/api/session-tabs?port=...` through the authenticated dashboard
  viewer-client session.
- Added P63 and locked P46 at S10 pending validation-backed retry clearance.

Validation run:

- `node --check scripts/run-p46-stress-scenario.js`
- `node --check scripts/lib/p46-scenario-harness.js`
- `node scripts/test-p47-scenario-harness.js`
- `node scripts/test-dashboard-workspace-nodes.js`

Result:

- No-live checks pass after the authenticated inventory fix.
- Failed live artifacts:
  `/tmp/agent-browser-p46-s10-2026-06-27T22-17-57-154Z` and
  `/tmp/agent-browser-p46-s10-2026-06-27T22-20-21-552Z`.
- P46 is locked at S10 pending P63. Do not run another S10 retry until P63's
  green preflight authorizes exactly one retry.

## Turn 64 | 2026-06-27

Scope: complete P62 and clear P46 S9.

Actions:

- Repaired dashboard selected-target recovery so an explicitly selected live
  blank tab is preserved as the selected target, while missing or dead stale
  selections still recover to a live tab.
- Updated S9 viewer-client and evaluator checks to accept exact blank-target
  preservation or typed stale-target recovery before requiring final blank
  navigation.
- Rebuilt and installed the dashboard runtime with
  `pnpm publish:local-dashboard -- --skip-smoke --json`.
- Verified installed runtime convergence, then reran S9 from the installed
  binary authority.
- Marked P62 complete and advanced P46 to S10.

Validation run:

- `node --check scripts/lib/p47-viewer-client.js`
- `node --check scripts/run-p46-stress-scenario.js`
- `node scripts/test-dashboard-view-streams.js`
- `node scripts/test-p47-viewer-client-separation.js`
- `node scripts/test-p47-scenario-harness.js`
- `git diff --check -- packages/dashboard/src/components/workspace-remote-viewport.tsx scripts/lib/p47-viewer-client.js scripts/run-p46-stress-scenario.js scripts/test-dashboard-view-streams.js scripts/test-p47-viewer-client-separation.js scripts/test-p47-scenario-harness.js`
- `agent-browser --json install doctor`
- `node scripts/smoke-local-dashboard-runtime.js --dashboard-url http://127.0.0.1:4848/ --agent-browser-bin /home/ecochran76/.local/bin/agent-browser --skip-browser --json`
- `node scripts/run-p46-stress-scenario.js --scenario s9 --reset-before --reset-after --agent-browser-command /home/ecochran76/.local/bin/agent-browser --require-explicit-agent-browser-command --require-agent-browser-daemon-command-match`

Result:

- Installed runtime doctor passed with zero issues.
- S9 passed with artifact
  `/tmp/agent-browser-p46-s9-2026-06-27T22-03-14-950Z`.
- The pass proved exact initial blank-target selection, blank navigation to
  IANA, duplicate same-origin tab isolation, browser-window-visible route
  display, route-bound finalization, one default-profile browser row, and zero
  active incidents after reset-after.
- P46 is now in progress at S10.

## Turn 63 | 2026-06-27

Scope: implement P46 S9 stale target and duplicate tab stress, then record the
S9 lock.

Actions:

- Added S9 scenario metadata, live capture, evaluator checks, and no-live
  harness assertions.
- Added a narrow viewer-client stale selected-tab recovery option for the S9
  operator C blank-tab proof.
- Ran S9 live attempts from the explicit rebuilt-binary lane with reset-before
  and reset-after.
- Added P62 for the selected-target recovery follow-up.
- Updated P46 and the P46 execution note with the S9 lock.

Validation run:

- `node --check scripts/lib/p47-viewer-client.js`
- `node --check scripts/run-p46-stress-scenario.js`
- `node scripts/test-p47-viewer-client-separation.js`
- `node scripts/test-p47-scenario-harness.js`
- `git diff --check -- scripts/lib/p47-viewer-client.js scripts/lib/p46-scenario-harness.js scripts/run-p46-stress-scenario.js scripts/test-p47-viewer-client-separation.js scripts/test-p47-scenario-harness.js`

Result:

- S9 did not pass. Corrected failure artifact:
  `/tmp/agent-browser-p46-s9-2026-06-27T21-42-54-990Z`.
- The run proved stale blank-tab recovery notice and CLI navigation of the blank
  target, but the dashboard rewrote operator C back to duplicate target A when
  the harness re-requested the blank-tab dashboard URL.
- Reset-after reported zero sessions, zero browsers, zero tabs, and zero active
  incidents.
- P46 is locked at S9 pending P62. Do not run another S9 retry until P62
  records validation-backed retry authorization.

## Turn 62 | 2026-06-27

Scope: implement and clear P46 S8 display-access recovery.

Actions:

- Added P61 for the S8 display-access denial and recovery proof.
- Added S8 metadata, live capture, evaluator checks, and no-live harness
  assertions.
- Used a temporary `timeout` shim in `PATH` to safely simulate display-access
  denial without mutating host X11 permissions.
- Reran the same route-bound open with normal display access as the recovery
  proof.
- Updated P46 and the P46 execution note with S8 clearance.

Validation run:

- `node --check scripts/run-p46-stress-scenario.js`
- `node scripts/test-p47-scenario-harness.js`
- `git diff --check -- scripts/lib/p46-scenario-harness.js scripts/run-p46-stress-scenario.js scripts/test-p47-scenario-harness.js`
- `node scripts/run-p46-stress-scenario.js --scenario s8 --reset-before --reset-after --agent-browser-command ./cli/target/debug/agent-browser --require-explicit-agent-browser-command --require-agent-browser-daemon-command-match`

Result:

- S8 passed with artifact
  `/tmp/agent-browser-p46-s8-2026-06-27T21-07-22-844Z`.
- The pass proved typed `display_access_grant_failed` denial before browser
  launch, cleanup rollback of display allocation, remote-view route, and
  route-pool entry, no retained denied-profile browser row, terminal-free route
  displays after denial, successful recovery open with
  `displayAccessGrant.state: already_ready`, and zero active incidents after
  reset-after.
- P46 is now in progress at S9.

## Turn 61 | 2026-06-27

Scope: implement and clear P46 S7 route-pool exhaustion.

Actions:

- Added P60 for the S7 route-capacity diagnostic repair.
- Added S7 metadata, live capture, evaluator checks, and no-live harness
  assertions for third-demand route-pool exhaustion and retry after release.
- Tightened `plan_remote_view_acquisition` so unpinned route-bound demand that
  lands on a checked-out pool display owned by another session reports
  `route_pool_exhausted`.
- Rebuilt `./cli/target/debug/agent-browser`, restarted the stale default
  daemon, and ran the rebuilt-binary S7 verifier.
- Updated P46 and the P46 execution note with the S7 clearance.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml acquisition_plan_reports_route_pool_exhausted -- --nocapture`
- `node scripts/test-p47-scenario-harness.js`
- `cargo build --manifest-path cli/Cargo.toml`
- `node scripts/run-p46-stress-scenario.js --scenario s7 --reset-before --reset-after --agent-browser-command ./cli/target/debug/agent-browser --require-explicit-agent-browser-command --require-agent-browser-daemon-command-match`

Result:

- S7 passed with artifact
  `/tmp/agent-browser-p46-s7-2026-06-27T20-58-30-721Z`.
- The pass proved both route-pool entries occupied, third demand failing with
  `route_pool_exhausted`, no retained profile C browser row after the failed
  demand, no terminal fallback on occupied displays, successful profile C retry
  after releasing profile A, and zero active incidents after reset-after.
- P46 is now in progress at S8.

## Turn 60 | 2026-06-27

Scope: clear P46 S6 and advance to S7.

Actions:

- Ran the P55-authorized S6 retry from the explicit rebuilt-binary lane.
- Manually closed the two retained S6 profile sessions after reset-after missed
  them.
- Added P56 to reconnect the external viewer-client CDP websocket after swapped
  dashboard navigation.
- Added a 32 MiB `spawnSync` output buffer to the P46 runner so large
  `service status` payloads remain parseable during reset.
- Added no-live coverage for swapped reconnect artifacts and reset buffer
  hardening.
- Ran one P56-authorized S6 retry after green preflight.
- Added P57 to require DevTools target-discovery evidence before another S6
  retry.
- Added P58 to wait for the swapped DevTools page URL before reconnecting.
- Added P59 to use same-origin `history.pushState` plus `popstate` for
  dashboard workspace swaps.
- Ran the P59-authorized S6 retry after green preflight.

Validation run:

- `node --check scripts/run-p46-stress-scenario.js`
- `node scripts/test-p47-scenario-harness.js`
- `node scripts/test-p47-viewer-client-separation.js`
- `git diff --check -- scripts/lib/p47-viewer-client.js scripts/run-p46-stress-scenario.js scripts/test-p47-viewer-client-separation.js scripts/test-p47-scenario-harness.js docs/dev/plans/0046-2026-06-24-remote-view-stress-hardening-plan.md docs/dev/plans/0055-2026-06-27-s6-dashboard-swap-navigation-plan.md docs/dev/plans/0056-2026-06-27-s6-dashboard-reconnect-and-reset-buffer-plan.md docs/dev/notes/2026-06-24-p46-stress-hardening-execution.md RUNBOOK.md`

Result:

- P55 retry failed with artifact
  `/tmp/agent-browser-p46-s6-2026-06-27T20-06-51-508Z`.
- The failure moved to post-swap state polling:
  `CDP command Runtime.evaluate timed out after 30000ms`.
- P56 retry failed with artifact
  `/tmp/agent-browser-p46-s6-2026-06-27T20-15-52-909Z`.
- The failure moved to reconnect command enablement:
  `CDP command Page.enable timed out after 30000ms`.
- P56 reset-after closed both retained S6 profile sessions, and final readback
  showed zero sessions, zero browsers, zero tabs, zero active incidents, and
  both route-pool entries available.
- P57 retry showed the selected DevTools page URL still pointed at profile A
  after requesting profile B.
- P58 retry showed `location.assign()` did not change the selected DevTools
  page URL for the same-origin dashboard workspace swap.
- P59 retry passed with artifact
  `/tmp/agent-browser-p46-s6-2026-06-27T20-32-54-709Z`.
- S6 pass proved swapped selected-browser readback for both operators, working
  swapped refresh controls, swapped screenshots, distinct route-bound profile
  checkouts, profile B readiness after profile A closed, and clean reset-after.
- P46 is now in progress at S7.

## Turn 59 | 2026-06-27

Scope: repair P46 S5 viewer-client port allocation, pass S5, and start S6.

Actions:

- Added P54 for the S5 viewer-client DevTools port collision that locked P46
  after S5 attempt 2.
- Changed the external dashboard viewer-client launch path to use Chromium
  dynamic DevTools allocation with `--remote-debugging-port=0` by default.
- Added `DevToolsActivePort` readback before viewer-client `/json/version` and
  `/json` calls.
- Kept explicit viewer-client DevTools port overrides only for diagnostics.
- Extended the P47 viewer-client no-live test to cover dynamic launch metadata,
  `DevToolsActivePort` parsing, override validation, and absence of the old
  random fixed-port selector.
- Updated P46 and the P46 execution note with the P54 repair and S5 pass.
- Added S6 metadata, runner support, and no-live coverage for two-profile
  cross-observation with swapped dashboard selection.
- Added a CDP command timeout to the viewer-client adapter after S6 attempt 1
  hung before writing swapped selection artifacts.
- Updated reset handling to close retained browser rows from `activeSessionIds`
  and `session:<name>` browser IDs when session rows are missing.

Validation run:

- `node --check scripts/lib/p47-viewer-client.js`
- `node --check scripts/test-p47-viewer-client-separation.js`
- `node --check scripts/run-p46-stress-scenario.js`
- `node scripts/test-p47-viewer-client-separation.js`
- `node scripts/test-p47-scenario-harness.js`
- `pnpm test:p47-viewer-client-separation`
- `git diff --check -- scripts/lib/p47-viewer-client.js scripts/test-p47-viewer-client-separation.js scripts/run-p46-stress-scenario.js scripts/lib/p46-scenario-harness.js scripts/test-p47-scenario-harness.js docs/dev/plans/0046-2026-06-24-remote-view-stress-hardening-plan.md docs/dev/plans/0054-2026-06-27-s5-viewer-client-port-allocation-plan.md docs/dev/notes/2026-06-24-p46-stress-hardening-execution.md`
- `node scripts/run-p46-stress-scenario.js --scenario s5 --reset-before --reset-after --agent-browser-command ./cli/target/debug/agent-browser --require-explicit-agent-browser-command --require-agent-browser-daemon-command-match`
- `node scripts/run-p46-stress-scenario.js --scenario s6 --reset-before --reset-after --agent-browser-command ./cli/target/debug/agent-browser --require-explicit-agent-browser-command --require-agent-browser-daemon-command-match`

Result:

- S5 passed with artifact
  `/tmp/agent-browser-p46-s5-2026-06-27T19-41-29-598Z`.
- The pass proved profile A on route `guacamole:3` and display `:13`, profile
  B on route `guacamole:4` and display `:14`, finalized route-bound checkouts
  for both profiles, working refresh controls for both external dashboard
  viewer clients, browser-visible route displays for both routes, and profile B
  staying ready after profile A closed.
- Reset-after and final readback showed zero sessions, zero browsers, zero
  tabs, and zero active incidents.
- S6 attempt 1 artifact
  `/tmp/agent-browser-p46-s6-2026-06-27T19-49-19-793Z` proved both profile
  browsers and both initial dashboard viewers became ready, but the run hung
  before swapped dashboard selection artifacts were written.
- S6 attempt 2 artifact
  `/tmp/agent-browser-p46-s6-2026-06-27T19-56-33-105Z` failed in bounded form
  with `CDP command Page.navigate timed out after 30000ms` during the swapped
  dashboard selection step.
- Manual cleanup closed
  `p46-s6-profile-a-2026-06-27T19-56-29-450Z` and
  `p46-s6-profile-b-2026-06-27T19-56-29-450Z`; final readback showed zero
  sessions, zero browsers, zero tabs, zero active incidents, route-pool entries
  available, and idle displays.
- P54 is complete. P46 is locked at S6 by the two-consecutive-failure rule.

## Turn 57 | 2026-06-27

Scope: diagnose the P46 S4 lock and create the S4 topology follow-up plan.

Actions:

- Re-read P46, the P46 execution note, repo validation and memory policies, and
  the S4 attempt 2 artifact.
- Confirmed Graphiti was healthy, but the focused read did not add S4-specific
  authority beyond repo files and artifacts.
- Classified S4 attempt 2 as a same-profile topology and typed-blocker gap:
  window A reached `operatorVisible.state=ready` on `p46-s4-profile`, route A,
  and display `:13`; window B then tried the same runtime profile on route B,
  timed out, and left route-bound finalization cleanup evidence.
- Added P53 to decide and implement the supported S4 topology before any live
  S4 retry.
- Implemented the P53 Goal 1 no-live S4 topology guard. The S4 runner now
  writes `s4-topology-preflight.json` and stops with
  `same_profile_multi_process_unsupported` before launching window B for the
  current one-profile, two-session, two-route-pool-entry shape.
- Selected the P53 Goal 2 topology: one retained remote-headed browser process,
  one route lease, one runtime profile, and two top-level same-profile windows.
- Added `agent-browser window new [url] --same-profile` and rewired S4 window B
  to use that same-session window target instead of a second route-bound
  browser process.
- Switched S4 to a unique `p46-s4-window-<timestamp>` daemon session per run
  after the first P53-shaped retry reused a stale named session and exercised
  the older window handler.
- Updated P46 and the P46 execution note to keep the lock in place pending P53.

Validation run:

- `node --check scripts/run-p46-stress-scenario.js`
- `node scripts/test-p47-scenario-harness.js`
- `git diff --check -- docs/dev/plans/0046-2026-06-24-remote-view-stress-hardening-plan.md docs/dev/plans/0053-2026-06-27-s4-single-profile-window-topology-plan.md docs/dev/notes/2026-06-24-p46-stress-hardening-execution.md RUNBOOK.md`
- Read-only service status: zero sessions, zero browsers, zero tabs, and zero
  active incidents.
- Read-only install doctor: success, zero issues, one matching default socket
  listener, and zero deleted default-socket listeners.

Result:

- P46 remains locked at S4 by its two-failure rule.
- No live S4 retry was run.

## Turn 56 | 2026-06-23

Scope: complete the P44 Slice H dashboard inventory class inspector and local
publish smoke.

Actions:

- Added `WorkspaceInventoryClass` to the shared dashboard workspace node model.
- Classified service-owned controllable browsers, service-owned view-only
  browsers, service-owned diagnostic browsers, detected non-owned browsers,
  viewer clients, retained history, service-owned sessions, and profile action
  rows.
- Exposed the inventory class through selected-workspace context, diagnostic
  bundles, and evidence rows so inspector, chat, console, and automation
  consumers do not infer ownership from URL shape.
- Added the selected Workspace inspector Class row backed by the canonical
  `WorkspaceInventoryClass` value.
- Published the dashboard runtime locally and ran the full local dashboard
  smoke against `/home/ecochran76/.local/bin/agent-browser`.
- Updated README, dashboard docs, Plan 0044, `ROADMAP.md`, and repo plus
  installed skill guidance.

Validation run:

- `pnpm test:dashboard-workspace-nodes`
- `pnpm test:dashboard-selected-workspace-context`
- `pnpm test:dashboard-selected-workspace-chat-packet`
- `pnpm test:dashboard-selected-workspace-console`
- `pnpm test:dashboard-view-streams`
- `pnpm test:dashboard-workspace-navigator`
- `pnpm test:dashboard-inspector-actions`
- `pnpm --dir docs build`
- `pnpm build:dashboard`
- `pnpm publish:local-dashboard -- --expect-marker service-owned-controllable-browser --skip-browser --json`
- `pnpm smoke:local-dashboard-runtime -- --expect-marker service-owned-controllable-browser --agent-browser-bin /home/ecochran76/.local/bin/agent-browser --json`
- `agent-browser install doctor --json`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `git diff --check`
- `pnpm validation:select -- --base HEAD`
- `node scripts/dev/select-validation.js --base HEAD --json`

Result:

- Focused dashboard workspace model, selected workspace, chat-packet, console,
  view-stream, navigator, inspector action, docs, dashboard build, local
  publish, runtime smoke, skill-sync, and hygiene checks passed.
- Local publish restarted `agent-browser-dashboard.service` and installed
  executable SHA
  `6c7c9b879c1b564130fb74e4d2abec7502252033be14e66586c20477e7762649`
  with dashboard bundle SHA
  `10177dc55ce0a76f29fbcce7ede2acf8e7b5cbb896d83987ddff2e2aaa193967`.
- Runtime smoke loaded `http://127.0.0.1:4848/`, found
  `service-owned-controllable-browser`, and confirmed the workspace pane in
  browser session `local-dashboard-runtime-smoke-1606766`.
- Closing stale daemon session `default` brought install doctor runtime
  convergence back to `converged` with stale daemon count `0`.
- Slice H dashboard inventory refactor is complete. P44 remains open for the
  installed privileged helper refresh and the later Slice I and Slice J work.
  Doctor still reports `remote_view_route_desktop_helper_stale`, which needs
  `agent-browser install --with-remote-view-privileges` from an interactive
  sudo shell, plus readiness-impacting stale resource candidates.

## Turn 55 | 2026-06-23

Scope: start P44 Slice H dashboard inventory actionability.

Actions:

- Moved non-ready RDP gateway operator-visible proof rows out of the active
  workspace control group and into `needs-attention`.
- Kept View and Control disabled with route-proof reasons while enabling Repair
  for actionable route-proof failures.
- Extended dashboard workspace fixture coverage for terminal-only, unbound,
  missing-proof, wrong-tab, unavailable-route, missing-CDP-target, and
  stale-route rows, while preserving active controllable service-owned rows and
  detected non-owned CDP rows.
- Updated README, dashboard docs, commands docs, Plan 0044, `ROADMAP.md`, and
  repo plus installed skill guidance.

Validation run:

- `pnpm test:dashboard-workspace-nodes`
- `pnpm test:dashboard-view-streams`
- `pnpm --dir docs build`
- `pnpm build:dashboard`
- `pnpm test:dashboard-selected-workspace-context`
- `pnpm test:dashboard-selected-workspace-chat-packet`
- `pnpm test:dashboard-selected-workspace-console`
- `pnpm test:dashboard-workspace-navigator`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `git diff --check`
- `pnpm validation:select -- --base HEAD`
- `node scripts/dev/select-validation.js --base HEAD --json`

Result:

- The focused dashboard workspace, adjacent view-stream, selected workspace,
  docs, dashboard build, skill-sync, and hygiene checks passed.
- P44 remains open. The installed helper refresh still needs interactive sudo,
  and Slice H still needs inspector and manual publish smoke coverage before it
  can be called complete.

## Turn 54 | 2026-06-23

Scope: start P44 Slice G fast route preflight.

Actions:

- Added `fastPreflight` to the existing
  `service_remote_view_route_preflight` no-launch action.
- The response now reports `ready`, `partial`, `stale`, or `blocked` from
  component evidence for acquisition planning, Guacamole route URL shape,
  retained Guacamole web/login/permission and RDP TCP readiness, display access,
  and route desktop state.
- Added HTTP `GET /api/service/remote-view/route-preflight`, MCP
  `service_remote_view_route_preflight`, and
  `getServiceRemoteViewRoutePreflight()` as first-class no-launch convenience
  surfaces over the same fast preflight response.
- Bounded the shared display-access probe with `timeout --kill-after=1 2` so
  fake or unreachable route displays cannot hang preflight or route-open display
  access checks.
- Updated README, CLI help, service-mode docs, service-request schema
  description, Plan 0044, `ROADMAP.md`, and repo plus installed skill guidance.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml test_remote_view_route_and_lease_actions_mutate_service_state -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml remote_view_route -- --test-threads=1`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:service-client`
- `pnpm test:remote-view-route-preflight-timing`
- `pnpm --dir docs build`
- `pnpm validation:select -- --base HEAD`
- `node scripts/dev/select-validation.js --base HEAD --json`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `git diff --check`

Result:

- Focused route preflight and route-action Rust tests, clippy, docs,
  service-client, API/MCP parity, skill-sync, and hygiene checks passed.
- P44 remains open. Slice G now has HTTP/MCP/client convenience surfaces and a
  bounded timing smoke; remaining live boundaries are still the installed helper
  refresh and guarded route-bound repeat-open smoke.

## Turn 53 | 2026-06-23

Scope: continue P44 Slice F route-bound repeat-open target convergence.

Actions:

- Routed `remote_view_open` tab acquisition through same-origin live target reuse
  before opening a new tab.
- Added `tabAcquisitionDecision` and `duplicateTargetCleanup` evidence to
  successful route-bound tab acquisition results.
- Extended the remote-view-open live smoke to assert that CLI first, CLI repeat,
  and HTTP helper opens converge to one active intended target in service state.
- Updated README, CLI help, docs site, Plan 0044, `ROADMAP.md`, and repo plus
  installed skill guidance for the repeat-open convergence contract.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_reusable_live_target -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open -- --test-threads=1`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `node --check scripts/smoke-remote-view-open-live.js`
- `pnpm --dir docs build`
- `pnpm validation:select -- --base HEAD`
- `node scripts/dev/select-validation.js --base HEAD --json`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `git diff --check`
- `agent-browser install doctor --json`

Result:

- Static, docs, and focused Rust checks passed.
- The live route-bound repeat-open smoke is implemented but not run in this
  turn because install doctor still reports
  `remote_view_route_desktop_helper_stale`; refreshing the installed helper
  requires an interactive sudo boundary.
- P44 remains open. Slice D still needs the interactive helper refresh and cold
  route desktop proof; Slice F still needs the guarded live smoke run after that
  refresh.

## Turn 52 | 2026-06-23

Scope: continue P44 Slice F dashboard stale-target URL recovery.

Actions:

- Updated the workspace remote viewport to treat missing, closed, blank, or
  target-shaped stale `tab=target:*` URL selections as recoverable stale target
  identity for the selected browser.
- Replaced stale workspace tab URL selections with the current live service tab
  before control mode queues `view_focus`.
- Preserved the existing `stale_target_recovered` UX and readiness vocabulary
  while adding a focused recovery message that names the stale selection and
  current live tab.
- Added dashboard view-stream fixture assertions for stale URL replacement and
  target-shaped stale tab recovery.
- Updated README, docs site, Plan 0044, `ROADMAP.md`, and repo skill guidance.

Validation run:

- `pnpm test:dashboard-view-streams`
- `pnpm build:dashboard`
- `pnpm test:dashboard-workspace-navigator`
- `pnpm test:dashboard-workspace-nodes`
- `pnpm validation:select -- --base HEAD`
- `git diff --check`

Result:

- All listed checks passed.
- P44 remains open. Slice F still needs a route-bound repeat-open live proof
  that verifies one intended active target.
- Slice D remains open on the interactive sudo boundary for refreshing the
  installed privileged helper and proving a cold browser-control-ready route
  desktop.

## Turn 51 | 2026-06-23

Scope: start P44 Slice F tab acquisition cleanup with a duplicate-replacement
refresh policy.

Actions:

- Added `replace_duplicates` to `tab_handle_refresh` repair-policy validation in
  the daemon, HTTP ingress, MCP ingress, service schema, generated client
  template, and service-client helper.
- Implemented best-effort compatible duplicate cleanup for `replace_duplicates`.
  The refresh path reuses or opens one compatible target, preserves that selected
  target, closes other compatible live targets when possible, and returns
  `duplicateTargetCleanup` evidence.
- Added Rust coverage for compatible duplicate target selection and client
  coverage proving the new policy is accepted and forwarded.
- Updated README, CLI help, docs site, Plan 0044, `ROADMAP.md`, and repo plus
  installed skill guidance.

Validation run:

- `pnpm generate:service-client`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml tab_handle_refresh -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_model -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm test:service-request-client`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:service-client`
- `pnpm test:route-confusion-gates`
- `pnpm --dir docs build`
- `git diff --check`
- `pnpm validation:select -- --base HEAD`
- `node scripts/dev/select-validation.js --base HEAD --json`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`

Result:

- All listed checks passed.
- P44 remains open. Slice F still needs dashboard stale-target URL recovery and a
  route-bound repeat-open live proof that verifies one intended active target.
- Slice D also remains open on the interactive sudo boundary for refreshing the
  installed privileged helper and proving a cold browser-control-ready route
  desktop.

## Turn 21 | 2026-06-23

Scope: start P44 Slice E by returning structured route-bound operator-visible
proof components.

Actions:

- Extended successful `remote_view_open` `operatorVisible` output with selected
  target evidence plus route, display, browser, tab, stream, and Guacamole
  component states while preserving the existing `proof` field.
- Updated `summarizeServiceRemoteViewOpenProof()` to prefer
  `operatorVisible.target` and `operatorVisible.components` before falling back
  to the tab result.
- Updated CLI help, README, docs site, and repo plus installed skill guidance
  for the richer `operatorVisible` proof shape.
- Added selected-target URL readiness so a visible browser with the wrong
  selected tab reports `operatorVisible.state=wrong_tab`, with
  `components.display.state=ready` and `components.tab.state=wrong_tab`.
- Added Guacamole route availability to the same proof vocabulary so ready
  display and tab evidence with a missing or non-ready operator route reports
  `operatorVisible.state=guacamole_route_unavailable`.
- Added CDP target availability to the selected-tab proof so URL-bearing tab
  results without a CDP `targetId` report
  `operatorVisible.state=cdp_target_unavailable`.
- Added retained route metadata to the route proof so stale or mismatched
  route-pool allocation records report
  `operatorVisible.state=stale_route_record`.
- Added dashboard readiness fixture coverage so workspace rows preserve
  `wrong_tab`, `guacamole_route_unavailable`, `cdp_target_unavailable`, and
  `stale_route_record` from structured stream readiness while keeping View and
  Control disabled with state-specific reasons.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml test_remote_view_open_operator_visible_reports_ready_proof -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_operator_visible -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_model -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm test:service-request-client`
- `pnpm test:service-client`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:route-confusion-gates`
- `node --no-warnings --experimental-strip-types scripts/test-dashboard-workspace-nodes.js`
- `pnpm test:dashboard-view-streams`
- `pnpm test:service-cdp-tab-streaming-live`
- `pnpm --dir docs build`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `git diff --check`
- `pnpm validation:select -- --base HEAD`

Result:

- Focused Rust proof coverage, the full `remote_view_open` filter, service
  model and CDP stream tests, clippy, service-client helpers, API/MCP parity,
  route-confusion gates, docs build, skill sync, diff hygiene, and the live CDP
  tab streaming smoke passed. Slice E remains open for failure-case proof
  vocabulary and dashboard readiness fixture coverage.

## Turn 20 | 2026-06-23

Scope: continue P44 Slice D by making stale installed route desktop helpers
visible in fast doctor surfaces.

Actions:

- Added `helperDesktopSession` inspection to `agent-browser install doctor` and
  `agent-browser doctor remote-view`; both parse the installed privileged
  helper's `.xsession` heredoc and classify terminal-first, missing, unreadable,
  incomplete, or browser-control-ready templates.
- Added `remote_view_route_desktop_helper_stale` issue reporting to both doctor
  surfaces when a root-owned helper exists but still writes a terminal-first
  route desktop.
- Added text output for route desktop helper state and focused Rust coverage for
  terminal-first rejection, idle Openbox acceptance, and stale-helper issue
  generation.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml remote_view_helper_desktop_session -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml reports_stale_remote_view_helper_desktop_template -- --nocapture`
- `pnpm test:route-confusion-gates`
- `cargo build --manifest-path cli/Cargo.toml`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cli/target/debug/agent-browser --json install doctor`
- `cli/target/debug/agent-browser --json doctor remote-view`

Result:

- Both focused Rust test groups passed, route-confusion gates passed, debug build
  passed, and clippy passed.
- The rebuilt debug `install doctor` and `doctor remote-view` readbacks both
  reported `helperDesktopSession.state=terminal_first_template`,
  `terminalStartupDetected=true`, and issue code
  `remote_view_route_desktop_helper_stale` for the currently installed helper.
- The live route proof remains blocked on refreshing the root-owned helper from
  an interactive sudo shell and then starting a cold route session.

## Turn 19 | 2026-06-21

Scope: repair the Plan 0039 audit findings after closeout review.

Actions:

- Made `agent-browser remote-view open` accept the documented
  `--browser-build stealthcdp_chromium` and `--provider rdp_gateway` flags.
- Added post-launch failure cleanup to `remote_view_open`: tab open, focus, visible-window proof, or checkout failures now clean up before returning the typed error. New
  browser launches close the browser; reused retained browsers preserve the
  browser process and close only the opened tab when possible.
- Updated CLI help, README, docs command page, repo skill guidance, Plan 0039,
  and P16 roadmap text for the accepted flags and cleanup boundary.

Validation run:

- `cargo test --manifest-path cli/Cargo.toml test_remote_view_open_builds_route_bound_service_action -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml test_remote_view_open_cleanup_reports_new_browser_close_on_failure -- --test-threads=1`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml remote_view -- --test-threads=1`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml service_model -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_access_plan -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_health -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_contracts -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_config -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:service-client`
- `pnpm --dir docs build`
- `pnpm test:dashboard-view-streams`
- `pnpm test:dashboard-browser-row-actions-render`
- `pnpm test:dashboard-browser-table`
- `pnpm test:dashboard-workspace-navigator`
- `pnpm test:dashboard-inspector-actions`
- `pnpm build:dashboard`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `git diff --check`
- `python /home/ecochran76/workspace.local/agent-policies/repo-policy-selector/scripts/audit_planning_contract.py --repo-root /home/ecochran76/workspace.local/agent-browser --json`

Result:

- The focused Plan 0039 parser and cleanup tests passed, the non-live Rust,
  client, docs, and dashboard gates above passed, and the installed skill copy
  matches the repo skill.
- The direct documented dry-run command
  `agent-browser remote-view open --runtime-profile stealthcdp-default
  --browser-build stealthcdp_chromium --provider rdp_gateway --url
  https://www.linkedin.com/ --dry-run` returned `success=true` and
  `status=planned`.
- The repo-wide planning audit still reports older unrelated drift, but the
  Plan 0039 row remains clean: `state=CLOSED`, `current_state_ok=true`,
  `wired_in_roadmap=true`, and `wired_in_runbook=true`.

## Turn 18 | 2026-06-21

Scope: close Plan 0039 by making the route-specific `remote_view_open` lane the
documented default and proving it on the installed binary.

Actions:

- Added prelaunch route-display access repair to `remote_view_open`: it probes
  the selected route display, invokes the installed privileged helper when
  access is missing, and fails with typed display-access errors if access still
  cannot be proven.
- Fixed route binding selection so checked-out retained routes reuse their
  existing display allocation when no inline route material overrides them.
- Updated README, CLI help, docs site, service-request contract description,
  repo skill, installed skill, Plan 0039, ROADMAP, and downstream handoff note
  `docs/dev/notes/2026-06-21-remote-view-open-route-specific-handoff.md`.
- Rebuilt and installed binary SHA
  `54248451b6bea3ced7acb6df8dd3e0f7514c866e08584bb025569a2ec6ad28ad` into
  `~/.local/bin/agent-browser`, `bin/agent-browser-linux-x64`, and the pnpm
  package binary.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open -- --test-threads=1`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm --dir docs build`
- `pnpm test:service-client`
- `cargo test --manifest-path cli/Cargo.toml remote_view_doctor -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_contracts -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_model -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_access_plan -- --test-threads=1`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:dashboard-view-streams`
- `pnpm test:dashboard-inspector-actions`
- `pnpm build:dashboard`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `agent-browser install doctor --json`
- `agent-browser doctor remote-view --json`
- `pnpm test:remote-view-open-fixture-live`
- `pnpm test:rdp-guac-many-to-many-live`
- `git diff --check`
- `python /home/ecochran76/workspace.local/agent-policies/repo-policy-selector/scripts/audit_planning_contract.py --repo-root /home/ecochran76/workspace.local/agent-browser --json`

Result:

- `agent-browser install doctor --json` passed with no issues and aligned SHA
  `54248451b6bea3ced7acb6df8dd3e0f7514c866e08584bb025569a2ec6ad28ad`.
- `agent-browser doctor remote-view --json` reported `status=ready`,
  `remoteControl.status=ready`, `remoteControl.routeId=guacamole:3`,
  `remoteControl.displayName=:11`, and `manyToMany.status=ready`.
- `pnpm test:remote-view-open-fixture-live` passed with artifact directory
  `/tmp/agent-browser-remote-view-open-live-2026-06-21T01-24-32-095Z`.
- `pnpm test:rdp-guac-many-to-many-live` passed with artifact directory
  `/tmp/agent-browser-rdp-guac-many-to-many-2026-06-21T01-24-32-207Z`.
- `git diff --check` passed.
- The repo-wide planning audit still reports older unrelated planning-contract
  drift, but the Plan 0039 row is clean: `state=CLOSED`,
  `current_state_ok=true`, `wired_in_roadmap=true`, and
  `wired_in_runbook=true`.
- Plan 0039 and P16 are closed.

## Turn 17 | 2026-06-20

Scope: continue Plan 0039 remote-control ready command hardening after the
route-specific Guacamole/RDP lane exposed stale retained route state.

Actions:

- Repaired the retained service route pool from the current route-pool
  readiness report after backing up
  `~/.agent-browser/service/state.json.pre-route-pool-refresh-2026-06-21T00-56-42-211Z`.
- Changed `remote_view_open` route binding to prefer supplied/current
  route-pool identity over stale retained route id and display allocation
  state.
- Made requested route-pool entry id authoritative for allocation lookup and
  allowed top-level `readiness.state=ready` route-pool entries to be used even
  when informational nested components are not ready.
- Updated the remote-view open live smoke to use the selected route entry's
  display name and display isolation for CLI, HTTP, state, and X11 checks.
- Rebuilt and installed the local binary into `~/.local/bin/agent-browser`,
  `bin/agent-browser-linux-x64`, and the pnpm global package binary.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_dry_run_prefers_inline_route_pool_identity_over_stale_state -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml remote_view_doctor -- --test-threads=1`
- `node --check scripts/smoke-rdp-guac-route-pool-readiness.js`
- `node --check scripts/open-rdp-guac-route-displays.js`
- `node --check scripts/test-rdp-guac-many-to-many-live.js`
- `node --check scripts/smoke-remote-view-open-live.js`
- `pnpm test:remote-view-open-fixture-live`
- `pnpm test:rdp-guac-many-to-many-live`
- `agent-browser install doctor --json`
- `agent-browser doctor remote-view --json`
- `git diff --check`

Result:

- Route-specific `remote-view open` dry-run resolves `guacamole-rdp-a` to
  `guacamole:3`, display `:11`, and display allocation
  `remote-view-display:11`.
- `pnpm test:remote-view-open-fixture-live` passed with artifact directory
  `/tmp/agent-browser-remote-view-open-live-2026-06-21T01-05-37-262Z`.
- `pnpm test:rdp-guac-many-to-many-live` passed with artifact directory
  `/tmp/agent-browser-rdp-guac-many-to-many-2026-06-21T01-05-55-809Z`.
- `agent-browser doctor remote-view --json` reports `status=ready`,
  `remoteControl.status=ready`, and `manyToMany.status=ready`.
- Plan 0039 remains open only for Slice F documentation and downstream
  handoff closeout.

## Turn 1 | 2026-05-26

Scope: repair the planning contract after adopting Graphiti and CodeGraph
policy modules.

Actions:

- Added top-level `ROADMAP.md` as the planning index.
- Added top-level `RUNBOOK.md` as the dated execution log.
- Wired `docs/dev/plans/0001-2026-05-26-rdp-guac-hardening-test-plan.md`
  into both planning authorities.
- Changed plan 0001's deterministic plan state to `CLOSED` while preserving
  its `VALIDATED` outcome.

Validation run:

- `python /home/ecochran76/workspace.local/agent-policies/repo-policy-selector/scripts/audit_planning_contract.py --repo-root /home/ecochran76/workspace.local/agent-browser --json`
- `git diff --check`

Result:

- Both checks passed for the planning-contract repair.

## Turn 2 | 2026-05-27

Scope: create the Guacamole remote-view routing hardening lane after roadmap
alignment review.

Actions:

- Added `docs/dev/plans/0002-2026-05-27-guac-remote-view-routing-hardening-plan.md`.
- Added P02 to `ROADMAP.md`.
- Kept P01 closed and made the hardcoded Guacamole route, metadata-only
  `view_takeover`, and external-open race the explicit P02 scope.

Validation run:

- `python /home/ecochran76/workspace.local/agent-policies/repo-policy-selector/scripts/audit_planning_contract.py --repo-root /home/ecochran76/workspace.local/agent-browser --json`
- `git diff --check`

Result:

- Both checks passed for the P02 planning turn.

## Turn 3 | 2026-05-27

Scope: implement the first Guacamole route hardening slices.

Actions:

- Added `docs/dev/notes/2026-05-27-guac-route-authority-audit.md`.
- Added service-owned `ViewStream` route metadata: `frameUrl`,
  `externalUrl`, `routeId`, `connectionId`, `connectionName`, and
  `routeSource`.
- Removed production Guacamole client-hash repair from Rust service status
  handling and the dashboard workspace viewport.
- Changed dashboard external open to await `view_takeover` acceptance before
  opening `externalUrl`.
- Changed `view_takeover` to return typed acceptance metadata and persist a
  `viewer_takeover_requested` service event with `viewerLeaseId` and route
  details.
- Updated README, CLI help, docs site pages, service contracts, generated
  observability client, harness artifacts, and the repo plus installed
  `agent-browser` skill.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml remote_headed_view_stream -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml guacamole -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml view_takeover -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_events -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml apply_remote_headed_launch_env_hints -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml apply_daemon_env_forwards_keychain_settings -- --test-threads=1`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm test:dashboard-view-streams`
- `pnpm test:dashboard-workspace-nodes`
- `pnpm test:dashboard-workspace-navigator`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:service-client-contract`
- `pnpm test:service-client-types`
- `pnpm --dir docs build`
- `pnpm build:dashboard`
- `pnpm test:dashboard-browser-table`
- `pnpm test:dashboard-browser-row-actions-render`
- `pnpm test:dashboard-launcher-eligibility`
- `pnpm test:dashboard-inspector-actions`
- `node --check scripts/smoke-remote-headed-utils.js`
- `node --check scripts/test-rdp-guac-browser-switch-live.js`
- `node --check scripts/test-rdp-guac-viewer-transfer-live.js`
- `pnpm test:rdp-gateway-readiness-live -- --require-html5-client`
- `AGENT_BROWSER_RDP_TEST_CLIENT_A_EXECUTABLE=/usr/bin/google-chrome AGENT_BROWSER_RDP_TEST_CLIENT_B_EXECUTABLE=/usr/bin/brave-browser AGENT_BROWSER_REMOTE_HEADED_DISPLAY=:0 pnpm test:rdp-guac-viewer-transfer-live`
- `AGENT_BROWSER_RDP_TEST_CLIENT_A_EXECUTABLE=/usr/bin/google-chrome AGENT_BROWSER_RDP_TEST_CLIENT_B_EXECUTABLE=/usr/bin/brave-browser AGENT_BROWSER_REMOTE_HEADED_DISPLAY=:0 pnpm test:rdp-guac-browser-switch-live`
- `git diff --check`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`

Result:

- Local source and contract validation passed.
- Live readiness, viewer-transfer, and browser-switch validation passed for
  the configured shared Guacamole route.
- Viewer-transfer artifacts:
  `/tmp/agent-browser-rdp-guac-hardening-2026-05-27T19-40-36-319Z`
- Browser-switch artifacts:
  `/tmp/agent-browser-rdp-guac-browser-switch-2026-05-27T19-41-29-855Z`

## Turn 4 | 2026-05-29

Scope: refactor the P05 handoff after maintainer clarification that the
Guacamole/RDP campaign is not ready for a formal release.

Actions:

- Reframed P05 as a validated installed-runtime checkpoint instead of a release
  preparation lane.
- Replaced the P05 plan with
  `docs/dev/plans/0005-2026-05-29-runtime-checkpoint-and-no-release-handoff-plan.md`.
- Added P06 in
  `docs/dev/plans/0006-2026-05-29-guac-rdp-productization-hardening-plan.md`.
- Removed the public docs changelog `v0.27.0` entry and kept current work under
  `## Unreleased` in `CHANGELOG.md`.
- Kept `CHANGELOG.md` release markers around the latest published `0.26.1`
  release entry.
- Changed `.github/workflows/release.yml` to manual dispatch only so ordinary
  pushes to `main` cannot publish checkpoint work as a GitHub release.
- Updated `AGENTS.md` and `ROADMAP.md` with the formal release boundary:
  release only after the hardened many-to-many Guacamole/RDP operational
  milestone, including one-time-sudo install and fully diagnostic doctors.

Validation run:

- `git diff --check`
- `pnpm --dir docs build`
- `pnpm validation:select -- --base HEAD`
- `pnpm version:sync`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `agent-browser --version`
- `agent-browser install doctor --json`
- `agent-browser doctor remote-view --json`

Result:

- Checks passed. The installed runtime reports `agent-browser 0.27.0`, install
  doctor is successful with matching installed, workspace, and pnpm package
  binary checksum
  `e99093bb46891983afe71c2bf992a5f5c1ded16ecbbd29504a3e9e55a16be33f`, and
  remote-view doctor reports route pool, route displays, display access,
  privileged helper, and simultaneous viewing readiness with
  `requiresInteractiveSudo=false`.

## Turn 5 | 2026-05-29

Scope: execute and refactor the first P06 slice after auditing the installed
checkpoint against the productization issues from P05.

Actions:

- Added install-doctor remote-view privilege readiness fields for helper,
  sudoers, group, membership, helper check, nested issues, and
  `requiresInteractiveSudo`.
- Added remote-view doctor top-level issue codes, remediations, viewer browser
  and OCR prerequisites, install drift propagation, sudoers readiness, and
  many-to-many prerequisite status.
- Changed the many-to-many live harness to prefer installed `agent-browser`,
  hydrate route-pool and route-display environment from remote-view doctor
  output, auto-discover common viewer browsers, and fail public Guacamole route
  URLs with `non_embeddable_guacamole_url`.
- Updated README, CLI help, docs site pages, the repo skill guidance, the P06
  plan, the roadmap, and the P06 validation note.
- Rebuilt and installed the checkpoint binary to the local command, workspace
  binary, and pnpm package binary.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml install_doctor -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml remote_view_doctor -- --test-threads=1`
- `node --check scripts/test-rdp-guac-many-to-many-live.js`
- `node --check scripts/smoke-utils.js`
- `pnpm --dir docs build`
- `agent-browser install doctor --json`
- `agent-browser doctor remote-view --json`
- `AGENT_BROWSER_RDP_TEST_USE_INSTALLED=1 node scripts/test-rdp-guac-many-to-many-live.js`
- `AGENT_BROWSER_REMOTE_VIEW_URL=http://127.0.0.1:8092/guacamole/ AGENT_BROWSER_RDP_TEST_USE_INSTALLED=1 node scripts/test-rdp-guac-many-to-many-live.js`

Result:

- Installed doctor and remote-view doctor passed with no issues. The installed
  runtime checksum is
  `1b67077ccdb5e80d8667d3bcc8327e9c2a1a8521417c25280f71d059bc3b1694`.
- The public Guacamole URL invocation failed fast with the intended
  `non_embeddable_guacamole_url` precondition diagnostic.
- The local embeddable Guacamole many-to-many gate passed from the installed
  command with artifacts at
  `/tmp/agent-browser-rdp-guac-many-to-many-2026-05-29T14-06-07-291Z`.
- P06 remains open for clean-machine first-install sudo proof and the
  install-doctor service-readiness ownership decision.

## Turn 6 | 2026-05-29

Scope: continue P06 by resolving the remaining install-doctor service
ownership decision and strengthening the already-provisioned privilege
installer re-run contract.

Actions:

- Added `data.service` to `agent-browser install doctor --json` using an
  isolated no-launch service-status probe.
- Made install doctor fail with `service_status_not_ready` when the no-launch
  service probe does not report ready.
- Changed `scripts/install-agent-browser-privileges.sh --apply` to exit before
  privileged changes when the helper source matches the installed helper, the
  sudoers file exists, the operator is in the `agent-browser` group, and
  `sudo -n <helper> check` succeeds.
- Updated CLI help, README, docs site installation/service-mode pages, skill
  guidance, the P06 plan, roadmap, and validation note.

Validation run:

- `cargo run --quiet --manifest-path cli/Cargo.toml -- install doctor --json`
- `cargo test --manifest-path cli/Cargo.toml install_doctor -- --test-threads=1`
- `bash -n scripts/install-agent-browser-privileges.sh`
- `AGENT_BROWSER_PRIVILEGED_HELPER_SOURCE=scripts/libexec/agent-browser-privileged-helper bash scripts/install-agent-browser-privileges.sh --dry-run`
- `AGENT_BROWSER_PRIVILEGED_HELPER_SOURCE=scripts/libexec/agent-browser-privileged-helper bash scripts/install-agent-browser-privileges.sh --apply`
- `pnpm build:native`
- `agent-browser install doctor --json`
- `agent-browser doctor remote-view --json`

Result:

- The source-build install doctor showed the new service probe as ready and
  no-launch, while still correctly reporting source/install binary drift.
- The already-provisioned helper installer re-run exited with "already ready"
  and made no privileged changes.
- The rebuilt installed runtime checksum is
  `1ec7a0528944fad76fc4b3c2539b57b15944a503126038e47fb9d8727bdfa53a`.
- Installed doctor and remote-view doctor passed with no issues, and install
  doctor reports `data.service.ready=true` plus `data.service.noLaunch=true`.
- P06 remains open for clean-host or equivalent reset-fixture proof that first
  install uses one clear sudo authorization boundary.

## Turn 7 | 2026-05-29

Scope: finish P06 by proving the first-install sudo boundary with an equivalent
clean reset fixture, validating route-pool restart durability, and running the
final installed gates.

Actions:

- Added `pnpm test:install-privileges-clean-fixture`, which runs the privilege
  installer against fake `sudo`, `getent`, `id`, `groupadd`, `usermod`, and
  `visudo` under a temp install root.
- Reordered the Linux install path so
  `agent-browser install --with-deps --with-remote-view-privileges` runs
  remote-view privilege setup before dependency installation.
- Added a Rust guard that keeps remote-view privilege setup before Linux
  dependency installation.
- Updated README, docs site installation guidance, skill guidance, P06 plan,
  roadmap, and P06 validation note.
- Rebuilt and installed the checkpoint binary to the local command, workspace
  binary, and pnpm package binary.

Validation run:

- `pnpm test:install-privileges-clean-fixture`
- `cargo test --manifest-path cli/Cargo.toml install_orders_remote_view_privileges_before_linux_deps -- --test-threads=1`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm --dir docs build`
- `pnpm build:native`
- `agent-browser install doctor --json`
- `agent-browser doctor remote-view --json`
- `node scripts/smoke-rdp-guac-route-pool-readiness.js --report-only`
- `docker restart agent-browser-guacamole agent-browser-guacd && node scripts/smoke-rdp-guac-route-pool-readiness.js --report-only`
- `pnpm sync:rdp-guac-existing-user-route-pool`
- `pnpm grant:rdp-route-display-access -- --apply`
- `agent-browser --json get title`
- `AGENT_BROWSER_REMOTE_VIEW_URL=http://127.0.0.1:8092/guacamole/ AGENT_BROWSER_RDP_TEST_USE_INSTALLED=1 node scripts/test-rdp-guac-many-to-many-live.js`
- `AGENT_BROWSER_RDP_TEST_USE_INSTALLED=1 node scripts/test-rdp-guac-many-to-many-live.js`

Result:

- The clean-fixture smoke proved first apply uses exactly one explicit
  `sudo -v` boundary and second apply does not add another prompt boundary or
  repeat privileged install commands.
- Installed doctor and remote-view doctor passed with no issues. The final
  P06 installed runtime checksum is
  `cb9f81a245464c516d313aee875fa076049cdc5559e9342250c9680463faa9e4`.
- Route-pool readiness survived Guacamole web and guacd restarts.
- Route sync and route-display access grant reruns passed without interactive
  sudo.
- Default command attach passed.
- The local embeddable Guacamole many-to-many gate passed with artifacts at
  `/tmp/agent-browser-rdp-guac-many-to-many-2026-05-29T14-39-55-085Z`.
- The public Guacamole URL invocation failed fast with the intended
  `non_embeddable_guacamole_url` diagnostic and artifacts at
  `/tmp/agent-browser-rdp-guac-many-to-many-2026-05-29T14-40-34-292Z`.
- P06 is closed. Formal release work remains a separate lane.

## Turn 8 | 2026-05-29

Scope: open the formal release lane now that P06 closed the Guacamole/RDP
productization blocker.

Actions:

- Created
  `docs/dev/plans/0007-2026-05-29-v0-27-0-formal-release-plan.md`.
- Moved `CHANGELOG.md` release extraction markers from `0.26.1` to `0.27.0`.
- Added the public docs changelog entry for `v0.27.0` dated May 29, 2026.
- Added P07 to `ROADMAP.md`.
- Added release-preparation validation note
  `docs/dev/notes/2026-05-29-p07-v0-27-0-release-prep-validation.md`.

Validation run:

- `git log v0.26.1..HEAD --format='%an <%ae>' | sort -u`
- `pnpm version:sync`
- `git diff --check`
- `pnpm validation:select -- --base HEAD`
- `pnpm --dir docs build`
- `agent-browser install doctor --json`
- `agent-browser doctor remote-view --json`

Result:

- Local release-preparation validation passed. The installed runtime checksum
  remains
  `cb9f81a245464c516d313aee875fa076049cdc5559e9342250c9680463faa9e4`.
- P07 remains open for release PR merge, release workflow dry run, real
  release workflow run, and GitHub release asset verification.

## Turn 9 | 2026-05-29

Scope: respond to the first manual `Release` workflow dry-run failure.

Actions:

- Ran the `Release` workflow with `dry_run=true` on `main`.
- Confirmed release-state precheck passed.
- Diagnosed the platform build failures as a Rust cfg leak in
  `cli/src/native/cdp/chrome.rs`.
- Kept the private remote-headed virtual-display fallback inside the Linux cfg
  block so non-Linux targets do not reference Linux-only helpers.
- Added
  `docs/dev/notes/2026-05-29-p07-release-dry-run-cross-target-fix.md`.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml private_remote_display -- --test-threads=1`
- `cargo check --manifest-path cli/Cargo.toml --target x86_64-pc-windows-gnu`

Result:

- Format, clippy, and the focused private remote-display unit test passed.
- The local Windows cross-target check advanced past the previous missing
  symbols, then stopped because this workstation lacks
  `x86_64-w64-mingw32-gcc` for the `ring` build script.
- The release workflow dry run must be retried after this fix lands on `main`.

## Turn 10 | 2026-05-29

Scope: respond to the second manual `Release` workflow dry-run failure.

Actions:

- Reran the `Release` workflow with `dry_run=true` on `main`.
- Confirmed Windows x64, macOS x64, and macOS ARM64 passed after the cfg fix.
- Diagnosed Linux target failures as release-time `-lX11` linking from the
  browser-focus helper.
- Changed the Linux X11 focus helper to load `libX11` dynamically with
  `dlopen` and `dlsym` at runtime instead of statically linking X11.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml browser -- --test-threads=1`
- `git diff --check`
- `cargo build --release --manifest-path cli/Cargo.toml`
- `rg -n "#\\[link\\(name = \\\"X11\\\"\\)|-lX11" cli/src`

Result:

- Local validation passed.
- No static X11 link remains in `cli/src`.
- The local machine does not have `cargo-zigbuild`, so the release workflow
  dry run must be retried after this fix lands on `main`.

## Turn 11 | 2026-05-29

Scope: publish and verify the formal `v0.27.0` GitHub release.

Actions:

- Reran the manual `Release` workflow with `dry_run=true`.
- Ran the manual `Release` workflow with `dry_run=false` after the dry run
  passed.
- Verified the public GitHub release and asset list.
- Closed P07 in the roadmap and plan surfaces.

Validation run:

- `gh run view 26648621169 --json conclusion,url,headSha`
- `gh run view 26649196974 --json conclusion,url,headSha`
- `gh release view v0.27.0 --json tagName,name,url,isDraft,isPrerelease,assets,targetCommitish`
- `git fetch --tags origin`
- `git rev-list -n1 v0.27.0`
- `git rev-parse origin/main`

Result:

- Dry run succeeded:
  `https://github.com/CochranResearchGroup/agent-browser/actions/runs/26648621169`
- Real release run succeeded:
  `https://github.com/CochranResearchGroup/agent-browser/actions/runs/26649196974`
- Release URL:
  `https://github.com/CochranResearchGroup/agent-browser/releases/tag/v0.27.0`
- Release commit and `origin/main` both resolve to
  `17a284f8624e6108473970e2ec2b380debf9f7ac`.
- The release is not a draft, is not a prerelease, and has seven assets:
  `agent-browser-darwin-arm64`, `agent-browser-darwin-x64`,
  `agent-browser-linux-arm64`, `agent-browser-linux-musl-arm64`,
  `agent-browser-linux-musl-x64`, `agent-browser-linux-x64`, and
  `agent-browser-win32-x64.exe`.

## Turn 12 | 2026-05-29

Scope: repair stale planning-audit residue after the `v0.27.0` release.

Actions:

- Normalized historical runbook headings to the deterministic
  `## Turn N | YYYY-MM-DD` format.
- Changed P02 plan state from `VALIDATED` to deterministic `CLOSED` while
  preserving `Outcome: VALIDATED`.
- Changed P03 plan state from `COMPLETE` to deterministic `CLOSED` while
  preserving `Outcome: COMPLETE`.
- Wired the existing P03 and P04 plan filenames into this runbook:
  `docs/dev/plans/0003-2026-05-28-guac-rdp-many-to-many-viewing-plan.md` and
  `docs/dev/plans/0004-2026-05-29-release-candidate-install-validation-plan.md`.

Validation run:

- `python /home/ecochran76/workspace.local/agent-policies/repo-policy-selector/scripts/audit_planning_contract.py --repo-root /home/ecochran76/workspace.local/agent-browser --json`
- `git diff --check`

Result:

- Both checks passed. The planning audit now reports `ok: true`, no problems,
  no open roadmap lanes, deterministic state for every plan, and runbook plus
  roadmap wiring for every plan.

## Turn 13 | 2026-05-30

Scope: open the CDP tab streaming lane for non-remote browsers.

Actions:

- Ran Graphiti discovery against `agent_browser_main` for prior CDP streaming
  context.
- Inspected the existing CDP stream server, stream WebSocket, service
  view-stream model, action-derived view streams, dashboard view-stream
  rendering, roadmap, and runbook surfaces.
- Added
  `docs/dev/plans/0008-2026-05-30-cdp-tab-streaming-for-non-remote-browsers-plan.md`.
- Added P08 to `ROADMAP.md`.

Validation run:

- `python /home/ecochran76/workspace.local/agent-policies/repo-policy-selector/scripts/audit_planning_contract.py --repo-root /home/ecochran76/workspace.local/agent-browser --json`
- `git diff --check`
- `pnpm validation:select -- --base HEAD`

Result:

- Planning audit passed with `ok: true`, no problems, and P08 wired through the
  roadmap, runbook, and open plan file.
- `git diff --check` passed.
- `pnpm validation:select -- --base HEAD` selected only `git diff --check` for
  the documentation-only planning slice.

## Turn 14 | 2026-06-04

Scope: open a resource monitor and garbage collector lane after live
agent-browser resource pressure cleanup.

Actions:

- Ran Graphiti discovery against `agent_browser_main` for prior resource
  cleanup and service lifecycle context.
- Confirmed the related retained orphan profile cleanup plan exists, but it
  covers service-state/profile metadata rather than live OS process pressure.
- Added
  `docs/dev/plans/0026-2026-06-04-resource-monitor-and-garbage-collector-plan.md`.
- Added P13 to `ROADMAP.md` with the dry-run-first resource monitor and GC
  recommendation.

Validation run:

- `git diff --check`
- `pnpm validation:select -- --base HEAD`
- `python /home/ecochran76/workspace.local/agent-policies/repo-policy-selector/scripts/audit_planning_contract.py --repo-root /home/ecochran76/workspace.local/agent-browser --json`

Result:

- P13 is open for Slice A and Slice B: read-only resource inventory plus
  conservative stale classification before any apply-mode garbage collection.
- `git diff --check` passed.
- `pnpm validation:select -- --base HEAD` included the pre-existing dirty
  dashboard files in its recommendation set, so it selected dashboard checks in
  addition to the documentation-only change.
- The planning audit still fails due to pre-existing roadmap/runbook drift for
  older plans, but the new P13 plan is wired in both `ROADMAP.md` and
  `RUNBOOK.md`.

## Turn 15 | 2026-06-05

Scope: open and start the minimal runtime-profile reuse lane after Plan 0026
closed the resource-monitor and GC cleanup surface.

Actions:

- Ran Graphiti discovery against `agent_browser_main` for profile reuse,
  service queue, lease, and access-plan context.
- Added
  `docs/dev/plans/0027-2026-06-05-minimal-runtime-profile-reuse-plan.md`.
- Updated P13 in `ROADMAP.md` so Plan 0026 is the closed cleanup surface and
  Plan 0027 is the prevention surface.

Current target:

- Plan 0027 Slice A: add a read-only access-plan `profileReuse` advisory that
  recommends `reuse_existing_browser`, `wait_for_profile_lease`, or
  `launch_new_browser` before any launch mutates runtime state.

## Turn 16 | 2026-06-13

Scope: write an implementation handoff note for AuraCall-driven browser
service feature requests.

Actions:

- Ran Graphiti discovery against `agent_browser_main` and verified the local
  Graphiti runtime was healthy.
- Reviewed the existing access-plan service-request handoff note and the
  service request/client contract surfaces.
- Added
  `docs/dev/notes/2026-06-13-auracall-cdp-feature-requests.md`.
- Patched the note so AuraCall source paths are explicitly relative to the
  sibling `../auracall` repository.

Validation run:

- `git diff --check`
- Verified the listed agent-browser source surfaces exist in this repository.
- Verified the listed AuraCall source surfaces exist under the sibling
  `../auracall` repository.
- Ran Graphiti discovery against `agent_browser_main` for AuraCall CDP
  migration, BYOP, controlled CDP attach, bounded evaluate, and service tab
  handle context.

Result:

- The handoff note requests profile-origin and BYOP registration, a
  lease-backed service tab handle, controlled CDP attach, bounded evaluate
  jobs, readiness and identity probe recipes, tab reuse repair, diagnostic
  evidence bundles, and service-client ergonomics.
- The note keeps provider-specific ChatGPT, Gemini, Grok, and AuraCall
  semantics out of agent-browser and frames the work as service-owned browser
  primitives for a future implementation agent.

## Turn 17 | 2026-06-13

Scope: open a high-level upgrade plan suitable for subagents and goal-driven
execution.

Actions:

- Added
  `docs/dev/plans/0033-2026-06-13-auracall-service-cdp-upgrade-plan.md`.
- Added P14 to `ROADMAP.md`.
- Structured the plan as a parent goal with slice-level subagent prompts,
  acceptance criteria, coordination rules, validation matrix, and open
  questions.

Validation run:

- `git diff --check`
- `python /home/ecochran76/workspace.local/agent-policies/repo-policy-selector/scripts/audit_planning_contract.py --repo-root /home/ecochran76/workspace.local/agent-browser --json`
- `pnpm validation:select -- --base HEAD`

Result:

- P14 is open for profile origin/BYOP, lease-backed service tab handles,
  controlled CDP attach, bounded evaluate, diagnostics/readiness evidence, and
  client ergonomics.
- The first recommended implementation slice is P14 Slice A: profile-origin
  schema plus explicit BYOP registration/readback.

## Turn 18 | 2026-06-13

Scope: implement P14 Slice A profile-origin and BYOP registration/readback.

Actions:

- Added durable service profile origin values:
  `agent_browser_owned`, `external_byop`, and `external_observed`.
- Added external profile registration metadata and browser compatibility
  evidence to service profile records.
- Added `registerExternalProfile()` to
  `@agent-browser/client/service-observability` for explicit BYOP or observed
  external profile registration.
- Exposed `profileOrigin` through service profile allocation readback and
  access-plan selected profiles.
- Hardened retained-state orphan profile pruning so `external_byop` and
  `external_observed` profiles are never pruned as owned profile data.
- Preserved profile origin and external metadata through the dashboard profile
  config save path.
- Updated service schemas, generated client types, README, docs site, and the
  installed agent-browser skill.

Validation run:

- `git diff --check`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml service_model -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_access_plan -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_profiles -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml test_prune_retained_service_state_removes_orphaned_custom_profiles -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `pnpm test:service-client`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:dashboard-profile-allocation`
- `pnpm --dir docs build`
- `pnpm build:dashboard`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`

Result:

- Slice A is implemented as a no-launch contract slice.
- Access-plan and profile readback can distinguish owned, BYOP, and observed
  external profile lanes.
- Explicit external profile registration records caller identity, target
  identities, account ids, user-data directory, and browser compatibility
  evidence.
- The next recommended P14 slice is Slice B: lease-backed service tab handles.

## Turn 19 | 2026-06-13

Scope: implement P14 Slice B lease-backed service tab handles.

Actions:

- Added `ServiceTabHandle` and `ServiceTabHandleTraceFilter` to the service
  model.
- Derived tab handles from service state for `service tabs`, grouped browser
  `tabHandles`, and tab lifecycle trace event details.
- Extended direct `tab_new` responses with CDP target/session IDs and a
  conservative immediate `serviceTabHandle`.
- Added `getServiceTabHandle()` and `requireServiceTabHandle()` to
  `@agent-browser/client/service-request`.
- Updated service tab/browser schemas, generated client declarations, README,
  docs site, and the installed agent-browser skill.
- Added no-launch Rust and service-client fixtures for valid handles, binding
  fields, and stale-handle rejection.

Validation run:

- `git diff --check`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml service_model -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_health -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `pnpm test:service-client`
- `pnpm test:service-api-mcp-parity`
- `pnpm --dir docs build`
- `pnpm validation:select -- --base HEAD`
- `node scripts/dev/select-validation.js --base HEAD --json`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`

Result:

- Slice B is implemented as a no-launch contract slice.
- Software clients can use the returned service tab handle instead of
  rediscovering browser, session, profile, tab, target, lease, or trace
  identity.
- Stale handles fail closed through the client helper and expose explicit
  stale reasons in service readbacks.
- The selector recommended `pnpm test:service-cdp-tab-streaming-live` because
  browser/tab surfaces changed; that live smoke was deferred to Slice C unless
  live proof is requested before controlled CDP attach work starts.
- The next recommended P14 slice is Slice C: controlled CDP attach for leased
  service tab handles.

## Turn 20 | 2026-06-13

Scope: implement P14 Slice C controlled CDP attach for leased service tab
handles.

Actions:

- Added `cdp_attach` and `cdp_detach` to the service request action metadata,
  HTTP relay, MCP service request surface, Rust daemon dispatcher, JSON schema,
  generated client types, and `@agent-browser/client/service-request` helpers.
- Gated attach on a valid `serviceTabHandle`, `cdpAttachmentAllowed: true`,
  non-CDP-free posture, matching service session, handle freshness, and target
  identity.
- Returned a service-owned attach descriptor with browser, session, tab,
  target, profile, lease, cleanup, trace, websocket, and detach metadata.
- Made detach preserve the browser process by default and return explicit
  detach metadata.
- Updated README, docs site, repo skill, and installed agent-browser skill for
  the new attach/detach helper path.
- Updated P14 plan and ROADMAP so Slice D bounded evaluate is the next
  implementation target.

Validation run:

- `git diff --check`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml service_request_command -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml service_contracts -- --test-threads=1`
- `pnpm test:service-client`
- `pnpm test:service-api-mcp-parity`
- `pnpm --dir docs build`
- `pnpm validation:select -- --base HEAD`
- `node scripts/dev/select-validation.js --base HEAD --json`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `pnpm test:service-cdp-tab-streaming-live`

Result:

- Slice C is implemented with no-launch policy and stale-handle coverage.
- Live CDP tab-streaming smoke passed for `session:cdp-tab-stream-98925`,
  stream `37669`.
- A dedicated attach-read-detach live smoke remains as the validation gap before
  treating controlled attach as AuraCall migration proof.
- The next recommended P14 slice is Slice D: bounded evaluate against leased
  service tab handles.

## Turn 21 | 2026-06-13

Scope: implement P14 Slice D bounded evaluate against leased service tab
handles.

Actions:

- Added `evaluate` to the service request action metadata, HTTP relay, MCP
  service request surface, JSON schema, generated client types, and
  `@agent-browser/client/service-request` helpers.
- Required `serviceTabHandle`, `script` or `expression`, positive `timeoutMs`,
  and positive `maxReturnBytes` for service-owned evaluate requests.
- Made service-bound evaluate skip browser auto-launch, switch to the handle's
  CDP target, execute with a daemon-side timeout, cap serialized return data,
  and return URL/title plus truncation metadata.
- Added no-launch HTTP, MCP, and service-client coverage for missing handles,
  missing caps, stale handles, and helper request shape.
- Updated README, docs site, repo skill, installed agent-browser skill, P14
  plan, and ROADMAP for the new bounded evaluate helper path.

Validation run:

- `git diff --check`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml service_request_command -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml service_contracts -- --test-threads=1`
- `pnpm test:service-client`
- `pnpm test:service-api-mcp-parity`
- `pnpm --dir docs build`
- `pnpm validation:select -- --base HEAD`
- `node scripts/dev/select-validation.js --base HEAD --json`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `pnpm test:service-cdp-tab-streaming-live`

Result:

- Slice D is implemented with no-launch contract coverage.
- Live CDP tab-streaming smoke passed for `session:cdp-tab-stream-73918`,
  stream `37595`.
- A dedicated live bounded-evaluate smoke remains as the validation gap before
  treating bounded evaluate as AuraCall migration proof.
- Screenshot-on-failure capture is deferred to Slice E diagnostic bundles so
  screenshot storage, caps, and trace links are implemented in one evidence
  surface.
- The next recommended P14 slice is Slice E: diagnostics and readiness
  evidence.

## Turn 22 | 2026-06-13

Scope: implement the P14 Slice E diagnostic bundle sub-slice for leased service
tab handles.

Actions:

- Added `diagnostics` to service request action metadata, HTTP relay, MCP
  service request validation, Rust daemon dispatch, JSON schema, generated
  client types, and `@agent-browser/client/service-request` helpers.
- Required a valid `serviceTabHandle` and reused the service-owned queue and
  handle validation path rather than adding a caller-owned browser path.
- Returned a compact evidence bundle with URL/title, browser/session/tab
  identity, profile readiness, route/view metadata, browser health, console
  entries, page errors, recent request summaries, snapshot summary, caller
  context, trace filter, and optional screenshot path.
- Added no-launch client helper coverage for request shape, stale handles, and
  evidence count caps.
- Updated README, docs site, repo skill, P14 plan, and ROADMAP for the new
  diagnostic helper path.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml service_request_command -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_contracts -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `pnpm generate:service-client`
- `pnpm test:service-client`
- `pnpm test:service-api-mcp-parity`
- `pnpm --dir docs build`
- `pnpm validation:select -- --base HEAD`
- `node scripts/dev/select-validation.js --base HEAD --json`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `pnpm test:service-cdp-tab-streaming-live`

Result:

- Slice E diagnostic bundles are implemented with no-launch contract coverage.
- Live CDP tab-streaming smoke passed for `session:cdp-tab-stream-95746`,
  stream `36831`.
- Slice E remains open for readiness/freshness lifecycle gating and any focused
  live diagnostics smoke requested before AuraCall migration proof.

## Turn 23 | 2026-06-20

Scope: open the corrective planning lane for recurring Guacamole/RDP
false-ready states after the live LinkedIn manual-auth route repair.

Actions:

- Added
  `docs/dev/plans/0039-2026-06-20-remote-control-ready-command-plan.md`.
- Added P16 to `ROADMAP.md`.
- Made the combined readiness invariant explicit: a remote-control browser is
  ready only when the selected browser window is loaded, visible, and
  controllable through the selected external Guacamole/RDP route.
- Captured the two recurring failure classes as plan gates:
  - Guacamole unhappy document or internal error caused by schema, route, URL,
    or permission drift.
  - Terminal-only remote desktop caused by browser/display mismatch.
- Scoped the next fix as a generic one-command/API path,
  `agent-browser remote-view open` and service action `remote_view_open`,
  rather than a LinkedIn-specific or AuraCall-specific repair.

Validation run:

- `python /home/ecochran76/workspace.local/agent-policies/repo-policy-selector/scripts/audit_planning_contract.py --repo-root /home/ecochran76/workspace.local/agent-browser --json`
- `git diff --check`

Result:

- Focused Plan 0039 validation passed. The broad planning audit remains red
  from pre-existing historical plan drift, but it reports no Plan 0039
  problems.
- Implementation remains open under Plan 0039. Slice A and Slice B are the
  recommended parallel starting points.

## Turn 24 | 2026-06-22

Scope: open the runtime convergence lane after remote-view and dashboard
binary harmonization exposed remaining runtime identity confusion.

Actions:

- Added `docs/dev/plans/0042-2026-06-22-runtime-convergence-plan.md`.
- Added P42 to `ROADMAP.md`.
- Captured the missing invariant: the dashboard runtime manifest proves only
  the dashboard service identity, not every active daemon session, stream
  backend, route helper, retained browser row, or foreign CDP browser.
- Scoped executable slices for active runtime inventory, daemon executable
  SHA-256 convergence, actionable doctor remedies, idempotent remote-view
  bootstrap, live rail boundaries, and one-command local convergence.
- Kept P41 foreign CDP discovery as a separate dependency so non-owned browser
  addressability is not confused with lifecycle ownership.

Validation run:

- `git diff --check`

Result:

- P42 is active and not complete. Slice D is already in progress through the
  Guacamole Postgres/schema bootstrap guard. The next implementation slice is
  daemon executable SHA convergence and active runtime inventory.

## Turn 25 | 2026-06-22

Scope: execute P42 Slice B daemon executable SHA convergence.

Actions:

- Added daemon executable SHA metadata next to the existing daemon version
  metadata.
- Made daemon reuse compare the invoking executable SHA-256 against the daemon
  SHA metadata when the invoking executable can be hashed.
- Treated missing daemon SHA metadata as stale by default, with
  `AGENT_BROWSER_ALLOW_LEGACY_DAEMON_SHA_REUSE=1` as an explicit reviewed
  compatibility escape hatch.
- Extended stale daemon cleanup to remove `<session>.sha256`.
- Updated P42 Slice B completion notes.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml daemon_executable_sha -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml cleanup_stale_files_removes_version_and_executable_sha -- --nocapture`

Result:

- Focused daemon SHA convergence tests passed. P42 remains open for active
  runtime inventory, doctor remedies, live rail convergence boundaries, and
  one-command local convergence.

## Turn 26 | 2026-06-22

Scope: execute P42 Slice A active runtime inventory in doctor output.

Actions:

- Added `runtimeInventory` to `agent-browser install doctor --json`.
- The inventory scans the daemon socket metadata directory without launching
  Chrome and reports daemon session PID, PID liveness, package version match,
  executable SHA-256 match, stream port, and metadata presence.
- Added `active_runtime_stale_executable` install doctor issues for active
  daemon sessions whose metadata is stale or incomplete.
- Lifted the install doctor's runtime inventory into
  `agent-browser doctor remote-view --json` as top-level `runtimeInventory`.
- Updated P42 Slice A completion notes.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml install_doctor -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml runtime_inventory_from_install -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml daemon_executable_sha -- --nocapture`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `./cli/target/debug/agent-browser install doctor --json`
- `./cli/target/debug/agent-browser doctor remote-view --json`

Result:

- Focused tests and clippy passed.
- The rebuilt debug-binary install doctor reported
  `runtimeInventory.status=stale`, `runtimeCount=4`, and `staleCount=4`.
- The rebuilt debug-binary remote-view doctor lifted the same inventory and
  reported `runtimeInventory.status=stale`. This intentionally made the
  debug-binary readback not remote-control ready against the installed runtime,
  proving stale active runtimes are no longer omitted from readiness.

## Turn 27 | 2026-06-22

Scope: execute the first P42 Slice C convergence doctor remedy.

Actions:

- Added session-scoped remedy metadata to `active_runtime_stale_executable`
  install doctor issues.
- Each stale daemon issue now carries `session`,
  `nextAction=restart_stale_daemon_session`, and an argv-safe remedy for
  `agent-browser close --session <session>`.
- Made remote-view doctor prefer
  `restart_stale_daemon_sessions_then_rerun_doctor` when install readiness is
  blocked by stale active daemon sessions.
- Updated P42 Slice C progress notes.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml install_doctor -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml recommend_next -- --nocapture`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo build --manifest-path cli/Cargo.toml`
- `./cli/target/debug/agent-browser install doctor --json`
- `./cli/target/debug/agent-browser doctor remote-view --json`

Result:

- Focused tests, clippy, and debug CLI build passed.
- The rebuilt debug-binary install doctor reported four
  `active_runtime_stale_executable` issues; the first issue carried
  `session=default`, `nextAction=restart_stale_daemon_session`, and
  `remedy.argv=["agent-browser","close","--session","default"]`.
- The rebuilt debug-binary remote-view doctor reported
  `nextAction=restart_stale_daemon_sessions_then_rerun_doctor` and a
  next-command explanation that points operators back to each issue's
  session-scoped `remedy.argv`.

## Turn 28 | 2026-06-22

Scope: execute P42 local binary/runtime convergence after publishing the
structured commits.

Actions:

- Extended `pnpm publish:local-dashboard` so it synchronizes the user-scoped
  install binary, ignored workspace package binary, and user pnpm package
  binary to the same freshly built executable by default.
- Added `--skip-reference-sync` for operator cases that intentionally do not
  want reference binaries changed.
- Published the current debug build to the local dashboard runtime and restarted
  `agent-browser-dashboard.service`.
- Applied the stale daemon restart path by invoking the three
  session-scoped remedies reported by install doctor. Those commands returned
  nonzero because `close --session` still routes through daemon restart, but
  the restart path did replace the stale daemon metadata and all active daemon
  rows converged.
- Reran publish after adding reference-binary sync so install doctor no longer
  failed on pnpm/workspace binary drift.

Validation run:

- `pnpm publish:local-dashboard -- --skip-browser --json`
- `agent-browser install doctor --json`
- `agent-browser doctor remote-view --json`

Result:

- The publish report synchronized
  `/home/ecochran76/.local/bin/agent-browser`,
  `bin/agent-browser-linux-x64`, and the user pnpm global package binary to
  `94d1d022b4f1315b2f3eb9ff08fdc3faa816d77960500c6b6854cab98161cfa8`.
- Installed `agent-browser install doctor --json` reported `success=true`,
  `runtimeInventory.status=converged`, `staleCount=0`, no issue codes, and
  matching PATH, pnpm, and workspace binary SHA-256 values.
- Installed `agent-browser doctor remote-view --json` reported `success=true`,
  `status=ready`, `remoteControl.ready=true`,
  `runtimeInventory.status=converged`, and
  `nextAction=run_many_to_many_live_gate`.
- Follow-up required: make stale daemon close remedies return success without
  depending on a daemon restart side effect.

## Turn 29 | 2026-06-22

Scope: finish P42 close/remedy and install-doctor probe convergence discovered
during local execution.

Actions:

- Added a `close --session` prestart path that targets an existing daemon
  before daemon convergence startup.
- Added explicit-session stale metadata cleanup for unauthorized or non-ready
  daemon close attempts, returning success with a warning instead of trying to
  start a replacement daemon.
- Classified running PID metadata without an addressable socket, stream, or
  port as `diagnostic` instead of stale active runtime inventory.
- Changed `service status` to execute locally before daemon startup.
- Changed install doctor service-status probing to use a unique owned session,
  terminate the owned probe daemon after reading status, and treat the isolated
  no-state probe as no-launch ready.
- Ran service GC apply for the orphaned Xvfb candidate that was blocking local
  install doctor readiness.
- Published the final local runtime and synchronized reference binaries.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml force_close_session_from_metadata -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml close_targets_existing_daemon_before_prestart -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml service_status_locally_before_daemon -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml install_doctor -- --nocapture`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm publish:local-dashboard -- --skip-browser --json`
- `agent-browser install doctor --json`
- `agent-browser doctor remote-view --json`

Result:

- Final local publish succeeded and restarted
  `agent-browser-dashboard.service`.
- Final installed executable SHA-256:
  `19ba0d616388e1eb84241eea5ddcffa56a1803831c5085acc25abb01277b78e6`.
- Reference binaries in `~/.local/bin`, ignored workspace `bin/`, and user
  pnpm global package path matched the installed executable SHA-256.
- Final installed `agent-browser install doctor --json` reported
  `success=true`, no issue codes, `runtimeInventory.status=none`,
  `runtimeCount=0`, and `staleCount=0`.
- Final installed `agent-browser doctor remote-view --json` reported
  `success=true`, `status=ready`, `remoteControl.ready=true`,
  `runtimeInventory.status=none`, `staleCount=0`, and
  `nextAction=run_many_to_many_live_gate`.

## Turn 30 | 2026-06-22

Scope: finish P42 live-rail and one-command local runtime convergence.

Actions:

- Added `pnpm converge:local-runtime` as a dry-run by default local operator
  convergence command.
- In apply mode, the command runs local dashboard publication, applies only
  doctor-reported `agent-browser close --session <name>` stale-daemon
  remedies, runs the Guacamole Postgres schema ensure, runs route-pool
  readiness, applies route display-access grants only when remote-view doctor
  asks for them, and reruns final doctors.
- Added `pnpm test:local-runtime-convergence` to lock the command contract,
  foreign-process refusal boundary, display-grant sequencing, and retained
  evidence behavior.
- Marked P42 Slice E done from the dashboard live-rail contract tests and
  Slice F done from command validation.

Validation run:

- `node --check scripts/converge-local-runtime.js`
- `node --check scripts/test-local-runtime-convergence.js`
- `pnpm test:local-runtime-convergence`
- `pnpm --silent converge:local-runtime -- --json`
- `pnpm --silent converge:local-runtime -- --apply --json --evidence-path /tmp/agent-browser-converge-local-runtime-evidence.json`
- `pnpm test:dashboard-workspace-navigator`
- `pnpm test:dashboard-workspace-nodes`

Result:

- Dry-run convergence returned `success=true`, final install doctor ready,
  final remote-view ready, zero safe stale remedies, and zero skipped remedies.
- Apply convergence returned `success=true`, wrote
  `/tmp/agent-browser-converge-local-runtime-evidence.json`, final install
  doctor ready, final remote-view ready, and zero skipped remedies.
- Dashboard workspace tests passed, proving the live rail keeps retained and
  no-action attention rows out of the live control surface and groups
  reachable non-owned CDP browsers separately.
- P42 remains open for Slice C stale dashboard/stream classifications and
  Slice D bootstrap hardening.

## Turn 31 | 2026-06-22

Scope: continue P42 Slice C by classifying stale or unreadable live dashboard
runtime manifests.

Actions:

- Added an install-doctor live dashboard manifest probe for the local
  `/api/runtime/manifest` endpoint.
- Kept dashboard-not-running as non-drift, but classified a running dashboard
  that serves no readable manifest or a mismatched executable SHA-256 as
  `dashboard_runtime_stale_or_unreadable`.
- Added a bounded remedy pointing to
  `pnpm converge:local-runtime -- --apply --json`.
- Updated remote-view doctor so that dashboard runtime drift recommends
  `converge_local_runtime_then_rerun_doctor` before generic install drift.
- Updated `pnpm converge:local-runtime -- --apply --json` so initial nonzero
  doctor JSON is treated as repairable input in apply mode instead of aborting
  before local publish.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml install_doctor -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml recommend_next -- --nocapture`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo build --manifest-path cli/Cargo.toml`
- `./cli/target/debug/agent-browser install doctor --json`
- `pnpm --silent converge:local-runtime -- --apply --json --evidence-path /tmp/agent-browser-converge-local-runtime-turn31-final.json`
- `agent-browser install doctor --json`

Result:

- Format, focused Rust tests, clippy, and debug CLI build passed.
- The rebuilt debug install doctor reported
  `dashboard_runtime_stale_or_unreadable` with `state=stale_executable` when
  the running dashboard manifest executable SHA-256 did not match the debug
  executable.
- Convergence apply started with initial install issue
  `dashboard_runtime_stale_or_unreadable`, published the new local runtime, and
  ended with final install doctor ready, final remote-view ready, zero skipped
  remedies, and retained evidence at
  `/tmp/agent-browser-converge-local-runtime-turn31-final.json`.
- Direct installed `agent-browser install doctor --json` then reported
  `success=true`, no issue codes, `liveDashboardRuntime.ready=true`,
  `liveDashboardRuntime.state=ready`, and `runtimeInventory.status=none`.
- P42 Slice C still has remaining stale stream-backend classification work.

## Turn 32 | 2026-06-22

Scope: continue P42 Slice C by adding explicit runtime convergence summary
states.

Actions:

- Added install-doctor `runtimeConvergence` with schema
  `agent-browser.runtime-convergence.v1`.
- Derived summary status from runtime inventory plus live dashboard manifest
  state, using `converged`, `partial`, `stale`, and
  `manual_review_required`.
- Lifted the summary into remote-view doctor and printed it in text output
  separately from raw runtime inventory status.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml runtime_convergence -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml install_doctor -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml recommend_next -- --nocapture`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm --silent converge:local-runtime -- --apply --json --evidence-path /tmp/agent-browser-converge-local-runtime-turn32.json`
- `agent-browser install doctor --json`

Result:

- Format, focused Rust tests, and clippy passed.
- Unit coverage now locks the `converged`, `partial`, `stale`, and
  `manual_review_required` summary statuses plus remote-view summary lifting.
- Convergence apply published the summary-state build and ended with final
  install doctor ready, final remote-view ready, zero skipped remedies, and
  retained evidence at `/tmp/agent-browser-converge-local-runtime-turn32.json`.
- Direct installed `agent-browser install doctor --json` reported
  `success=true`, no issue codes, `runtimeConvergence.status=converged`,
  `liveDashboardRuntime.state=ready`, and `runtimeInventory.status=none`.
- P42 Slice C still has remaining stale stream-backend and diagnostic
  retained-row classification work.

## Turn 33 | 2026-06-22

Scope: finish P42 Slice C stale stream-backend classification.

Actions:

- Extended runtime inventory to probe advertised daemon stream ports.
- Added runtime row `streamReachable` and `driftReasons` evidence.
- Classified live daemon sessions with unreachable or invalid stream metadata
  as stale instead of converged.
- Added install-doctor issue code `active_runtime_stale_stream_backend` with
  the bounded `agent-browser close --session <session>` remedy.
- Updated remote-view doctor to treat stale stream backends as a
  session-scoped daemon restart prerequisite before generic install drift.
- Marked P42 Slice C done.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml stream_backend -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml install_doctor -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml recommend_next -- --nocapture`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm --silent converge:local-runtime -- --apply --json --evidence-path /tmp/agent-browser-converge-local-runtime-turn33.json`
- `agent-browser install doctor --json`

Result:

- Format, focused Rust tests, and clippy passed.
- Unit coverage now proves unreachable stream metadata produces a stale runtime
  inventory row, install doctor emits
  `active_runtime_stale_stream_backend`, and remote-view doctor recommends the
  same session-scoped restart prerequisite.
- Convergence apply published the stream-backend build and ended with final
  install doctor ready, final remote-view ready, zero skipped remedies, and
  retained evidence at `/tmp/agent-browser-converge-local-runtime-turn33.json`.
- Direct installed `agent-browser install doctor --json` reported
  `success=true`, no issue codes, `runtimeConvergence.status=converged`,
  `staleRuntimeCount=0`, and `runtimeInventory.status=none`.

## Turn 34 | 2026-06-22

Scope: close P42 by auditing and validating Slice D idempotent remote-view
bootstrap.

Actions:

- Verified `pnpm ensure:rdp-guac-postgres -- --apply` exists and is invoked by
  local convergence.
- Verified route-pool setup, existing-user route sync, and legacy autologin
  setup call the shared Guacamole Postgres schema guard before writing records.
- Verified the schema guard refuses partial `guacamole_*` relation state,
  imports only absent schema state, waits for Postgres readiness, and
  checkpoints after ready/imported states.
- Verified the live Guacamole compose file keeps explicit Postgres durability
  settings for WSL hard-stop resilience.
- Marked P42 `State: CLOSED`.

Validation run:

- `bash scripts/ensure-rdp-guac-postgres.sh --dry-run`
- `pnpm --silent test:rdp-guac-route-pool-readiness -- --report-only`
- `agent-browser doctor remote-view --json`

Result:

- Schema guard dry-run reported `Guacamole Postgres schema is ready.`
- Route-pool readiness reported `success=true`; Postgres, schema, Guacamole
  web/login, guacd, RDP connections, connection permissions, distinct targets,
  and both RDP backend TCP checks were ready.
- Direct installed remote-view doctor reported `success=true`, `status=ready`,
  `remoteControl.ready=true`, `runtimeConvergence.status=converged`,
  `runtimeInventory.status=none`, and
  `nextAction=run_many_to_many_live_gate`.

## Turn 35 | 2026-06-22

Scope: investigate the `last30days` Facebook remote-view friction and open the
next route-handoff audit lane.

Actions:

- Read the incident note at
  `docs/dev/notes/2026-06-22-facebook-remote-view-open-friction.md`.
- Used Graphiti discovery for advisory prior context and CodeGraph for the
  route-binding and dashboard stream helper joins.
- Captured live readbacks from `agent-browser doctor remote-view --json`,
  `agent-browser service browsers --json`, and
  `agent-browser service tabs --json`.
- Added P43 in
  `docs/dev/plans/0043-2026-06-22-route-handoff-confusion-audit-plan.md`.
- Updated `ROADMAP.md` with the open P43 lane.

Findings:

- P42 binary/runtime convergence remains green. The failure sits above that
  layer.
- `session:default` owns the `last30days-facebook` browser on display `:11`
  with Facebook tabs and a generic Guacamole stream.
- `session:litscout-ai-smoke-clean` is a separate browser on display `:93`
  with several `127.0.0.1` tabs and its own generic Guacamole stream.
- The dashboard has stream metadata that can embed Guacamole, but it does not
  yet require row-bound proof that the stream is showing the intended browser
  instead of a terminal.

Validation run:

- `git diff --check`
- `pnpm validation:select -- --base HEAD`
- `python /home/ecochran76/workspace.local/agent-policies/repo-policy-selector/scripts/audit_planning_contract.py --repo-root /home/ecochran76/workspace.local/agent-browser --json`

Result:

- `git diff --check` passed.
- `pnpm validation:select -- --base HEAD` selected only `git diff --check` for
  the docs-only change set.
- The planning-contract audit still fails on pre-existing older plan wiring and
  deterministic-state debt. The new P43 plan itself is reported with
  `filename_ok=true`, `lane_ok=true`, `state_ok=true`,
  `wired_in_roadmap=true`, and `wired_in_runbook=true`.

## Turn 36 | 2026-06-22

Scope: execute P43 Slice A with a read-only route-handoff audit surface.

Actions:

- Added `scripts/audit-route-handoff.js`.
- Added package command `pnpm audit:route-handoff`.
- Added no-launch fixture coverage in `scripts/test-route-handoff-audit.js`.
- Added package command `pnpm test:route-handoff-audit`.
- Documented the audit command in `README.md`.
- Marked P43 Slice A done and updated `ROADMAP.md` next recommendation.

Validation run:

- `node --check scripts/audit-route-handoff.js`
- `node --check scripts/test-route-handoff-audit.js`
- `pnpm test:route-handoff-audit`
- `pnpm --silent audit:route-handoff -- --json --skip-doctor`
- `pnpm --silent audit:route-handoff -- --json`
- `git diff --check`
- `pnpm validation:select -- --base HEAD`
- `node scripts/dev/select-validation.js --base HEAD --json`
- `pnpm test:route-handoff-audit && pnpm --silent audit:route-handoff -- --json --skip-doctor | jq -e '.success == true and .data.summary.route_bound_ready == 2 and .data.summary.direct_remote_headed == 11'`

Result:

- Syntax checks passed.
- The fixture test passed and covers `route_bound_ready`,
  `route_bound_proof_missing`, `route_bound_terminal_only`,
  `direct_remote_headed`, `foreign_cdp`, and `stale_or_retained`
  classifications.
- The live read-only audit with `--skip-doctor` returned `success=true`,
  `collections.browsers=2`, `collections.tabs=13`, and summary
  `route_bound_ready=2`, `direct_remote_headed=11`.
- The full live audit also returned `success=true`, no collection errors,
  `runtime.convergenceStatus=converged`,
  `runtime.inventoryStatus=converged`, `runtime.runtimeCount=1`, and
  `runtime.remoteControlStatus=ready`.
- `git diff --check` passed.
- `pnpm validation:select -- --base HEAD` recommended `git diff --check` and
  `node scripts/dev/select-validation.js --base HEAD --json`; both passed.
- The combined fixture plus live summary assertion passed.

## Turn 37 | 2026-06-22

Scope: execute P43 Slice B one-line CLI contract and help.

Actions:

- Added command-specific `remote-view` help covering
  `agent-browser remote-view open`.
- Added the Facebook-style one-liner and flag placement guidance to CLI help.
- Changed `parse_remote_view_open` to copy global `--session-name` into the
  `remote_view_open` request.
- Added parser tests for post-subcommand `--runtime-profile`, `--session`,
  `--session-name`, `--browser-build`, and `--provider` placement.
- Updated `README.md`, `docs/src/app/commands/page.mdx`, and
  `skills/agent-browser/SKILL.md`.
- Marked P43 Slice B done and updated `ROADMAP.md` next recommendation.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open -- --nocapture`
- `cargo run --quiet --manifest-path cli/Cargo.toml -- remote-view open --help | rg -n "Facebook|Global placement|--session selects|last30days-facebook"`
- `pnpm --dir docs build`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm validation:select -- --base HEAD`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`

Result:

- Rust format passed after applying `cargo fmt`.
- Focused Rust tests passed: 10 passed, 0 failed.
- Help output includes the global placement section, Facebook examples, and
  the `--session` versus `--session-name` distinction.
- Docs build passed.
- Clippy passed with `-D warnings`.
- Validation selector required the Rust format, focused Rust test, clippy,
  docs build, diff hygiene, and repo-installed skill sync checks.
- The repo and installed `agent-browser` skill copies now match.

## Turn 38 | 2026-06-22

Scope: execute P43 Slice C route allocation diagnostics.

Actions:

- Added compact route-pool diagnostic JSON for `route_pool_unavailable`.
- Added the same diagnostic context to stale explicit pool-entry failures:
  `route_pool_entry_missing` and `route_pool_entry_unavailable`.
- Included requested route, route-pool entry, display allocation, display name,
  display isolation, owner browser, owner session, profile, provider, matching
  pool entries, available pool entries, ready display allocation IDs, existing
  remote-view routes, and recommended commands.
- Kept the existing string error-code contract intact so callers that check
  `route_pool_unavailable` continue to work.
- Tightened route-pool tests to parse the diagnostic suffix and assert the
  actionable identity fields.
- Marked P43 Slice C done and updated `ROADMAP.md` next recommendation.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml`
- `cargo test --manifest-path cli/Cargo.toml route_pool -- --test-threads=1`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `git diff --check`
- `pnpm validation:select -- --base HEAD`
- `pnpm test:service-cdp-tab-streaming-live`
- Direct temp-session probe:
  `HOME=<temp> AGENT_BROWSER_HOME=<temp>/.agent-browser AGENT_BROWSER_SOCKET_DIR=<temp>/s cargo run --quiet --manifest-path cli/Cargo.toml -- --json --session daemon-probe stream status`

Result:

- Focused route-pool Rust tests passed: 12 passed, 0 failed.
- Focused CDP stream Rust tests passed: 3 passed, 0 failed.
- Rust format, clippy, and diff hygiene passed.
- The new route-pool unavailable test verifies stable error code retention and
  machine-readable diagnostic fields for requested display identity, checked
  out matching pool entry, ready display allocations, and recommended repair
  command.
- The selector-recommended live CDP smoke did not reach the CDP path. It failed
  while starting a temporary daemon with
  `Daemon failed to start (socket: <temp>/s/<session>.sock)`. A direct
  temp-session `stream status` probe reproduced the same daemon-start failure
  and left only pid/token/version files. This appears independent of the
  route-pool diagnostic change and should be handled as a separate daemon
  startup validation issue.

## Turn 39 | 2026-06-22

Scope: execute P43 Slice D profile-lock ownership diagnostics.

Actions:

- Added profile-lock diagnostic JSON to Chrome profile lock failures while
  preserving the existing hard stop against launching a second Chrome process
  on the same user-data-dir.
- The diagnostic includes lock PID, user-data-dir, matching runtime profile
  state, matching service browser rows, primary owner, and safe remedies.
- Known service-owned locks now identify browser ID, active session, profile,
  host, health, PID, CDP endpoint, display, display allocation, and view stream
  IDs when persisted service state has them.
- Remedies include exact session-scoped service-status reuse and close commands
  for known owners, runtime-profile inspection and attach commands for matching
  runtime state, service-status inspection for unknown owners, and explicit
  separate-profile guidance for intentionally separate identities.
- Updated README, CLI runtime help, docs command page, and the
  `agent-browser` skill.
- Marked P43 Slice D done and updated `ROADMAP.md` next recommendation.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml`
- `cargo test --manifest-path cli/Cargo.toml locked_profile -- --test-threads=1 --nocapture`
- `pnpm --dir docs build`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `git diff --check`
- `pnpm validation:select -- --base HEAD`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`

Result:

- Focused profile-lock tests passed for known service/runtime owner diagnostics
  and unknown-owner diagnostics.
- Docs build passed with the known Next.js multiple-lockfile root warning.
- Clippy, diff hygiene, validation selector, and installed-skill sync passed.

## Turn 40 | 2026-06-22

Scope: execute P43 Slice E operator-visible success contract.

Actions:

- Added top-level `operatorVisible` to `remote-view open` dry-run and opened
  responses.
- Dry-runs report `operatorVisible.state=not_checked` with route, browser,
  session, display, provider, and display allocation identity.
- Successful opened responses report `operatorVisible.state=ready` and include
  the visible-window proof that already gates success.
- Added dry-run assertions and a pure ready-proof unit test for the
  `operatorVisible` contract.
- Updated README, CLI remote-view help, docs command page, and the
  `agent-browser` skill to tell clients to require
  `operatorVisible.state=ready`.
- Marked P43 Slice E done and updated `ROADMAP.md` next recommendation.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open -- --test-threads=1`
- `pnpm --dir docs build`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `git diff --check`
- `pnpm validation:select -- --base HEAD`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`

Result:

- Focused remote-view-open tests passed, including the new
  `operatorVisible` dry-run and ready-proof coverage.
- Docs build passed with the known Next.js multiple-lockfile root warning.
- Clippy, format check, diff hygiene, validation selector, installed-skill
  sync, and the no-launch CDP stream test passed.
- The selector still recommends `pnpm test:service-cdp-tab-streaming-live`
  because `actions.rs` changed; the same live smoke was already attempted in
  Turn 38 and failed before CDP validation while starting a temporary daemon.

## Turn 41 | 2026-06-22

Scope: execute P43 Slice F dashboard row binding and route-proof UX.

Actions:

- Added `operatorVisibleState` and `operatorVisibleReason` to dashboard
  workspace view-stream rows.
- Required current browser-window proof before RDP gateway View, Control, or
  external open actions are enabled.
- Kept terminal-only, idle-display, and missing-proof route rows in the live
  owned group as disabled diagnostics rather than moving them into a no-action
  attention category.
- Preserved detected non-owned browser grouping and retained-record filtering in
  the live workspace navigator.
- Updated README, docs dashboard/service/commands pages, the `agent-browser`
  skill, P43, and `ROADMAP.md`.

Validation run:

- `pnpm test:dashboard-workspace-nodes`
- `pnpm test:dashboard-workspace-navigator`
- `pnpm test:dashboard-selected-workspace-context`
- `pnpm test:dashboard-selected-workspace-chat-packet`
- `pnpm test:dashboard-selected-workspace-console`
- `pnpm --dir docs build`
- `pnpm build:dashboard`
- `pnpm validation:select -- --base HEAD`
- `node scripts/dev/select-validation.js --base HEAD --json`
- `git diff --check`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `pnpm publish:local-dashboard -- --expect-marker "operator-visible proof missing" --skip-browser --json`

Result:

- Dashboard workspace node, navigator, selected-context, chat-packet, and
  console smokes passed.
- Docs and dashboard builds passed with the known Next.js multiple-lockfile and
  static-export rewrite warnings.
- Diff hygiene, validation selector, selector JSON, and installed-skill sync
  passed.
- The local dashboard runtime was rebuilt into
  `/home/ecochran76/.local/bin/agent-browser`, `agent-browser-dashboard.service`
  restarted, `/api/runtime/manifest` matched the installed executable SHA
  `f626320b5d084f824917560bdad60c8111678896cf81299a602c2d3a35c9d0a6`, and the
  served chunk contained `operator-visible proof missing`.
- The full publish browser smoke was attempted first and failed at the known
  temp-daemon startup boundary:
  `Daemon failed to start (socket: /run/user/1000/agent-browser/local-dashboard-runtime-smoke-2846318.sock)`.
  The final publish used `--skip-browser`, so live browser launch remains
  covered by the separate temp-daemon startup blocker rather than this Slice F
  dashboard contract.

## Turn 42 | 2026-06-22

Scope: execute P43 Slice G downstream client contract and last30days handoff
guidance.

Actions:

- Made `requestServiceRemoteViewOpen` require `operatorVisible.state=ready`
  before returning non-dry-run handoff success.
- Added service-client helpers for reading operator-visible state, checking
  readiness, throwing on invalid handoff proof, and logging one compact route,
  tab, profile, and visual-proof summary line.
- Kept dry-run remote-view open responses allowed as `not_checked` and made
  infrastructure-only readiness an explicit client opt-in that is not posted to
  the service API.
- Updated README, docs commands page, service-client examples, generated client
  types, and the installed `agent-browser` skill.
- Updated `last30days` so Facebook uses the route-bound
  `agent-browser remote-view open` one-liner with the `last30days-facebook`
  runtime profile and rejects missing-proof, CDP-only, or terminal-only
  Guacamole/RDP handoff success.
- Marked P43 Slice G done and moved `ROADMAP.md` to Slice H live gates.

Validation run:

- `git diff --check`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:service-client-contract`
- `pnpm test:service-client-types`
- `pnpm test:service-client`
- `pnpm --dir docs build`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `uv run pytest tests/test_facebook.py`
- `python3 -m py_compile skills/last30days/scripts/lib/facebook.py skills/last30days/scripts/lib/env.py`

Result:

- Service API/MCP parity, service-client contract/type/export/request/helper
  smokes, docs build, diff hygiene, and installed-skill sync passed.
- Focused last30days Facebook tests passed with 9 tests, including the
  terminal-only rejection case.
- P43 remains open for Slice H. The next gate needs no-launch route-confusion
  fixtures and an OCR-backed live route proof that fails on terminal-only
  route displays.

## Turn 43 | 2026-06-22

Scope: execute P43 Slice H live gates and close the route-handoff confusion
audit lane.

Actions:

- Added `pnpm test:route-confusion-gates` as the focused no-launch gate for
  route-handoff confusion regressions.
- Covered wrong flag placement, named-session route-pool mismatch,
  same-owner route-pool repeat checkout, known-owner profile-lock messaging,
  direct remote-headed audit classification, and dashboard missing-proof plus
  terminal-only row classification.
- Updated validation selection so route, dashboard stream, service-client, and
  remote-view command changes recommend the route-confusion gate.
- Strengthened the live `remote-view open` fixture smoke with isolated daemon
  session and runtime profile defaults, bounded daemon-start retry, available
  route-pool selection, repeat handoff through the first route/display
  identity, route-handoff audit assertion, and OCR of the route display.
- Fixed the route-pool checkout resolver so an already checked-out route is
  reusable only for the same ready route, browser, session, and display
  allocation. Other owners still receive `route_pool_unavailable`.
- Marked P43 complete in the plan and roadmap.

Validation run:

- `git diff --check`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `node scripts/dev/select-validation.js --base HEAD --json`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `pnpm test:route-confusion-gates`
- `AGENT_BROWSER_COMMAND=/home/ecochran76/workspace.local/agent-browser/cli/target/debug/agent-browser pnpm test:remote-view-open-fixture-live`
- `pnpm test:service-cdp-tab-streaming-live`

Result:

- Diff hygiene, Rust formatting, validation selector JSON, clippy, no-launch
  CDP stream regressions, and the route-confusion gate passed.
- The OCR-backed live route gate passed with artifact directory
  `/tmp/agent-browser-remote-view-open-live-2026-06-22T16-23-29-784Z`,
  route `guacamole:5`, display allocation `remote-view-display:12`,
  route-handoff classification `route_bound_ready`, visual state
  `browser_window_visible`, and fixture text
  `REMOTE VIEW OPEN FIXTURE 3815575`.
- `pnpm test:service-cdp-tab-streaming-live` was retried twice and failed
  before CDP validation at the known temporary-daemon startup boundary:
  `Daemon failed to start`.

## Turn 44 | 2026-06-22

Scope: diagnose and repair `pnpm test:service-cdp-tab-streaming-live`.

## Turn 45 | 2026-06-22

Scope: execute P44 Slice A intent normalization and remote-view provider
harmonization.

Actions:

- Added `RemoteViewOpenIntent` normalization for `remote_view_open` before
  route binding or launch.
- Made `viewStreamProvider` the canonical remote-view stream field and kept
  `provider=rdp_gateway` as a compatibility alias.
- Rejected provider/view-stream conflicts before acquisition.
- Updated CLI help, README, docs site, repo skill guidance, Plan 0044, and
  `ROADMAP.md`.
- Added focused CLI parser tests and remote-view intent normalization tests.

Validation run:

- `cargo test --manifest-path cli/Cargo.toml remote_view_open -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml normalize_remote_view_open_intent -- --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `git diff --check`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `pnpm test:route-confusion-gates`
- `pnpm --dir docs build`
- `pnpm validation:select -- --base HEAD`
- `pnpm test:browser-capability-registry-draft`
- `pnpm test:service-client`
- `pnpm test:service-api-mcp-parity`
- `node scripts/dev/select-validation.js --base HEAD --json`

Result:

- All listed no-launch checks passed.
- `pnpm test:service-cdp-tab-streaming-live` was selected by validation but
  not rerun in this slice because the prior P43 closeout recorded the existing
  temporary-daemon startup boundary before CDP validation.

Actions:

- Reproduced the original failure with an isolated temp home and debug daemon
  logs. The client timed out before the daemon bound its socket, but the daemon
  stayed alive and became usable seconds later.
- Added daemon startup milestones under `--debug`.
- Moved Unix control-socket bind ahead of stream-server startup.
- Moved executable SHA calculation out of the daemon startup critical path by
  writing a short-lived `pending` marker and filling the real SHA in a
  background task. The client tolerates `pending` only during startup grace.
- Avoided hashing the current executable on fresh daemon startup unless an
  already-running daemon must be compared.
- Added a bounded smoke retry around first `stream status` daemon startup.
- Fixed service-owned `navigate` so it persists the active tab record and
  service tab handle, matching the existing `tab_new` retained-tab contract.
- Hardened the CDP tab streaming smoke diagnostics and allowed data-URL marker
  matching when Chrome has not populated a tab title yet.

Validation run:

- `git diff --check`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml test_daemon_executable_sha_pending_is_startup_grace_only -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `pnpm test:route-confusion-gates`
- `pnpm test:service-cdp-tab-streaming-live`
- `pnpm validation:select -- --base HEAD --json`

Result:

- Isolated first-command startup dropped from roughly 10 to 13 seconds to
  about 145 ms on the debug binary.
- The original live smoke passed end to end:
  `Service CDP tab streaming live smoke passed`.

## Turn 46 | 2026-06-22

Scope: execute P44 Slice B no-mutation acquisition planner.

Actions:

- Added `RemoteViewAcquisitionPlan` for route-bound remote-view acquisition.
- Routed `remote_view_open`, route preflight, and route checkout through the
  planner before state mutation.
- Moved route/display fallback selection into named planner decisions and
  surfaced `acquisitionPlan` in dry-run, opened, preflight, and checkout
  responses.
- Added blockers and diagnostics for unavailable route-pool entries and
  named-session display-allocation mismatches.
- Added planner fixtures for stale browser fallback ordering, checked-out
  same-owner reuse, checked-out other-owner rejection, named-session mismatch
  diagnostics, and dry-run acquisition-plan output.
- Updated Plan 0044 and `ROADMAP.md` so Slice C is the next P44 boundary.

Validation run:

- `cargo test --manifest-path cli/Cargo.toml remote_view -- --nocapture`
- `pnpm test:route-handoff-audit`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `git diff --check`
- `pnpm validation:select -- --base HEAD`
- `pnpm test:route-confusion-gates`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:service-client`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `pnpm --dir docs build`
- `node scripts/dev/select-validation.js --base HEAD --json`
- `pnpm test:service-cdp-tab-streaming-live`

Result:

- All listed no-launch checks passed.
- The live CDP tab streaming smoke passed with
  `session:cdp-tab-stream-2019837` and stream `38157`.

## Turn 47 | 2026-06-22

Scope: execute the P44 Slice C no-launch acquisition lease and rollback
foundation.

Actions:

- Added persisted `RemoteViewAcquisitionLease` state to `ServiceState`.
- Wrapped `remote_view_open` in an acquisition lease that marks selected
  route-pool entry, display allocation, and remote-view route records as
  pending before display access, browser launch, tab acquisition, proof, and
  checkout complete.
- Added rollback for display-access, launch, tab-open, focus, proof, and
  checkout failures.
- Changed failure cleanup summaries to typed JSON with cleanup and lease
  rollback evidence.
- Added a no-launch fixture proving a failed pending acquisition restores the
  available route-pool entry and removes pending display/route rows.
- Updated Plan 0044 and `ROADMAP.md` to record Slice C progress and keep the
  forced-proof live smoke as remaining Slice C work.

Validation run:

- `cargo test --manifest-path cli/Cargo.toml remote_view -- --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `git diff --check`
- `pnpm validation:select -- --base HEAD`
- `pnpm test:route-handoff-audit`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:route-confusion-gates`
- `cargo test --manifest-path cli/Cargo.toml service_model -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `pnpm test:service-client`
- `pnpm --dir docs build`
- `node scripts/dev/select-validation.js --base HEAD --json`
- `pnpm test:service-cdp-tab-streaming-live`

Result:

- All listed checks passed.
- The live CDP tab streaming smoke passed with
  `session:cdp-tab-stream-2342690` and stream `38445`.
- Slice C is not complete yet. The focused forced-proof failing live smoke and
  remaining service-contract metadata coverage still need to run before moving
  to Slice D.

## Turn 48 | 2026-06-22

Scope: close the P44 Slice C no-launch service-contract metadata gap.

Actions:

- Added a service-state wire-contract assertion for
  `remoteViewAcquisitionLeases`.
- Extended the nested service-state round-trip fixture with an acquisition
  lease and previous route-pool, display-allocation, and remote-view-route
  snapshots.
- Strengthened route checkout assertions for acquisition-plan metadata,
  checked-out route-pool entry state, and route provider event metadata.
- Strengthened route release assertions for release status, released
  viewer-lease metadata shape, and route release provider event metadata.
- Updated Plan 0044 and `ROADMAP.md` so the remaining Slice C gap is the
  focused forced-proof failing live smoke.

Validation run:

- `cargo test --manifest-path cli/Cargo.toml service_state_round_trips_nested_entities -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_route_and_lease_actions_mutate_service_state -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml service_model -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml remote_view -- --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `git diff --check`
- `pnpm validation:select -- --base HEAD`
- `pnpm test:route-confusion-gates`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:service-client`
- `pnpm --dir docs build`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `node scripts/dev/select-validation.js --base HEAD --json`
- `pnpm test:service-cdp-tab-streaming-live`

Result:

- All listed checks passed.
- The live CDP tab streaming smoke passed with
  `session:cdp-tab-stream-2420462` and stream `37215`.
- Slice C still needs the focused forced-proof failing live smoke before it can
  be marked complete.

## Turn 49 | 2026-06-23

Scope: close the P44 Slice C forced-proof live-smoke gap.

Actions:

- Added a forced visible-window-proof failure hook behind
  `AGENT_BROWSER_REMOTE_VIEW_FORCE_PROOF_FAILURE`.
- Added `--force-proof-failure` support to
  `scripts/smoke-remote-view-open-live.js`, including assertions for typed
  cleanup JSON, route-pool rollback, display/route rollback, and failed
  acquisition-lease state.
- Fixed post-launch failure ordering so rollback happens before browser/tab
  cleanup can prune pending lease state, then records actual cleanup back onto
  the failed lease when the lease is still retained.
- Restored missing acquisition-lease snapshots before rollback and completion
  when service mutations overwrite pending lease state.
- Allowed released or orphaned display allocations from previous sessions to be
  reclaimed by a new acquisition.
- Allowed same-owner pending route reservations to be reused during checkout and
  repeat open.
- Updated Plan 0044 and `ROADMAP.md` to mark Slice C complete and point the next
  P44 slice at browser-only route desktop work.

Validation run:

- `node --check scripts/smoke-remote-view-open-live.js`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_acquisition_lease_rollback -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml acquisition_plan_reclaims_released_display_allocation_from_previous_session -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml acquisition_plan_reuses_same_owner_pending_route_reservation -- --nocapture`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `AGENT_BROWSER_COMMAND=/home/ecochran76/workspace.local/agent-browser/cli/target/debug/agent-browser pnpm test:remote-view-open-fixture-live -- --force-proof-failure`
- `AGENT_BROWSER_COMMAND=/home/ecochran76/workspace.local/agent-browser/cli/target/debug/agent-browser pnpm test:remote-view-open-fixture-live`

Result:

- All listed checks passed.
- The forced-proof live smoke passed with route `guacamole:4`, display
  allocation `remote-view-display:16`, cleanup state `closed_new_browser`,
  rollback state `rolled_back`, and artifact directory
  `/tmp/agent-browser-remote-view-open-live-2026-06-23T03-16-04-025Z`.
- The normal fixture smoke passed afterward with repeat open, HTTP helper, CDP
  readback, X11 PID proof, route-handoff classification `route_bound_ready`,
  visual state `browser_window_visible`, OCR proof, and artifact directory
  `/tmp/agent-browser-remote-view-open-live-2026-06-23T03-17-24-564Z`.
- P44 Slice C is complete. P44 remains open for Slice D and later closeout
  criteria.

## Turn 50 | 2026-06-23

Scope: start P44 Slice D browser-only route desktop work.

Actions:

- Removed foreground terminal startup from new route-pool XRDP user sessions.
- Updated the installed-helper source and the route-pool setup fallback so
  generated `.xsession` files start Openbox when available, keep XRDP alive with
  an idle sleep loop, and do not launch terminal UI.
- Added `scripts/test-rdp-route-xsession.js` to guard maintained route
  `.xsession` writers against terminal startup.
- Wired the xsession guard into `pnpm test:route-confusion-gates`.
- Added `terminal_topmost` route display classification and
  `terminal_topmost_route` proof failure coverage so visible browser proof
  cannot pass when a terminal is the top application window over the browser.
- Updated README, docs install page, Plan 0044, and `ROADMAP.md`.

Validation run:

- `bash -n scripts/libexec/agent-browser-privileged-helper scripts/setup-rdp-guac-route-pool.sh`
- `pnpm test:rdp-route-xsession`
- `cargo test --manifest-path cli/Cargo.toml display_content_rejects_terminal_topmost_over_browser -- --nocapture`
- `pnpm test:route-confusion-gates`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm --dir docs build`
- `git diff --check`
- `pnpm validation:select -- --base HEAD`
- `cargo test --manifest-path cli/Cargo.toml service_model -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:service-client`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `node scripts/dev/select-validation.js --base HEAD --json`
- `pnpm test:service-cdp-tab-streaming-live`

Result:

- All listed checks passed.
- The live CDP tab streaming smoke passed with
  `session:cdp-tab-stream-3021946` and stream `37741`.
- Slice D is not complete yet. The installed privileged helper still needs to be
  refreshed on the host, then a cold route session and route display inspection
  should prove the desktop is browser-control-ready instead of terminal-first.
- Installed helper readback showed
  `/usr/local/libexec/agent-browser/agent-browser-privileged-helper` still
  writes the old `xterm` `.xsession`; `sudo -n true` failed with password
  required, so this session could not refresh the helper or run the cold-route
  proof.

## Turn 58 | 2026-06-27

Scope: close P53 and unlock P46 S4.

Actions:

- Added `agent-browser window new [url] --same-profile` and wired S4 to create
  the second top-level window inside the same retained browser process instead
  of launching a second same-profile Chrome process.
- Rebuilt and converged the local runtime with
  `pnpm converge:local-runtime -- --apply --json`.
- Updated the S4 harness to accept explicit-command runs with no pre-existing
  daemon listener, while still rejecting duplicated or mismatched daemon
  authority.
- Updated S4 evaluation to require one retained same-profile browser row for
  the P53 topology.
- Marked P53 complete and moved P46 to S5.

Validation run:

- `node scripts/test-p47-scenario-harness.js`
- `node --check scripts/run-p46-stress-scenario.js`
- `cargo test --manifest-path cli/Cargo.toml test_window_new_same_profile_with_url`
- `node scripts/run-p46-stress-scenario.js --scenario s4 --reset-before --reset-after --agent-browser-command ./cli/target/debug/agent-browser --require-explicit-agent-browser-command --require-agent-browser-daemon-command-match`

Result:

- S4 passed with artifact
  `/tmp/agent-browser-p46-s4-2026-06-27T19-12-55-449Z`.
- The pass proved one retained browser process
  `session:p46-s4-window-2026-06-27T19-12-53-709Z`, one runtime profile
  `p46-s4-profile`, one route `guacamole:3`, one display `:13`, two
  same-profile top-level windows, working refresh controls for both dashboard
  operators, and window B staying ready after closing window A.
- Reset-before and reset-after both ended with zero active incidents.

## Turn 66 | 2026-06-27

Scope: close P63 and unlock P46 S11 after the S10 foreign CDP inventory lock.

Actions:

- Added dashboard `/api/session-tabs?port=<foreign-cdp-port>` fallback from
  agent-browser `/api/tabs` to raw Chrome CDP `/json/list`.
- Changed the local dashboard proxy to stop reading once declared
  `Content-Length` is satisfied, with bounded per-read timeout and response
  size cap.
- Updated S10 to read dashboard inventory through the authenticated
  viewer-client session.
- Made foreign CDP browser cleanup best-effort so profile removal cannot mask
  the scenario failure.
- Updated selected workspace probing to accept viewport-route context when the
  optional detail panel is not mounted.
- Scoped S10 foreign route-borrow detection to selected-workspace evidence
  instead of global workspace-list text.
- Marked P63 complete and advanced P46 to S11.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml dashboard -- --nocapture`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `node --check scripts/run-p46-stress-scenario.js`
- `node --check scripts/lib/p46-scenario-harness.js`
- `node scripts/test-p47-scenario-harness.js`
- `node scripts/test-dashboard-workspace-nodes.js`
- `git diff --check -- cli/src/native/stream/dashboard.rs scripts/run-p46-stress-scenario.js scripts/test-p47-scenario-harness.js docs/dev/plans/0063-2026-06-27-s10-foreign-cdp-inventory-plan.md`
- `pnpm publish:local-dashboard -- --skip-smoke --json`
- `/home/ecochran76/.local/bin/agent-browser --json install doctor`
- `node scripts/smoke-local-dashboard-runtime.js --dashboard-url http://127.0.0.1:4848/ --agent-browser-bin /home/ecochran76/.local/bin/agent-browser --skip-browser --json`
- `/home/ecochran76/.local/bin/agent-browser --json service incidents --summary`
- `node scripts/run-p46-stress-scenario.js --scenario s10 --reset-before --reset-after --agent-browser-command /home/ecochran76/.local/bin/agent-browser --require-explicit-agent-browser-command --require-agent-browser-daemon-command-match`

Result:

- S10 passed with artifact
  `/tmp/agent-browser-p46-s10-2026-06-27T22-52-43-936Z`.
- The pass proved authenticated foreign CDP inventory, normalized foreign tab
  inventory, no service route/display borrowing, stable foreign and
  service-owned selected workspace context, service-owned control readiness,
  and complete route-bound finalization.
- Reset-before and reset-after ended with zero active incidents.
- Installed executable SHA:
  `502f05830dfb756cda44eae7d6bb8c71999dd4ce39ee109eb51ff36136de155a`.

## Turn 67 | 2026-06-27

Scope: implement P46 S11, clear P64, and advance P46 to S12.

Actions:

- Added S11 scenario metadata for one route-bound service-owned browser and one
  zero-lease dashboard viewer-client.
- Added S11 capture for dashboard reload, stale workspace URL navigation,
  viewer-client reconnect, viewport refresh, direct Guacamole frame URL
  readback, route display inspection, service status, incidents, and
  route-bound finalization evidence.
- Added S11 evaluator checks for stale target recovery, reconnect proof,
  refresh control function, direct Guacamole reachability, route display state,
  stream binding, finalization, and incident cleanliness.
- Ran two live S11 attempts from the installed binary authority. Both reset
  cleanly with zero active incidents but failed in the harness before S11
  evaluation.
- Added P64 to repair the stale URL live-target recovery acceptance boundary.
- Added `allowRecoveredLiveTab` for S11 so immediate rewrite from a stale target
  to a current live target can satisfy the stale URL recovery criterion without
  weakening default tab matching.
- Ran the P64-authorized S11 retry and marked P64 complete.
- Advanced P46 to S12.

Validation run:

- `node --check scripts/run-p46-stress-scenario.js`
- `node --check scripts/lib/p46-scenario-harness.js`
- `node --check scripts/lib/p47-viewer-client.js`
- `node scripts/test-p47-scenario-harness.js`
- `node scripts/test-p47-viewer-client-separation.js`
- `git diff --check -- scripts/lib/p47-viewer-client.js scripts/lib/p46-scenario-harness.js scripts/run-p46-stress-scenario.js scripts/test-p47-scenario-harness.js scripts/test-p47-viewer-client-separation.js docs/dev/plans/0046-2026-06-24-remote-view-stress-hardening-plan.md docs/dev/plans/0064-2026-06-27-s11-stale-url-live-target-recovery-plan.md docs/dev/notes/2026-06-24-p46-stress-hardening-execution.md RUNBOOK.md`
- `/home/ecochran76/.local/bin/agent-browser --json service incidents --summary`
- `node scripts/run-p46-stress-scenario.js --scenario s11 --reset-before --reset-after --agent-browser-command /home/ecochran76/.local/bin/agent-browser --require-explicit-agent-browser-command-match --require-agent-browser-daemon-command-match`

Result:

- First failed S11 artifact:
  `/tmp/agent-browser-p46-s11-2026-06-27T23-02-14-303Z`.
- Second failed S11 artifact:
  `/tmp/agent-browser-p46-s11-2026-06-27T23-05-10-207Z`.
- Both failures proved the dashboard rejected the stale target and recovered to
  a live target, but the harness expected either exact stale URL persistence or
  explicit stale-recovery notice text.
- S11 passed with artifact
  `/tmp/agent-browser-p46-s11-2026-06-27T23-09-57-372Z`.
- The pass proved dashboard reload restoration, stale URL recovery to a live
  target, viewer-client reconnect, viewport refresh, direct Guacamole HTTP 200,
  route display `browser_window_visible`, route-bound finalization, and zero
  active incidents after reset-after.
- Command metadata caveat: the pass used an explicit installed binary command
  and daemon realpath matching passed, but the explicit-command guard flag was
  misspelled, so the artifact reports `requireExplicit: false` while also
  reporting `explicit: true`.
- P46 is now in progress at S12.

## Turn 68 | 2026-06-27

Scope: implement P46 S12 soak harness and stop at the S12 lock.

Actions:

- Added S12 scenario metadata for repeated normal-use drift and reset soak.
- Added S12 runner support for ten cycles of route-bound open, dashboard
  reload, viewer-client reconnect, viewport refresh, navigate, tab creation,
  tab switch, direct Guacamole readback, route-bound finalization, close,
  reset, and cycle-boundary doctor and incident probes.
- Added active-pressure and route-pool-baseline evaluation for each cycle.
- Corrected the pressure classifier after the first S12 run showed completed
  acquisition-lease history was being counted as active pressure.
- Ran a second S12 attempt and stopped after it exposed real route-pool reset
  drift.
- Repaired the failed live state through authenticated `service_route_pool_repair`
  dry-run and apply, followed by service reconcile and incident resolution.

Validation run:

- `node --check scripts/run-p46-stress-scenario.js`
- `node --check scripts/lib/p46-scenario-harness.js`
- `node --check scripts/test-p47-scenario-harness.js`
- `node scripts/test-p47-scenario-harness.js`
- `node scripts/test-p47-viewer-client-separation.js`
- `/home/ecochran76/.local/bin/agent-browser --json service incidents --summary`

Result:

- First S12 artifact:
  `/tmp/agent-browser-p46-s12-2026-06-27T23-20-14-868Z`.
- The first run completed ten cycles with zero active incidents, zero retained
  sessions/browsers/tabs, route-pool baseline true, and direct Guacamole HTTP
  200 in every cycle, but failed due to the harness counting completed
  acquisition-lease history as active pressure.
- Second S12 artifact:
  `/tmp/agent-browser-p46-s12-2026-06-27T23-39-14-415Z`.
- The second run exposed real drift: cycle 3 left `guacamole:3` orphaned on
  `remote-view-display:13`, with `guacamole-rdp-a` still checked out; cycle 4
  then failed with `route_pool_entry_unavailable`.
- `service_route_pool_repair` dry-run found one stale checkout, one stale
  route, and one stale display allocation; apply repaired all three.
- Post-repair reconcile showed both route-pool entries available,
  `guacamole:3` released, and `remote-view-display:13` released.
- Final incident summary has no active incidents; the transient cycle-browser
  incident is recovered with an explicit resolution note.
- Historical state, superseded by the later repair and S12 pass: P46 was
  locked at S12 until a follow-up plan addressed orphaned route-bound display
  cleanup after normal close.

## Turn 69 | 2026-06-27

Scope: repair P46 S12 route cleanup and classify the remaining selector
failure.

Actions:

- Updated service-health reconciliation to preserve newer remote-view release
  mutations for display allocations, routes, and route-pool entries.
- Updated normal close cleanup to release session-owned display allocations and
  routes when process-exit reconcile removed the browser row before close
  persistence.
- Added regression coverage for the absent-browser close race.
- Rebuilt and converged the installed runtime after closing stale daemon
  listeners.
- Ran S12 against installed SHA
  `43d85bebf6c2e68fb7b86a5e9a1628f6e20698d7140533bb033bf932dd26c113`.
- Classified the remaining S12 failure as a harness selector defect and patched
  S12 to prefer the current cycle's `tab new` result by service tab ID or exact
  returned index and URL.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml close_releases_session_owned_route_after_process_exit_removed_browser -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml native::service_health::tests:: -- --nocapture`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo build --release --manifest-path cli/Cargo.toml`
- `pnpm converge:local-runtime -- --apply --json`
- `node --check scripts/run-p46-stress-scenario.js`
- `node --check scripts/test-p47-scenario-harness.js`
- `node scripts/test-p47-scenario-harness.js`

Result:

- Third S12 artifact:
  `/tmp/agent-browser-p46-s12-2026-06-28T00-43-57-985Z`.
- The run completed ten cycles with zero active incidents at all boundaries,
  route-pool baseline true after every reset, no post-reset pressure increase,
  and direct Guacamole HTTP 200 in every cycle.
- The only failures were switched-tab URL assertions caused by stale positional
  tab selection under repeated-cycle tab accumulation.
- S12 is unlocked for one selector-repaired retry.

## Turn 70 | 2026-06-27

Scope: clear P46 S12 after selector repair.

Actions:

- Reran S12 with the selector-repaired harness against
  `/home/ecochran76/.local/bin/agent-browser`.
- Captured final install doctor, remote-view doctor, and incident-summary
  evidence.
- Updated the P46 plan and execution note to mark S12 cleared.

Validation run:

- `node scripts/run-p46-stress-scenario.js --scenario s12 --reset-before --reset-after --agent-browser-command /home/ecochran76/.local/bin/agent-browser --require-explicit-agent-browser-command --require-agent-browser-daemon-command-match`
- `/home/ecochran76/.local/bin/agent-browser --json install doctor`
- `/home/ecochran76/.local/bin/agent-browser --json doctor remote-view`
- `/home/ecochran76/.local/bin/agent-browser --json service incidents --summary`

Result:

- S12 pass artifact:
  `/tmp/agent-browser-p46-s12-2026-06-28T01-05-24-861Z`.
- The pass reports `requireExplicit: true`, `explicit: true`, and daemon
  realpath matching passed.
- All ten cycles completed with route-pool baseline true after every reset.
- Active incidents stayed zero at every boundary and reset point.
- Post-reset pressure did not increase; checked-out route-pool, active
  remote-view routes, sessions, browsers, and tabs were zero after every reset.
- Direct Guacamole returned HTTP 200 in every cycle.
- Final install doctor succeeded with no issues and installed SHA
  `43d85bebf6c2e68fb7b86a5e9a1628f6e20698d7140533bb033bf932dd26c113`.
- Final remote-view doctor status was `ready`.
- Final incident summary count was 0.
- P46 S12 is cleared.

## Turn 71 | 2026-06-27

Scope: close P46 after auditing completion criteria.

Actions:

- Re-read the P46 plan closeout criteria and audited current evidence against
  each required proof point.
- Updated the P46 plan state to `COMPLETE`.
- Added the missing S9 through S12 entries to the current execution ledger.
- Added the campaign summary, residual risks, and next hardening target to the
  P46 plan and execution note.
- Captured fresh final service status after the S12 pass.

Validation run:

- `~/.local/bin/graphiti-runtime doctor`
- `~/.local/bin/graphiti-runtime discover --group-id agent_browser_main --max-facts 8 --max-nodes 5 --max-episodes 5 "agent-browser P46 plan 0046 S12 route cleanup selector repair final closeout residual risk next hardening target"`
- `/home/ecochran76/.local/bin/agent-browser --json service status`
- `/home/ecochran76/.local/bin/agent-browser --json install doctor`
- `/home/ecochran76/.local/bin/agent-browser --json doctor remote-view`
- `/home/ecochran76/.local/bin/agent-browser --json service incidents --summary`
- `node scripts/smoke-rdp-guac-route-pool-readiness.js --report-only`

Result:

- P46 is complete through S12.
- Final service status artifact:
  `/tmp/agent-browser-p46-final-service-status.json`.
- Final install doctor artifact:
  `/tmp/agent-browser-p46-final-install-doctor.json`.
- Final remote-view doctor artifact:
  `/tmp/agent-browser-p46-final-remote-view-doctor.json`.
- Final incident summary artifact:
  `/tmp/agent-browser-p46-final-incidents-summary.json`.
- Final route-pool readiness artifact:
  `/tmp/agent-browser-p46-final-route-pool-readiness.json`.
- Final service status reported zero service browsers, zero service sessions,
  zero tabs, and zero active incidents.
- Final install doctor succeeded with no issues and runtime status `converged`.
- Final remote-view doctor status was `ready`.
- Final incident summary count was 0.
- Final route-pool readiness succeeded.
- Route-pool entries `guacamole-rdp-a` and `guacamole-rdp-b` were available
  with no current route allocation.
- Residual risk: historical orphaned display-allocation records remain visible
  in service status, but they are not live control rows and do not hold
  route-pool capacity.
- Next hardening target: retained-state compaction and doctor-surface cleanup
  for historical orphaned display allocations and stale metadata visibility.

## Turn 72 | 2026-06-28

Scope: plan the retained display-state compaction follow-up after P46.

Actions:

- Created `docs/dev/plans/0065-2026-06-28-retained-display-state-compaction-plan.md`.
- Scoped P65 to classify, explain, and safely compact retained historical
  display-allocation metadata without weakening P46 route-pool or live-control
  guarantees.
- Reused prior retained-state cleanup conventions: dry-run before apply,
  service-owned actions only, no manual service-state edits, and doctor/readback
  proof after live cleanup.

Result:

- P65 is `PLANNED`.
- First implementation slice should add the retained display-state classifier
  and focused tests before adding apply behavior.

## Turn 73 | 2026-06-28

Scope: execute P65 retained display-state compaction.

Actions:

- Added retained display-allocation classification to service state model.
- Extended `service prune-retained` with `--display-allocations` dry-run/apply.
- Added `retainedDisplayAllocations` to service status JSON and text output.
- Updated service request/status contracts, generated client types, README,
  docs site pages, and `skills/agent-browser/SKILL.md`.
- Rebuilt and installed the local debug binary, ran
  `pnpm publish:local-dashboard -- --skip-browser --json`, and removed two
  stale deleted-executable default daemon listeners reported by install doctor.

Validation:

- `cargo test --manifest-path cli/Cargo.toml service_prune_retained -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml test_prune_retained_service_state_classifies_display_allocations -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml test_service_status_via_actions_does_not_launch_browser -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml test_format_service_status_text_includes_profile_and_session_summaries -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml service_status_and_collection_response_contracts_match_wire_shape -- --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm test:service-client-contract`
- `pnpm test:service-client-types`
- `pnpm --dir docs build`

Live proof:

- Artifact directory:
  `/tmp/agent-browser-p65-retained-display-20260628T174225Z`.
- `display-prune-dry-run.json` reported zero apply-safe display allocation
  candidates; apply was skipped.
- Final status retained 22 display allocations: 16 `diagnostic-retained`, 6
  `live`, 0 apply-safe.
- `final2-incidents-summary.json` reported incident count 0.
- `final2-install-doctor.json` succeeded with no issues.
- `final2-remote-view-doctor.json` succeeded with status `ready`.
- `final2-route-pool-readiness.json` succeeded with status `ready`.

Result:

- P65 is complete.
- No retained display allocation compaction is needed until a future dry-run
  reports apply-safe candidates.

## Turn 84 | 2026-07-06

Scope: continue P69 Slice C by sharing one acquisition-result builder.

Actions:

- Added `remote_view_handoff::shared_profile_acquisition_result` as the common
  JSON constructor for shared-profile acquisition evidence.
- Rewired route-bound `remote_view_open` shared acquisition records and
  `tab_new_shared_acquisition_evidence` in `cli/src/native/actions.rs` through
  that one builder.
- Extended the existing tab evidence tests to assert common acquisition-result
  fields such as `duplicateProcessPolicy`, `plannedProfile`, and
  `routeHintFields`.
- Updated P69 to narrow the remaining shared acquisition-result gap to plain
  remote-headed `open`.

Validation:

- `node --check packages/client/src/service-request.js`
- `node --check scripts/test-service-request-client.js`
- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml tab_new_shared_acquisition -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `pnpm test:service-client`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- HTTP/MCP-routed `tab_new` responses and route-bound `remote_view_open`
  responses now share the same acquisition-result constructor. P69 remains open
  for plain remote-headed `open` convergence and Slice F live proof.

## Turn 83 | 2026-07-06

Scope: continue P69 Slice C shared acquisition-result convergence.

Actions:

- Added route-bound `sharedAcquisition` records to `remote_view_open` planned
  and opened responses through
  `remote_view_handoff::route_bound_handoff_shared_acquisition`.
- Kept `routeBoundHandoff` as the detailed route/display/operator-visible proof
  surface while exposing the same top-level acquisition-result name already
  used by access-plan and service-request tab responses.
- Extended `summarizeServiceSharedProfileAcquisition()` in
  `packages/client/src/service-request.js` to summarize route-bound
  `remote_view_open` responses from `data.intent`, `data.sharedAcquisition`,
  nested `data.tab`, and nested `serviceTabHandle`.
- Added focused Rust and service-client coverage for route-bound shared
  acquisition summaries.

Validation:

- `node --check packages/client/src/service-request.js`
- `node --check scripts/test-service-request-client.js`
- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `pnpm test:service-client`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `git diff --check -- cli/src/native/remote_view_handoff.rs packages/client/src/service-request.js scripts/test-service-request-client.js docs/dev/plans/0069-2026-07-06-shared-profile-routing-and-handoff-deepening-plan.md RUNBOOK.md`

Result:

- P69 Slice C now has a common named acquisition-result record on
  `remote_view_open` responses and on the existing access-plan/tab response
  path. Remaining Slice C work is to route plain remote-headed `open`, HTTP
  `service_request`, and MCP `service_request` through that same acquisition
  result as a real shared planning artifact, not just a response field.
- P69 Slice F live proof remains open.

## Turn 82 | 2026-07-06

Scope: continue P69 Slice C retained-browser failure cleanup deepening.

Actions:

- Added handoff-owned cleanup decision helpers in
  `cli/src/native/remote_view_handoff.rs` for route-bound failure recovery:
  close only the opened tab for a reused retained browser, close a newly
  launched browser, or skip cleanup when no opened-tab index is available.
- Rewired `remote_view_open_cleanup_after_failure` in
  `cli/src/native/actions.rs` so the dispatcher executes the selected async
  browser command while the handoff module owns cleanup decision and result
  vocabulary.
- Updated P69 with the current Slice C progress and remaining work.

Validation:

- `cargo fmt --manifest-path cli/Cargo.toml`
- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml test_remote_view_open_cleanup_reports_new_browser_close_on_failure -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- P69 Slice C is deeper but still partially implemented. The handoff module now
  owns failure-cleanup recovery decisions for retained versus newly launched
  browsers, but the full shared acquisition-result routing across
  `remote-view open`, plain remote-headed `open`, HTTP `service_request`, and
  MCP `service_request` remains open.
- P69 Slice F live proof remains open.

## Turn 81 | 2026-07-06

Scope: continue P69 Slice C begin-acquisition lease reservation deepening.

Actions:

- Moved route-bound begin-acquisition lease reservation into
  `cli/src/native/remote_view_handoff.rs` behind
  `begin_route_bound_handoff_acquisition`.
- Rewired the `remote_view_open` begin-acquisition adapter in
  `cli/src/native/actions.rs` to supply the observation timestamp and
  provider-derived default control-input adapter, while the handoff module now
  owns pending route-pool, display-allocation, route, and lease repository
  mutations.
- Updated P69 with the current Slice C progress and remaining work.

Validation:

- `cargo fmt --manifest-path cli/Cargo.toml`
- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml test_remote_view_open_acquisition_lease_rollback_restores_route_state -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml test_remote_view_open_dry_run_plans_route_bound_launch_without_existing_display -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- P69 Slice C is deeper but still partially implemented. The handoff module now
  owns begin-acquisition reservation, planned/opened response assembly,
  cleanup/rollback summary reporting, acquisition completion, lease
  restoration, rollback mutation, and cleanup update mutation. Retained-browser
  recovery sequencing and shared acquisition-result routing remain open.
- P69 Slice F live proof remains open.

## Turn 80 | 2026-07-06

Scope: continue P69 Slice C acquisition lease lifecycle deepening.

Actions:

- Moved route-bound acquisition completion, lease restoration, rollback
  mutation, and post-cleanup rollback update into
  `cli/src/native/remote_view_handoff.rs`.
- Rewired `remote_view_open` helper adapters in `cli/src/native/actions.rs` to
  delegate those service-state mutations to the handoff module while retaining
  timestamp generation and browser/repository orchestration in the dispatcher.
- Kept begin-acquisition lease reservation in `actions.rs` for now because it
  still constructs a pending `RemoteViewRoute` with the local
  `default_control_input_provider` helper. That is the next obvious Slice C
  sequencing move.
- Updated P69 with the current Slice C progress and remaining work.

Validation:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml test_remote_view_open_acquisition_lease_rollback_restores_route_state -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml test_remote_view_open_dry_run_plans_route_bound_launch_without_existing_display -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- P69 Slice C is deeper but still partially implemented. The handoff module now
  owns planned/opened response assembly, cleanup/rollback summary reporting,
  acquisition completion, lease restoration, rollback mutation, and cleanup
  update mutation. Begin-acquisition reservation and retained-browser recovery
  sequencing still remain to move behind the handoff module interface.
- P69 Slice F live proof remains open.

## Turn 79 | 2026-07-06

Scope: continue P69 Slice C handoff cleanup reporting.

Actions:

- Added `route_bound_handoff_cleanup_summary()` to
  `cli/src/native/remote_view_handoff.rs`.
- Rewired `remote_view_open` failure paths in `cli/src/native/actions.rs` to
  use the handoff module's cleanup summary for rollback and cleanup reporting.
- Removed the duplicate local cleanup-summary formatter from `actions.rs`.
- Added handoff-module coverage for cleanup summary shape.
- Updated P69 with the new Slice C progress. Repository rollback mutation still
  remains in `actions.rs` for the next deeper sequencing pass.

Validation:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml test_remote_view_open_cleanup_reports_new_browser_close_on_failure -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml test_remote_view_open_acquisition_lease_rollback_restores_route_state -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- P69 Slice C is deeper but still partially implemented. The handoff module now
  owns planned/opened response assembly and cleanup/rollback summary reporting.
  Full acquisition, finalization, rollback mutation, and retained-browser
  recovery sequencing still remain to move behind the handoff module interface.
- P69 Slice F live proof remains open.

## Turn 78 | 2026-07-06

Scope: continue P69 Slice C route-bound handoff deepening.

Actions:

- Added `planned_route_bound_handoff_response()` and
  `opened_route_bound_handoff_response()` to `cli/src/native/remote_view_handoff.rs`.
- Rewired `remote_view_open` dry-run and opened success paths in
  `cli/src/native/actions.rs` to call the handoff response builders instead of
  assembling the authoritative profile, browser, session, route, display, tab,
  operator-visible proof, and verification fields in the command dispatcher.
- Preserved existing response shape for dry-run and opened handoffs while
  concentrating that shape behind the handoff module interface.
- Fixed `parse_remote_view_open` to preserve global `--browser-build` into
  the `remote_view_open` command payload. The broader `remote_view_open_`
  filter caught this as a P69 flag-preservation regression.
- Updated P69 and the routing-failure note.

Validation:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml test_remote_view_open_dry_run_plans_route_bound_launch_without_existing_display -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo test --manifest-path cli/Cargo.toml open_preserves_runtime_profile -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml shared_profile -- --nocapture`

Result:

- P69 Slice C is deeper but still partially implemented. The handoff module now
  owns planned/opened response assembly. Full acquisition, finalization,
  rollback, and retained-browser recovery sequencing still remains to move
  behind the handoff module interface.
- P69 Slice F live proof remains open.

## Turn 77 | 2026-07-06

Scope: continue P69 Slice E shared-profile client ergonomics.

Actions:

- Added generated TypeScript declaration coverage for
  `ServiceSharedProfileAcquisitionSummary`.
- Added `summarizeServiceSharedProfileAcquisition()` to
  `@agent-browser/client/service-request`.
- The helper accepts either an access-plan response or a tab response and
  returns compact requested profile, planned profile, runtime profile, profile
  id, retained browser/session route hints, tab/target ids, service tab handle,
  acquisition mode, route-hint requirement, and duplicate-process policy.
- Added service-request client tests for summaries from both access-plan
  `decision.profileReuse.sharedAcquisition` and tab response
  `data.sharedAcquisition`.
- Updated P69, the routing-failure note, README, docs site service-mode
  guidance, and the agent-browser skill so software clients use the helper
  instead of parsing raw profile reuse state.

Validation:

- `node scripts/generate-service-request-client.js`
- `pnpm test:service-request-client`
- `pnpm test:service-client-contract`
- `pnpm test:service-client-types`
- `pnpm test:service-client`

Result:

- P69 Slice E shared-profile client ergonomics is implemented. P69 remains
  open for Slice C's full handoff-module sequencing and Slice F live proof.

## Turn 76 | 2026-07-06

Scope: continue P69 Slice D workspace inventory actionability.

Actions:

- Added `WorkspaceProfileActionability` to the dashboard workspace inventory
  projection.
- Marked compatible live service-owned retained browser rows with
  `openSharedProfileTab` and enabled their `add-tab` action as the recommended
  shared-profile operation.
- Marked profile-only lock rows with `waitForProfileHolder` or
  `rejectDuplicateProcess` so the dashboard distinguishes agent-browser-owned
  retained profile sharing from unknown or incompatible profile holders.
- Surfaced profile actionability in workspace navigator search and selected
  row detail.
- Wired the service-owned browser row `add-tab` action to HTTP
  `service_request` `tab_new` using the retained owner route hints, then
  refreshed service status and selected the returned browser/tab identity.
- Added viewer-controller lease and route-switch actionability to the same
  workspace inventory interface. Those rows now recommend `takeOverViewer` or
  `routeSwitch`, carry the lease or attachability reason, and keep `add-tab`
  disabled when opening another shared-profile tab is not the correct
  operation.
- Updated P69, the routing-failure note, README, dashboard docs, and the
  agent-browser skill.

Validation:

- `pnpm test:dashboard-workspace-nodes`
- `pnpm test:dashboard-workspace-navigator`
- `pnpm test:dashboard-profile-allocation`
- `pnpm test:dashboard-inspector-actions`
- `pnpm build:dashboard`
- `git diff --check -- packages/dashboard/src/lib/service-workspaces.ts packages/dashboard/src/components/workspace-navigator.tsx scripts/test-dashboard-workspace-nodes.js scripts/test-dashboard-workspace-navigator.js docs/dev/plans/0069-2026-07-06-shared-profile-routing-and-handoff-deepening-plan.md docs/dev/notes/2026-07-06-last30days-profile-routing-failure.md RUNBOOK.md README.md docs/src/app/dashboard/page.mdx skills/agent-browser/SKILL.md`

Result:

- P69 Slice D no-launch workspace inventory actionability is implemented. The
  inventory projection now carries the retained-owner versus duplicate-process
  distinction, viewer takeover, and route-switch recommendations, and the
  executable service-owned browser `add-tab` dashboard flow uses the
  service-request tab creation path.
- P69 remains open for Slice C's full handoff-module sequencing and Slice E/F
  contract, client, and live proof work.

## Turn 75 | 2026-07-06

Scope: continue P69 Slice C route-bound handoff deepening.

Actions:

- Added `cli/src/native/remote_view_handoff.rs` as the named module for the
  route-bound handoff proof record.
- Wired `remote_view_open` dry-run and success responses to publish
  `routeBoundHandoff` with one authoritative profile, browser, session, tab,
  route, display, and operator-visible proof surface.
- Reworked operator-visible proof failure diagnostics to include a focused
  `routeBoundHandoff` failure record for the failing route binding. Final
  post-checkout proof failures keep pre-checkout evidence separately labeled
  instead of blending it into the final proof record.
- Updated P69 and the profile-routing failure note to record Slice C progress
  and keep the remaining full handoff-module sequencing work open.

Validation:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml remote_view_handoff -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml test_remote_view_open_dry_run_plans_route_bound_launch_without_existing_display -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml service_request -- --nocapture`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- P69 Slice C is partially implemented. The response proof vocabulary is now a
  named module, while the larger plan/acquire/finalize/rollback sequencing
  still needs to move behind the handoff module interface.

## Turn 74 | 2026-07-06

Scope: continue P69 shared-profile routing and service-request parity.

Actions:

- Added an access-plan-owned helper that applies shared-profile route hints to
  `tab_new` service requests when a compatible retained same-profile browser
  already owns the requested runtime profile.
- Wired HTTP `POST /api/service/request` through that helper with the live
  service state and taught non-focus relay to honor synthesized top-level
  command hints while continuing to ignore `params.sessionName`.
- Wired MCP `service_request` through the same persisted-plus-configured service
  state used by `agent-browser://access-plan` and routed hinted requests to the
  owner daemon session.
- Extended MCP `service_request` command/schema handling for
  `runtimeProfile`, `profileId`, `profile`, `browserHost`,
  `viewStreamProvider`, `controlInputProvider`, and `displayIsolation`.
- Repaired service-request contract drift for access-plan planned tab requests
  by adding `profileClass` to the JSON schema, generated client, HTTP adapter,
  and MCP adapter.

Validation:

- `AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD=./cli/target/debug/agent-browser pnpm test:service-request-live`
- `pnpm test:service-client-contract`
- `pnpm test:service-request-client`
- `cargo test --manifest-path cli/Cargo.toml service_request -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml shared_profile -- --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Result:

- P69 Slice B now covers plain navigation plus HTTP/MCP `service_request`
  `tab_new` route-hint parity in no-launch tests and live service-request
  smoke proof.
- The live smoke opened two same-profile service tabs through one retained
  browser, released one physical target, preserved the browser/session route,
  and successfully evaluated the surviving tab handle.
- Remaining P69 work starts at Slice C route-bound handoff deepening.
