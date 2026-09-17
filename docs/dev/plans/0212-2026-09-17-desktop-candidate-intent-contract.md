# Plan 0212 | Desktop Candidate Intent Contract

Date: 2026-09-17

Plan version: 2

State: SOURCE_ACCEPTED

Consolidation: required

Product lane: PL-PLATFORM

Lane: P212

Related challenge plan: Plan 0187 W7-D

Work item: `CochranResearchGroup/agent-browser#127`; issue #66 retains every
browser, provider-backed, CAPTCHA and desktop-input acceptance effect

Branch: `platform/p212-desktop-candidate-intent-contract`

Target: `main`

Integrated prerequisite: P210 exact head `343a61b9`, merged through PR #192 as
`06972a5e17e83232201ea1e8f80267092f511f33`

Source baseline: `06972a5e17e83232201ea1e8f80267092f511f33`

## Objective

Add the provider-neutral desktop-services contract required to bind an exact
candidate-only intent to fresh physical-pixel geometry and current controller
authority before any input event is possible. The contract accepts no
challenge-specific type and emits no event. It freezes the platform seam that
a later challenge-owned adapter can consume without inventing a parallel
desktop authority model.

## Current State

P206 through P210 are integrated. The visual trunk can produce one bounded
P209 `VisualRoundIntent`, but `agent-browser-desktop-services` currently admits
only recipe-specific interactions whose provider returns one already-selected
candidate. It has no generic, effect-free contract for an exact ordered
candidate set, candidate geometry, source intent, effect budget and controller
authority.

P212 is admitted from clean canonical `main@06972a5e` in the reassigned P210
worktree; no new worktree was created. P211 is the active PL-PLATFORM writer
for CLI shutdown and workstation-install routing. Its source diff does not
touch `crates/agent-browser-desktop-services/` or the desktop-services
architecture guard. P211 remains primary writer for its CLI surfaces; P212 is
primary writer only for the candidate-intent module and fixtures. Both branches
will reconcile their shared roadmap, runbook and lane-catalog projections at
integration.

Source checkpoint `00f41715` implements the effect-free contract and fixture
matrix. All 12 desktop-services tests, the strengthened architecture guard,
formatting, strict workspace Clippy, documentation links and diff hygiene pass
locally. GitHub CI is operator-disabled and was not restored or dispatched.
Publication and protected integration remain.

## Architecture Boundary

The platform crate owns generic desktop geometry, controller authority and
input admission. It must not depend on the challenge-control or visual-adapter
crates. The challenge lane will later map a P209 intent into this platform
contract on a separate branch.

P212 adds these pure contracts:

- `DesktopCandidateGeometry`: one opaque candidate identity, physical-pixel
  bounds and a center point contained by those bounds;
- `DesktopCandidateObservation`: exact browser, display, stream, route,
  coordinate-space and geometry-epoch binding plus frame, context, geometry
  and ordered candidate-set digests, capture time, expiry and candidate list;
- `DesktopCandidateIntent`: one source-intent digest, exact observation,
  evidence and candidate-set binding, selected candidate identities,
  controller lease, provider capability digest, expiry and fixed step, pointer
  and key budgets;
- `DesktopCandidateEffectPermit`: selected candidate geometry, exact desktop
  binding, current authority digest, bounded counts and a deterministic permit
  digest; and
- `admit_desktop_candidate_intent`: one deterministic, effect-free decision
  function returning a permit or a typed intervention.

The new module may reuse existing `DesktopBinding`, `PixelBounds`,
`PixelPoint`, `ControllerAuthority` and digest conventions. It must not call
`run_desktop_interaction`, claim a route, execute an event, read Service State,
capture a frame, resolve a browser or persist state.

## Invariants

1. Every identifier and digest is nonempty and every digest has canonical
   SHA-256 shape.
2. Candidate identities are nonempty and unique. Their order and complete
   geometry bind the candidate-set digest.
3. Every candidate center is inside its nonempty bounds, and every bound is
   inside the declared physical desktop dimensions.
4. Observation coordinate space is exactly `desktop_physical_pixels`; browser,
   display, stream, route and geometry epoch are exact.
5. The intent evidence, frame, context, geometry and candidate-set digests
   match one fresh observation.
6. Selected candidates are nonempty, unique and contained in the bound set.
7. The current controller authority matches browser, display, stream, route,
   lease and viewer identities, has consistent nonzero epochs, remains
   controlling and unexpired, and exposes one matching non-manual machine-input
   provider on both route and stream.
8. Planned steps and event counts are nonzero only within explicit intent
   bounds; admission cannot add coordinates, events, retries or another
   attempt.
9. Observation, intent and authority validation completes before a permit is
   returned. Rejection emits no effect and returns no partial permit.
10. The permit digest binds every output field and is deterministic under
    exact replay.

## Consolidated Batch

1. Add one public-interface tracer proving a valid synthetic candidate intent
   returns an effect-free permit with exact geometry and authority.
