# Plan 0193 | Challenge Task Service Orchestration

Date: 2026-09-15

State: OPEN

Consolidation: required

Product lane: PL-CHALLENGE

Lane: P169

Parent plan: Plan 0187 W5

Work item: `CochranResearchGroup/agent-browser#127`; issue #66 remains the
separately live-gated challenge acceptance leaf

Branch: `challenge/p169-task-orchestration`

Target: `main`

Integration: merge through the protected `main` workflow after provider-free W5 validation

## Objective

Add one durable, challenge-aware Service task that owns observation, policy
decision, one bounded resolution intent, verification, cooldown, and downstream
admission. Return one attributable receipt that keeps effect delivery,
challenge completion, and downstream admission distinct.

## Current State

W5 is admitted for one provider-free implementation packet from merged-main
checkpoint `85b4ee92f5f3703e70040a89aaef54175f073940`. The primary worktree is
clean on `challenge/p169-task-orchestration`. No W5 source change, build,
browser, challenge, provider, credential, installation, shared-runtime,
production, or release effect has occurred.

## Current Baseline

The packet starts from merged `main` checkpoint
`85b4ee92f5f3703e70040a89aaef54175f073940`. W0 through W4 are integrated.
The pure challenge-control crate owns provider-neutral decisions and registered
Turnstile and hCaptcha profiles. The W4 `challenge_control_evaluate` action is
an intentionally effect-free scenario evaluator, not a task orchestrator.

The existing Authentication Run supplies the repository pattern for durable
task identity, exact tab-handle binding, idempotency, transition budgets,
pending-effect custody, caller ownership, and status projection. W5 may reuse
those Service seams, but it must not couple challenge state to authentication
provider fields or create a second Service State repository.

## Consolidated Batch

1. Add a pure challenge-task state machine and receipt contract above the W3
   decision engine.
2. Add durable Service task records with exact authority binding, request and
   idempotency digests, deadline, transition budget, and pending-effect custody.
3. Add start, status, resume, and cancel request actions that return the same
   task projection and reject caller or operation replay.
4. Drive the full lifecycle through registered provider-free fixture adapters
   before connecting any browser or desktop effect.
5. Project active, terminal, cooldown, and intervention summaries through
   Service Status and Service resources without exposing low-level desktop
   primitives or private evidence.

## Public Contract

The task start request accepts only:

- a repository-owned challenge profile ID;
- an exact Service tab handle and caller principal binding;
- a site-policy digest and registered downstream-intent ID;
- an idempotency key, deadline, and bounded transition count; and
- a repository-owned provider-free fixture scenario during this packet.

It does not accept pixels, coordinates, selectors, detector thresholds,
executables, process IDs, credentials, arbitrary event sequences, provider
controls, retry counts, or caller-selected terminal outcomes.

The task receipt includes task identity and binding, lifecycle phase, attempt
budget, observation and decision summaries, delivery state, verification state,
cooldown state, intervention state, downstream admission, transition receipts,
effect classification, and replay status. Durable projections contain digests
and allowlisted metadata only.

## State And Effect Model

The initial state sequence is:

```text
ready
  -> observing
  -> deciding
  -> resolving | verifying | cooling_down | intervention_required
  -> verifying | cooling_down | intervention_required
  -> admitted | not_admitted | intervention_required | cancelled
```

Every mutation consumes one unique operation ID. Resolution is reserved before
delivery and committed only from an attributable receipt. At most one automated
attempt may be started. Verification consumes a fresh observation and is
independent of delivery acknowledgement. Downstream admission is explicit and
occurs only after the configured terminal challenge policy allows it.

The provider-free driver emits no browser, provider, network, or desktop-input
effect. It exercises the same orchestration boundaries with registered fixture
receipts and reports `emittedEffects=false`.

## Scope And Safety Boundary

This plan authorizes repository source, contracts, documentation, and
provider-free validation only. It does not authorize a browser launch, desktop
input, CAPTCHA attempt, credential use, challenge retry, provider mutation,
installation, shared-runtime mutation, production effect, or release.

The W4 evaluator remains available as a narrow diagnostic fixture surface. W5
does not reinterpret its caller-selected scenario as observed task evidence.
Low-level desktop capture, locate, evidence, and interaction actions remain
internal capability surfaces and are not exposed as task parameters.

## Expected Write Surface

- `crates/agent-browser-challenge-control/` for the pure task lifecycle and
  receipt invariants;
- `cli/src/native/service_challenge_task.rs` and narrow Service State,
  dispatch, status, and resources adapters;
- the Service request schema, action registry, MCP and HTTP parity, field-role
  ledger, and generated `@agent-browser/client` types;
- focused provider-free fixtures and tests;
- required CLI help, README, agent skill, docs-site, plan, roadmap, runbook,
  and active-lane projections.

Authentication Run behavior, challenge-family detector heuristics, desktop
transaction internals, runtime installation, and provider integration are
outside this packet.

## Delivery Sequence And Budget

- Attempt 1: freeze the pure task and composite receipt contract, then add
  table-driven lifecycle tests.
- Attempt 2: add durable Service orchestration, request normalization,
  provider-free fixture execution, and status and resources projection.
- Attempt 3: align generated clients and user guidance, then run the selected
  validation ladder.
- Rework budget: one bounded correction pass for a defect caused by this
  packet. A second semantic defect, unexpected shared-surface conflict, or need
  for live evidence stops for replanning.

No subagent, browser worker, benchmark worker, fixture worktree, runtime
operator, or provider operator is assigned.

## Validation And Exit

Exit requires evidence that:

- table-driven tests cover every phase, terminal result, budget exhaustion,
  operation replay, cancellation, cooldown, and intervention transition;
- start is exact-handle-bound, principal-owned, idempotent, deadline-bounded,
  secret-free, and durable across repository reload;
- one registered provider-free fixture traverses observe, decide, resolve,
  verify, cooldown when applicable, and downstream admission;
- the final receipt distinguishes delivery, verification, challenge outcome,
  and admission without reconstructing multiple Service jobs;
- status and resources expose bounded task and intervention summaries;
- CLI, HTTP, MCP, schema, field-role ledger, generated client, and docs agree;
- challenge-control and desktop-services architecture guards pass; and
- selected focused tests, formatting, strict workspace Clippy, and exact-head
  CI pass.

Stop before any live challenge, browser, provider, credential, installation,
shared-runtime, production, or release action.
