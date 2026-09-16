# Plan 0202 | Abandoned Service Browser Retirement

Date: 2026-09-16

Plan version: 22

State: OPEN

Consolidation: required

Product lane: PL-PLATFORM

Lane: P202

Work item: `CochranResearchGroup/agent-browser#103`

Branch: `platform/p202-abandoned-browser-retirement`

Target: `main`

Integration: merge through the protected `main` workflow after provider-free
regression, one isolated disposable real-browser acceptance, and complete
changed-surface validation

Source baseline: `151ebccd610869f457ea684b416e31408fee9fb3`

## Objective

Make abandoned service-owned browser lanes observable and safely reclaimable
without treating age, retained display allocation, or Service State absence as
effect authority. Status must project exact per-lane resource and activity
evidence. A sealed plan must bind one eligible browser root and its descendants,
profile, process group, owner generation, and current inactivity. Apply must
revalidate those identities and activity before exact-tree shutdown, then prove
process exit, profile-lock release, and coherent terminal Service State.

## Current State

Issue #103 is claimed and P202 is admitted from `main@151ebccd`. Existing code
already separates two useful primitives: `service_browser_retirement` can
remove an entirely inert unreferenced browser row through a sealed record
digest, while `service_resources` can terminate one reviewed exact process tree
after checking process, profile, package-launch, and owner-generation identity.
Neither primitive classifies a still-live but inactive service-owned browser as
an abandoned candidate or seals current lease/activity evidence into one
retirement transaction. Current status and GC can therefore protect stale
lanes indefinitely, including through retained display allocation.

The first provider-free reproducer was red and is now green. The exact-owned fixture
`resources_classify_inactive_owned_lane_despite_retained_display` models a
`Retained/Owned` browser with complete root identity, an expired
`CloseBrowser` session, and a retained display. The focused runner reports
`candidateCount = 0` where the contract requires one retirement candidate.
localized the defect to unconditional retained-browser/display protection
before activity-aware ownership classification. The repair now projects the
joined lane, classifies only complete service-owned evidence, and uses a sealed
reserve, effect, and finalize transaction without changing the core store CAS
algorithm.

Final published-diff review reopened the source before merge readiness. Packet
9 now repairs both provider-free P1 counterexamples: authoritative occupancy
was invisible until finalization after the process effect, and apply could
replace a reviewed candidate with a newly observed process tree for the same
browser ID. Focused validation and comprehensive provider-free requalification
are green at source checkpoint
`844eba2671681cd3e0c884a9cae5c9f839d6d565`. The final two-axis published-diff
review then found that the reviewed identity still omitted descendants, so a
same-root replacement child set could become the freshly sealed plan. Packet
10 owns that one provider-free binding repair. Packet 11 exposed a terminal
status harness-contract mismatch. Packet 12 corrected that mismatch
provider-free and the one renewed real-browser acceptance passed. No additional
browser replay is authorized.

P190 and P197 remain active. P190 has no expected P202 source overlap but is
the current writer for shared CLI help, README, agent skill, and command docs.
P197 is the current writer for generated service-request contracts. P202 will
keep those writers authoritative, use disjoint lifecycle/status surfaces where
possible, and record a dependency or serialize integration before touching an
overlapping shared surface. If the repair requires changing the core
`service_store.rs` lock/CAS algorithm, P202 stops and joins the owning
persistence lane.

## Consolidated Batch

1. Freeze the per-lane observation, eligibility, sealed-plan, apply, terminal
   receipt, and typed-recourse contract using the existing Service State,
   process identity, lease authority, and reviewed process-tree vocabulary.
2. Add the cheapest deterministic fixture that reproduces the defect: a live,
   service-owned, inactive lane remains permanently protected or cannot produce
   an exact retirement plan despite complete identity evidence. Preserve active,
   retained, protected-profile, foreign, revision-drift, and concurrent-activity
   controls.
3. Project browser-root count, descendant count, tab count, RSS, last lease
   observation, and cleanup disposition per service-owned lane plus bounded
   workstation totals and thresholds.
4. Add configurable inactivity and resource-budget policy that can classify an
   exact service-owned lane as an abandoned candidate while keeping age alone
   non-authoritative and explicit retention, protected profiles, foreign
   applications, and incomplete evidence ineligible.
5. Extend sealed plan/apply so one exact browser root, process group, profile,
   owner generation, expected descendant set, state revision, and activity
   evidence are revalidated immediately before effect. Return typed recourse on
   current activity, identity drift, revision drift, or incomplete authority.
6. Prove successful retirement by complete process-group exit, profile-lock
   release, and one coherent terminal Service State transition. Add provider-free
   fixtures and one isolated disposable real-browser acceptance with independent
   residue readback.
7. Complete user-facing and contract documentation parity on an explicitly
   reconciled shared-doc surface, then run every selected changed-surface gate.

