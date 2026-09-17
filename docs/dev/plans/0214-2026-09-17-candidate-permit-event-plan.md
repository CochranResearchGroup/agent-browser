# Plan 0214 | Candidate Permit Event Plan

Date: 2026-09-17

Plan version: 1

State: OPEN

Consolidation: required

Product lane: PL-PLATFORM

Lane: P214

Related challenge plan: Plan 0187 W7-D

Work item: `CochranResearchGroup/agent-browser#197`; issue #66 retains every
browser, provider-backed, CAPTCHA and desktop-input acceptance effect

Branch: `platform/p214-candidate-event-plan`

Target: `main`

Integration: merge through the protected pull-request workflow after local
provider-free validation without restoring or dispatching operator-disabled
GitHub CI

Integrated prerequisite: P213 exact head `c8012bd0`, merged through PR #196 as
`7e56d9c7c9108470a66b839e0bae3a76e48c7f50`

Source baseline: `7e56d9c7c9108470a66b839e0bae3a76e48c7f50`

## Objective

Freeze a provider-neutral, effect-free event-plan contract that converts one
P212 candidate permit and reviewed starting pointer into a deterministic raw
pointer-event sequence. Every interpolated move, button-down and button-up must
consume the permit's declared pointer budget. The planner must stop before any
provider call, route claim or event emission.

## Current State

P213 now joins a state-machine-authorized visual intent to exact candidate
geometry and current controller authority, returning the P212 effect-free
permit. That permit carries ordered targets plus total step, pointer and key
budgets. The existing desktop transaction planner independently chooses a
smooth path and records its raw motion-point count. Treating the W7 planned
pointer budget as interchangeable with that variable raw count would weaken
the challenge budget or reject valid plans.

P214 resolves only that semantic boundary. It plans an exact number of raw
pointer events from the permit and emits nothing. It does not reuse the
existing unbounded motion planner, execute the result or change any current
recipe path.

P214 is admitted in the clean reassigned P213 worktree; no checkout was
created or removed. P205 retains the service-model extraction plus root Cargo
manifest and lockfile, and P211 retains CLI shutdown and workstation routing.
P214 touches neither source surface and changes no manifest. Shared roadmap,
runbook and active-lane projections remain explicit P211 overlaps.

GitHub CI is operator-disabled. P214 uses local provider-free validation and
will not restore, dispatch, rerun or wait on a workflow.

## Architecture Boundary

P214 adds one pure desktop-services planner:

- `DesktopCandidateEventPlan`: exact source permit, starting pointer, ordered
  `InputEvent` sequence, duration, expiry and deterministic plan digest;
- `validate_desktop_candidate_effect_permit`: canonical structural validation
  for a permit crossing into another pure stage; and
- `plan_desktop_candidate_events`: a checked function that distributes the
  exact move budget across ordered candidate centers and appends one left-down
  and one left-up per candidate.

For `N` selected candidates, the plan requires at least `3 * N` pointer events:
one or more moves plus one down and one up for each candidate. After reserving
the two button events per target, remaining move events are distributed by
quotient and remainder in candidate order. Linear interpolation uses widened,
checked arithmetic and ends exactly at each bound center. Event timestamps are
monotonic across a caller-supplied nonzero duration and cannot exceed permit
expiry.

This slice is pointer-only. A nonzero key budget fails closed rather than
inventing key semantics. The output reuses the existing inert `InputEvent`
vocabulary but never calls `DesktopInteractionProvider`, the coordinator,
authority repository, ledger, handoff, capture, probe or executor.

## Invariants

1. The permit digest is canonical and every carried identifier, digest,
   binding, selected geometry, budget and expiry is structurally valid.
2. Planned steps equal selected candidate count.
3. Key-event budget is zero for this pointer-only plan.
4. Pointer-event budget is exact, not advisory. Output event count equals the
   permit count.
5. Every candidate receives at least one move followed by exactly one down and
   one up, and candidate order is preserved.
