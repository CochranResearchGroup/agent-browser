# Plan 0119: Architecture Line Reconciliation

Date: 2026-08-22

State: ACTIVE

Lane: P119

Source baseline: `acf7466296099b58d08078cfc08cb3f9148999c1`

Depends on:

- `docs/dev/plans/0069-2026-07-06-shared-profile-routing-and-handoff-deepening-plan.md`
- `docs/dev/plans/0116-2026-08-15-runtime-adoption-and-transactional-upgrade-plan.md`
- `docs/dev/plans/0117-2026-08-19-runtime-lifecycle-authority-and-convergence-plan.md`
- `docs/dev/plans/0118-2026-08-21-agent-operating-knowledge-closure-plan.md`

## Goal

Make `architecture-deepening-20260809` the authoritative integration line for
the completed runtime, profile, and remote-view architecture. Reconcile every
unique behavior still held in the three preserved repair branches, validate it
through the current module interfaces, then retire only worktrees and branches
whose source is either integrated or intentionally superseded.

## Architecture Decision

Preserve the architecture-deepening line and deepen two existing modules:

1. the runtime convergence module owns candidate-host selection, selected
   ingress, retained-browser ownership transfer, bounded rollback, and recovery;
2. the retained-browser acquisition module owns profile proof, retained-browser
   reuse, attributed service tab handles, and bounded physical tab close.

Old branch patches are evidence, not integration authority. A behavior lands
only through the current module interface and current lifecycle invariants.
Patch-equivalent history is not replayed. Superseded implementations remain in
git history but do not create another seam.

## Preserved Inputs

### Runtime convergence

- `fix/p0204-retained-browser-routing`
  - route retained-browser adoption by durable logical identity;
  - bind the shadow dashboard to the candidate runtime host;
  - keep candidate host discovery and monitor behavior transaction-consistent.
- `fix/runtime-host-ingress-stale-selected-20260821`
  - reject rollback from a stale transaction after a newer commit;
  - recover an exact dead selected backend before restaging.

### Retained-browser acquisition

- `fix/legacy-retained-profile-route-20260815`
  - reuse a retained browser only with exact profile and owner proof;
  - allow an attributed service tab handle to route without reacquiring a
    launch-profile lease;
  - close only the selected physical target for service-handle release;
  - preserve the retained browser and unrelated tabs.

## Execution Slices

### Slice A: ingress transaction fencing

Reconcile the stale rollback and dead-selected-backend behavior into the
current `RuntimeHostIngressRepository` interface. Preserve revision,
transaction ID, generation ID, host ID, and candidate/fallback checks. Validate
the complete runtime-host-ingress test module plus workstation integration.

### Slice B: candidate convergence

Compare each P0204 patch with the current workstation and retained-browser
adoption implementation. Keep current behavior when later Plan 0116 or Plan
0117 work already provides stronger evidence. Implement only missing behavior
and add a current-interface regression for every accepted gap.

### Slice C: retained-browser acquisition

Reconcile exact legacy profile reuse, attributed service-tab routing, and
bounded physical close through the current profile-lease and browser-lifecycle
interfaces. Do not restore direct caller knowledge that the architecture line
has already moved behind a deeper module.

### Slice D: branch and worktree retirement

Classify each auxiliary worktree as integrated, intentionally superseded, or
still carrying unique evidence. Remove only exact duplicate temporary builds
and branches with remote or integrated recovery. Preserve the three source
branches until their acceptance ledger is complete.

## Acceptance Criteria

1. Every unique source commit in the three preserved branches has an explicit
   disposition: integrated, superseded by named current evidence, or rejected
   with a concrete invariant.
2. Runtime ingress cannot roll back a newer transaction and can recover only
   the exact dead selected backend before restaging.
3. Candidate dashboard and monitor behavior use the candidate runtime-host
   ingress without falling back to retired per-session daemon assumptions.
4. Retained-browser adoption preserves durable logical browser identity across
   daemon replacement.
5. Exact retained-profile reuse does not create a duplicate browser lane.
6. An attributed service tab handle does not reacquire an unrelated launch
   profile lease.
7. Service-handle release closes at most the selected physical target and
   preserves the retained browser plus unrelated tabs.
8. Selected validation from `pnpm validation:select` passes. Rust formatting,
   strict Clippy, focused owner/ingress/profile/route tests, and the applicable
   provider-free contract gates pass.
