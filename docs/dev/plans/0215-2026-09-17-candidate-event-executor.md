# Plan 0215 | Candidate Event Executor

Date: 2026-09-17

Plan version: 1

State: OPEN

Consolidation: required

Product lane: PL-PLATFORM

Lane: P215

Related challenge plan: Plan 0187 W7-E

Work item: `CochranResearchGroup/agent-browser#199`; issue #66 retains every
browser, provider-backed, CAPTCHA and installed desktop-input acceptance effect

Branch: `platform/p215-candidate-event-executor`

Target: `main`

Integration: merge through the protected pull-request workflow after local
provider-free validation without restoring or dispatching operator-disabled
GitHub CI

Integrated prerequisite: P214 exact head `1ea70c47`, merged through PR #198 as
`bea093764544f74db7ceb70d33ff90f1e718a4fa`

Source baseline: `bea093764544f74db7ceb70d33ff90f1e718a4fa`

## Objective

Bind one canonical P214 event plan to the existing desktop-services controller
claim, authority fence, surface probe, provider acknowledgement, cleanup and
effect-certainty primitives. Execute only the exact frozen pointer sequence,
require fresh exact candidate evidence immediately before each button-down,
and prove the transaction with injected fakes only.

## Current State

P212 admits exact candidate geometry and controller authority. P213 proves the
visual intent is the current state-machine-authorized intent. P214 converts the
resulting permit into an exact-budget, deterministic raw pointer sequence but
intentionally stops before any provider call or event emission.

The existing desktop transaction engine already owns route-local claims,
per-event authority fences, current surface probes, provider acknowledgements,
emergency release and typed effect certainty. Its public recipe path is
single-target and replans its own motion, so it cannot safely consume a
multi-candidate P214 plan. P215 adds the narrow candidate-plan adapter instead
of bypassing or replacing those primitives.

P214 exact head `1ea70c47` merged through PR #198 as canonical
`main@bea09376`; issue #197 closed and no GitHub Actions branch or merge-head
run started. P215 is admitted from that exact canonical baseline in the clean
reassigned P214 worktree. No checkout was created or removed.

P205 retains the root Cargo manifests and Service/runtime-owner extraction.
P211 retains CLI shutdown and workstation routing. P215 touches neither source
surface. Shared roadmap, runbook and active-lane projections remain explicit
integration overlaps.

GitHub CI is operator-disabled. P215 uses local provider-free validation and
will not restore, dispatch, rerun or wait on a workflow.

## Architecture Boundary

P215 adds one candidate execution transaction inside
`agent-browser-desktop-services`:

- a canonical request binds operation identity, the exact candidate permit and
  the exact event plan;
- a candidate observation seam supplies a newly captured, canonical observation
  immediately before each `LeftDown`;
- the existing `ControllerAuthorityRepository`, `DesktopControlCoordinator`,
  `DesktopInteractionProvider`, `InteractionClock`, `SurfaceSnapshot`,
  `InputEvent` and acknowledgement types remain the lower-level primitives;
- a candidate-specific ledger prevents exact terminal replay from emitting a
  second effect and rejects operation identity conflicts; and
- a privacy-safe receipt binds attempted and acknowledged event order, cleanup,
  stop reason, replay state and final effect certainty.

The visual provider capability digest remains evidence about candidate
selection. It is retained in the permit and receipt but must not be confused
with the desktop input provider capability. The input provider is independently
validated from its current evidence and surface probe.

This packet adds no CLI, Service State, HTTP, MCP, browser adapter or installed
provider wiring. The public transaction can emit only when an embedding caller
supplies implementations of all injected seams; P215 acceptance supplies fakes
only.

## Invariants

1. Permit and event-plan digests, source linkage, event count, schedule and
   expiry are canonical before the first provider or authority call.
2. Operation identity binds the complete request. Exact terminal replay returns
   the retained receipt with no authority, observation, probe or event call.
3. The route claim is acquired once, before the first event, and every event
   uses the existing process-local authority fence.
4. Current controller authority must match the permit digest before execution
   and before every event.
5. Input-provider evidence and the focused surface identity, browser process,
   geometry, coordinate mapping and provider generation remain stable.
6. Every `LeftDown` requires a newly captured canonical candidate observation
   whose binding, evidence, candidate set and exact selected geometry still
   match the permit. Motion cannot extend observation freshness.
7. The executor never replans, reorders, inserts or accepts coordinates from a
   provider. Only an emergency matching `LeftUp` may be added for cleanup.
8. Attempted and acknowledged effect keys and events remain separately
   attributable in exact order.
