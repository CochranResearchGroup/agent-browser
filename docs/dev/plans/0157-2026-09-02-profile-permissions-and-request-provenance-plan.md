# Plan 0157: Profile Permissions And Request Provenance

Date: 2026-09-02

State: PLANNED

Execution state: `architecture_and_contract_registered`

Lane: P157

Branch: plan/profile-permissions-and-request-provenance

Target: main

Source baseline: `15e2a4e49e8a9e76757c5697b3b4a520cf87ddbf`

Integration: merge

Authority: ARCHITECTURE, PLAN, SOURCE, DOCUMENTATION, PROVIDER-FREE FIXTURES,
AND ISOLATED DEVELOPMENT-RUNTIME VALIDATION ARE IN SCOPE. PRODUCTION POLICY
MUTATION, CLIENT EVICTION, BROWSER OR RUNTIME TERMINATION, PROFILE ACCESS,
CREDENTIAL ENTRY, PROVIDER NAVIGATION, AND TENANT DATA MUTATION ARE OUT OF
SCOPE.

Dependencies: [P111, P142, P144, P156]

Overlaps: [P144, P156]

## Incident

Research.gov fieldwork exposed two coupled product failures.

First, ordinary clients encounter profile identity and lifecycle proofs as if
those proofs were universal access credentials. The current model conflates
client identity, profile permission, work coordination, browser process
ownership, and lifecycle recovery. This makes a persistent shared profile act
more strictly than its intended policy and gives clients circular recourse such
as acquiring a profile in response to an identity error produced during that
same acquisition.

Second, the failed requests cannot be reconstructed reliably. The runtime host
removes its lane selector before the request enters the control-plane worker,
but `ControlRequest` does not retain that lane as provenance. Scheduler profile
lease rejection persists a failed job and returns before
`attach_service_failure_recourse` runs. The response and job therefore omit the
structured failure object, and the job and event projections may lack the
session and profile identities needed to connect a rejection to the browser
that caused it. The observed `existing_session_profile_identity_unproven`
failure had real browser and job evidence, but no complete causal record.

## Objective

Make ordinary profile reuse frictionless inside its declared trust boundary,
while preserving strict opt-in isolation and exact lifecycle safety. Introduce
one access-policy evaluator, one request-provenance envelope, and one terminal
outcome path so authorization and observability agree at every boundary.

The completed system must let a local client say who it is and what it wants,
then either receive a usable tab or one typed, executable explanation. Clients
must never reconstruct browser ownership proofs, infer a session from process
inspection, or parse logs to decide whether retry is safe.

## Architecture Decision

Three independent evaluators compose in this order:

```text
current access policy
    -> current coordination lease
        -> exact runtime proof when the authorized operation affects lifecycle
```

Access policy answers whether a subject may perform an operation. A lease
coordinates currently admitted work. Runtime proof verifies the exact physical
target before adoption, transfer, close, shutdown, or another lifecycle effect.
No layer may manufacture evidence for another.

Plan 0144 remains the authority for fenced coordination and effect admission,
but strict claim recovery is not a profile sharing policy. Its principal and
capability requirements apply where the selected access policy or ingress
assurance requires them. They do not force every trusted local shared client to
enroll a cryptographic identity. Plan 0156 remains the authority for exact full
shutdown effects, while this plan supplies the operator permission and complete
causal receipt that precede those effects.

## Frozen Permission Contract

1. `shared-local` is the default for a profile without an explicit policy in a
   trusted single-user local runtime.
2. `restricted` and `exclusive` are explicit modes. Remote or multi-tenant
   ingress supplies an authenticated subject automatically.
3. A self-declared stable client subject receives a service-generated
   connection-instance identity. Policy grants follow the stable subject;
   live commands and coordination belong to the connection instance.
4. Human-facing administrator, participant, and observer presets compile into
   granular permissions. Presets are not the storage or enforcement model.