6. Every interpolated point is inside the desktop and each final point equals
   its candidate center.
7. Start point, duration, timestamp arithmetic and coordinate interpolation
   use checked bounds and fail closed on overflow.
8. Planning begins before permit expiry and its last event does not exceed that
   expiry.
9. Construction performs no provider, route, authority, browser, capture,
   persistence or input operation.
10. Exact replay returns the same event sequence and plan digest.

## Consolidated Batch

1. Add a public tracer proving one admitted synthetic permit becomes an exact
   four-event, effect-free plan.
2. Add canonical permit validation and the deterministic checked planner.
3. Add a consolidated mutation matrix for permit shape, geometry, budgets,
   starts, schedules, expiry, order and replay.
4. Extend the desktop-services architecture guard with a file-local no-effect
   rule for the candidate planner.
5. Run desktop-services and challenge-control tests, architecture, formatting,
   strict workspace Clippy, documentation, planning, selection and diff
   hygiene.

## Provider-Free Fixture Matrix

The minimum matrix proves:

1. one target and four pointer events produce two moves, down and up in order;
2. two targets distribute an odd move remainder deterministically while
   preserving target order;
3. permit digest, source digest, authority digest, capability digest, binding,
   geometry or selected-candidate mutation fails closed;
4. zero or mismatched steps, nonzero key events and fewer than three pointer
   events per target fail closed;
5. negative or out-of-surface start and target geometry fail closed;
6. zero duration, timestamp overflow, coordinate overflow, stale start or an
   end beyond expiry fails closed;
7. every final move equals the candidate center and every event count exactly
   matches the permit; and
8. exact replay returns the same plan without calling an executor.

## Delivery Sequence And Budget

- Attempt 1: public tracer and canonical permit validation.
- Attempt 2: exact-budget interpolation and consolidated mutation matrix with
  at most one packet-local semantic repair.
- Attempt 3: architecture guard, refactor and final local validation.
- Maximum work-unit attempts: 3.
- Maximum review and rework cycles: 1.
- Reassess after two checkpoints or 30 active minutes without outcome
  progress.
- Overall source effort ceiling: 180 active minutes.

## Worker Assignments

- Primary: this P214 session owns the permit validator, candidate event planner,
  provider-free fixtures, file-local architecture guard and bounded planning
  records.
- P205 retains the service-model extraction, root Cargo manifest and lockfile.
  P214 does not edit those surfaces.
- P211 retains cold shutdown, workstation install and its CLI surfaces. P214
  does not edit them.
- No subagent, browser worker, provider worker, runtime operator, benchmark
  checkout or live-acceptance worker is assigned.

Expected source writes are limited to:

- `crates/agent-browser-desktop-services/src/candidate_event_plan.rs`;
- the smallest permit-validator change in `candidate_intent.rs` and export in
  `lib.rs`;
- provider-free desktop-services fixtures;
- `scripts/test-desktop-services-crate-architecture.js`; and
- this plan plus P214's bounded roadmap, runbook and active-lane projections.

## Evidence And Exit

Source acceptance requires the complete provider-free matrix, strengthened
desktop-services architecture guard, all desktop-services and challenge-control
tests, workspace formatting, strict workspace Clippy, changed-surface
selection, documentation links, active planning audit and diff hygiene at one
clean checkpoint.

Publication requires durable remote custody and a normal protected merge from
the exact P213 canonical baseline. GitHub CI is operator-disabled and is not
part of this packet's validation path. P214 closes after its exact source
enters canonical `main`; an executor, provider adapter or live fixture remains
a separate later packet.

## Stop Condition

Stop and replan if the planner needs a provider, route claim, authority read,
capture, probe, event acknowledgement, operation ledger, handoff, CLI or
Service State change, new workspace member, public schema, filesystem, network
or credential. Stop before any browser launch, image capture, provider call,
CAPTCHA attempt, desktop input, installed-runtime mutation, shared Service
State mutation, production effect or release.
