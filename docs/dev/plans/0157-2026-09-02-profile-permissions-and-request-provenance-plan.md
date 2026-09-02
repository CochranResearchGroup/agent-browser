# Plan 0157: Profile Permissions And Request Provenance

Date: 2026-09-02

State: OPEN

Execution state: `w10_public_contract_alignment_in_progress`

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

### Deep Module Seams

The architecture review at
`/tmp/architecture-review-20260902-052924.html`, produced against ancestor
`6e89771b`, is accepted as design input for this plan.

Before changing permission or public-contract semantics, extract one deep
in-process Profile acquisition module behind the current interfaces. Its small
interface accepts a normalized request intent plus current Service State,
access-policy, lease-authority, and runtime observations, then returns one
executable acquisition decision or exact denial. The implementation owns:

- identity joins and assurance classification;
- access-policy evaluation and admitted policy revision;
- coordination admission and wait disposition;
- dominant-blocker selection and supporting evidence;
- retained-browser reuse, lifecycle replacement, and stale-history rejection;
- route and runtime-lane derivation; and
- executable recourse when acquisition cannot proceed.

Service Access, Profile recovery, and action runtime consume this result. They
must not reconstruct a decision from JSON, mutate route hints after the fact,
or independently reinterpret access, lease, or lifecycle evidence. Keep this
seam in-process until cohesive behavior and dependency direction are proven;
this plan does not add a generic service-core crate or another lease-policy
crate.

The protected lease-authority kernel remains canonical. P157 operations use
one cohesive client interface that owns request encoding, protected exchange,
challenge lifecycle, response validation, and typed error translation. The
client is not a second evaluator.

Workstation convergence is a separate deep module. Rust owns desired state,
observed state, the sealed convergence plan, and the final receipt. Privileged
shell logic is an effect adapter that executes only that plan and returns
evidence. Dashboard health consumes the typed convergence result and never
derives installation health from ACL ambiguity or generic warning text.

One semantic contract oracle owns canonical scenarios and invariants for the
Profile acquisition and convergence interfaces. HTTP, MCP, generated clients,
dashboard projections, and execution admission must all pass the same oracle.
Small explicit end-to-end examples remain to protect user-visible behavior.

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
| W1 | Extract behavior-preserving Profile acquisition decision ownership and establish the semantic contract oracle | none | Service Access, recovery, and action runtime consume one typed executable decision; current public behavior passes canonical fixtures before policy changes |
| W2 | Freeze permission, provenance, failure, migration, and dashboard-health schemas with red provider-free regressions | W1 | Tests reproduce missing scheduler recourse, lost lane correlation, circular identity recourse, shared-profile overblocking, and warning-axis conflation |
| W3 | Capture immutable request provenance at ingress and preserve it through runtime-lane dispatch | W2 | Every queued request has stable lane, subject, assurance, connection, profile, and causal identities without retaining private routing payloads |
| W4 | Replace split terminal persistence with one typed outcome builder | W2, W3 | Every terminal exit returns and persists identical structured failure and provenance; pre-dispatch rejection is fully traceable |
| W5 | Add revisioned access-policy evaluation and frictionless presets inside the Profile acquisition owner | W1, W2, W4 | `shared-local`, `restricted`, and `exclusive` decisions are deterministic; clients never need runtime-owner proof for ordinary authorized work |
| W6 | Add attributable tab participation, reconnect, and inherited child policy | W5 | Concurrent local clients reuse one browser, receive independent tabs, and close only authorized resources |
| W7 | Add drain-and-restrict, graceful release, explicit eviction, and revision conflicts | W5, W6 | Narrowing cannot race new admission or commit while incompatible occupancy remains; force is explicit and receipted |
| W8 | Integrate the cohesive lease-authority client, human takeover, lifecycle proof, and full-shutdown authorization | W4, W5, W7 | Controller leases remain separate; protected exchange is not repeated across helpers; lifecycle effects require both permission and exact physical proof |
| W9 | Deepen workstation installation into one Rust convergence owner with a privileged effect adapter | W2, W4, W8 | Desired state, observed state, sealed plan, and receipt have one owner; ACL ambiguity cannot set runtime readiness and shell code does not own policy |
| W10 | Migrate legacy state and align CLI, HTTP, MCP, generated clients, dashboard, doctor, help, README, skill, and docs site | W4, W5, W8, W9 | Existing profiles retain intended access; dashboard separates access, acquisition, and convergence axes; every client receives executable recourse |
| W11 | Run isolated installed acceptance and adversarial concurrency scenarios through the contract oracle | W10 | Development doctor, multi-client sharing, live policy edits, eviction, crash recovery, logging completeness, warning taxonomy, and disposable shutdown pass without production effects |