## Scope And Effect Boundary

Expected implementation surfaces are browser lifecycle and retirement,
service-resource observation and GC classification, status projection, service
models and contracts, focused provider-free fixtures, and one disposable
real-browser acceptance harness. Plan, roadmap, runbook, active-lane, and
required user-facing documentation projections are in scope.

This plan does not authorize installed-runtime cleanup, production or staging
mutation, shutdown of any retained or foreign browser, protected-profile
cleanup, provider mutation, Service State edits outside disposable fixtures,
runtime restart, release, or broad process cleanup. The real-browser acceptance
may launch and retire only its own disposable profile and exact process group,
then must prove no residue.

## Delivery Sequence And Budget

- Critical path: contract freeze to red fixture to source repair to focused
  provider-free validation to isolated real-browser acceptance to full
  changed-surface validation to protected integration and closeout.
- Parallel W1: resource/status projection and candidate-classification seam
  inventory, read-only, using `gpt-5.6-luna` at medium effort.
- Parallel W2: sealed retirement/apply and process-tree fixture seam inventory,
  read-only, using `gpt-5.6-sol` at medium effort.
- Parallel W3: fresh acceptance, overlap, and regression-risk review, read-only,
  using `gpt-5.6-terra` at medium effort.
- Parallel W4: resource/status projection, policy, classification, and
  provider-free decision matrix implementation, using `gpt-5.6-terra` at high
  effort with sole write ownership of `service_resources.rs`.
- Parallel W5: sealed reserve/effect/finalize retirement transaction and
  provider-free drift fixtures in a new focused module, using `gpt-6-astra` at
  high effort without shared resource-module writes.
- Intended active concurrency: one primary plus three shallow workers; no
  nested subagents. Workers return evidence and stop before edits or effects.
- Original maximum work-unit attempts: 3. Renewed Acceptance Packet 5 extends
  the cumulative maximum to 4 for one exact discriminating replay without
  resetting prior attempt history.
- Maximum review and rework cycles: 1.
- Maximum consecutive hardening checkpoints: 2.
- Reassess after two checkpoints or 30 active minutes without outcome progress.
- Overall effort ceiling: 300 active minutes through integration and closeout.
- The first required outcome artifact is one deterministic red-capable focused
  fixture within 60 active minutes.

## Worker Assignments

The P202 lane owner holds the critical path, contract decisions, issue and lane
state, branch, all writes, red/green adjudication, integration, and final
acceptance. W1 reports existing resource/status seams and the smallest fixture
that can prove the missing candidate. W2 reports the exact retirement/apply
identity and activity gaps plus a provider-free process-tree test design. W3
independently maps every issue criterion to current evidence or a missing proof,
checks P190/P197 overlap, and may return no additional finding.

W1, W2, and W3 completed read-only and agreed that the existing status lacks a
joined per-lane projection, the retained browser/display guards are
unconditional, and record-only retirement cannot seal live activity or process
identity. W2 additionally found that generic GC performs external work inside
a replayable repository mutation and can unlink `SingletonLock` while claiming
release; P202 must not reuse either behavior. W4 and W5 own disjoint
implementation packets. Worker outputs remain advisory until primary review,
focused validation, and integration.

P190 remains primary writer for its candidate-orchestration source and current
shared command documentation. P197 remains primary writer for its challenge
consumer source and generated service-request contracts. P202 is primary writer
for abandoned-browser lifecycle, resource/status projection, and its focused
fixtures after the worker join.

## Evidence And Exit

Exit requires current evidence that:

- status projects the required exact per-lane and bounded workstation resource,
  activity, and cleanup-disposition evidence;
- configurable inactivity and resource policy identifies eligible abandoned
  service-owned lanes without making age or display retention sufficient;
- active work, explicit retention, protected profiles, foreign processes, and
  incomplete or unknown identity evidence remain ineligible;
- a sealed plan binds one exact browser root, descendant set, process group,
  profile, owner generation, state revision, activity observation, and expiry;
- apply revalidates every sealed identity and current activity before effect and
  returns typed recourse for concurrent activity or drift;
- success proves complete process-group exit, profile-lock release, and a
  coherent terminal Service State transition;
- provider-free fixtures cover the decision matrix and one isolated disposable
  real-browser acceptance leaves no process or profile-lock residue;
- all required documentation parity and selected changed-surface validation
  pass; and
- the exact source checkpoint enters `main` through the linked pull request
  with applicable exact-head CI green.

