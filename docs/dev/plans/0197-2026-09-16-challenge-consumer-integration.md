# Plan 0197 | Challenge Consumer Integration

Date: 2026-09-16

Plan version: 4

State: OPEN

Consolidation: required

Product lane: PL-CHALLENGE

Lane: P197

Parent plan: Plan 0187 W6

Work item: `CochranResearchGroup/agent-browser#127`; issue #66 remains the
separately live-gated challenge acceptance leaf

Branch: `challenge/p197-consumer-integration`

Target: `main`

Integration: merge through the protected `main` workflow after provider-free
W6 validation

Source baseline: `faee8887fb420e8e46bd5a165ce6c44fa10059d2`

## Objective

Integrate authentication and navigation as the first two consumers of the
provider-neutral challenge task. Both consumers must use one typed admission
contract that verifies the completed challenge receipt, effective site policy,
downstream intent, principal, and exact current tab before either consumer may
continue. Preserve challenge outcome, admission outcome, and later consumer
failure as separate facts.

## Current State

Plan 0187 W0 through W5 are integrated. Plan 0193 added the durable challenge
task and one composite receipt for observation, decision, bounded resolution,
verification, cooldown, intervention, and downstream admission. The task is
principal-owned, exact-handle-bound, idempotent, deadline-bounded, and exercised
through provider-free fixtures.

Source checkpoint `210e28af` implements one pure consumer-admission contract
plus one shared Service adapter. Authentication Run calls that adapter before
its first effect and durably retains the admission beside later authentication
failure. Navigation admission now wraps the outer command boundary before
confirmation, runtime admission, recovery, auto-launch, or action dispatch. The
outer response wrapper retains the typed admission beside any later navigation
success or failure, and the inner handler rejects a challenged call that tries
to bypass the preflight.

P190 integrated through PR #152 as merge commit `38e4cb9d` and released its
shared public-documentation custody. P197 merged the resulting `main` through
checkpoint `b9b8afc4`; its challenge-control, authentication, navigation,
schema, and generated-client changes remain intact. P202 is now the primary
writer for `README.md`, `cli/src/output.rs`, `skills/agent-browser/SKILL.md`,
and `docs/src/app/commands/page.mdx` through PR #168. P197 continues source and
validation work independently, then will merge P202's integrated documentation
baseline before adding the bounded challenge-consumer guidance.

## Contract

The pure challenge-control crate will expose one consumer-admission decision
over a completed `ChallengeTaskReceipt`. The request identifies:

- a registered consumer kind, initially authentication or navigation;
- the expected downstream intent;
- the SHA-256 digest of the effective site policy; and
- an attributable consumer operation identity.

The pure decision validates that the receipt is terminal, admitted, internally
consistent, bound to the same policy digest and downstream intent, and not in
intervention. It preserves cooldown and prior-effect evidence rather than
misinterpreting either as a failed admission. It returns a typed receipt that
keeps the challenge decision and consumer admission separate. It does not
execute the consumer or reinterpret a later consumer failure as challenge
failure.

The Service adapter resolves the effective `SitePolicy`, hashes its canonical
serialized value, verifies principal ownership and the exact current
`ServiceTabHandle`, then invokes the pure contract. Authentication start and
navigation call that same adapter before their first consumer effect. Requests
without an explicit challenge task retain their current behavior unless the
effective site policy requires challenge admission.

## Consolidated Batch

1. Freeze the pure consumer-admission request, decision, receipt, and failure
   taxonomy with table-driven challenge-control tests.
2. Add one Service adapter that resolves and hashes the effective site policy,
   reloads the durable challenge receipt, and verifies principal, downstream
   intent, and exact current tab binding.
3. Integrate Authentication Run start through the adapter without adding
   credentials, provider behavior, or a second challenge state machine.
4. Integrate navigation through the same pre-effect adapter and prove denied or
   stale admission prevents navigation dispatch.
5. Prove that an admitted challenge followed by authentication or navigation
   failure reports consumer failure while retaining the successful challenge
   decision.
6. Align only the request schema, Service contract metadata, field-role ledger,
   generated client, and focused provider-free documentation needed for this
   contract. Shared README, global CLI help, command docs, and agent-skill edits
   remain deferred while P190 owns those surfaces.
7. Hoist challenge-aware navigation admission to the outer command boundary so
   it runs before confirmation, runtime admission, browser recovery, launch, or
   dispatch. Carry the same typed admission through both successful and failed
   navigation responses without mutating the durable challenge outcome.

## Scope And Effect Boundary

Authorized scope:

- `crates/agent-browser-challenge-control/` consumer admission contract and
  provider-free tests;
- `cli/src/native/service_challenge_task.rs` or one narrow adjacent Service
  adapter;
- `cli/src/native/service_authentication_run.rs` and the navigation pre-effect
  seam;
- exact Service request schema, contract metadata, field-role ledger, generated
  client, and focused provider-free fixtures;
- this plan, active-lane projection, issue, pull-request, and integration
  receipts.

This plan does not authorize a browser launch, navigation against a browser,
desktop input, CAPTCHA attempt, credential use, challenge retry, provider
mutation, installation, shared-runtime mutation, production effect, or release.
All validation is repository-owned and provider-free. Issue #66 retains every
live Turnstile or hCaptcha acceptance action.