9. The architecture worktree is clean and its remote tip equals local HEAD.
10. Obsolete auxiliary worktrees are removed only after their exact content is
    proven recoverable from the accepted architecture line or a preserved
    remote branch.

## Non-goals

- Merge or rebase `architecture-deepening-20260809` onto `main`.
- Incorporate the 91 current upstream-only commits.
- Rewrite any shared remote branch.
- Install another runtime generation or disturb retained browsers.
- Add Guacamole capacity without measured presentation demand.
- Treat a historical branch implementation as preferable merely because it
  cherry-picks cleanly.

## Validation And Stop Rules

- Use CodeGraph before changing current architecture modules.
- Use `scripts/ci/cargo-safe.sh` for every compiling Cargo command on WSL.
- Stop an integration slice when the old patch weakens current owner,
  generation, process, profile, target, ingress, or cleanup-obligation proof.
- Do not delete a dirty worktree until its patch ID or exact file content is
  recoverable from a named pushed commit.
- Keep `main` and upstream synchronization as a separate reviewed integration
  seam after this plan closes.

## Initial Evidence

- Architecture baseline and `origin/architecture-deepening-20260809` both
  resolve to `acf7466296099b58d08078cfc08cb3f9148999c1`.
- The primary worktree is clean.
- All three preserved branches have remote backup.
- `fix/runtime-host-ingress-stale-selected-20260821` merge analysis is clean.
- The legacy retained-profile branch conflicts at the current profile-lease
  seam and therefore requires behavior-level reconciliation.
- The P0204 branch conflicts in `workstation_install.rs` and therefore requires
  behavior-level reconciliation.
- Six other auxiliary worktrees are patch-equivalent, absorbed, or superseded
  retirement candidates; two dirty candidates are exact copies of pushed
  commits `5684fb6e` and `b86b75a2`.

## Source Commit Disposition Ledger

### `fix/runtime-host-ingress-stale-selected-20260821`

- `4c39e52f` is integrated as `4a8901b7`. The current ingress repository now
  fences rollback by selected transaction ID and can restore only the exact
  dead selected backend before restaging.

### `fix/p0204-retained-browser-routing`

- `74a5b4d0` is integrated through the current route-bound runtime interface as
  part of `545daca9`. Adoption passes both source session and durable logical
  browser identity.
- `b0a97190` is intentionally superseded by the stronger candidate-host
  bootstrap in `candidate_dashboard_command`. The candidate receives its
  transaction socket directory, generation, and runtime-host mode while
  deliberately omitting backend-only mode. The regression
  `candidate_dashboard_targets_the_transaction_runtime_host` proves this
  current invariant.
- `e88fd3d9` is integrated through current workstation and stream-discovery
  interfaces as part of `545daca9`. A singular runtime-host PID is accepted as
  lane liveness evidence, and monitor lock contention is a healthy skip only
  while an admission-drain record proves upgrade ownership.

### `fix/legacy-retained-profile-route-20260815`

- `1cffcfbf` is integrated through the current profile-mismatch seam as part of
  `be8a7273`. Metadata-free retained connections are accepted only when the
  service repository proves the exact browser, session, profile, and PID or CDP
  identity.
- `640310f6` is integrated through the profile-lease metadata seam as part of
  `be8a7273`. An attributed service tab handle routes to an existing tab and
  does not acquire a launch-profile lane.
- `fab9b1d1` is integrated through the browser lifecycle seam as part of
  `be8a7273`. Handle release closes only its target, does not reinitialize an
  unrelated tab, preserves the last tab, and reports whether CDP acknowledged
  the close.
- `5d69c525` is retained as historical git evidence and is intentionally not
  replayed into `RUNBOOK.md`. Its runtime claims describe the 2026-08-15 live
  census and are not current acceptance evidence. The durable behavior it
  records is represented by the three integrated source commits above.

## Validation Progress

- Runtime-host-ingress module: 6 tests passed.
- Exact logical browser adoption: 2 focused regressions passed.
- Candidate host discovery and monitor ownership: 2 focused regressions
  passed.
- Exact retained-profile route proof: 2 focused regressions passed.
- Attributed service tab lease bypass: 1 focused regression passed.
- Selected service-tab release projection: 1 focused regression passed.
- Rust formatting and diff hygiene pass for every committed slice so far.

Final selector-driven validation, strict Clippy, remote readback, and worktree
retirement remain open.