Critical path: `W1 -> W2 -> W3 -> W4 -> W5 -> W6 -> W7 -> W8 -> W9 -> W10 -> W11`.
W6 may prepare reconnect fixtures while W5 completes, but policy enforcement
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
- The dashboard shows profile-access ambiguity as nonblocking observation,
  acquisition denial only on the affected request, and `Runtime status out of
  sync` only for a typed install-convergence failure with one executable action.
- Canonical scenarios produce the same semantic decision through HTTP, MCP,
  generated clients, dashboard projection, and execution admission.
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
- Do not add a permanent facade or new crate around Profile acquisition before
  the cohesive in-process owner and dependency direction are proven.
- Do not let the ACL evaluator, dashboard, or privileged shell adapter decide
  workstation convergence.
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

Focused Rust tests must cover the Profile acquisition owner interface,
access-policy evaluator, cohesive lease-authority client, request provenance,
scheduler terminalization, failure recourse, claim integration, tab ownership,
drain and eviction, install-convergence owner, state migration, and
full-shutdown authorization. The semantic contract oracle covers transport and
dashboard parity. Installed acceptance uses only disposable profiles and
browsers.

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
10. One deep Profile acquisition module structurally co-owns planning and
    execution truth; callers cannot reconstruct or override its decision.
11. Dashboard runtime health is produced by the install-convergence owner and
    distinguishes nonblocking access ambiguity from actual runtime drift.

## Initial Checkpoint

State transition: `unregistered -> planned`.

Acceptance state: architecture and contract are registered; W1 through W11
remain.

Progress classification: `outcome_progress`.

Evidence: Research.gov fieldwork reproduced a scheduler rejection that returned
before failure decoration, persisted no structured recourse, and lost runtime
lane correlation after selector removal. Current source and Plans 0111, 0142,
0144, and 0156 establish the sharing, recourse, fenced coordination, and exact
shutdown foundations. The accepted ADR freezes access policy as a separate
axis with `shared-local` as the trusted local default.

Material blocker: acquisition truth remains distributed across Service Access,
Profile recovery, and action runtime, so policy or provenance changes would
otherwise deepen the split before a semantic oracle can detect divergence.

Next action: implement W1 only. Extract the current joined acquisition decision
behind one typed in-process interface and freeze canonical current-behavior
fixtures before changing permission, provenance, or public-contract semantics.

## Execution Checkpoint W1-A

Source checkpoint: `2566592f60235ad6fb27eef3c2f15fe0799ec4b6`.

State transition: `planned -> open`.

Completed in W1 attempt 1:

- introduced one typed `ProfileAcquisitionDecision` artifact while preserving
  the existing public access-plan projection;
- changed action-runtime route application to consume that artifact directly,
  removing its JSON blocker, lifecycle, and route reconstruction;
- made the existing recovery coordinator an internal child of the Profile
  acquisition module and redirected all production callers to that owner;
- moved exact lifecycle-replacement evaluation under the same module; and
- froze current reuse, launch, and foreign-principal denial behavior as the
  first semantic-oracle cases.