5. Profile policy supplies inherited defaults to browsers, sessions, tabs, and
   views. A child may narrow authority but cannot silently exceed its parent.
6. Widening access commits immediately at a new policy revision.
7. Narrowing an occupied profile is a drain-and-restrict transaction. It fences
   new admission, drains incompatible occupancy, and commits only when the
   required occupancy reaches zero.
8. Forced eviction requires explicit `evict` permission and an explicit
   operation. Editing the ACL never implies eviction.
9. Graceful release is attempted first. An authorized operator may select an
   immediate or post-grace forced path.
10. Forced eviction cancels queued work, terminates incompatible leases and
    streams, and closes foreign tabs. It preserves minimal redacted receipts,
    not page bodies or form state.
11. One already-dispatched atomic command may finish under its admitted policy
    revision. Later commands are fenced; long-lived interaction is terminated.
12. Reconnecting clients may reclaim surviving tab handles only when stable
    subject identity and current policy still agree. A client label cannot
    hijack another live connection instance.
13. Human takeover changes the controller lease, not the profile ACL.
14. Full shutdown requires operator authorization followed by Plan 0156 exact
    managed-process proof. It clears occupancy without changing profile policy.
15. Concurrent policy edits use expected-revision compare-and-swap and return
    the current revision plus a redacted diff on conflict.
16. Audit records contain subject, assurance level, resource, operation,
    revisions, drain and eviction outcomes, and timestamps. They exclude
    secrets, page contents, raw profile paths, and bearer material.
17. Existing shared profiles migrate to `shared-local`. Positively proven
    strict profiles receive a compatibility-restricted policy. Ambiguous
    legacy identity records remain observable but do not block installation or
    ordinary shared use.
18. Every denial returns the subject, assurance, resource, operation, missing
    permission, policy revision, blocking occupancy, and one executable next
    action.

## Request Provenance Contract

Create one immutable, redacted envelope before runtime-host routing fields are
removed. Every response and persisted projection receives the same identities:

```text
requestId
jobId
traceId
causedByRequestId
clientSubjectId
identityAssurance
connectionInstanceId
runtimeEnvironmentId
runtimeLaneId
profileId or canonical opaque profile resource key
browserId
sessionId
tabId
serviceName
agentName
taskName
action
policyRevision
accessDecisionId
```

Unknown fields remain explicitly null. A subsystem may add a positively proven
identity, but it may not infer one from a label, historical session, or browser
process name. The envelope never contains credentials, URLs requiring privacy,
profile paths, capability bearers, process digests, or raw provider data.

All terminal paths use one outcome builder before responding or persisting:

```text
typed failure plus provenance
    -> redacted response
    -> terminal ServiceJob
    -> ServiceEvent and trace projection
    -> incident grouping when applicable
```

This includes queue-full rejection, stopped worker, cancellation before
dispatch, scheduler rejection, wait-reschedule failure, timeout, cancellation
while running, ordinary failure, and success. The response and durable job must
carry the same structured failure code, phase, effect state, retry disposition,
and causal locators.

`existing_session_profile_identity_unproven` is retired as a generic client
remediation. If shared access is authorized, the broker acquires attributable
tab work without requiring runtime-owner proof from the client. If an exact
lifecycle effect lacks proof, the denial names that operation and offers
service-owned re-observation, rebind, bounded shutdown, or quarantine recourse.

## Work Units

