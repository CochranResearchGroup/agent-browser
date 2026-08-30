# Plan 0143 | Workstation Upgrade Self-Admission Repair

Date: 2026-08-30

State: OPEN

Execution state: `development_runtime_validation_ready`

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
- Development publication, doctor, browser-launch smoke, and production
  installation remain pending.
