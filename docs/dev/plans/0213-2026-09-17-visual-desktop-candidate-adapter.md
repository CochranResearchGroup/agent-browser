# Plan 0213 | Visual Desktop Candidate Adapter

Date: 2026-09-17

Plan version: 2

State: OPEN

Consolidation: required

Product lane: PL-CHALLENGE

Lane: P213

Parent plan: Plan 0187 W7-D

Work item: `CochranResearchGroup/agent-browser#194`; issue #66 retains every
browser, provider-backed, CAPTCHA and desktop-input acceptance effect

Branch: `challenge/p213-visual-desktop-adapter`

Target: `main`

Integration: merge through the protected pull-request workflow after local
provider-free validation without restoring or dispatching operator-disabled
GitHub CI

Integrated prerequisite: P212 exact head `dc20155e`, merged through PR #193 as
`ddae1897bbee8b8b234054a45d89ba2243bfe43c`

Source baseline: `ddae1897bbee8b8b234054a45d89ba2243bfe43c`

## Objective

Join one current, state-machine-authorized `VisualRoundIntent` to the P212
desktop candidate observation and controller-authority contract. The adapter
must produce one deterministic effect-free desktop permit or fail closed. It
must not reinterpret geometry, weaken either policy boundary, claim a route or
emit input.

## Current State

P209 can adjudicate a candidate-only visual response and P206 can authorize one
`VisualRoundIntent` inside cumulative challenge budgets. P210 owns artifact
custody and the injected provider transport. P212 now owns exact physical-pixel
candidate geometry and current controller-authority admission in desktop
services. All four contracts are integrated in canonical `main@ddae1897`.

No composition function currently proves that a supplied visual intent is the
active `IntentAuthorized` result of its challenge snapshot before mapping it to
the P212 contract. Raw component matching alone would allow a forged but
self-digested intent to bypass challenge snapshot and cumulative-policy
custody.

P213 is admitted in the clean reassigned P212 worktree; no worktree was created
or removed. P205 owns its service-model source plus root Cargo manifest and
lockfile. P213 avoids those surfaces by adding the pure adapter inside the
existing challenge-control crate, which already consumes desktop services.
P211 remains primary writer for CLI shutdown and workstation routing. P213
touches neither lane's source. Shared roadmap, runbook and active-lane
projections are explicit P211 reconciliation overlaps.

GitHub CI is operator-disabled. P213 uses local provider-free validation and
will not restore, dispatch, rerun or wait on a workflow.

Source checkpoint `1bad68e3` implements the current-intent validator and pure
visual-to-desktop adapter. All 58 challenge-control tests and all 12
desktop-services tests pass with the strengthened challenge architecture
guard, formatting, strict workspace Clippy, documentation links, planning
audit, selector readback and diff hygiene. Publication and protected
integration remain.

## Architecture Boundary

P213 adds two pure entrypoints inside `agent-browser-challenge-control`:

- a current-intent validator that proves the visual policy, snapshot, active
  evidence, active selection and intent are one canonical
  `IntentAuthorized` state; and
- a visual-to-desktop admission function that requires exact candidate order,
  evidence, frame, context, geometry, observation time, expiry and provider
  capability before constructing and delegating one `DesktopCandidateIntent`
  to P212.

The adapter returns the P212 `DesktopCandidateEffectPermit` directly. It does
not create another permit vocabulary, candidate digest algorithm, authority
model or error reinterpretation. Visual candidate-set digests continue to bind
ordered candidate identities. Desktop candidate-set digests continue to bind
ordered identities plus physical geometry. The adapter verifies their shared
ordered identities without pretending the two digests are interchangeable.

## Invariants

1. The visual policy and snapshot validate before mapping, and the snapshot is
   exactly `IntentAuthorized` with one active evidence, selection and digest.
2. Every public intent field equals the canonical intent reconstructed from
   that active evidence and selection.
3. Visual evidence candidate identities exactly equal desktop observation
   candidate identities in order.
4. Evidence, frame, context and geometry digests, observation time and expiry
   match exactly across the visual and desktop contracts.
5. The provider capability digest is copied from the validated visual intent;
   capability identity or version drift fails before desktop admission.
6. Selected identities and planned step, pointer and key budgets are copied
   exactly. The adapter cannot reorder, add coordinates, add an event, retry or
   synthesize another attempt.
7. Controller lease and viewer identities are explicit inputs and remain
   subject to P212's exact current-authority validation.
8. P212 owns geometry, freshness and controller-authority decisions. Typed
   desktop admission errors remain distinguishable from challenge-state or
   cross-contract mismatches.
9. Rejection returns no partial permit. Success claims no route and emits no
   event.
10. Exact replay returns the same permit digest.

## Consolidated Batch

1. Add a public tracer that starts from a state-machine-produced visual intent
   and returns one exact P212 permit.
2. Extract the smallest public current-intent validation helper from the
   existing visual-round invariants.