## Delivery Sequence And Budget

- Attempt 1: pure consumer contract and red-capable table tests.
- Attempt 2: one shared Service admission adapter plus authentication and
  navigation integration tests.
- Attempt 3: contract parity, generated client, selected validation, and one
  bounded correction pass if a defect was introduced by this packet.
- Maximum work-unit attempts: 3.
- Maximum review and rework cycles: 1.
- Reassess after two checkpoints or 30 active minutes without outcome progress.
- Overall effort ceiling: 300 active minutes through protected integration.

No subagent, browser worker, provider operator, benchmark checkout, or runtime
operator is assigned.

Plan version 3 is a material replan after the completion audit exposed one
shared-dispatch ordering defect and its response-projection consequence. It does
not reset cumulative effort, attempt, review, or discovery bounds. One bounded
provider-free repair packet remains: outer-boundary navigation admission,
failure projection, focused red-to-green coverage, then the already selected
changed-surface gates. Another semantic defect ends this source packet for a
successor decision rather than starting another repair loop.

Plan version 4 records dependency progress rather than changing scope. P190 is
integrated, P197 is refreshed onto that source, and P202 temporarily owns the
four shared public-documentation files. The critical path is now refreshed
exact-head validation, P202 integration, the bounded P197 documentation delta,
and protected P197 integration. The refreshed `b9b8afc4` workflow passed every
fast gate, including comprehensive Rust. Provider-free local revalidation then
exposed one parity-harness assumption: the checker inspected only the public
`execute_command` wrapper after P197 moved dispatch arms behind navigation
admission. The checker now includes that explicit delegated dispatcher, and the
real parity command proves all 101 native and 118 service-request actions remain
covered.

## Implementation Checkpoint

The pure challenge-control contract verifies terminal receipt consistency,
effective site-policy digest, registered downstream intent, and admitted
outcome semantics. The Service adapter additionally verifies the current
principal and exact Service tab. Authentication stores the typed admission on
its durable run record. Challenge-aware navigation requires the site policy,
operation identity, and exact Service tab handle before dispatch.

Provider-free validation at source checkpoint `3158503b` passes:

- all 16 challenge-control crate tests, including six consumer-admission cases;
- focused Service adapter, Authentication Run, navigation, and request-shape
  tests;
- challenge-control architecture and route-confusion gates;
- Service API and MCP parity, generated-client contract and type checks, and
  Service collection no-launch parity;
- formatting, strict workspace Clippy, and validation-selector readback.

The parity pass found one packet-local omission in the MCP projection of
`sitePolicyId`. Checkpoint `3158503b` corrects that projection, and the failed
gate passes on the repaired exact head. No browser, provider, credential,
installed-runtime, or production effect occurred.

Draft PR #157 carries the published branch. Exact-head CI for `38f7b1e2` may
finish as retained evidence, but it cannot establish merge readiness because
the navigation ordering defect changed executable source afterward. Checkpoint
`210e28af` hoists admission around the shared dispatcher and retains it on
later failure responses. The outer-denial, inner-bypass, failure-projection,
existing admission, route-confusion, and CDP stream derivation tests pass, as do
format and strict workspace Clippy. The selector-recommended live CDP streaming
smoke is excluded because it launches a browser outside this plan's explicit
provider-free boundary. P197 merged current `main` through published checkpoint
`b9b8afc4`. Its exact-head workflow passed Version Sync, Rust Quality,
Dashboard, Service Client, Workstation Fixtures, and comprehensive Rust. The
post-merge challenge-control crate, service parity, generated-client contract
and type checks, no-launch collection smoke, and route-confusion gates also
pass. P202 retains primary-writer custody of the four shared user-facing
documentation surfaces through PR #168. After P202 integrates, P197 will merge
that baseline, add only its bounded challenge-consumer guidance, and complete
the final changed-surface and protected integration gates.

## Validation And Exit

Exit requires current evidence that:

- the pure admission contract rejects nonterminal, withheld, mismatched-policy,
  mismatched-intent, intervention, and internally inconsistent receipts while
  preserving valid cooldown and prior-effect evidence;
- exact replay returns the same decision without executing a consumer;
- Authentication Run and navigation both call the same Service adapter;
- challenge-aware navigation admission runs at the outer command boundary
  before confirmation, runtime admission, browser recovery, browser launch, or
  action dispatch;
- principal, effective site policy, downstream intent, and exact current tab
  mismatches fail before consumer execution;
- a later authentication or navigation failure remains a consumer failure and
  does not rewrite challenge outcome or admission, and the navigation failure
  response retains the typed admission as a separate fact;
- no provider name, desktop coordinate, selector, credential, or browser
  implementation leaks into the pure contract;
- challenge-control tests, focused Service and navigation tests, contract
  parity, generated-client checks, format, strict workspace Clippy, and the
  validation selector pass; and
- the published exact source checkpoint enters `main` through a linked pull
  request with applicable CI green.

## Stop Condition

Stop before any live or installed-runtime action. Replan if either consumer
requires provider-specific logic, a second challenge lifecycle, a shared
contract owned by P190, or a public surface beyond the bounded Service contract
parity named above. A second semantic defect or any inability to prove
pre-effect denial ends the implementation attempt without weakening the gate.
