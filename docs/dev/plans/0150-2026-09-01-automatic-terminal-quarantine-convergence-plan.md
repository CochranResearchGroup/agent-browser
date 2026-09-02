# Plan 0150 | Automatic Terminal Quarantine Convergence

Date: 2026-09-01

State: ACCEPTED

Execution state: `installed_acceptance`

Lane: P150

Source baseline: `09e1d6f69eecd0a5e2590a44f2ecba903e36214d`

Source checkpoint: `1986e004bf4f1328cbc08d98638e52501e8f621c`

Branch: `fix/automatic-terminal-quarantine-convergence`

Target: `main`

Integration model: one cohesive validated checkpoint on a short-lived topic
branch, followed by a merge to `main`.

Authority: PLAN, SOURCE, DOCUMENTATION, PROVIDER-FREE FIXTURES, ISOLATED
DEVELOPMENT-RUNTIME VALIDATION, EXACT PRODUCTION-CANDIDATE INSTALLATION,
BOUNDED PRODUCTION RECONCILIATION, AND ONE ROUTE-BOUND RETRY ARE IN SCOPE.
Credential entry, authentication completion, tenant mutation, broad process
cleanup, profile replacement, formal release, and provider reconfiguration are
out of scope.

Depends on:

- Plan 0114 exact terminal quarantine repair; and
- Plan 0144 canonical authority semantics already integrated through the source
  baseline.

## Incident

An acquisition rollback lost its browser, process identity, and session. Normal
service reconciliation correctly classified the retained route and display as
orphaned and returned the route-pool entry to available. The terminal quarantine
repair accepts only absent or released route and display records, so it reports
`route_not_released` and leaves the failed `rollback_incomplete` lease active.
Every later acquisition for the same logical browser or session is rejected by
that historical quarantine.

The individual reconciliation and repair tests pass. Their composition is
missing: no ordinary control path advances safely orphaned route and display
records to released before evaluating terminal quarantine repair.

## Goal

Make service reconciliation automatically close one provably inactive terminal
acquisition quarantine after all physical and logical owners disappear. Preserve
the quarantine whenever any browser, process, session, viewer, controller,
handoff, route, display, route-pool checkout, or other retained evidence could
still represent a live effect.

## Execution Graph

| Slice | Depends on | Work | Exit condition |
| --- | --- | --- | --- |
| A | none | Add one provider-free regression for the orphaned-state deadlock | The fixture fails before implementation |
| B | A | Add dependency-ordered automatic convergence and negative fences | Focused safe, unsafe, and idempotence tests pass |
| C | B | Align repair guidance and all required operator documentation | Help, README, skill, docs site, plan, roadmap, and runbook agree |
| D | C | Run selected source validation and qualify an isolated development candidate | Focused tests, format, Clippy, development doctor, and launch smoke pass |
| E | D | Install the exact accepted candidate and run bounded production reconciliation | Installed identity converges and the exact quarantine becomes terminal |
| F | E | Retry one route-bound acquisition without completing authentication | A ready durable remote-view handoff is returned, or a new typed blocker is preserved |

## Frozen Safety Contract

1. A historical failed lease cannot reserve resources after its exact cleanup
   obligation is proven inactive and complete.
2. Orphaned is not synonymous with safe to release. The convergence step must
   prove every retained owner and active viewer or controller reference absent
   before changing route or display state.
3. Route, display, and pool state converge before the lease advances from
   `rollback_incomplete` to `rollback_complete`.
4. The transition is atomic in the service-state repository and preserves the
   original failure and cleanup evidence.
5. Reconciliation is idempotent. A second pass performs no additional
   transition and does not recreate quarantine.
6. Ambiguous or live evidence remains quarantined with a typed reason.
7. No browser launch, termination, profile mutation, or provider action occurs
   during source and development qualification.
8. A detached pending lease is never auto-completed until its valid creation
   timestamp is at least 15 minutes old. Resource ID reuse is not ownership
   without matching browser, session, or allocation identity.

## Acceptance Criteria

- A focused Rust regression is red before the repair and green afterward.
- Negative fixtures preserve quarantine for live viewer, controller, browser,
  process, session, route-pool, or conflicting route ownership evidence.