W1 remains open. `profile_reuse_decision`, dominant-blocker selection, and the
executable service-request projection still originate in `service_access.rs`.
The acquisition module temporarily validates the completed projection at its
seam. Attempt 2 must move those cohesive computations behind the typed
interface and delete `ProfileAcquisitionDecision::from_access_plan`; policy and
public-contract semantics remain frozen until that checkpoint is green.

Validation evidence:

- 51 Service Access tests pass;
- 87 action-runtime route-host tests pass;
- 23 Profile acquisition recovery tests pass;
- workspace clippy passes with warnings denied;
- the complete provider-free Rust gate passes, including 1,881 parallel-safe
  tests and every serial environment-mutating partition; and
- service API/MCP parity, generated-client contract drift, and client type
  checks pass.

## Execution Checkpoint W4

Source checkpoint: `2ab08e87`.

State transition: `w3_complete -> w4_complete`.

Completed in W4 attempt 1:

- introduced one typed terminal outcome builder for success, failure,
  cancellation, timeout, and rejection across every control-plane exit;
- persisted the exact same failure, provenance, state, phase, and completion
  identity in the response, ServiceJob, terminal ServiceEvent, and trace
  projection;
- replaced the split enqueue, execution, timeout, cancellation, and scheduler
  persistence helpers with the single terminal finalizer; and
- turned only the scheduler-rejection regression case green, leaving the three
  W5 and W9 cases intentionally red.

W4 is complete without changing profile-access policy or production runtime
state. W5 is open to add revisioned `shared-local`, `restricted`, and
`exclusive` evaluation inside the Profile acquisition owner.

Validation evidence:

- the full provider-free Rust gate passes with 1,886 parallel-safe tests and
  every serial environment-mutating partition;
- formatting and workspace clippy with warnings denied pass;
- the P157 oracle passes with six schemas, two green cases, and three bounded
  red cases; and
- generated-client drift, service-client contracts, client types, and patch
  whitespace checks pass.

## Execution Checkpoint W5

Source checkpoint: `f7166030`.

State transition: `w4_complete -> w5_complete`.

Completed in W5 attempt 1:

- added one revisioned access-policy evaluator inside the Profile acquisition
  owner with deterministic `shared-local`, `restricted`, and `exclusive`
  decisions;
- made a missing explicit policy resolve to the frictionless `shared-local`
  default while preserving strict identity checks for opt-in strict modes;
- separated caller self-identification from trusted assurance so request
  metadata cannot self-promote a client to authenticated, registered, or
  operator authority;
- carried the admitted policy revision, access-decision identity, subject, and
  assurance into the executable service request and immutable provenance; and
- replaced circular identity-error recourse with typed permission context and
  one executable service-owned recovery action.

W5 is complete without production profile, browser, runtime, or ACL effects.
W6 is open to add attributable tab participation, reconnect, and inherited
child policy.

Validation evidence:

- the full provider-free Rust gate passes with 1,891 parallel-safe tests and
  every serial environment-mutating partition;
- all 53 Service Access tests, 23 Service Request tests, 129 MCP tests, and the
  focused access-policy tests pass;
- formatting and workspace clippy with warnings denied pass;
- service API/MCP parity, route-confusion, no-launch service collection, and
  complete service-client gates pass; and
- the P157 oracle passes with six schemas, four green cases, and the one W9
  convergence case intentionally red.

## Execution Checkpoint W6

Source checkpoint: `83319369`.

State transition: `w5_complete -> w6_complete`.

Completed in W6 attempt 1:

- generated one service-owned connection instance per daemon transport and
  bound each admitted tab child to that connection and stable subject;
- persisted inherited child permissions on tab records and service-owned tab
  handles, while intersecting every operation with the current parent policy;
- made one-shot HTTP and MCP requests reconnect disconnected children for the
  same subject without allowing matching labels to steal a live connection;
- authorized refresh, observation, control, and exact tab release through the
  child policy, with `tab-close-own` limited to the attributed resource; and
- protected internal connection and child-policy fields from caller injection
  while preserving the subject route through generated client helpers.