| Requirement | Current evidence | State |
| --- | --- | --- |
| Existing exact-tree effect primitive | `service_resources` rechecks reviewed process, process-group, profile, package-launch, and owner-generation identity before signaling and proves exit plus profile-lock release | reusable partial primitive |
| Existing record-only retirement | `service_browser_retirement` seals an inert row digest and removes only an unreferenced browser row | reusable but insufficient |
| Abandoned live-lane classification | Decision matrix now accepts complete inactive exact-owned lanes and rejects active work, explicit retention, protected profiles, foreign identity, and incomplete census | provider-free green |
| Per-lane resource and activity projection | Status joins browser root, descendants, tabs, RSS, lease activity, policy thresholds, and cleanup disposition | implemented; broader validation pending |
| Sealed activity-aware plan/apply | Nine focused transaction tests cover reserve, revalidation, drift, repository CAS revisions, normalization, and terminal finalize | provider-free green |
| Terminal Service State convergence | Finalize removes the exact browser, session, tab, display, route, viewer, acquisition, pool, and capacity records after effect proof | provider-free green; real-browser replay pending |
| Provider-free and real-browser acceptance | Provider-free suites pass. The cumulative fourth and terminal disposable replay reached the exact-tree effect but finalize returned `abandoned_browser_retirement:ExitUnproven`. Independent readback found no fixture process or temporary-directory residue. | provider-free green; real-browser gate blocked on exit proof |
| Integration | Draft PR #168 is open from validated head `e7ae2a9a`; P190 retains shared documentation ownership | draft; blocked on documentation and renewed live acceptance |

## Implementation Checkpoint 1

- Published plan checkpoint: `f99cd8a6` on
  `origin/platform/p202-abandoned-browser-retirement`.
- W1 `/root/p202_resource_status`: completed read-only resource/status seam and
  red-fixture inventory.
- W2 `/root/p202_retirement_apply`: completed read-only apply-path audit and
  pure reserve/effect/finalize transaction design.
- W3 `/root/p202_acceptance_audit`: completed acceptance matrix and P190/P197
  overlap audit.
- Red command:
  `scripts/ci/rust-tests.sh --focused resources_classify_inactive_owned_lane_despite_retained_display`.
- Red result: one selected test failed at the candidate-count assertion, with
  `left: 0`, `right: 1`; 3,222 tests were filtered out.
- No browser, service, installed runtime, provider, retained profile, or
  foreign process was changed.

## Implementation Checkpoint 2

- Source checkpoint: `60c71f68`.
- Added bounded per-lane and workstation resource projections plus configurable
  inactivity and resource thresholds.
- Added an activity-aware decision matrix that keeps active work, explicit
  retention, protected profiles, foreign or unproven identities, and incomplete
  census evidence ineligible.
- Added `retire_abandoned_browser_lane` with a sealed plan and pure
  reserve/finalize mutations. The external exact-tree effect occurs outside the
  replayable repository mutation.
- Added coherent terminal cleanup for the exact browser lane and profile-claim
  blocking while a retirement transaction is pending.
- `scripts/ci/rust-tests.sh --focused abandoned_lane_decision_matrix` passes
  its selected test.
- `scripts/ci/rust-tests.sh --focused service_abandoned_browser_retirement`
  passes all nine selected tests, including the real JSON repository CAS and
  persistence-normalization regression.
- `node --check scripts/smoke-service-resource-gc-live.js` and
  `git diff --check` pass.
- Three bounded disposable real-browser fixture cycles were used. The final
  cycle reached apply and returned `BrowserRecordChanged`; the subsequent
  deterministic repository regression proved that persistence normalization
  changed derived tab handles before the reserved browser digest was checked.
  The reserve path now normalizes derived views before sealing that digest, and
  the regression passes. The live attempt budget is exhausted, so real-browser
  acceptance remains explicitly unverified rather than inferred from the
  deterministic repair.
- Fresh process and temporary-directory readback found no disposable P202
  browser, host, profile, or fixture residue. No installed runtime, provider,
  protected profile, retained browser, or foreign process was changed.

## Validation Checkpoint 3

- Published head: `d544ed2e`.
- Strict workspace Clippy, formatting, patch hygiene, Node syntax, service
  API/MCP parity, generated service-client contract and type checks,
  lease-authority architecture, all 108 lease-authority tests, all 39 focused
  service-model tests, the decision matrix, and all nine retirement transaction
  tests pass.
- The comprehensive Rust runner completed every native compartment and every
  support compartment except `transport`. That compartment did not reach its
  tests because the sccache wrapper failed while spawning `rustc`.
- The documented deterministic cache opt-out was applied only to the failed
  compartment:
  `AGENT_BROWSER_CARGO_CACHE=off scripts/ci/rust-tests.sh --compartment transport`.
  All three transport tests and its doc tests then passed.
- Fresh branch and remote readback agree at `d544ed2e`. Fresh process and
  temporary-directory readback again found no disposable P202 residue.
- Provider-free validation is complete. Shared user-facing documentation, one
  renewed bounded real-browser acceptance, exact-head CI, review, integration,
  and issue closeout remain.

