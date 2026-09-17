# Plan 0209 | Visual Provider Protocol

Date: 2026-09-16

Plan version: 10

State: SOURCE ACCEPTED | P206 INTEGRATION PENDING

Consolidation: required

Product lane: PL-CHALLENGE

Lane: P209

Parent plan: Plan 0187 W7-B

Work item: `CochranResearchGroup/agent-browser#127`; issue #66 retains every
live Turnstile or hCaptcha acceptance effect

Branch: `challenge/p209-visual-provider-protocol`

Target: `main`

Dependency: P206 published head
`72eeea031d323901bac289d433b2e8b7c09b5fd7` through PR #180

Source baseline: `5da7d37d30ed15a1f7970e11e9581f3ad04e6599`

Local reconciliation: merge `9df85b9b` joins corrected published P206 head
`72eeea03`

Publication gate: do not push or open a pull request until P206 PR #180 enters
`main` and P209 reconciles that canonical checkpoint

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

P206 W7-A is source-complete and acceptance-complete, reconciled with current
`main@f5e3f31b`, and published at corrected head `72eeea03` for exact-head
evaluation in PR #180. Pre-merge review repaired cumulative budget saturation
and typed classification for an unrepresentable selection count. P206 defines
fresh evidence, exact candidate sets, provider capability identity, round
selection, effect receipt, after-state continuity, checked cumulative budgets
and zero-effect replay.

No type yet defines what a visual provider is allowed to receive or return.
Without that seam, each future adapter could invent its own request digest,
return coordinates or event instructions, omit capability identity, or silently
reinterpret ambiguity as a selection. P209 closes only that provider-free
protocol gap.

P204 owns CI validation tiering, P205 owns Service-model extraction, P207 owns
tab-handle refresh custody and P208 owns worktree closeout. P209 edits only the
challenge-control crate, its provider fixtures and the bounded challenge-lane
planning projections. P204 merged through PR #179. P209 has locally joined the
corrected published P206 head at `9df85b9b` but remains unpushed until P206
enters canonical `main`.

## Contract

The protocol interface consists of two pure functions:

```rust
prepare_visual_provider_request(policy, evidence, prepared_artifact, execution_plan)
adjudicate_visual_provider_response(policy, evidence, request, response, now_ms)
```

The prepared request binds the challenge task and attempt, monotonic round,
policy and profile revisions, evidence, frame, context, geometry, candidate
set, exact ordered candidate identities, prepared-artifact identity and digest,
a caller-owned, policy-checked execution budget, provider capability, request
time and expiry. Its canonical request digest covers every field. The provider
cannot set execution cost or cause it to be derived from selected candidate
count.

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
6. Reuse the accepted focused local validation while P206 integration is
   pending. Run exact-head forge validation after canonical reconciliation.

Deferred to separately admitted successors:

- any model SDK, HTTP client, provider credential or provider call;
- image capture, encoding, redaction, storage or artifact resolution;
- prompt construction or provider-specific response parsing;
- candidate-to-coordinate translation or desktop-services integration;
- Service State, CLI, HTTP, MCP, generated-client or dashboard exposure;
- browser, fixture-provider, installed-runtime or live challenge acceptance.

[Plan 0210](0210-2026-09-17-visual-artifact-and-provider-invocation-adapter.md)
is the planned W7-C successor. It is not admitted and cannot begin source work
until P209 enters canonical `main`.

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
local implementation ceiling because they follow the P206 dependency join.

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
Publication exit additionally requires P206 integration into `main`, canonical
reconciliation, a published branch and the then-current protected validation
path.

## Local Acceptance | 2026-09-16

Source checkpoint `4be65f06` implements the pure provider protocol and its
nine-case fake serialized fixture matrix. The request digest binds every
request field other than the digest itself, including exact candidate order
and prepared-artifact identity. It also binds a caller-owned execution budget
before provider adjudication, preventing provider candidate output from
inventing or implicitly determining pointer-event cost. Response adjudication
accepts only selected candidate identities or typed ambiguous, unsupported and
inconclusive abstention. Request, response, evidence, capability, freshness and
budget violations stop before selection or intent, while coordinate,
event-sequence, retry and free-form instruction fields fail strict
deserialization.