9. A failure before any attempted event is `no_effect`. A failure after an
   attempted but unacknowledged event is `effect_uncertain`. Authority loss
   after an acknowledgement is `cancelled_after_effect`.
10. A button-down acknowledgement creates a cleanup obligation until the
    matching button-up is acknowledged. Failed cleanup remains uncertain.
11. The receipt stores digests and bounded identifiers, never pixels, image
    bytes, credentials, provider payloads or raw private challenge content.
12. No test or acceptance step launches a browser or emits host desktop input.

## Consolidated Batch

1. Add a red tracer for one exact four-event fake execution and zero-effect
   terminal replay.
2. Add canonical cross-stage validation for permit, plan, authority and fresh
   candidate evidence.
3. Add the injected executor, ledger and privacy-safe receipt with exact event
   attribution and emergency release.
4. Add the consolidated failure matrix for stale or changed evidence,
   authority and surface drift, provider failures, partial acknowledgement,
   replay conflict and cleanup failure.
5. Extend the crate architecture guard and run the full provider-free local
   acceptance set.

## Provider-Free Fixture Matrix

The minimum matrix proves:

1. one canonical four-event plan reaches the fake provider in exact order and
   returns a verified exact-plan receipt;
2. exact terminal replay makes zero authority, observation, probe and event
   calls, while changed input under the same operation ID fails closed;
3. invalid permit, plan digest, source linkage, event count, schedule or expiry
   makes zero provider calls;
4. initial or per-event authority mismatch and route-claim conflict stop at the
   correct effect boundary;
5. input-provider capability, surface identity, browser process, geometry or
   provider-generation drift fails closed;
6. every button-down obtains a fresh candidate observation, and stale, changed,
   missing, reordered or extra candidate evidence fails before that down;
7. event provider failure before acknowledgement is uncertain, while a prior
   acknowledgement remains separately attributable;
8. a failure after acknowledged button-down attempts one emergency left-up and
   records cleanup success or failure without continuing the plan;
9. no failure path emits a non-cleanup event after the first failed boundary;
   and
10. receipt digests are deterministic and contain no captured pixels or raw
    provider payload.

## Delivery Sequence And Budget

- Attempt 1: public tracer, request validation and terminal replay ledger.
- Attempt 2: guarded exact event execution, fresh candidate boundary and
  consolidated failure matrix with at most one packet-local semantic repair.
- Attempt 3: receipt hardening, cleanup, architecture guard and final local
  validation.
- Maximum work-unit attempts: 3.
- Maximum review and rework cycles: 1.
- Reassess after two checkpoints or 45 active minutes without outcome progress.
- Overall source effort ceiling: 240 active minutes.

## Worker Assignments

- Primary: this P215 session owns the candidate executor contract, its injected
  ledger and observation seam, provider-free fixtures, architecture guard and
  bounded planning records.
- P205 retains the service-model extraction, root Cargo manifest and lockfile.
  P215 does not edit those surfaces.
- P211 retains cold shutdown, workstation install and its CLI surfaces. P215
  does not edit them.
- No subagent, browser worker, provider worker, runtime operator, benchmark
  checkout or live-acceptance worker is assigned.

Expected source writes are limited to:

- `crates/agent-browser-desktop-services/src/candidate_event_execution.rs`;
- the smallest shared-validator changes in `candidate_event_plan.rs`,
  `candidate_intent.rs`, `transaction.rs` and the crate export;
- provider-free desktop-services fixtures;
- `scripts/test-desktop-services-crate-architecture.js`; and
- this plan plus P215's bounded roadmap, runbook and active-lane projections.

## Evidence And Exit

Source acceptance requires the complete provider-free matrix, strengthened
desktop-services architecture guard, all desktop-services and challenge-control
tests, workspace formatting, strict workspace Clippy, changed-surface
selection, documentation links, active-only planning audit and diff hygiene at
one clean checkpoint.

Publication requires durable remote custody and a normal protected merge from
the exact P214 canonical baseline. GitHub CI is operator-disabled and is not
part of this packet's validation path. P215 closes after its exact source enters
canonical `main`. CLI wiring, a concrete candidate observation adapter, an
installed provider, browser fixture or CAPTCHA acceptance remains a separate
later packet.

## Stop Condition

Stop and replan if the adapter requires a CLI or Service State change, a new
workspace member, a public HTTP or MCP schema, browser lifecycle ownership,
durable pixel storage, network access or credentials. Stop before any browser
launch, real capture, real provider call, CAPTCHA attempt, host desktop input,
installed-runtime mutation, shared Service State mutation, production effect,
release or GitHub CI execution.