3. Add the pure visual-to-desktop mapping and typed cross-contract errors.
4. Add one consolidated fail-closed fixture matrix for snapshot, intent,
   evidence, candidate, time, capability, budget and authority drift.
5. Strengthen the challenge-control architecture guard and run focused crate,
   formatting, strict workspace Clippy, documentation, planning, selection and
   diff-hygiene checks.

## Provider-Free Fixture Matrix

The minimum matrix proves:

1. one canonical authorized intent returns one deterministic P212 permit and
   no effect;
2. a non-authorized phase or changed active evidence, selection or intent
   digest fails closed;
3. changed task, attempt, round, evidence or candidate binding fails closed;
4. missing, duplicate, out-of-set or reordered candidate identities fail
   closed;
5. frame, context, geometry, observation time or expiry drift fails closed;
6. provider capability identity, version or digest drift fails closed;
7. changed steps, pointer events or key events fails closed;
8. browser, display, stream, route, geometry epoch, lease, viewer or authority
   drift remains a typed P212 rejection; and
9. exact replay yields the same permit without transport, route claim, capture
   or input.

## Delivery Sequence And Budget

- Attempt 1: public tracer and current-intent validator.
- Attempt 2: adapter mapping and consolidated fail-closed matrix with at most
  one packet-local semantic repair.
- Attempt 3: architecture guard, refactor and final local validation.
- Maximum work-unit attempts: 3.
- Maximum review and rework cycles: 1.
- Reassess after two checkpoints or 30 active minutes without outcome
  progress.
- Overall source effort ceiling: 180 active minutes.

## Worker Assignments

- Primary: this P213 session owns the visual-desktop module, minimal
  visual-round validator extraction, provider-free fixtures, challenge-control
  architecture guard and bounded planning records.
- P205 remains primary writer for the service-model extraction, root Cargo
  manifest and lockfile. P213 does not edit those surfaces.
- P211 remains primary writer for cold shutdown, workstation install and its
  CLI surfaces. P213 does not edit them.
- No subagent, browser worker, provider worker, runtime operator, benchmark
  checkout or live-acceptance worker is assigned.

Expected source writes are limited to:

- `crates/agent-browser-challenge-control/src/visual_desktop.rs`;
- the smallest validator change in `visual_round.rs` and export in `lib.rs`;
- provider-free challenge-control fixtures;
- `scripts/test-challenge-control-crate-architecture.js`; and
- this plan plus P213's bounded roadmap, runbook and active-lane projections.

## Evidence And Exit

Source acceptance requires the complete provider-free matrix, strengthened
challenge-control architecture guard, all challenge-control and
desktop-services tests, workspace formatting, strict workspace Clippy,
changed-surface selection, documentation links, active planning audit and diff
hygiene at one clean checkpoint.

Publication requires durable remote custody and a normal protected merge from
the exact P212 canonical baseline. GitHub CI is operator-disabled and is not
part of this packet's validation path. P213 closes after its exact source
enters canonical `main`; any input-capable executor or live fixture acceptance
remains a separate later packet.

## Source Acceptance | 2026-09-17

Checkpoint `1bad68e3` adds `validate_visual_round_intent` and the pure
`visual_desktop` module inside `agent-browser-challenge-control`. The validator
reuses the full policy and restored-snapshot invariants, requires the exact
`IntentAuthorized` phase and compares every public intent field with active
evidence and selection. The adapter then requires exact ordered candidate
identity, evidence, frame, context, geometry, observation-time and effective
expiry binding before delegating the constructed intent to P212.

The public tracer first failed on the absent P213 API, then returned one exact
effect-free permit from a state-machine-produced intent. The consolidated
mutation matrix proves typed rejection of changed phase, snapshot, intent,
candidate order or membership, digests, observation time, expiry, capability,
budget, geometry and controller authority. Review found that evidence could
outlive the challenge policy deadline; a focused red regression added the
effective expiry boundary. The accepted permit now expires at the earlier of
evidence expiry and policy deadline, and admission at that boundary returns
`StaleVisualIntent`.

Local acceptance passes:

- all 58 challenge-control tests, including six P213 adapter fixtures;
- all 12 desktop-services tests;
- the strengthened challenge-control architecture guard;
- workspace formatting and strict workspace Clippy; and
- documentation links, active planning audit, changed-surface selection,
  selector self-check and diff hygiene.

The selector expands to broad validation because the modified architecture
guard is conservatively classified as an unknown repository-tooling surface.
Its exact local recommendations all pass. P213 changes no manifest, lockfile,
CLI, Service State, capture, transport, executor or public schema. No browser,
provider, credential, CAPTCHA, route claim, desktop input, runtime, production
or CI effect occurred.

## Stop Condition

Stop and replan if the adapter requires a challenge-specific change in desktop
services, a new Cargo workspace member, CLI or Service State change, public
schema, coordinate heuristic, capture, transport, filesystem, network,
credential or event implementation. Stop before any browser launch, image
capture, provider call, CAPTCHA attempt, route claim, desktop input, installed
runtime mutation, shared Service State mutation, production effect or release.
