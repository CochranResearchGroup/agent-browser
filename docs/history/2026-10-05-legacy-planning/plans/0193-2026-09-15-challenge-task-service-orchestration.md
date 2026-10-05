# Plan 0193 | Challenge Task Service Orchestration

Date: 2026-09-15

State: CLOSED

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

W5 is integrated. PR #142 merged exact source head
`709641e94c207bb6f0427812267267793ec2006f` into `main` as
`81de07cfb5790685ff506bdabff3609e92fd5eeb`. Source-head CI run 34993316164
passes every fast gate. Merge-commit CI run 34997057075 also passes every fast
gate on attempt 2 after one unchanged control-plane timing assertion failed on
attempt 1 and passed on the failed-job-only rerun. The durable Service task owns
start, status, resume, cancel, exact tab-handle and principal binding,
digest-only idempotency and operation replay, deadlines, transition budgets,
provider-free execution, and bounded status and resource summaries. No browser,
challenge, provider, credential, installation, shared-runtime, production, or
release effect occurred.

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

## Requirement Evidence

| Requirement | Implemented | Qualified | Integrated | Evidence or remaining gate |
| --- | --- | --- | --- | --- |
| Pure lifecycle and composite receipt | yes | yes | yes | 10 challenge-control tests and both crate architecture guards pass; source head `709641e9` is integrated |
| Durable exact-handle and principal custody | yes | yes | yes | focused Service task and no-launch dispatch tests pass; persisted operation and idempotency identities are digest-only |
| Start, status, resume, cancel and replay | yes | yes | yes | one disposable Service State fixture exercises every action, terminal replay, cancellation, deadline refusal, and no browser launch |
| Status and resource summaries | yes | yes | yes | the dispatch fixture observes active, terminal, cooldown, intervention, and zero pending-effect projections |
| CLI, HTTP, MCP, schema, ledger, client, and docs parity | yes | yes | yes | API/MCP parity, full service-client suite, generated-client checks, TypeScript, and docs build pass |
| Workspace quality | yes | yes | yes | formatting, diff hygiene, strict workspace Clippy, source-head CI run 34993316164, and merge-commit CI run 34997057075 pass |
| Protected integration | yes | yes | yes | PR #142 merged `709641e9` as `81de07cf`; exact source-head and merge-commit fast CI pass |

The local qualification used repository-owned provider-free fixtures only.
Installed-runtime and live acceptance are intentionally not applicable to W5.

## Next Action

No Plan 0193 execution remains. Plan 0187 and issue #127 stay open for W6
through W8. W6 consumer integration is the next sequential workfront but is
unstarted and not admitted. Issue #66 retains the separately live-gated
challenge acceptance leaf.