## Integration Checkpoint 4

- Draft PR #168 is open against `main`:
  `https://github.com/CochranResearchGroup/agent-browser/pull/168`.
- Issue #103 remains open and records the provider-free evidence and remaining
  gates. No completion label or closure was applied.
- The first PR status readback shows Version Sync Check passed while Rust
  Quality, Dashboard, Service Client, and Workstation Fixtures are in progress.
  Per normal implementation closeout, P202 does not actively monitor them.

## Renewed Acceptance Packet 5

The three-attempt implementation budget is not reset. One cumulative fourth
attempt is added for a single discriminating acceptance replay because the
third attempt exposed an exact persistence-normalization cause and the real
JSON repository regression now proves that repair. This packet has one owner,
one disposable namespace, and one terminal run.

- Candidate: current P202 source with a freshly built debug binary.
- Effect scope: only the harness-created temporary home, managed one-time
  profile, exact process groups, and foreign-process control.
- Preconditions: clean branch, exact local/remote identity, Chrome present,
  no prior P202 fixture process or temporary-directory residue.
- Required evidence: one eligible inactive managed lane, active/protected and
  foreign controls preserved, apply success, terminal Service State cleanup,
  process-group exit, profile-lock release, and zero exact fixture residue.
- Stop rule: any failure is terminal for this plan version. Preserve its exact
  typed result and residue census; do not retry or broaden cleanup.

## Acceptance Result 5

- Fresh debug build and all preconditions passed at branch head `43ba2a86`.
- The single terminal replay classified the inactive managed lane and reached
  the exact-tree effect. Finalize returned
  `abandoned_browser_retirement:ExitUnproven`.
- The harness performed only its exact task-owned cleanup. Independent
  post-run readback found no `ab-managed-resource-gc-*` temporary directory and
  no Chrome, Node, or agent-browser process carrying the fixture identity.
- No retry was taken. The next implementation packet must first make exit
  evidence discriminating enough to identify which sealed condition remained
  false, cover that condition provider-free, and preserve fail-closed terminal
  behavior. A later live replay requires a newly bounded plan revision.

## Exit-Proof Diagnostic Packet 6

This provider-free packet changes tactic from browser replay to a tight
deterministic exit-proof oracle. It does not reset the four consumed live
attempts and authorizes no browser or runtime effect.

- Milestone: terminal recourse names every failed sealed exit predicate so the
  observed `ExitUnproven` can be attributed before another browser run.
- Primary writer: P202 lane owner, limited to the retirement owner, its Linux
  exit-observation adapter, focused tests, and current plan/runbook projection.
- W7 `/root/p202_exit_cause`: strongest-tier, high-effort, read-only causal
  analysis of the finalize and exact-tree shutdown path.
- W8 `/root/p202_resource_implementation`: normal-tier, read-only Linux process
  semantics and provider-free fixture design, reusing prior lane context.
- Routing goal: balanced wall-clock and allocation. Deterministic tools remain
  primary; workers return evidence only and own no Git or runtime custody.
- Feedback loop: one focused test must fail on the current generic
  `ExitUnproven` result and pass only when root, descendant, process-group,
  profile-lock, identity, revision, and time failures are distinguishable.
- Bound: one implementation attempt, one focused validation cycle, and 30
  active minutes. Stop without a live replay if the failure cannot be captured
  at the stable finalize seam.

## Exit-Proof Diagnostic Result 6

- Source checkpoint: `2243de8c`.
- W7 `/root/p202_exit_cause` traced the exact path and found that finalization
  collapsed nine identity, exit, lock, and time predicates into one unit error.
  It recommended preserving fail-closed behavior while returning every failed
  predicate with the observed evidence.
- W8 `/root/p202_resource_implementation` found that Linux zombies retained
  both the sealed start token and a signal-addressable process group. The older
  PID helper already treated zombies as exited, but the P202 sealed PID and
  group predicates did not.
- The Linux zombie-only process-group fixture was red twice in 0.01 seconds,
  reporting `(root_exited, process_group_empty) = (false, false)`. It now passes
  with `(true, true)` after sealed PID observation recognizes zombie state and
  group observation requires a non-zombie `/proc` member. Incomplete process
  census still falls back conservatively to the raw process-group probe.
- The exit-recourse fixture was red on the generic
  `abandoned_browser_retirement:ExitUnproven`. It now passes across plan,
  reservation, process-group, root, descendants, group emptiness, profile
  lock, and both time-bound predicates, plus a combined failure. The typed
  recourse preserves the complete observed evidence, state immutability,
  cleanup ownership, and stable serialized `exit_unproven` code.
- All ten retirement transaction tests, the zombie-only group regression, the
  abandoned-lane decision matrix, formatting, strict workspace Clippy, patch
  hygiene, and smoke-script syntax pass.
