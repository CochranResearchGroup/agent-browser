# Plan 0206 | Visual And Multi-Round Challenge Contract

Date: 2026-09-16

Plan version: 7

State: SOURCE ACCEPTED | INTEGRATION READY

Consolidation: required

Product lane: PL-CHALLENGE

Lane: P206

Parent plan: Plan 0187 W7-A

Work item: `CochranResearchGroup/agent-browser#127`; issue #66 retains every
live Turnstile or hCaptcha acceptance effect

Branch: `challenge/p206-visual-round-contract`

Target: `main`

Integrated prerequisite: P197 published head
`cd22a39f989e29a27d6ec80283ae60667093fc9c`, merged as `c855fc33`

Source baseline: `cd22a39f989e29a27d6ec80283ae60667093fc9c`

Integration: P206 joined canonical `main@59928044` at merge checkpoint
`db987e4e`; publication and protected integration remain. GitHub CI is
operator-disabled and must not be restored or dispatched for this packet.

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

Plan 0187 W0 through W6 are integrated. P197 head `cd22a39f` passed every
ordinary required forge check and merged through PR #157 as `c855fc33`. P206
joined that canonical checkpoint at tree-preserving merge `5d6e3d57` and is
source-complete and acceptance-complete at `ac9f50a7`. The pure
`agent-browser-challenge-control` crate now owns deterministic round identity,
fresh visual evidence, exact candidate-set and intent binding, per-round plus
cumulative budgets, acknowledged-effect custody, after-state continuity and
zero-effect terminal replay inside one attempt.

P204 CI validation tiering merged through PR #179 as `f5e3f31b`. Operator
direction subsequently disabled the GitHub CI and Lease Authority workflows
through PRs #185 and #186. P208 then merged through PR #182 as canonical
`main@59928044`. P206 joined those checkpoints at `bb961c96` and `db987e4e`,
resolving only the shared runbook projection. P205 owns Service-model
extraction and remains source-disjoint.
P206 owns only the pure challenge-control crate, its provider-free tests, this
plan and the bounded challenge-lane projections. P206 started from P197's exact
published head rather than recreating or cherry-picking that contract. The
canonical reconciliations do not alter the accepted challenge-control source.
Because GitHub CI is intentionally disabled, no new forge run is expected or
authorized. Publication and protected P206 integration remain.

## Source Checkpoint

Validated source checkpoint `ac9f50a7` contains implementation commit
`5a98dbfc` and a 23-case synthetic `visual_round` integration matrix. Review
hardening closed two persisted state defects before checkpointing: an
after-state can no longer be classified without a recorded effect receipt, and
a restored selection cannot escape its evidence-bound candidate set even if
its intent digest is recomputed. The final matrix also directly proves
duplicate candidate ambiguity, repeated and reordered round rejection, policy
digest mismatch, and the existing checkbox profiles' one-attempt ceiling.

Validation at that source checkpoint:

- `scripts/ci/cargo-safe.sh test --manifest-path Cargo.toml -p
  agent-browser-challenge-control -- --nocapture`: 39 passed;
- `node scripts/test-challenge-control-crate-architecture.js`: passed;
- `scripts/ci/cargo-safe.sh fmt --all --manifest-path Cargo.toml -- --check`:
  passed;
- `AGENT_BROWSER_CARGO_BUILD_JOBS=4 AGENT_BROWSER_CARGO_CACHE=off
  scripts/ci/cargo-safe.sh clippy --workspace --manifest-path Cargo.toml -- -D
  warnings`: passed after one host-process-pressure retry that did not reach
  project linting;
- `git diff --check`: passed; and
- `pnpm validation:select -- --base 46412f43`: read back the Rust quality gate
  plus shared-lockfile-triggered extracted-crate suggestions. No extracted
  crate source changed in this packet.

No browser, CAPTCHA, model provider, credential, desktop input, Service State,
installed runtime, production or CI-policy effect occurred. P197 integration
and canonical P206 reconciliation are complete.

## Canonical Reconciliation | 2026-09-17

Merge checkpoint `89bdfdbf` joins `main@f5e3f31b`, including the merged P204
path-selected validation machinery. The reconciled exact-head local gate set
passed:

- all 39 challenge-control tests, including the 23-case visual-round matrix;
- strict workspace Clippy and workspace formatting;
- challenge-control, CDP, Lease Authority and desktop-services architecture
  guards;
- all 3 CDP and 3 desktop-services tests; and
- all 108 Lease Authority tests on the authoritative isolated rerun.

The first selector-expanded Lease Authority run observed one stale-process
result in `browser_launch_completion_derives_an_exact_direct_child_process_identity`
while running alongside another Cargo claim. The exact test then passed 20 of
20 isolated executions and the complete 108-test package rerun passed. No P206
source or Lease Authority source changed in response; the failed sample remains
diagnostic evidence rather than P206 invalidation.

## Integration Review Repair | 2026-09-17

Pre-merge self-check found that cumulative `u8` budget comparisons used
saturating addition. With a maximum cumulative selection budget of 255, a
restored total of 250 plus a valid 10-selection round saturated to 255 and was
incorrectly admitted. A response selecting 256 candidates also returned the
generic `InvalidTransition` error before reaching the typed per-round budget
decision.

Checkpoint `4813d385` widens selection-count comparisons to `usize`, uses
checked addition for every cumulative budget axis, and uses checked addition
when recording completed effect counts. The two minimal regressions first
failed with an admitted intent and `InvalidTransition`, respectively, then
passed with `CumulativeBudgetExceeded` and `RoundBudgetExceeded` after the
repair.

The repaired checkpoint passes all 41 challenge-control tests, including 25
visual-round cases, the challenge-control architecture guard, workspace
formatting, strict workspace Clippy and diff hygiene. The prior CDP, Lease
Authority and desktop-services evidence remains reusable because the repair
changes only challenge-control arithmetic and its fixture. The corrected
source was published at `72eeea03`. GitHub CI was then disabled by explicit
operator direction, so the superseded old-head run is not exact-head evidence
and no replacement run is expected or authorized.

## Current-Main Reconciliation | 2026-09-17

Merge checkpoint `db987e4e` joins `main@59928044`, including the CI shutdown
and P208 worktree-closeout transaction. Neither canonical slice changes the
challenge-control crate, workspace Cargo metadata, or the repaired P206
fixtures. The exact repaired Rust source therefore retains the accepted 41-test
package, architecture, formatting, strict Clippy and diff-hygiene evidence.
Conflict-affected repository controls pass locally: policy wiring,
documentation links, validation-selection fixtures, P208 closeout fixtures,
the active planning audit and merge-result diff hygiene. No workflow was
restored, dispatched, retried or otherwise run.

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