Local acceptance passed:

- all 48 challenge-control crate tests, including 9 provider-protocol and 23
  visual-round cases;
- strict workspace Clippy and workspace formatting;
- challenge-control, CDP, Lease Authority and desktop-services architecture
  guards;
- 3 CDP, 108 Lease Authority and 3 desktop-services tests selected because
  the dev-only JSON fixture dependency updated the shared lockfile; and
- diff hygiene and changed-surface validation selection.

No browser, image, provider, credential, CAPTCHA, desktop-input, runtime or
production effect was performed. The branch remains local and unpushed. CI and
publication wait for P206 PR #180 to enter `main` and P209 to reconcile the
canonical result.

## Published-Dependency Reconciliation | 2026-09-17

Local merge `24aee872` joins published P206 head `99793061`, including current
`main@f5e3f31b` and its path-selected validation machinery. Documentation
checkpoint `8ada33d5` removes the superseded PR #179 CI hold while preserving
the P206 integration gate. At that exact checkpoint, all 48 challenge-control
tests, the challenge-control architecture guard, workspace formatting and
strict workspace Clippy pass. The previously accepted CDP, Lease Authority and
desktop-services gates remain reusable because P209 changes none of their
source or dependency closure.

The branch remains local and unpushed until PR #180 enters `main`. That is an
integration-order boundary, not a stop on local development or validation.

## Review Hardening | 2026-09-17

Source review found no protocol defect. Checkpoint `3aed9a4b` closes one
explicit coverage gap by proving that serialized requests reject top-level
coordinate, event-sequence, retry and instruction fields as well as nested
artifact bytes and execution-plan repeat authority. This complements the
existing response-smuggling matrix and makes strict request and response
deserialization independently visible in the fixture suite.

All 49 challenge-control tests, workspace formatting, strict workspace Clippy
and diff hygiene pass at this checkpoint. No provider, image, browser,
credential, CAPTCHA, desktop-input, runtime or production effect occurred.

Checkpoint `e7250217` then consolidates every response-time boundary into the
existing stale-response fixture. It proves rejection when provider output
predates the request, claims a future production time, is produced at its own
expiry, expires before adjudication, extends beyond request expiry, or is
adjudicated at request expiry. The complete 49-test crate and strict workspace
Clippy remain green.

Checkpoint `89edafd5` completes request-preparation guard coverage in the
existing execution-budget fixture. Invalid policy, mutated evidence, malformed
artifact identity or digest, pre-observation preparation, expired evidence and
over-budget execution plans all fail before a provider request exists. The
complete 49-test crate, formatting, strict workspace Clippy and diff hygiene
remain green.

Checkpoint `f9987721` adds the distinct response-integrity invariant. The
response digest changes for request, evidence, candidate-set, capability,
selected-candidate order, production-time and expiry mutations. All 50
challenge-control tests, formatting, strict workspace Clippy and diff hygiene
pass.

## Corrected P206 Reconciliation | 2026-09-17

Local merge `9df85b9b` joins corrected published P206 head `72eeea03`, including
its checked cumulative budget arithmetic and two boundary regressions. The
combined dependency head passes all 52 challenge-control tests, including 25
visual-round and 11 provider-protocol cases, the challenge-control architecture
guard, workspace formatting, strict workspace Clippy and diff hygiene. The
prior unaffected extracted-crate evidence remains reusable.

P209 remains local and unpushed until corrected P206 enters canonical `main`.

## Stop Condition

Stop before any push or workflow trigger until P206 PR #180 merges and P209
reconciles that canonical checkpoint. Also stop before any browser launch,
image capture, model or provider call, credential use, desktop input, CAPTCHA
attempt, retry, Service State mutation, installed runtime action, production
effect or release. Replan if the pure protocol needs a public Service schema,
provider-specific parser, image payload, coordinate or event authority, or a
second challenge attempt.