- Progress classification: blocker reduction. The observed live cause now has
  a deterministic reproducer and repair, but this packet authorizes no browser
  replay. Complete changed-surface requalification remains before any decision
  about another bounded acceptance run.

## Final Acceptance Packet 7

Complete provider-free requalification passed at branch head `ba18521c` with
source checkpoint `2243de8c`: the comprehensive runner reports
`nativeLane=0`, `supportLane=0` after 736 seconds with Cargo caching disabled.
This preserves all focused, formatting, strict Clippy, and comprehensive
evidence on the repaired executable input.

One cumulative fifth live attempt is justified because the preceding live
failure now has a direct 0.01-second red/green Linux reproducer for both false
exit predicates, the terminal error surface is discriminating, and the exact
changed source passed comprehensive requalification. This is not a reset of
attempt history.

- Candidate: current P202 executable inputs at source checkpoint `2243de8c`,
  built fresh into `cli/target/debug/agent-browser`.
- Fixture: unchanged isolated `smoke-service-resource-gc-live.js` namespace and
  exact task-owned cleanup.
- Preconditions: local and remote branch agree, worktree clean, Chrome exists,
  no prior fixture process or temporary directory, and no other browser
  acceptance run is active.
- Acceptance: exact managed lane retires with a terminal receipt; protected and
  foreign controls survive until fixture teardown; root, descendants, and
  process group exit; profile lock releases; terminal Service State converges;
  independent residue census is empty.
- Bound: one run and no retry. Any failure records the new typed failed
  conditions and ends live execution for P202.

## Final Acceptance Result 7

- The single terminal run reached finalization and returned one exact failed
  condition: `profile_lock_released`.
- The same receipt proves `root_exited=true`, `descendants_exited=true`, and
  `process_group_empty=true`; the zombie-only repair therefore reached its
  intended real-browser boundary.
- No retry was taken. Independent post-run readback found no fixture process or
  `ab-managed-resource-gc-*` temporary directory. That readback follows harness
  teardown and is residue evidence, not proof that product retirement released
  the lock before teardown.
- Progress classification: outcome progress. The generic exit blocker is now a
  single lock-convergence defect with preserved typed evidence.

## Profile-Lock Convergence Packet 8

This provider-free packet owns the last observed product defect and authorizes
no additional browser replay.

- Milestone: after exact root, descendants, and process-group exit are proven,
  retirement releases only the exact planned profile's stale `SingletonLock`
  and then observes it absent; any live, mismatched, protected, or unproven
  process remains fail-closed before lock mutation.
- W9 `/root/p202_exit_cause`: strongest-tier read-only review of lock ordering,
  exact-profile authority, and terminal receipt semantics.
- W10 `/root/p202_resource_implementation`: normal-tier read-only inventory of
  existing stale-lock cleanup primitives and the smallest deterministic
  filesystem fixture.
- Primary writer: P202 lane owner, limited to the exact shutdown adapter and
  focused provider-free tests.
- Feedback loop: an isolated temporary profile with a dangling Chromium-shaped
  `SingletonLock` must be red on the current P202 adapter and green only when
  the lock is removed after exact process exit. A live-process control must
  retain the lock and avoid cleanup.
- Bound: one implementation attempt, one focused validation cycle, and 30
  active minutes. Stop with no further live execution if exact ordering cannot
  be proven at the existing reviewed-shutdown seam.

Packet 8 result at source checkpoint `979b5cb8854b0a49980f2dff7fa7523ebc999544`:

- W9 confirmed that the reviewed shutdown invokes profile-lock handling only
  after exact root, descendant, and process-group exit. It rejected both the
  generic GC unlink and Chrome's multi-artifact cleanup as broader than P202's
  authority.
- W10 independently found the same boundary and specified the dangling-link
  regression plus retained-lock controls. Neither worker edited files or
  performed runtime effects.
- The isolated dangling `SingletonLock` regression failed red with
  `assertion failed: released`. The implementation then added a post-exit
  reservation recheck that does not reuse the vanished pre-signal census.
- Cleanup now requires the exact planned profile path and canonical identity,
  the unchanged pending reservation and CAS revision, the unchanged browser
  and owner identity, `Closing` plus `Owned` custody, a fresh exact-exit check,
  and a symlink target PID equal to the sealed root PID. It removes only
  `SingletonLock` and proves absence afterward.
- Missing locks succeed idempotently. Unauthorized cleanup, a foreign PID, or
  a non-symlink lock remains preserved and yields no release proof.
- Green evidence: 18 focused retirement tests, including the zombie-only
  process-group case and new lock controls; 11 transaction tests; workspace
  format; strict workspace Clippy; and diff hygiene all pass.
- Comprehensive provider-free Rust requalification passed at the same source
  checkpoint with `nativeLane=0`, `supportLane=0`, and
  `elapsedSeconds=656`.
