# Plan 0206 | Visual And Multi-Round Challenge Contract

Date: 2026-09-16

Plan version: 1

State: OPEN

Consolidation: required

Product lane: PL-CHALLENGE

Lane: P206

Parent plan: Plan 0187 W7-A

Work item: `CochranResearchGroup/agent-browser#127`; issue #66 retains every
live Turnstile or hCaptcha acceptance effect

Branch: `challenge/p206-visual-round-contract`

Target: `main`

Dependency: P197 published head
`cd22a39f989e29a27d6ec80283ae60667093fc9c`

Source baseline: `cd22a39f989e29a27d6ec80283ae60667093fc9c`

Integration: P197 must enter `main` before P206 rebases or merges current main
and enters the protected integration path

## Objective

Extend the provider-neutral challenge trunk with a pure evidence and capability
contract for visual, multi-round challenges. Multiple rounds remain subordinate
to one policy-authorized challenge attempt. Every provider selection binds to
one fresh synthetic observation and an exact candidate set, and every round
consumes both per-round and cumulative budgets without renewing attempt
authority.

The packet proves the contract entirely with repository-owned synthetic
fixtures. It does not implement a visual solver, call a model, expose a public
Service action, translate candidates to desktop coordinates, emit input, or
attempt a CAPTCHA.

## Current State

Plan 0187 W0 through W5 are integrated. P197 W6 is source-complete and
published at `cd22a39f` through draft PR #157. The existing pure
`agent-browser-challenge-control` crate owns one attempt, step and event
budgets, cooldown, deadline, verification, intervention and zero-effect replay.
It does not yet represent round identity, fresh visual evidence, candidate-set
binding, per-round budgets or cumulative multi-round accounting.

P204 owns CI validation tiering and P205 owns Service-model extraction. P206 is
source-disjoint from both: it owns only the pure challenge-control crate, its
provider-free tests, this plan and the bounded challenge-lane projections.
P206 depends on P197 because both touch the challenge-control crate; this branch
starts from P197's exact published head rather than recreating or cherry-picking
that contract.

## Contract Boundary

The round contract is subordinate to the existing `ChallengeSnapshot` attempt
lifecycle. It may authorize a registered round intent and return an aggregate
effect summary to the existing `EffectFinished` and `Verified` transitions. It
must not create a second challenge lifecycle, start another attempt, change
Service State, or emit an effect.

One round evidence envelope binds:

- challenge task, attempt, profile and policy digests;
- a monotonic round index and deterministic round identity;
- fresh frame, context and geometry digests plus observation and expiry times;
- the exact ordered candidate-identity set and its digest; and
- the provider capability identity, version and digest.

One provider selection may name only candidate identities from that evidence.
It cannot provide pixels, coordinates, selectors, event sequences, executable
names, credentials, retry instructions or replacement authority. Duplicate,
missing, reordered, stale, mismatched or out-of-set identities fail closed.

The policy declares a single-attempt maximum plus explicit maximum rounds,
selections, steps, pointer events and key events for each round and for the
whole attempt. A new round consumes the existing cumulative allowance. It does
not reset counters, deadlines, cooldown or replay identity.

After each acknowledged round, a fresh after-state classification returns one
of passed, denied, next round, ambiguous, unsupported or inconclusive. Partial
or uncertain delivery, stale or changed evidence, authority mismatch,
unexpected extra rounds, ambiguity and any exhausted budget produce typed
human intervention without another effect.

## Consolidated Batch

1. Freeze typed round policy, evidence, candidate selection, after-state,
   aggregate counters, decision and receipt contracts in the pure crate.
2. Implement deterministic validation and transition logic subordinate to one
   existing challenge attempt.
3. Prove exact replay emits no new round intent or effect and returns the prior
   terminal receipt identity.
4. Add one table-driven synthetic matrix covering the safe continuation and
   fail-closed boundaries.
5. Run focused crate tests, the crate architecture guard, formatting, strict
   workspace Clippy and changed-surface selection once on the completed batch.

Deferred to separately admitted successors:

- any visual-reasoning provider or model call;
- image, OCR, accessibility or network transport;
- candidate-to-coordinate translation and desktop-service integration;
- Service State, CLI, HTTP, MCP, generated-client or dashboard exposure;
- browser, fixture-provider, installed-runtime or live challenge acceptance;
- W8 portability or an external host.

## Delivery Sequence And Budget

- Attempt 1: red-capable contract fixtures plus the smallest coherent pure
  implementation.
- Attempt 2: complete the matrix and repair at most one packet-local semantic
  defect exposed by focused tests.
- Attempt 3: changed-surface validation and at most one integration correction.
- Maximum work-unit attempts: 3.
- Maximum review and rework cycles: 1.
- Reassess after two checkpoints or 30 active minutes without outcome progress.
- Overall effort ceiling: 240 active minutes through a published source
  checkpoint. Protected integration time is reported separately because P197
  must merge first.

No subagent, browser worker, model provider, runtime operator or benchmark
checkout is assigned.

## Worker Assignments

- Primary: owns contract design, implementation, fixtures, validation and
  reconciliation.
- P204: independent CI-policy writer; P206 does not edit CI workflow, selector
  or testing-policy surfaces.
- P205: independent Service-model extraction writer; P206 does not edit CLI
  Service models or adapters.
- P197: dependency provider; P206 preserves its published history and integrates
  only after P197 lands.

## Evidence And Exit

The minimum provider-free matrix must prove:

1. a deterministic two-round synthetic challenge passes inside one attempt;
2. the second round inherits cumulative counters and cannot renew authority;
3. candidate ambiguity or a selection outside the bound candidate set stops
   before an intent is authorized;
4. stale evidence, changed geometry, changed frame or changed candidate set
   stops before an intent is authorized;
5. skipped, repeated, reordered or unexpected extra rounds fail closed;
6. per-round and cumulative selection, step, pointer and key budgets are
   enforced independently;
7. partial or uncertain effect classification requires intervention and cannot
   continue to another round;
8. provider capability or policy mismatch fails closed;
9. exact replay returns the prior decision and emits no new intent or effect;
10. checkbox profiles retain their existing one-round behavior and one-attempt
    ceiling.

Exit also requires the focused challenge-control tests, challenge-control
architecture guard, repository formatting, strict workspace Clippy, diff
hygiene and validation-selector readback to pass on one exact clean checkpoint.
The branch must be published with its dependency and effect boundary visible.

## Stop Condition

Stop before any browser launch, image or private pixel capture, model-provider
call, desktop input, CAPTCHA attempt, credential use, retry, Service State
mutation, installed-runtime action, production effect or release. Replan rather
than expanding this packet if the pure contract requires a public schema,
desktop-services change, Service-model edit, second challenge lifecycle or more
than one policy-authorized attempt.