2. Add the provider-neutral contracts and canonical digest helpers in a focused
   desktop-services module.
3. Add a fail-closed fixture matrix for identity, geometry, freshness,
   candidate-set, selection, budget and controller-authority mutations.
4. Extend the desktop-services architecture guard to prohibit challenge-crate,
   CLI, Service State, network, filesystem and runtime dependencies.
5. Run desktop-services tests, architecture, formatting, strict workspace
   Clippy, changed-surface selection and diff hygiene.

## Provider-Free Fixture Matrix

The minimum matrix proves:

1. one valid exact selection returns one deterministic permit and no effect;
2. candidate ordering, bounds and centers change the candidate-set digest;
3. duplicate, missing, out-of-set and reordered selection fails closed;
4. evidence, frame, context, geometry or candidate-set mismatch fails closed;
5. zero, overflowing or out-of-surface bounds and centers fail closed;
6. stale capture, expired observation and intent-expiry mismatch fail closed;
7. browser, display, stream, route, coordinate space or geometry epoch drift
   fails closed;
8. controller lease, viewer, role, state, route membership, stream membership,
   writability, provider or epoch drift fails closed;
9. zero or internally inconsistent effect budgets fail closed; and
10. exact replay returns the same permit without claiming a route or emitting
    input.

## Delivery Sequence And Budget

- Attempt 1: public tracer, contract vocabulary and valid permit.
- Attempt 2: consolidated fail-closed geometry and authority matrix with at
  most one packet-local semantic repair.
- Attempt 3: architecture guard, refactor and final local validation.
- Maximum work-unit attempts: 3.
- Maximum review and rework cycles: 1.
- Reassess after two checkpoints or 30 active minutes without outcome
  progress.
- Overall source effort ceiling: 180 active minutes.

## Worker Assignments

- Primary: this P212 session owns the candidate-intent module, provider-free
  fixtures, desktop-services architecture guard and bounded planning records.
- P211 remains primary writer for cold shutdown, workstation install and its
  CLI surfaces. P212 does not edit them.
- P205 remains primary writer for the service-model extraction. P212 does not
  edit Service State or the extracted service-model crate.
- No subagent, browser worker, provider worker, runtime operator, benchmark
  checkout or live-acceptance worker is assigned.

Expected source writes are limited to:

- `crates/agent-browser-desktop-services/src/candidate_intent.rs`;
- the smallest export change in
  `crates/agent-browser-desktop-services/src/lib.rs`;
- provider-free desktop-services fixtures;
- `scripts/test-desktop-services-crate-architecture.js`; and
- this plan plus P212's bounded roadmap, runbook and active-lane projections.

## Evidence And Exit

Source acceptance requires the complete provider-free matrix, the strengthened
desktop-services architecture guard, the full desktop-services crate tests,
workspace formatting, strict workspace Clippy, changed-surface selection and
diff hygiene at one clean source checkpoint.

Publication requires a branch based on `main@06972a5e`, durable remote custody
and a normal protected merge. GitHub CI is operator-disabled and is not part of
this packet's validation path. P212 closes after its exact source enters
canonical `main`; the later challenge consumer remains a separate packet.

## Source Acceptance | 2026-09-17

Checkpoint `00f41715` adds the pure `candidate_intent` module to
`agent-browser-desktop-services`. It binds one ordered candidate set and
selected ordered subset to canonical observation, evidence, frame, context,
geometry and candidate-set digests. Physical-pixel bounds and centers are
validated before current controller authority, exact route and stream
membership, matching non-manual machine-input provider and checked effect
budgets produce one deterministic permit. Permit construction claims no route
and emits no event.

The initial tracer moved from a missing public API to one exact effect-free
permit. The fail-closed matrix then exposed and repaired admission of reordered
selection and inconsistent or oversized budgets. Review exposed a further
binding gap: component digests alone did not prove the geometry epoch belonged
to the observation used by the intent. The accepted contract therefore binds
the exact observation digest, requires canonical lowercase SHA-256 text and
rejects blank optional profile and machine-input provider identities.

Local acceptance passes:

- all 12 desktop-services tests, including nine candidate-intent fixtures;
- the strengthened desktop-services architecture guard;
- workspace formatting and strict workspace Clippy;
- documentation links, changed-surface selection and diff hygiene; and
- the active planning and catalog audits.

The selector expands to broad validation because the modified architecture
guard is conservatively classified as an unknown repository-tooling surface.
The source change remains limited to desktop services and its provider-free
guard; no CLI, Service State, challenge crate, browser, capture, provider,
credential, CAPTCHA, desktop-input, runtime, production or CI effect occurred.

## Stop Condition

Stop and replan if the platform contract needs a challenge-specific type,
recipe-specific CAPTCHA behavior, CLI or Service State change, capture
implementation, coordinate translation heuristic, event construction, route
claim, filesystem, network, credential or public schema. Stop before any
browser launch, image capture, provider call, CAPTCHA attempt, desktop input,
installed-runtime mutation, shared Service State mutation, production effect
or release.