- The first published Packet 8 CI run then exposed one Rust 1.98-only quality
  diagnostic: `unnecessary_sort_by` at the lane-activity observation ordering
  seam. Local Clippy 1.94.1 had accepted the exact code. Checkpoint
  `646bb8639ee0e6ab345c09b9b520e310b9a5f05c` adopts the equivalent
  `sort_by_key` form. Strict local Clippy, the focused abandoned-lane decision
  matrix, format, selector output, and diff hygiene pass. The prior
  comprehensive result remains applicable because ordering semantics and its
  exercised behavior are unchanged; a fresh CI run must prove the 1.98 lint.
- No browser replay, installed-runtime mutation, provider effect, or shared
  P190/P197 documentation edit occurred. Changed-surface validation is
  complete; shared-documentation reconciliation remains the next gate.

## Pre-Effect Authority Packet 9

This provider-free packet reopens the candidate after final published-diff
review and must close both P1 authority gaps before documentation or merge
readiness.

- Milestone A: planning, reservation, and every pre-signal revalidation reject
  current authoritative occupancy, including an occupied presentation slot and
  any exact-profile claim or lease evidence that grants current work custody.
  A finalization-only occupancy check is not effect authority.
- Milestone B: a valid review token authorizes only the exact candidate identity
  present in the reviewed candidate set. The per-candidate fresh snapshot may
  confirm that identity, but must never substitute a new root, descendant set,
  process group, profile, owner generation, or package-launch identity for the
  same logical browser ID.
- W11 `/root/p202_resource_implementation`: normal-tier read-only inventory of
  authoritative occupancy projections and the smallest provider-free fixtures.
- W12 `/root/p202_exit_cause`: strongest-tier read-only trace from reviewed
  candidate identity through fresh plan construction, with the minimum
  fail-closed binding contract and deterministic two-snapshot regression.
- Primary writer: P202 lane owner, limited to retirement planning/apply,
  resource classification, and focused provider-free fixtures.
- Feedback loop A: the existing eligible retirement fixture must fail planning
  and produce no reservation or signal when exact current occupancy is added.
- Feedback loop B: a reviewed candidate followed by a fresh eligible replacement
  with the same browser ID must be rejected before reservation; an unchanged
  candidate remains admissible.
- Validation: red/green focused authority fixtures, complete retirement tests,
  selector-required format and strict Clippy, then one comprehensive
  provider-free Rust requalification for the completed reopened batch.
- Bound: one consolidated implementation cycle and no browser, installed
  runtime, provider, protected-profile, foreign-process, or shared-doc effect.
  Stop if the repair requires the core `service_store.rs` lock/CAS algorithm or
  a Lease Authority mutation outside the existing read-only projection.

Packet 9 result at source checkpoint
`844eba2671681cd3e0c884a9cae5c9f839d6d565`:

- W11 confirmed that the exact profile claim, presentation-slot lease, active
  viewer or controller lease, and nonterminal remote-view acquisition are
  authoritative occupancy surfaces omitted from the prior classifier. W12
  confirmed that apply reloaded a fresh snapshot and process census but bound
  only the logical browser ID, permitting an independently eligible
  replacement identity to reach reservation. Both workers remained read-only.
- The authoritative-occupancy fixture failed red because an active exact
  profile claim still produced a retirement plan. It now covers all four
  occupancy classes and passes. The reservation control proves late occupancy
  leaves state unchanged, while the effect control proves late occupancy
  returns before the first process signal.
- The two-snapshot identity fixture failed red because a coherent replacement
  root with the same browser ID remained admissible. Apply now reconstructs an
  eligible candidate from the same fresh snapshot and process census and
  requires exact equality with the originally reviewed `candidateIdentity`
  before entering the repository mutation. The check is unconditional, so
  `--force-without-review` bypasses human review only, not identity consistency.
- Current profile claims are read from canonical Lease Authority state without
  mutation. Presentation occupancy, active or unresolved viewer and controller
  custody, and nonterminal acquisition custody feed the deterministic lane
  activity reasons and digest already rechecked by planning, reservation, and
  pre-signal validation. Existing released and completed fixture records remain
  eligible.
- Green evidence: all 21 retirement-focused tests, both targeted authority
  regressions, workspace formatting, strict workspace Clippy, selector output,
  and diff hygiene pass. The selector requires no additional contract or
  documentation gate for the two changed Rust files.
- Comprehensive provider-free Rust requalification passed at the exact source
  checkpoint with `nativeLane=0`, `supportLane=0`, and
  `elapsedSeconds=687`.
- No browser replay, installed-runtime mutation, provider effect, core store
  CAS edit, or P190/P197 shared-documentation edit occurred. Comprehensive Rust
  requalification is complete; P190 shared-documentation reconciliation
  remains.

