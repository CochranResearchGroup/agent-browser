# Plan 0143 | Workstation Upgrade Self-Admission Repair

Date: 2026-08-30

State: COMPLETE

Execution state: `production_installed_and_verified`

Lane: P143

Source baseline: `3b7f15a031dd93b74df37ff3f6b4cddc14040ffc`

Branch: `fix/workstation-upgrade-self-admission`

Target: `main`

Integration model: one cohesive repair branch followed by local validation.

Authority: SOURCE, TESTS, DOCUMENTATION, ISOLATED DEVELOPMENT-RUNTIME
PUBLICATION AND VALIDATION, EXACT PRODUCTION-CANDIDATE INSTALLATION, REQUIRED
RUNTIME TRANSITION, AND POST-INSTALL LIVE SMOKE ARE IN SCOPE. Production
installation is authorized only for the exact binary SHA that passes the
development gates in this plan. Tenant navigation, provider mutation,
duplicate profile lanes, broad process cleanup, and unrelated runtime-state
repair are out of scope.

## Goal

Prevent a workstation upgrade from blocking its own candidate runtime after
the upgrade has intentionally drained ordinary mutation admission. Preserve
the drain for unrelated clients and preserve Service State lock safety.

## Incident Evidence

Transaction `upgrade-c00fead7-bb30-4f60-a973-268fda858b34` reached runtime
transfer at revision 8, then its candidate command for session
`dashboard-service-backend` failed with `runtime_admission_draining`. The
candidate command launcher does not propagate the transaction ID and revision
environment already consumed by the CLI admission-claim path.

The currently selected production generation is
`0.28.0-3b7f15a031dd-79a80827b0b7`, with binary SHA-256
`3b7f15a031dd93b74df37ff3f6b4cddc14040ffc988778af690310b3e3dedba5`.
Install doctor was healthy before this repair.

## Acceptance Criteria

1. Every candidate command issued during transfer and rollback inherits the
   exact active transaction ID and revision.
2. A matching installer-owned reconciliation passes admission while an
   ordinary mutation and a stale claim remain rejected.
3. Existing handoff and rollback safety semantics remain unchanged.
4. Focused Rust tests demonstrate the missing claim before the fix and pass
   afterward.
5. Rust formatting, clippy, and selected tests pass through the repository
   Cargo safety wrapper.
6. Development publication, development install doctor, and three-cycle
   development browser-launch smoke pass for one exact binary SHA.
7. The production workstation installer selects that same SHA without a
   self-admission failure or Service State process-mutation lock timeout.
8. Production install doctor reports ready, selected runtime processes match
   the installed generation, and an exact named managed-profile smoke proves
   `set viewport 1440 1000` with JavaScript readback before closing cleanly.

## Execution Sequence

1. Add a focused regression at the candidate-command process boundary.
2. Propagate the existing transaction admission environment through candidate
   bootstrap, transfer, fallback adoption, and rollback commands.
3. Run focused tests, formatting, clippy, and changed-surface validation.
4. Publish and smoke the isolated development runtime.
5. Build and install the identical production candidate through the
   transactional installer.
6. Verify installed identity, doctor, runtime multiplicity, lock diagnostics,
   viewport behavior, and cleanup.

## Hard Stops

- Do not weaken or remove the global admission drain.
- Do not authorize arbitrary actions from the presence of environment
  variables alone. Existing action-specific admission matching remains the
  enforcement boundary.
- Do not retry an uncertain production mutation without reconciling the exact
  transaction and runtime state.
- Do not terminate or delete a process or state file without exact ownership
  evidence from the active transaction.
- Do not install a production binary whose SHA differs from the development
  acceptance binary.

## Validation Record

- Red proof: the focused candidate-bootstrap regression initially failed to
  compile because no candidate admission configuration seam existed.
- Focused child-process regression passed and proved transaction
  `upgrade-test`, revision `9`, no-browser `stream status` propagation.
- Exact CLI admission matcher test passed.
- Rust clippy passed with warnings denied.
- Source-free workstation install fixture passed.
- The 119-test workstation installer family passed serially. An initial
  parallel invocation was discarded because environment-mutating tests leaked
  injected failure controls across cases; both apparent failures passed under
  the required serial execution mode.
- Exact candidate SHA-256
  `ceb8f8a926e669a881da70edd8a00e9b3e2f043a423a3f954178cf0ab0f45c51`
  passed development publication, development install doctor, the three-cycle
  development browser-launch smoke, the development runtime fixture, and an
  explicit `set viewport 1440 1000` readback.
- Production transaction
  `upgrade-c3ba448f-5360-4381-9ca2-7ab6d43f47a6` was accepted at revision 13.
  Authenticated candidate resolution of durable handoff `r551610` returned
  `resolved=true`, `status=ready`, and a ready presentation receipt for
  generation `0.28.0-ceb8f8a926e6-178c836a535e` before payload commit.
- The installed binary and the development-approved candidate have the same
  SHA-256. Install doctor returned success and runtime convergence
  `converged`. Runtime multiplicity is steady with one executable generation,
  one dashboard process, one runtime host, and zero legacy daemons.
- Production session `p143-production-viewport-final` opened Example Domain,
  accepted `set viewport 1440 1000`, and returned JavaScript readback
  `{width:1440,height:1000,devicePixelRatio:1}`. Close succeeded, and the
  lifecycle registry records the browser terminal with its cleanup obligation
  satisfied.
- The production proof browser is also terminal with a satisfied cleanup
  obligation and an absent process group. The enabled runtime-host user unit
  remains stopped by the accepted-upgrade supervisor contract so it cannot
  start a duplicate host beside the selected candidate process; the live
  runtime census still reports exactly one healthy selected host.
- Final Service State lock diagnostics report zero process timeouts, zero file
  timeouts, zero poison recoveries, and no active lock holder.

## Completion Receipt

The installer no longer blocks its own candidate commands after admission
drain. Candidate bootstrap, stream status, Service State reconciliation,
handoff adoption, failure cleanup, and rollback now carry or honor only the
exact active installation claim. Strict tests continue to reject unrelated and
stale claims.

Two additional live-state contradictions exposed by the upgrade were repaired
without broad cleanup: exact owner-only dead process state can migrate to an
inert placeholder, and an exact stale-generation owner can be adopted only
when browser, profile, route, process, and owner evidence agree. Candidate
bootstrap failure now aborts every prepared transfer, including a guarded
administrative owner-generation compare-and-swap fallback when the source
daemon cannot perform the abort.

One retained pending transfer created by an earlier failed candidate bootstrap
was repaired under the explicit operator recovery authority after backing up
the exact authoritative files at
`~/.agent-browser/runtime-adoption/manual-recovery/plan0143-pending-transfer-20260830T1430/`.
Only that exact proposal was cleared. Production installation then completed
through the supported transactional path; no broad process or profile cleanup
was used.
