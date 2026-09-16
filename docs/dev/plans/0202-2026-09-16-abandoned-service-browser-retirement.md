# Plan 0202 | Abandoned Service Browser Retirement

Date: 2026-09-16

Plan version: 4

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
- Maximum work-unit attempts: 3.
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
| Provider-free and real-browser acceptance | Focused suites pass. Three bounded disposable fixture cycles reached classification and apply; the final live observation exposed persistence normalization drift, which is repaired and covered by a real JSON repository regression. The plan attempt budget is exhausted, so no fourth browser replay was taken. | provider-free green; real-browser gate unverified |
| Integration | Implementation checkpoint `60c71f68` and strict-lint follow-up `d544ed2e` are published while P190 retains shared documentation ownership | draft PR pending |

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

## Stop Condition

Stop before any installed, retained, protected, foreign, production, staging,
or provider effect. Stop if exact ownership, process-group identity, profile
identity, descendant closure, current inactivity, or effect authority cannot be
proven. Stop and join the persistence owner if the repair requires the core
`service_store.rs` lock/CAS algorithm. Stop and reconcile writer ownership
before editing a P190 or P197 shared surface. A disposable real-browser test
must terminate only the tree it launched and must report residue rather than
perform broad cleanup.
