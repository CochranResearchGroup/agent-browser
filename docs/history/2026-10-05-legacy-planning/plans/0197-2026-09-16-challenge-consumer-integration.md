# Plan 0197 | Challenge Consumer Integration

Date: 2026-09-16

Plan version: 10

State: CLOSED

Consolidation: required

Product lane: PL-CHALLENGE

Lane: P197

Parent plan: Plan 0187 W6

Work item: `CochranResearchGroup/agent-browser#127`; issue #66 remains the
separately live-gated challenge acceptance leaf

Branch: `challenge/p197-consumer-integration`

Target: `main`

Integration: merged through PR #157 as canonical merge commit `c855fc33`

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
schema, and generated-client changes remain intact. At operator direction,
P197 staged its bounded challenge-consumer guidance while P202 retained its
overlapping source custody. P202 has now merged through PR #168 as `528f2ef0`
and closed through canonical `main@2632e31c`. P197 joined that exact mainline at
merge checkpoint `7dc8a860`. The challenge implementation and four public
guidance files remain intact; P202 is no longer a dependency or active overlap.
Exact branch head `cd22a39f` passed every ordinary required forge check and
merged through PR #157 as canonical `main@c855fc33`. P197 is closed and removed
from the active-lane catalog. Its source branch remains preserved because P206
was built directly on that published ancestry.

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
   contract. Reconcile the bounded README, global CLI help, command-doc, and
   agent-skill guidance after the overlapping P202 baseline integrates.
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

Plan version 5 records the single planned self-review and rework cycle. The
spec-axis review found that Authentication Run serialized the typed admission
into an untyped `Value` before durable storage, contrary to this plan's typed
record requirement. A focused persistence test proved that malformed nested
admission JSON was accepted. The durable record now stores
`ChallengeConsumerAdmissionReceipt` directly; the same test rejects the
malformed shape, the later-authentication-failure projection remains green, and
all challenge-control tests pass. This is a contract-hardening correction
inside the existing W6 boundary, with no browser or external effect.

Plan version 6 records the operator-directed integration posture for shared
documentation. P197 proceeds with its bounded four-file consumer guidance while
P202 resolves its own current-main coordination conflict, then reconciles the
two branches after P202 integrates. This changes sequencing only. It does not
transfer P202 source custody, broaden W6, or authorize runtime effects.

Plan version 7 records the local dependency integration. P202 dependency head
`6f099292` includes current `main` through the closed P203 repair and joins P197
through merge checkpoint `6f71ea0b`. The merge is conflict-free and preserves
both four-file documentation deltas. P202 still owns its protected PR
transition; P197 will not publish or claim final integration until that source
enters `main`, shared governance is reconciled, and the combined exact head
passes its applicable checks.

Plan version 8 records dependency completion. P202 merged through PR #168 and
released the shared surfaces. P197 then joined canonical `main@2632e31c` at
`7dc8a860`, resolving only the current runbook projection while preserving both
histories. The remaining critical path is final publication, exact-head forge
evaluation under the repository's current CI policy, and protected P197
integration. No source expansion or runtime effect is introduced by this
reconciliation.

Plan version 9 records durable handoff. Exact head `4321961c` is published to
the branch and draft PR #157, the issue and PR projections identify the current
scope and remaining gates, and the clean checkout no longer needs to retain
primary custody while forge evaluation and protected integration proceed.

Plan version 10 records protected integration. Exact published head
`cd22a39f` passed Version Sync, Rust Quality, Dashboard, Service Client,
Workstation Fixtures and comprehensive Rust in run `35170777014`, then merged
through PR #157 as `c855fc33`. No browser, provider, credential, installed
runtime or production effect occurred during integration.

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
pass. The planned self-check additionally proved and repaired typed durable
Authentication Run admission storage: the focused persistence test is red on
the untyped field and green on `ChallengeConsumerAdmissionReceipt`; all nine
Authentication Run tests, four Service challenge-task tests, the navigation
bypass test, and the correctly stack-sized dispatch fixture pass. P197 joined
canonical `main@2632e31c` after P202 integration at merge checkpoint
`7dc8a860`; the shared documentation and governance histories remain intact.
Exact branch head `cd22a39f` passed all ordinary required checks and entered
canonical `main` through PR #157 as merge commit `c855fc33`.

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