| Unit | Scope | Depends on | Exit condition |
| --- | --- | --- | --- |
| W1 | Freeze permission, provenance, failure, and migration schemas with red provider-free regressions | none | Tests reproduce missing scheduler recourse, lost lane correlation, circular identity recourse, and shared-profile overblocking |
| W2 | Capture immutable request provenance at ingress and preserve it through runtime-lane dispatch | W1 | Every queued request has stable lane, subject, assurance, connection, profile, and causal identities without retaining private routing payloads |
| W3 | Replace split terminal persistence with one typed outcome builder | W1, W2 | Every terminal exit returns and persists identical structured failure and provenance; pre-dispatch rejection is fully traceable |
| W4 | Add revisioned access-policy evaluation and frictionless presets | W1, W3 | `shared-local`, `restricted`, and `exclusive` decisions are deterministic; clients never need runtime-owner proof for ordinary authorized work |
| W5 | Add attributable tab participation, reconnect, and inherited child policy | W4 | Concurrent local clients reuse one browser, receive independent tabs, and close only authorized resources |
| W6 | Add drain-and-restrict, graceful release, explicit eviction, and revision conflicts | W4, W5 | Narrowing cannot race new admission or commit while incompatible occupancy remains; force is explicit and receipted |
| W7 | Integrate human takeover, lifecycle proof, and full-shutdown authorization | W3, W4, W6 | Controller leases remain separate, and lifecycle effects require both permission and exact physical proof |
| W8 | Migrate legacy state and align CLI, HTTP, MCP, generated clients, dashboard, doctor, help, README, skill, and docs site | W3, W4, W7 | Existing profiles retain intended access, ambiguity is nonblocking, and every client receives executable recourse |
| W9 | Run isolated installed acceptance and adversarial concurrency scenarios | W8 | Development doctor, multi-client sharing, live policy edits, eviction, crash recovery, logging completeness, and disposable shutdown pass without production effects |

Critical path: `W1 -> W2 -> W3 -> W4 -> W5 -> W6 -> W7 -> W8 -> W9`.
W5 may prepare reconnect fixtures while W4 completes, but policy enforcement
remains the join. No work unit may independently invent subject, policy,
provenance, or terminal-outcome semantics.

## Required Regression Scenarios

- A local ephemeral debugger self-identifies, receives an automatic session and
  tab, and exits without lease choreography.
- Ten compatible clients share one authenticated profile through one browser,
  with attributable tabs and no profile-wide self-conflict.
- Two live clients use the same label but receive distinct connection instances
  and cannot steal each other's commands or tabs.
- A policy widens immediately while an occupied narrowing remains pending until
  its incompatible occupants drain.
- New admission is fenced during drain, preventing an endless restrict race.
- Graceful eviction preserves a bounded opportunity to release; forced eviction
  closes exact foreign tabs only and emits a redacted receipt.
- Revocation during an atomic command permits that command's bounded terminal
  outcome but fences queued and subsequent effects.
- Human takeover pauses agent input without rewriting profile permissions.
- A browser crash refreshes service-owned runtime proof while clients retain
  policy identity and receive truthful handle recovery or stale-handle recourse.
- Full shutdown rejects an unauthorized caller, then an authorized disposable
  fixture closes only exact managed targets without mutating ACLs or profiles.
- Scheduler rejection returns the same structured failure in the immediate
  response and persisted job, including the real runtime lane and profile.
- A runtime-host request retains lane provenance after the routing selector is
  removed from its command payload.
- Queue-full, worker-stop, cancellation, timeout, and wait-reschedule failures
  each create one terminal job and one causally connected trace outcome.
- Historical sessions, warnings, and unproved owner rows remain queryable but
  cannot create access denial or make installation unhealthy.
- No log or client response contains capability bearer material, raw profile
  paths, credentials, page bodies, or private provider payloads.

## Migration And Compatibility

- Additive readers accept missing access policy and provenance fields during
  migration and apply the `shared-local` local-runtime default.
- Existing `sharedServiceIds`, allocation policies, caller labels, and session
  principals become migration inputs, not independent post-migration
  authorities.
- Existing `ProfileOwner` evidence remains the lifecycle-proof source until the
  P144 authority kernel replaces it. It is never interpreted as profile access.
- Existing `LeaseState` projections remain readable while canonical claims own
  coordination. They do not grant permissions.
- Existing `failure` objects remain schema compatible. New provenance and
  permission recourse fields are additive, versioned, and redacted.