W6 is complete without production profile, browser, runtime, or ACL effects.
W7 is open to add revision-fenced drain-and-restrict, graceful release, and
explicit receipted eviction.

Validation evidence:

- the full provider-free Rust gate passes with 1,896 parallel-safe tests and
  every serial environment-mutating partition;
- formatting and workspace clippy with warnings denied pass;
- the complete service-client, generated-client, API/MCP parity,
  route-confusion, no-launch collection, and close-scope gates pass;
- focused connection, reconnect, subject isolation, inherited-permission, and
  exact own-tab close tests pass; and
- the P157 oracle remains at six schemas, four green cases, and the one W9
  convergence case intentionally red.

## Execution Checkpoint W7

Source checkpoint: `d3c12100`.

State transition: `w6_complete -> w7_complete`.

Completed in W7 attempt 1:

- added expected-revision compare-and-swap for every Profile policy mutation,
  with the current revision and a redacted structural diff on conflicts;
- made widening commit immediately at a new revision while narrowing an
  occupied Profile enters a persisted drain at the current revision;
- fenced new admission and later child control during a drain while preserving
  exact own-tab release so compatible occupants can leave gracefully;
- derived incompatible occupancy from attributed persisted tabs instead of
  trusting caller-supplied blockers, then committed narrowing only after the
  occupancy reached zero; and
- separated explicit eviction authorization from policy editing, with exact
  target plans and minimal receipts for graceful and forced outcomes.

W7 is complete without production profile, browser, runtime, ACL, or eviction
effects. W8 is open to integrate the cohesive lease-authority client, human
takeover, exact lifecycle proof, and full-shutdown authorization.

Validation evidence:

- the full provider-free Rust gate passes with 1,901 parallel-safe tests and
  every serial environment-mutating partition;
- formatting and workspace clippy with warnings denied pass;
- focused policy, repository persistence, revision-conflict, drain-fencing,
  graceful-release, and explicit-eviction receipt tests pass; and
- the P157 oracle remains at six schemas, four green cases, and the one W9
  convergence case intentionally red.

## Execution Checkpoint W8

Source checkpoint: `c46c6d43`.

State transition: `w7_complete -> w8_complete`.

Completed in W8 attempt 1:

- centralized protected lease-authority request encoding, exchange, and typed
  response validation in one cohesive client while retaining the kernel as the
  canonical policy evaluator;
- kept human takeover limited to controller authority, advanced its epoch to
  fence the former controller, and proved the Profile access policy remained
  unchanged across the transaction;
- converted exact W7 eviction plans into durable lifecycle authorizations and
  required current policy revision, permission, force mode, tab identity,
  daemon route, browser identity, CDP target, and attached-target observation
  before a physical close;
- made forced tab eviction close only the proven target, cancel matching
  queued work through the canonical terminalizer, release only an empty exact
  session, disconnect matching viewer authority, and persist a minimal
  idempotency receipt; and
- bound full-runtime shutdown authorization to Operator assurance, both
  lifecycle permissions, the reviewed P156 plan digest, and its exact managed
  browser targets before any shutdown effect.

W8 is complete without production Profile, browser, runtime, ACL, eviction, or
shutdown effects. W9 is open to deepen workstation installation into one Rust
convergence owner with a privileged effect adapter.

Validation evidence:

- the full provider-free Rust gate passes with 1,909 parallel-safe tests and
  every serial environment-mutating partition;
- formatting and workspace clippy with warnings denied pass;
- the source-free workstation install fixture, host-provision fixture, fresh
  VM harness, Guacamole durability, route-specific user synchronization,
  route-confusion, CDP tab-streaming, API/MCP parity, and generated-client
  gates pass;
- focused cohesive-client, takeover-fencing, lifecycle-authorization,
  exact-target, grace-deadline, full-shutdown, and repository persistence tests
  pass; and
- the P157 oracle remains at six schemas, four green cases, and the one W9
  convergence case intentionally red.

## Execution Checkpoint W9

Source checkpoint: `d806c74c`.

