# Plan 0198 | Retained Owner Inventory Coherence

Date: 2026-09-16

Plan version: 1

State: OPEN

Consolidation: required

Product lane: PL-BUGFIX

Lane: P198

Work item: `CochranResearchGroup/agent-browser#143`

Branch: `fix/p198-retained-owner-coherence`

Target: `main`

Integration: merge through the protected `main` workflow after provider-free regression and changed-surface validation

Source baseline: `faee8887fb420e8e46bd5a165ce6c44fa10059d2`

## Objective

Make retained managed-profile launch planning and the read-only profile,
session, and browser inventory projections derive one coherent lifecycle-owner
classification. A dead recorded browser with retained session evidence must
either be safely reusable or return exact conflicting owner evidence and
deterministic recourse. Read-only inventory failures must always report no
effect.

## Current State

Source checkpoint `21ec94f3bfc8f2db86d4b275691cf5fb15fd67d0` repairs the
provider-free defect. The minimized fixture proved that profiles and sessions
were readable while browser inventory applied a current-process PID validator
to structurally valid historical owner evidence for a terminal browser. The
repair separates record validity from current process authority: terminal
history remains visible, while live or nonterminal rows still require the exact
PID binding. Launch planning remains fail-closed with
`existing_session_profile_identity_unproven`, `no_effect`, and executable
`service_profile_recovery_plan` recourse when current owner identity is absent.
Read-only profile, session, and browser failures now retain their exact code but
are action-contextually classified as `no_effect`.

The red-capable cross-collection fixture, typed launch-conflict fixture,
read-only terminal-outcome fixture, existing forged-observation guard, and the
99-test `service_health` selection pass. Workspace formatting and strict Clippy
pass. No browser, profile, provider, installation, Service State, shared
runtime, production, or tenant effect occurred. Protected integration and CI
remain.

P190 owns candidate and workstation promotion surfaces. P197 owns challenge
consumer admission. P198 owns retained managed-profile lifecycle classification
and the three read-only inventory projections. It will not edit candidate,
installer, challenge-control, authentication, navigation, shared request-schema,
or public documentation surfaces unless diagnosis proves an unavoidable
dependency and the overlap is reconciled first.

## Consolidated Batch

1. Add one fast provider-free fixture containing an available managed profile,
   a dead recorded browser process, retained session evidence, and an unrelated
   active session. Make it assert the exact disagreement across launch planning
   and all three inventory projections.
2. Trace the lifecycle-owner evidence and classification path used by launch,
   profile inventory, session inventory, and browser inventory. Rank and test
   competing root-cause hypotheses before changing behavior.
3. Repair the narrowest shared classification or projection seam so identical
   evidence produces one result and exact recourse. Preserve unrelated active
   sessions and all fencing invariants.
4. Make every read-only inventory error explicitly `no_effect`; do not imply a
   browser, provider, profile, or tenant mutation may have occurred.
5. Run focused tests during repair, then the complete changed-surface gates once
   against the frozen batch. Publish, review, integrate, and close only after
   the exact source checkpoint and CI are green.

## Scope And Effect Boundary

Expected write surfaces are the existing Service lifecycle-owner and inventory
adapters under `cli/src/native/`, their provider-free fixtures, and the narrow
contract or output metadata that owns read-only effect classification if the
reproducer proves it is incorrect. This plan and the active-lane projection are
also in scope.

This plan does not authorize Chrome launch, profile mutation, broad cleanup,
credential use, provider access, Service State migration, installation,
supervisor or shared-runtime mutation, production effect, or release. It must
not edit the Service State persistence and lock algorithm owned by issue #76.

## Delivery Sequence And Budget

- Attempt 1: red-capable fixture plus minimized evidence classification.
- Attempt 2: one shared-seam repair and focused regression validation.
- Attempt 3: one bounded correction if changed-surface validation reveals a
  defect introduced by this packet.
- Maximum work-unit attempts: 3.
- Maximum review and rework cycles: 1.
- Reassess after two checkpoints or 30 active minutes without outcome progress.
- Overall effort ceiling: 240 active minutes through protected integration.

No subagent, browser worker, runtime operator, benchmark checkout, or provider
operator is assigned.

## Worker Assignments

The P198 lane owner holds the critical path, plan, branch, fixture, source
repair, validation, integration, and closeout. P190 remains primary writer for
candidate and workstation promotion. P197 remains primary writer for challenge
consumer integration. No parallel writer is assigned within P198.

## Evidence And Exit

Exit requires current evidence that:

- the minimized fixture is red-capable for the original disagreement and green
  after the repair;
- profiles, sessions, browsers, and launch planning classify the same retained
  evidence coherently;
- a reusable dead-browser state proceeds, or a conflict reports the exact owner
  evidence and deterministic recourse;
- every read-only inventory failure is classified as no effect;
- the unrelated active session is unchanged;
- no broad cleanup, profile deletion, force recovery, live retry, browser
  launch, provider call, or runtime mutation occurred;
- focused regression tests, formatting, strict workspace Clippy, and every
  additional check selected from the complete changed surface pass; and
- the published source checkpoint enters `main` through the linked pull
  request with applicable CI green.

## Stop Condition

Stop and record a dependency before editing Service State persistence or lock
coordination, P190-owned candidate and installer surfaces, P197-owned challenge
consumer surfaces, or any live runtime. Stop rather than weakening owner proof,
fencing, unrelated-session isolation, or the no-effect contract.