- A repeated reconciliation pass is a no-op after successful convergence.
- The explicit repair surface reports actionable guidance when an exact dry run
  has zero candidates and a typed skip reason.
- Required user and agent documentation describes automatic convergence and the
  remaining fail-closed repair boundary.
- Selected focused tests, Rust formatting, strict Clippy, documentation checks,
  and the validation selector pass.
- An isolated development candidate passes development doctor and disposable
  browser-launch smoke before production installation.
- Exact installed readback proves source, installed binary, service generation,
  and runtime ownership agree before production reconciliation.
- Production reconciliation closes only the exact inactive quarantine, keeps the
  route pool ready, and permits one fresh route-bound acquisition.

## Bounds And Stop Rules

- Implementation attempts: `1/2`.
- Review and rework cycles: `0/1`.
- Final merged-candidate apply attempts: `2/2`; the first preserved the old
  generation at the authenticated-presentation gate and the second accepted
  after the supported candidate journey was supplied.
- Production reconciliation applies: `1/1`.
- Route-bound acquisition retries: `1/1`.
- Checkpoint interval: three slices or ninety minutes.
- Stop before production installation if source or development qualification
  fails.
- Stop reconciliation if any active or ambiguous owner appears.
- Stop after one acquisition retry if the result is not a ready durable handoff.
- Do not repair retained state by direct file or database editing.
- Do not enter credentials or complete authentication in this plan.

## Lane Reconciliation

The active-lane projection listed P144 and P147 as active even though both
published lane tips are ancestors of the source baseline. P150 replaces those
stale projection rows on its branch. Its owned implementation surfaces are
`cli/src/native/service_health.rs`,
`cli/src/native/service_retained_state.rs`, focused remote-view service-state
tests, and the matching documentation. P150 does not modify runtime-host
supervisor adoption or canonical active-claim mutation. If another lane changes
either owned Rust file before integration, rebase and rerun focused plus selected
validation before merge.

## Initial Checkpoint

State transition: `ready -> active`.

Acceptance state: diagnosis complete; source repair not started.

Progress classification: `blocker_reduction`.

Evidence: the production-safe dry run returned zero candidates with
`route_not_released`; current source maps a dead owner to orphaned route and
display records, while terminal repair accepts only released records; the two
isolated focused tests pass and therefore do not cover their composition.

Material blocker: the automatic convergence seam has no regression or
implementation.

Next action: add the single red provider-free reconciliation fixture.

## Source Checkpoint

State transition: `active -> source_qualified`.

Acceptance state: Slices A through C complete; isolated development candidate
qualification remains.

Progress classification: `outcome_progress`.

Evidence:

- the composition fixture ran and failed on orphaned route state before the
  implementation;
- normal reconciliation now uses both current state and the pre-pass ownership
  snapshot before completing an inactive quarantine;
- a matching route-bound acquisition performs one scoped convergence inside the
  same repository transaction before scanning for historical blockers;
- live viewer, active route checkout, live browser, pool-route mismatch, and
  conflicting retained evidence preserve quarantine;
- repeat reconciliation is idempotent and the original failure reason remains;
- skipped explicit repair reports `repaired=false` and actionable typed
  guidance; and
- the quarantine suite, service-health suite, strict Clippy, formatting, docs
  build, remote-view documentation check, and selected source-free workstation
  checks pass.

Material blocker: the source checkpoint has not yet been built and installed in
the isolated development runtime.

Next action: build the optimized development candidate, publish it only to the
development pseudo-home, run development doctor and disposable launch smoke,
then re-evaluate the production installation gate.

## Production Fieldwork Checkpoint

State transition: `source_qualified -> live_handoff_ready_with_race_hardening_pending`.

The exact August 31 Research.gov quarantine converged to
`failed/rollback_complete`; its quarantine marker was removed, cleanup recorded
`confirmed_inactive_retained_state`, Route A and display 12 became released, and
the pool entry returned to available. The first fresh acquisition then exposed
a separate race: background reconciliation classified its seconds-old pending
lease as detached before the foreground path advanced it to `DisplayReady`.
The state machine rejected `RolledBack -> DisplayReady`, while the browser,
window, route, and Research.gov tab remained healthy.