## Published-Diff Descendant Binding Packet 10

The final review accepts one additional P1 spec finding and keeps the Standards
and Spec axes separate.

- Spec finding: Packet 9 compares the exact 14-field `candidateIdentity`, but
  that identity does not contain the reviewed descendant set. A fresh process
  census can therefore replace or add descendants beneath the same unchanged
  root, after which apply seals the replacement into a new retirement plan.
- Standards finding: the required user-facing environment and action
  documentation remains incomplete on four P190-owned files. That is the known
  shared-documentation gate, not authority for P202 to edit them concurrently.
  The repeated action string is accepted as nonblocking cleanup backlog.
- Primary writer: P202 lane owner, limited to candidate identity construction,
  the exact fresh-snapshot comparison, and one deterministic provider-free
  regression. W13 `/root/p202_exit_cause` supplied the read-only Spec review;
  W14 `/root/p202_resource_implementation` supplied the independent read-only
  Standards review.
- Feedback loop: a reviewed eligible root with one exact child must fail red
  when a fresh independently eligible snapshot keeps the root but adds or
  replaces a child. The unchanged root and child set remains admissible. The
  reviewed candidate output must identify the expected descendant identities.
- Validation impact: run the targeted red/green regression, all retirement
  tests, format, strict Clippy, selector, and one fresh comprehensive Rust
  requalification because executable candidate identity and review-token input
  change. Prior live acceptance remains diagnostic history and is not rerun.
- Bound: one implementation attempt and one provider-free validation cycle.
  No core store change, browser replay, installed-runtime mutation, provider
  effect, or P190/P197 shared-documentation edit is authorized.

Packet 10 source result at checkpoint
`e727162e156b161cb2fff51dbcadcffeedb0d3fa`:

- The reviewed candidate now includes a sorted `descendants` array containing
  each expected child's PID, start token, and executable path. The review token
  digest and the per-candidate fresh-snapshot equality check therefore bind the
  exact descendant physical identities as well as the root identity.
- Candidate construction fails closed if an identity cannot be recovered for
  any descendant PID produced by the same census. Generic non-P202 candidates
  preserve their existing serialized identity because an empty descendant set
  is omitted.
- The same-root changed-child regression failed red at the fresh-snapshot
  comparison and now passes. It also proves the reviewed output identifies the
  expected child, the unchanged tree remains admissible, and the earlier
  coherent root-replacement control still rejects.
- Green focused evidence: the exact descendant regression, all 21
  retirement-focused tests, four review-token tests, workspace format, strict
  workspace Clippy, selector output, and diff hygiene. Comprehensive Rust
  requalification passed at the exact source checkpoint with `nativeLane=0`,
  `supportLane=0`, and `elapsedSeconds=715`.
- The final review ledger retains two disclosed completion gates: successful
  exact-artifact real-browser acceptance is still unproven after the prior
  profile-lock failure, and all four required user-facing documentation files
  remain under P190 writer custody. No additional live replay is authorized in
  this packet.

## User-Authorized Final Acceptance Packet 11

On 2026-09-16 the operator explicitly authorized the real-browser acceptance.
This renews only the isolated disposable acceptance effect; it does not
authorize installed-runtime, production, staging, provider, protected-profile,
foreign-process, or broad cleanup effects.

- Candidate source: `e727162e156b161cb2fff51dbcadcffeedb0d3fa`.
  Subsequent branch commits through `fee7c0c0bc0ec0365509fd9ab57da6215f611d5d`
  change only the P202 plan and lane receipt. Exact-head fast CI and Rust are
  green, and comprehensive provider-free Rust passed at the source checkpoint.
- Fixture: the existing `scripts/smoke-service-resource-gc-live.js` harness in
  its generated `ab-managed-resource-gc-*` home, socket, profile, session, and
  process namespaces. It may terminate only process groups derived from that
  exact generated home and its recorded disposable fixture identities.
- Preconditions: build the normal debug binary from the frozen source; prove
  Chrome exists; prove no prior matching fixture process, directory, or active
  harness exists; preserve unrelated production and lane runtimes; and record a
  fresh process census before effect.
- Acceptance: one reviewed abandoned candidate; exact retirement receipt;
  root, descendants, and process group exit; `SingletonLock` absence before
  harness teardown; coherent terminal Service State; protected profile-holder
  and unrelated Chrome survive until their exact fixture teardown; and fresh
  post-run census finds no fixture process or temporary-directory residue.
- Bound: one run and no retry. Any failure preserves the exact typed evidence,
  performs only the harness's exact task-owned cleanup, records fresh residue,
  and ends Packet 11.
- Critical-path owner: the P202 lane owner performs preflight, the single live
  effect, and terminal readback. Subagents may review frozen evidence but may
  not hold runtime custody or execute the effect.