- The scheduler and clients retain the legacy error string during migration,
  but no new client behavior parses that string.
- Mixed-version runtimes fail before effect when they cannot preserve the
  provenance or policy revision required by the selected operation. They do
  not fall back to inferred session ownership.

## Bounds And Stop Rules

- Maximum implementation attempts per work unit: 2
- Maximum review and remediation cycles: 1
- Maximum consecutive hardening or no-progress checkpoints: 2
- Checkpoint interval: each completed work unit or 90 minutes
- Do not mutate production ACLs, evict production clients, close production
  tabs, or apply production full shutdown in this plan.
- Do not add a second lease kernel, lifecycle owner registry, failure
  classifier, or request-log subsystem.
- Stop before enforcement if P144 integration cannot preserve the distinction
  between permission, coordination, and physical proof.
- Stop before migration if an existing strict profile cannot be classified
  without weakening its present isolation.
- Stop before terminal persistence changes if the response and durable job
  cannot be produced from one typed outcome.
- Stop before logging private payloads or bearer material. Missing correlation
  is repaired with opaque identifiers, not sensitive data.

## Validation

At each source slice, use the cheapest deterministic test that proves the named
risk. Required final gates include:

```text
pnpm test:service-client
pnpm test:dashboard-inspector-actions
pnpm test:service-access-plan-no-launch
pnpm test:browser-capability-registry-draft
scripts/ci/rust-tests.sh
scripts/ci/cargo-safe.sh fmt --all --manifest-path Cargo.toml -- --check
scripts/ci/cargo-safe.sh clippy --workspace --manifest-path Cargo.toml -- -D warnings
pnpm validation:select -- --base <last-green-checkpoint>
pnpm build:development-candidate
pnpm development-runtime:install
agent-browser-dev install doctor
pnpm smoke:development-browser-launch
```

Focused Rust tests must cover the access-policy evaluator, request provenance,
scheduler terminalization, failure recourse, claim integration, tab ownership,
drain and eviction, state migration, and full-shutdown authorization. Installed
acceptance uses only disposable profiles and browsers.

## Acceptance Criteria

1. A trusted local client can reuse a shared profile through self-identification
   and receive an attributable tab without principal enrollment or runtime-owner
   proof.
2. Restricted and exclusive profiles preserve stronger identity and isolation
   without imposing that friction on shared profiles.
3. Every command is admitted against one current policy revision and one
   coordination snapshot; lifecycle effects additionally prove their exact
   physical target.
4. Permission widening, drain-and-restrict, eviction, reconnect, human takeover,
   and full shutdown follow the frozen contract.
5. Every submitted request has one causal chain across response, job, event,
   trace, and incident surfaces, including all pre-dispatch failures.
6. `existing_session_profile_identity_unproven` no longer leaves a client with
   circular, identity-free remediation.
7. Legacy ambiguity is visible but cannot block installation or ordinary
   shared-local use.
8. Generated clients and dashboard surfaces provide the simple presets and one
   executable next action without exposing runtime proof internals.
9. Provider-free and isolated installed validation prove the contract without
   mutating production profiles, credentials, browsers, or provider state.

## Initial Checkpoint

State transition: `unregistered -> planned`.

Acceptance state: architecture and contract are registered; W1 through W9
remain.

Progress classification: `outcome_progress`.

Evidence: Research.gov fieldwork reproduced a scheduler rejection that returned
before failure decoration, persisted no structured recourse, and lost runtime
lane correlation after selector removal. Current source and Plans 0111, 0142,
0144, and 0156 establish the sharing, recourse, fenced coordination, and exact
shutdown foundations. The accepted ADR freezes access policy as a separate
axis with `shared-local` as the trusted local default.

Material blocker: no versioned policy or provenance schema and no red
provider-free regression exists yet.

Next action: implement W1 only, beginning with the scheduler-rejection and
runtime-lane provenance regressions before changing production code.