State transition: `w8_complete -> w9_complete`.

Completed in W9 attempt 1:

- introduced one Rust convergence owner for desired workstation state,
  normalized observations, a digest-bound plan, one executable next action,
  and the final typed receipt;
- separated dashboard runtime, convergence, access, and request-scoped
  acquisition health so historical Profile ambiguity cannot create a runtime
  warning;
- classified one current selected runtime-host listener as authoritative
  without requiring the retired default daemon socket;
- constrained the privileged shell adapter to a Rust-sealed action set and
  required digest-bound helper, lease-authority, and workstation-dependency
  postconditions before Rust accepts its receipt; and
- removed dashboard interpretation of generic install issues and transaction
  text in favor of the convergence owner's blocking findings and one typed
  action.

W9 is complete without production Profile, browser, runtime, ACL, eviction, or
shutdown effects. W10 is open to align migration and every public contract,
client, doctor, help, skill, and documentation surface.

Validation evidence:

- the full provider-free Rust gate passes with 1,915 parallel-safe tests and
  every serial environment-mutating partition;
- formatting and workspace clippy with warnings denied pass;
- the source-free workstation install fixture, host-provision fixture,
  dashboard build, dashboard action and navigator contracts, API/MCP parity,
  generated-client contracts, and client type checks pass; and
- the P157 oracle passes with six schemas, five green cases, and no remaining
  reproducible red case.

## Execution Checkpoint W2

Source checkpoint: `cd0bdb1d`.

State transition: `w1_complete -> w2_complete`.

Completed in W2 attempt 1:

- froze additive v1 schemas for profile access policies, access decisions,
  immutable request provenance, terminal outcomes, legacy-policy migration,
  and independent dashboard health axes;
- made `shared-local` the exact default policy spelling and kept permission,
  coordination, lifecycle proof, and convergence as separate contracts;
- added one provider-free P157 oracle with source-backed red cases for missing
  scheduler recourse, lost runtime-lane provenance, circular identity recourse,
  shared-profile overblocking, and dashboard warning-axis conflation; and
- added the oracle to the normal service-client gate so later work units must
  explicitly retire each red case when its invariant turns green.

W2 is complete without changing runtime behavior or public transport surfaces.
W3 is open to capture the redacted provenance envelope before runtime routing
fields are consumed and preserve it through queue admission.

Validation evidence:

- `pnpm test:p157-contract-oracle` passes with six schemas and five red cases;
- the complete `pnpm test:service-client` gate passes;
- service API/MCP parity, generated-client contracts, and client types pass;
- the release-asset verifier fixture and validation selector pass; and
- patch whitespace and conflict-marker checks pass.

## Execution Checkpoint W1-B

Source checkpoint: `5166dabf`.

State transition: `open -> w1_complete`.

Completed in W1 attempt 2:

- moved profile-reuse evaluation, dominant-blocker selection, deterministic
  route naming, lifecycle replacement, and executable request projection into
  one in-process Profile acquisition owner;
- changed Service Access to consume the owner's typed result and project its
  compatibility JSON without rebuilding acquisition truth;
- deleted the temporary `ProfileAcquisitionDecision::from_access_plan` parser;
- kept recovery coordination as an internal child of the same owner and action
  runtime as a typed-decision consumer; and
- established one projection-consistency oracle across every Service Access
  fixture, with explicit reuse, launch, and foreign-principal denial cases.

W1 is complete without permission, public-contract, installation, or runtime
behavior changes. W2 is now open to freeze the permission, provenance, failure,
migration, and dashboard-health schemas with red provider-free regressions.

Validation evidence:

- 51 Service Access tests pass through the semantic projection oracle;
- 87 action-runtime route-host and 23 acquisition-recovery tests pass;
- workspace formatting and clippy with warnings denied pass;
- the complete provider-free Rust gate passes, including 1,881 parallel-safe
  tests and every serial environment-mutating partition; and
- service API/MCP parity, generated-client contract drift, and client type
  checks pass.