- Dependency readback: despite the operator's report that P190 landed, current
  GitHub and Git evidence still shows PR #152 open at `ce19901f` and
  `origin/main` at `151ebccd`. Shared documentation remains separately blocked
  and is not touched by this acceptance packet.

Packet 11 result:

- The exact debug candidate built successfully through
  `scripts/ci/cargo-safe.sh` from source checkpoint
  `e727162e156b161cb2fff51dbcadcffeedb0d3fa`; later branch commits through
  `60e17c1b9c64b16f6f6cbb51034bf13849dcb66c` were documentation-only. The
  binary SHA-256 was
  `cca0c54b94ecf07329c9c405e78350038833b15759ac7a64e6b4559b2556bc29`.
- The sole authorized `pnpm test:service-resource-gc-live` invocation failed
  with `Terminal Service State is incoherent: undefined`. It was not retried.
  The immediately preceding harness checks had already proved the reviewed
  retirement apply reported success, the exact managed process group exited,
  and the managed profile `SingletonLock` was absent.
- Source diagnosis localizes the failure to the acceptance harness contract.
  `service status` reconciles a cloned authority snapshot and intentionally
  removes a terminal `process_exited` browser operational row through
  `remove_post_termination_browser_history`. The harness instead requires that
  same browser row to remain present with `health=process_exited`, so it rejects
  the intentional terminal projection before checking the separately retained
  lifecycle record and retirement receipt.
- Fresh post-run evidence at `2026-09-16T16:45:56-05:00` found no matching
  `ab-managed-resource-gc-*` process, no matching temporary directory, and no
  worktree mutation. Aggregate host counts were 49 Chrome processes and 14
  agent-browser processes; none matched the generated fixture namespace.
- Acceptance remains failed because the authorized end-to-end contract did not
  complete. A provider-free harness correction may align terminal coherence
  with the existing status projection, but another real-browser run requires
  renewed operator authority.

## Corrected Acceptance Packet 12

On 2026-09-16 the operator authorized correction of the diagnosed harness
contract and one additional isolated real-browser attempt.

- The provider-free correction requires the reconciled terminal status to omit
  the browser operational row and process identity while retaining exactly one
  `terminal/satisfied` runtime lifecycle record and one retirement receipt.
- A focused Node contract test covers the accepted terminal projection plus
  controls for a lingering operational row, missing lifecycle evidence, and a
  missing receipt. It failed red before the extracted contract helper existed.
- The live harness consumes the same helper. No Rust source, retirement
  transaction, process signaling, cleanup scope, installed runtime, provider,
  production state, protected profile, or foreign process behavior changes.
- Validate the focused contract, Node syntax, diff hygiene, and repository
  validation selection before effect. Reuse the already built exact Rust
  binary because no executable Rust input changed.
- Run the isolated `pnpm test:service-resource-gc-live` harness exactly once.
  Preserve any failure without retry and always take a fresh process and
  temporary-directory residue census.

Packet 12 result:

- Corrected harness and provider-free contract checkpoint:
  `e6bf565872e691f26bddd2cecb0dea17d9150c1f`.
- The focused contract failed red before the extracted helper existed, then
  passed with controls for a lingering browser row, missing terminal lifecycle,
  and missing retirement receipt. All three Node files parse and diff hygiene
  passes.
- The changed-surface selector required the release-asset fixture and complete
  service-client lane because the package script and smoke files changed. Both
  passed before the browser effect.
- The sole renewed `pnpm test:service-resource-gc-live` invocation passed:
  `service-resource-gc-live: ok managed_pid=1969 helpers=13
  unrelated_protected=true terminal_receipt=true`.
- The run reused binary SHA-256
  `cca0c54b94ecf07329c9c405e78350038833b15759ac7a64e6b4559b2556bc29`;
  no Rust source or executable input changed after its build.
- Fresh post-run evidence at `2026-09-16T17:11:08-05:00` found no matching
  fixture process and no `ab-managed-resource-gc-*` temporary directory.
  Aggregate host counts were 49 Chrome processes and 8 agent-browser processes;
  none matched the generated fixture namespace.
- Isolated exact-artifact real-browser acceptance is complete. The remaining
  completion gate is reconciliation of the four P190-owned user-facing
  documentation files after P190 actually integrates or explicitly hands off
  writer custody.

## Stop Condition

Stop before any installed, retained, protected, foreign, production, staging,
or provider effect outside the single Packet 11 disposable acceptance. Stop if exact ownership, process-group identity, profile
identity, descendant closure, current inactivity, or effect authority cannot be
proven. Stop and join the persistence owner if the repair requires the core
`service_store.rs` lock/CAS algorithm. Stop and reconcile writer ownership
before editing a P190 or P197 shared surface. A disposable real-browser test
must terminate only the tree it launched and must report residue rather than
perform broad cleanup.
