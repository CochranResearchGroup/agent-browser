# Plan 0202 | Abandoned Service Browser Retirement

Date: 2026-09-16

Plan version: 1

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
checks P190/P197 overlap, and may return no additional finding. Worker outputs
are advisory evidence; the primary reconciles them before implementation.

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
| Abandoned live-lane classification | Current issue evidence reports stale lease activity with GC candidate count zero and retained-display overprotection | defect reported; focused reproducer pending |
| Per-lane resource and activity projection | Existing resource/status projections need requirement-level inspection | worker and primary inventory pending |
| Sealed activity-aware plan/apply | No current joined transaction is proven | missing |
| Terminal Service State convergence | Record-only retirement exists; live exact-tree lifecycle convergence is unproven | missing |
| Provider-free and real-browser acceptance | Existing reviewed-tree tests may be reusable; exact #103 matrix and residue proof are unproven | pending |
| Integration | Issue claimed at `main@151ebccd`; no P202 source checkpoint exists yet | pending |

## Stop Condition

Stop before any installed, retained, protected, foreign, production, staging,
or provider effect. Stop if exact ownership, process-group identity, profile
identity, descendant closure, current inactivity, or effect authority cannot be
proven. Stop and join the persistence owner if the repair requires the core
`service_store.rs` lock/CAS algorithm. Stop and reconcile writer ownership
before editing a P190 or P197 shared surface. A disposable real-browser test
must terminate only the tree it launched and must report residue rather than
perform broad cleanup.