A no-launch reattach recovered that exact live browser and finalized durable
handoff `r580584` with `operatorVisible.state=ready`; no duplicate browser was
started. The source now adds the 15-minute age fence and identity-aware reused
resource checks, with fresh-acquisition and live-unrelated-handoff regressions.

Development generation `0.28.0-4d2c8ce7ecba` passed doctor, provider-isolation
checks, and three disposable launch checks with the age fence installed.
Production candidate `672d311dd354` was built from the same source. Its first
handoff-safe activation preserved the old generation because candidate
presentation was not proven. A separate coordinated installer later accepted
generation `0.28.0-e2244cd2447c-c25a91eb0d2b` with an authenticated candidate
presentation receipt, finalized the Research.gov lane at owner generation 6,
and restored a green install doctor. The browser, route, display, Research.gov
tab, and durable handoff all remain ready. That accepted generation does not
contain this plan's age fence. Material remaining gate: merge and install the
qualified age fence through another handoff-safe transition without a new
browser launch. Do not start an overlapping workstation repair or another cold
acquisition.

## Development Qualification Checkpoint

State transition: `source_qualified -> development_qualified`.

Acceptance state: Slice D complete; production installation and bounded live
reconciliation remain.

Progress classification: `outcome_progress`.

Evidence:

- `pnpm build:development-candidate` produced binary SHA-256
  `52fde82a55d7abba7019fb06185f8836a48176b133d2443e8cab5d85a633e6e6`;
- development generation `0.28.0-52fde82a55d7` was installed while production
  remained unchanged;
- development doctor passed selected-generation, executable, runtime-host,
  dashboard, protected-authority, browser, skill, provider-isolation, route,
  and warm-display checks; and
- the disposable development browser smoke passed three open, URL-read, close,
  and residue iterations.

Material blocker: production still runs the prior generation and retains the
inactive quarantine.

Next action: integrate the qualified source checkpoint, dry-run the exact
production workstation candidate transaction, and apply only if census and
installed-runtime gates are green.

## Installed Acceptance

State transition: `live_handoff_ready_with_race_hardening_pending -> installed_acceptance`.

Acceptance state: Slices E and F complete; Plan 0150 accepted.

Progress classification: `accepted_progress`.

Evidence:

- source checkpoint `1986e004bf4f1328cbc08d98638e52501e8f621c`
  was integrated into `main` by `4b58b6bb12cb674ee37850d34e957c4f3c08a39c`;
- merged `origin/main` checkpoint `b73dafcccb2366805de4350d0af8c09a01431ef4`
  produced optimized binary SHA-256
  `4a92c42517e1441f5e30b6fcf52857123efa7eb8273a8b126fc504de966333f7`;
- transaction `upgrade-a65e0348-7d32-4f62-9889-b4908c8cbe91` accepted
  generation `0.28.0-4a92c42517e1-6121fd69672b` after an authenticated
  candidate handoff receipt and finalized the Research.gov lane at owner
  generation 9;
- installed command, selected generation, workspace candidate, and source
  candidate share the exact binary digest; production doctor succeeds with one
  current runtime host, one executable generation, and zero legacy daemons;
- durable handoff `r580584` remains ready at presentation generation 3 with the
  original Research.gov tab valid, browser PID 43472 healthy, and no browser
  relaunch; and
- the repository and user-scoped Agent Browser skill share SHA-256
  `de6aecad582058c2e0eca727675c2b5cadf727834c00b27cc5cf03acffe5550d`.

The first apply attempt failed closed because no authenticated request reached
the shadow candidate within its five-minute window. The old generation was
preserved. The accepted retry used the existing private dashboard bootstrap
credential only to authenticate the candidate and resolve the same opaque
handoff; no credential material was logged or copied.

Material blocker: none for Plan 0150. Legacy profile-lease provenance and an
inactive supervisor remain nonblocking doctor warnings owned by other lanes.

Next action: continue Research.gov fieldwork through the same durable handoff;
do not cold-launch a replacement browser.
