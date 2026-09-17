# Plan 0209 | Visual Provider Protocol

Date: 2026-09-16

Plan version: 1

State: OPEN

Consolidation: required

Product lane: PL-CHALLENGE

Lane: P209

Parent plan: Plan 0187 W7-B

Work item: `CochranResearchGroup/agent-browser#127`; issue #66 retains every
live Turnstile or hCaptcha acceptance effect

Branch: `challenge/p209-visual-provider-protocol`

Target: `main`

Dependency: P206 published head
`5da7d37d30ed15a1f7970e11e9581f3ad04e6599`

Source baseline: `5da7d37d30ed15a1f7970e11e9581f3ad04e6599`

Publication hold: do not push, start CI or open a pull request until issue #174
lands through merged PR #179

## Objective

Add the provider-neutral protocol between P206's visual-round evidence contract
and a future visual-reasoning adapter. One pure module prepares a canonically
bound request and adjudicates one typed provider response. A provider may
select only candidate identities from the bound evidence or abstain with a
typed ambiguous, unsupported or inconclusive result.

The packet uses repository-owned fake providers and serialized fixtures only.
It does not call a model, include image bytes, resolve an artifact, translate a
candidate to coordinates, emit desktop input, expose a Service action, launch a
browser, attempt a CAPTCHA or mutate any runtime.

## Current State

P206 W7-A is source-complete and acceptance-complete at `ac9f50a7`, reconciled
with canonical P197 integration at `5da7d37d`, and held in draft PR #180 while
the operator-directed CI change lands. It defines fresh evidence, exact
candidate sets, provider capability identity, round selection, effect receipt,
after-state continuity, cumulative budgets and zero-effect replay.

No type yet defines what a visual provider is allowed to receive or return.
Without that seam, each future adapter could invent its own request digest,
return coordinates or event instructions, omit capability identity, or silently
reinterpret ambiguity as a selection. P209 closes only that provider-free
protocol gap.

P204 owns CI validation tiering, P205 owns Service-model extraction, P207 owns
tab-handle refresh custody and P208 owns worktree closeout. P209 edits only the
challenge-control crate, its provider fixtures and the bounded challenge-lane
planning projections. It remains local during the CI hold.

## Contract

The protocol interface consists of two pure functions:

```rust
prepare_visual_provider_request(policy, evidence, prepared_artifact)
adjudicate_visual_provider_response(policy, evidence, request, response, now_ms)
```

The prepared request binds the challenge task and attempt, monotonic round,
policy and profile revisions, evidence, frame, context, geometry, candidate
set, exact ordered candidate identities, prepared-artifact identity and digest,
provider capability, request time and expiry. Its canonical request digest
covers every field.

The response binds that request digest, evidence digest, candidate-set digest,
provider capability and one typed disposition:

- select one or more bound candidate identities;
- ambiguous;
- unsupported; or
- inconclusive.

The response has its own canonical digest. Unknown fields fail deserialization.
There is no coordinate, selector, pixel, event-sequence, executable, credential,
retry, replacement-capability or free-form instruction field.

Adjudication returns either a P206 `VisualRoundSelection` ready for the existing
round transition or a typed intervention reason. It emits no effect, creates no
attempt, performs no retry and does not decide challenge completion.

## Consolidated Batch

1. Freeze request, prepared-artifact, response and disposition types with
   canonical length-prefixed SHA-256 digests.
2. Implement one pure prepare and adjudicate interface over the existing P206
   policy, evidence, selection and intervention types.
3. Prove a valid fake-provider selection crosses the existing P206 seam and
   authorizes exactly one bound round intent.
4. Prove stale, mutated, mismatched, ambiguous, unsupported, inconclusive,
   duplicate, out-of-set and over-budget responses fail closed without intent.
5. Prove serialized coordinate, event-sequence and retry smuggling is rejected
   as unknown protocol input.
6. Run only focused local validation during the CI hold. Preserve full
   publication and forge validation for after PR #179 merges.

Deferred to separately admitted successors:

- any model SDK, HTTP client, provider credential or provider call;
- image capture, encoding, redaction, storage or artifact resolution;
- prompt construction or provider-specific response parsing;
- candidate-to-coordinate translation or desktop-services integration;
- Service State, CLI, HTTP, MCP, generated-client or dashboard exposure;
- browser, fixture-provider, installed-runtime or live challenge acceptance.

## Delivery Sequence And Budget

- Attempt 1: one valid fake-provider tracer through request preparation,
  response adjudication and the P206 round interface.
- Attempt 2: one consolidated fail-closed fixture matrix and at most one
  packet-local semantic correction.
- Attempt 3: refactor for depth and run focused local validation.
- Maximum work-unit attempts: 3.
- Maximum review and rework cycles: 1.
- Reassess after two checkpoints or 30 active minutes without outcome progress.
- Overall effort ceiling: 180 active minutes through a clean local checkpoint.

Publication, forge CI and protected integration time are excluded from the
local implementation ceiling because the operator has explicitly held those
actions on merged PR #179.

## Worker Assignments

- Primary: owns the protocol design, implementation, fake fixtures, local
  validation and later P206 reconciliation.
- No subagent, model provider, browser worker, runtime operator or benchmark
  checkout is assigned.
- P204 remains the sole writer for CI workflow, selector and testing-policy
  surfaces; P209 does not edit them.

## Evidence And Exit

The minimum provider-free matrix must prove:

1. deterministic preparation binds every request field and exact candidate
   order into one digest;
2. one valid fake-provider response produces a P206 selection and exactly one
   bound intent without emitting an effect;
3. request, evidence, candidate-set, capability or response-digest mutation
   fails before selection;
4. stale request or response evidence fails before selection;
5. duplicate, missing, out-of-set or over-budget candidate identities fail
   before intent;
6. ambiguous, unsupported and inconclusive responses produce their matching
   typed intervention without a selection;
7. unknown coordinate, event-sequence and retry fields fail strict
   deserialization; and
8. replaying the same valid input is deterministic and effect-free.

Local exit requires the focused challenge-control tests, crate architecture
guard, formatting, strict local Clippy and diff hygiene at one clean commit.
Publication exit additionally requires merged PR #179, P206 integration into
`main`, canonical reconciliation, a published branch and the then-current
protected validation path.

## Stop Condition

Stop before any push or workflow trigger while PR #179 is unmerged. Also stop
before any browser launch, image capture, model or provider call, credential
use, desktop input, CAPTCHA attempt, retry, Service State mutation, installed
runtime action, production effect or release. Replan if the pure protocol needs
a public Service schema, provider-specific parser, image payload, coordinate or
event authority, or a second challenge attempt.
