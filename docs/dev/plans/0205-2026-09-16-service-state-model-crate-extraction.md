# Plan 0205 | Service State Model Crate Extraction

Date: 2026-09-16

Plan version: 1

State: OPEN

Consolidation: required

Product lane: PL-PLATFORM

Lane: P205

Work item: `CochranResearchGroup/agent-browser#178`

Branch: `platform/p205-service-model-crate`

Target: `main`

Integration: merge through the protected pull-request workflow after the
provider-free extraction and changed-surface local validation

Source baseline: `2632e31ce34e62873598088d0c92c498357aafe2`

## Objective

Extract the canonical provider-free Service State model into a deep
`agent-browser-service-model` workspace crate. The crate will own durable
records, compatibility decoding, deterministic encoding, pure transitions,
and pure projections behind one coherent module interface. Filesystem custody,
process and browser observation, runtime ownership, command dispatch, HTTP,
MCP, dashboard transport, and platform effects remain CLI adapters.

The finished extraction must increase locality and leverage rather than move a
large file unchanged. Deleting the crate must force callers to recreate the
canonical aggregate, compatibility rules, transition invariants, and
projection semantics across many adapters. A types-only or re-export-only
crate fails this plan.

## Current State

The recorded source baseline is `2632e31c`. The 11,000-line
`cli/src/native/service_model.rs` owns most canonical records and substantial
pure derivation logic, but `ServiceState` also embeds durable types from
presentation capacity, principal state, profile lease and recovery, lifecycle,
browser retirement, crash regeneration, runtime ownership, authentication,
and challenge modules. Two model paths discover the current boot epoch through
the CLI process-identity implementation. Those upward dependencies prevent a
correct direct source move.

CodeGraph reports 1,051 callers of `ServiceState`, so a broad caller rewrite is
not the first packet. The current repository and in-memory repositories already
form the real persistence adapter seam. The extraction will preserve that seam
outside the crate while replacing direct model decisions incrementally.

P202 is integrated and closed. P197 is published on its challenge branch.
P204's validation implementation is integrated into `origin/main` and was
merged into P205 after Checkpoint 18. P205 may consume the installed validation
selector and crate-compartment support, but P204 still has an active catalog
entry. P205 therefore will not independently rewrite P204's roadmap, runbook,
or active-lane sections. This plan and issue #178 remain the durable P205
authorities.

## Consolidated Batch

1. Freeze representative Service State wire and behavior equivalence fixtures,
   an exact embedded-type dependency ledger, and a pre-extraction focused-loop
   baseline.
2. Move passive durable record definitions behind the new crate seam and make
   host observations explicit inputs without moving their CLI effect adapters.
3. Move the canonical aggregate, compatibility codec, pure derivations, and
   invariant-preserving transitions into the crate without a second model.
4. Cut CLI repositories and transport adapters over to the crate, then delete
   the old `cli/src/native/service_model.rs` implementation and transitional
   facade.
5. Use the integrated dedicated crate compartment and changed-surface
   selection, then measure the focused edit and test loop.

## Module Interface

The external seam has three entry-point families:

1. snapshot and codec: decode persisted bytes, encode canonical bytes, expose
   deterministic read-only records, and prepare a revision-bound snapshot;
2. transition: apply one typed pure transition atomically and return the next
   aggregate plus a typed receipt; and
3. projection: derive one deterministic typed view from Service State plus
   explicitly supplied observation context.

The crate exports canonical durable record vocabulary needed by adapters, but
it does not expose filesystem, clock, process, browser, HTTP, MCP, dashboard,
or provider ports. Those dependencies already have production and test
adapters outside the model. Introducing ports inside a pure module would create
hypothetical seams.

The final aggregate fields become private. During migration, narrowly scoped
compatibility access is allowed only with a recorded caller-removal ledger and
must disappear before the deletion test. Generic JSON mutation, arbitrary
closures as a public model interface, and duplicated CLI decisions are
forbidden.

## Dependency Direction

Allowed crate dependencies are initially limited to `serde`, `serde_json`,
`chrono`, and `agent-browser-lease-authority`. Any additional dependency needs
an explicit provider-free justification in this plan.

```text
agent-browser-lease-authority
            |
            v
agent-browser-service-model
            ^
            |
CLI repositories and runtime adapters
            |
            +-- filesystem and transaction custody
            +-- process and browser observation
            +-- HTTP, MCP, dashboard, and command transport
            `-- provider and platform effects
```

`agent-browser-desktop-services` and `agent-browser-challenge-control` must not
import the Service State model. P205 moves passive shared records only when the
result preserves one-way dependencies and leaves effect orchestration in its
own adapter.

## Delivery Sequence And Budget

### P0 | Equivalence Freeze And Red Architecture Contract

- Record representative current JSON, defaults, unknown-field round trip,
  incident ordering, profile readiness, tab-handle, controller-fence,
  abandoned-retirement, and stale-session behavior through stable tests.
- Record every upward type and observation dependency with its owner and target
  disposition.
- Add a red-capable architecture contract for the absent crate and forbidden
  upward imports.
- Measure the smallest current focused Service State edit and test loop once.

Exit: the migration has a falsifiable compatibility oracle, dependency ledger,
architecture guard, and baseline.

### P1 | Passive Durable Records And Observation Inputs

- Create the workspace crate and move one cohesive set of passive durable
  record definitions with no behavior duplication.
- Inject boot epoch and other host observations into pure classification paths.
- Keep process discovery, clocks, persistence, and effects in CLI adapters.
- Cut every affected CLI owner over to the canonical record definitions before
  moving the next set.

Exit: the crate compiles provider-free, owns real durable vocabulary, and the
architecture guard rejects upward imports.

### P2 | Canonical Aggregate And Codec

- Move `ServiceState`, schema compatibility, unknown-field preservation,
  deterministic encoding, revision access, and canonical records into the
  crate.
- Keep the CLI repository responsible for locks, replay, file replacement,
  recovery, and multi-file custody.
- Permit a transitional import facade only while required to keep the move
  reviewable. It may contain re-exports only and has an explicit deletion gate.

Exit: there is one canonical aggregate and one persisted codec.

### P3 | Pure Transitions And Projections

- Move derived-view refresh, readiness, incident derivation, profile allocation,
  tab-handle derivation, retained-display classification, controller fencing,
  and lease admission into typed pure transitions and projections.
- Replace direct caller decisions rather than layering new calls over old ones.
- Delete superseded implementation-coupled tests when equivalent interface
  tests prove the retained risk.

Exit: callers use the module interface for model decisions and tests use the
same seam.

### P4 | Seam Closure And Measurement

- Delete the CLI model implementation and transitional facade.
- Prove zero duplicate canonical record and derivation definitions.
- Use P204's integrated dedicated crate compartment and validation selector
  route without weakening the comprehensive lane.
- Measure the same focused edit and test loop against the baseline. Report
  locality and correctness regardless of timing; claim acceleration only from
  comparable evidence.

Exit: deletion test, architecture guard, focused crate tests, affected CLI
adapter tests, formatting, strict workspace Clippy, and local changed-surface
validation pass. GitHub CI is explicitly skipped for this user-directed run.

### Bounds

- Maximum work-unit attempts per packet: 3.
- Maximum review and rework cycles: 1.
- Maximum consecutive hardening checkpoints: 2.
- Reassess after two checkpoints or 30 active minutes without outcome progress.
- Overall effort ceiling: 720 active minutes through the provider-free source
  extraction, local qualification, publication, and handoff.
- First evidence deadline: within 60 active minutes, publish the plan plus the
  dependency ledger, red architecture contract, and first focused baseline or
  record the exact resource reason one baseline cannot run.
- Expensive Cargo invocations remain capped by the repository admission wrapper.
  No comprehensive suite, release build, browser test, or CI wait belongs to
  this run.

## Parallel Plan And Worker Assignments

The P205 owner retains the critical path, interface decision, shared-file
coordination, integration, validation interpretation, and completion claim.
Subagents remain inside this worktree and receive no branch, runtime, build
candidate, or forge custody.

Completed design comparison:

- `/root/service_model_minimal`: requested `gpt-5.6-sol`, medium. Designed the
  minimum snapshot, transition, and projection interface and the final deletion
  test.
- `/root/service_model_flexible`: requested `gpt-5.6-terra`, medium. Designed a
  migration-flexible aggregate and identified compatibility and adapter risks.
- `/root/service_model_common`: requested `gpt-5.6-luna`, medium. Optimized the
  common CLI caller path and produced the first embedded-dependency inventory.

Runtime-reported effective model and effort were not independently exposed.
The primary selected the minimal three-family interface, retained the flexible
design's staged compatibility strategy, and rejected a permanent facade or a
generic JSON transition surface.

Implementation fan-out after this plan checkpoint:

| Worker | Requested route | Exact write scope | Evidence and stop condition |
| --- | --- | --- | --- |
| dependency and fixture worker | `gpt-5.6-sol`, medium | read-only dependency ledger, fixture recommendations, no edits | return exact type owners and stable invariants once |
| crate extraction worker | `gpt-5.6-terra`, medium | `crates/agent-browser-service-model/**` only | one cohesive provider-free record packet plus focused tests; stop at first unresolved upward dependency |
| architecture guard worker | `gpt-5.6-luna`, medium | one new P205 guard script and its direct fixture only; no package or CI files | guard detects absent crate and forbidden upward imports; stop after one repair |
| primary | strongest current session | manifests, CLI cutover, plan, reconciliation, validation, commits, and publication | integrate worker results and preserve one model |

Workers may not spawn children. Parallel writes must remain disjoint. The
primary inspects each returned diff and decisive evidence without repeating the
worker investigation.

## Checkpoint 1 | P0 Freeze And First P1 Cutover

State transition: the plan and issue are published, the dependency and fixture
freeze is complete, the architecture guard is green, and the first cohesive
durable family is owned by the new crate. P205 is still in progress because the
canonical aggregate and remaining record families remain in the CLI.

Acceptance state and progress classification: P0 is accepted for the first
packet and P1 has begun. This is outcome progress, not hardening. The extracted
profile-seeding family preserves the existing wire shape and owns its lifecycle
severity, message, lease-blocking, key, and codec behavior. The CLI retains a
temporary re-export only; it no longer defines a duplicate implementation.

Evidence:

- source baseline focused loop:
  `service_state_round_trips_nested_entities` passed after a cold build in
  171.01 seconds;
- crate loop: three profile-seeding compatibility and behavior tests passed in
  4.08 seconds;
- affected CLI loop: six profile-seeding command, MCP, action, model, and output
  tests passed after the expected monolith rebuild in 132 seconds;
- warm compatibility loop: the nested aggregate round-trip passed in 0.44
  seconds;
- architecture guard fixtures cover the absent crate, a clean crate, forbidden
  Cargo dependencies and CLI paths, and forbidden Rust imports; both the
  fixture suite and repository guard pass;
- dependency ledger identified 74 directly reachable Service State model types,
  plus presentation, migration, profile lifecycle, retirement, ownership,
  authentication, challenge, request, and terminal record families. Existing
  lease-authority types stay canonical in their provider-free crate. Host and
  process observations stay in CLI adapters.

Material blockers: P204 still owns shared CI, validation, roadmap, runbook, and
active-lane files. Their integration is not required for the next disjoint
model packet, but P205 will not edit them until that ownership is released.

Next action: publish this first extraction checkpoint, then move the next
cohesive model family and replace hidden boot-epoch reads with explicit
observation inputs before attempting the aggregate move.

## Checkpoint 2 | Profile Readiness And Observation Inputs

State transition: the crate now also owns profile readiness, compatibility
evidence, browser-build parsing, allocation and keyring policy, their wire-value
contracts, and the explicit freshness-evidence decision. Both Service State
paths that previously read the host boot epoch now receive it as an explicit
input sampled by CLI adapters.

Acceptance state and progress classification: the second P1 packet is accepted
locally. This is outcome progress. The CLI compatibility module re-exports the
canonical crate types while all duplicate definitions and the duplicate
freshness predicate are removed. Retained-display classification now uses a
pure fail-closed prior-boot comparison.

Evidence:

- eight crate tests pass, including full readiness enum/default coverage,
  browser-build aliases, record wire shape, and freshness decisions;
- four focused CLI readiness derivation tests pass;
- the CLI collection-record contract and persisted-freshness tests pass;
- stale-session expiry passes with an explicit test boot epoch;
- current, prior, missing, and unavailable boot-epoch comparisons pass as a
  pure model test;
- the architecture guard and formatting checks remain green.

Material blockers: the aggregate still embeds durable records owned by other
CLI modules, so it cannot move without first relocating those records or
depending on an existing provider-free crate. P204 still owns the shared
validation and planning surfaces.

Next action: publish this checkpoint, then select the next self-contained
durable family that reduces aggregate cross-module dependencies without moving
effect adapters.

## Checkpoint 3 | Entity Provenance And Profile-Access Kernel

State transition: the crate now owns in-memory entity provenance plus the
complete provider-free profile-access policy kernel. The CLI policy module is a
named transitional re-export and one failure-projection adapter test. Service
State fields for profile policy and child access refer directly to crate types,
removing three aggregate upward edges.

Acceptance state and progress classification: this P1 dependency-reduction
packet is accepted locally and materially deepens the crate. The move preserves
admission decisions, inherited child authority, reconnect fencing, policy
canonicalization, revision conflicts, drain and eviction planning,
deterministic identifiers, and privacy-bounded evidence. Filesystem custody,
runtime observation, failure classification, and repository mutation remain
CLI adapters.

Evidence:

- 21 crate tests pass in 2.11 seconds, including 11 profile-access authorization
  and transition tests, two entity-source tests, and the eight earlier profile
  tests;
- strict Clippy for `agent-browser-service-model` passes;
- the focused CLI privacy-projection adapter test passes without warnings;
- the profile-policy repository mutation and configured-entity overlay tests
  pass through the compatibility facade;
- formatting, architecture guard, fixture guard, and diff checks remain green;
- the CLI policy file shrinks from a 1,606-line implementation and test module
  to a narrow named facade plus one adapter-specific regression test.

Material blockers: `BrowserProfile` can now move after its remaining local
record dependencies are extracted. The full aggregate still depends on several
other CLI-owned durable record families listed in Checkpoint 1.

Next action: publish this deep-module checkpoint, then move the neutral profile
record family and `BrowserProfile` without moving its CLI derivation or
persistence adapters.

## Checkpoint 4 | Browser Profile Aggregate

State transition: the crate now owns `BrowserProfile`, browser host, profile
origin and class, registration, and profile and site-policy source projection
records. Readiness derivation, source joins, overlay precedence, persistence,
profile selection, launch parsing, and runtime decisions remain CLI adapters.

Acceptance state and progress classification: this P1 record packet is accepted
locally. The deletion scan finds each moved profile symbol exactly once under
the crate and no duplicate CLI definitions. The high-fan-out CLI import seam is
a re-export only.

Evidence:

- 25 crate tests pass in 0.67 seconds on the final warm packet candidate;
- the representative external-profile test round-trips nested access policy,
  readiness, registration, compatibility evidence, host, class, and origin;
- aggregate defaults and exact host, class, and origin wire labels are frozen;
- the nested Service State round-trip and collection wire-contract tests pass
  after cutover; the warning-free nested round-trip rerun passes after narrowing
  unused facade imports;
- formatting, architecture guard, and duplicate-definition checks pass.

Material blockers: the crate still does not own the full Service State
aggregate. Remaining dependencies include presentation capacity, migration,
lease and lifecycle receipts, browser retirement and recovery transactions,
runtime ownership, authentication, challenge tasks, and the remaining core
browser, session, tab, job, route, event, and policy records.

Next action: publish this checkpoint and reassess the next dependency cluster
against the remaining delivery budget before expanding the move.

## Checkpoint 5 | Session And Tab Model Family

State transition: the crate now owns browser session, browser tab, stable tab
handle, trace-filter, actor, lease, cleanup, profile-selection, profile-lease,
and tab-lifecycle records and wire constants. The model depends directly on
the provider-free Lease Authority principal provenance type. Session expiry,
disconnect handling, profile selection, handle refresh, and Service State
mutation remain CLI adapters.

Acceptance state and progress classification: this P1 dependency-reduction
packet is accepted locally. The compatibility module re-exports the canonical
crate records, and the deletion scan finds no duplicate CLI definition. The
former CLI-private principal and work-lease fields are public at the crate
boundary because CLI authority adapters must inspect and mutate them; their
serde attributes and wire behavior are unchanged.

Evidence:

- four focused crate tests pass for defaults, wire names and omission rules,
  actor inference, and exact constant values;
- the CLI collection wire-contract and nested Service State round-trip tests
  pass through the compatibility facade;
- focused handle refresh, stale-session expiry, and profile-child disconnect
  isolation tests pass;
- strict workspace Clippy passes with warnings denied after removal of one
  unused transitional re-export;
- formatting, architecture guard, guard fixtures, duplicate-definition scan,
  and diff checks pass;
- no browser, provider, runtime, install, staging, production, release, or
  GitHub CI effect was performed.

Material blockers: the full Service State aggregate still depends on the
browser-process family, which closes over health, process identity, view
stream, and tab-handle records, plus the remaining job, remote-view, monitor,
incident, challenge, provider, receipt, and transaction families. P204 still
owns the shared validation and roadmap surfaces.

Next action: publish this checkpoint. The next architecture packet should move
the provider-free remote-view record family before the browser-process family,
so `ViewStream` and route controller fencing have one canonical downward owner
without pulling provider orchestration into the crate.

## Checkpoint 6 | Presentation Records And Controller Fencing

State transition: the crate now owns the provider-free presentation record
family: durable handoffs and presentation receipts, display allocations,
remote-view routes, route-pool entries, acquisition rollback snapshots, viewer
leases, view streams, provider and input enums, and the retained-allocation
candidate projection record. Route controller advancement and stream
projection move with the records as pure deterministic fencing behavior.

Acceptance state and progress classification: this P1 dependency-reduction
packet is accepted locally. The source module is named `presentation` because
the architecture guard reserves `remote_view` for effect adapters. Route
checkout, handoff resolution, Guacamole and RDP integration, display capture,
viewer effects, health reconciliation, retained-allocation classification,
and full Service State joins remain in CLI adapters.

Evidence:

- five focused crate tests pass for exact defaults, camel-case and omission
  rules, strict receipt round-trip, provider enum values, retained-candidate
  JSON, and same-ID controller regrant fencing;
- CLI collection wire-contract, nested aggregate, durable handoff, legacy
  controller epoch, and former-controller and ABA fencing tests pass;
- human-takeover controller transition and expired-viewer reconciliation
  adapter tests pass;
- strict workspace Clippy passes with warnings denied;
- formatting, architecture guard, guard fixtures, duplicate-definition scan,
  and diff checks pass;
- a test-only `ProfileChildAccess` import repaired an older narrowed façade
  without restoring the unused production re-export;
- no browser, provider, runtime, install, staging, production, release, or
  GitHub CI effect was performed.

Material blockers: `BrowserProcess` can now move after extracting its health,
provenance, and process-identity record closure. Process capture and owner
observation must remain CLI adapters. The full Service State aggregate still
depends on job, monitor, incident, challenge, provider, receipt, transaction,
and presentation-capacity families. P204 still owns the shared validation and
roadmap surfaces.

Next action: publish this checkpoint, then extract the provider-free browser
process family without importing OS process observation or retained-owner
authority into the model crate.

## Checkpoint 7 | Browser Process Records

State transition: the crate now owns browser process, health observation,
record provenance, protected-owner observation, and service process-identity
records, plus the provider-free recorded process identity. The CLI
`process_identity` module re-exports that record while retaining boot-epoch,
namespace, executable, PID, handle, signaling, and platform observation.

Acceptance state and progress classification: this P1 dependency-reduction
packet is accepted locally. `BrowserProcess` now closes entirely over
crate-owned host, view-stream, tab-handle, health, provenance, and identity
records. The protected-owner record still rejects
`operationalAuthority=true` during deserialization and remains observational,
never effect authority.

Evidence:

- five focused crate test groups pass for browser-health wire labels, process
  defaults and omissions, provenance and observation defaults, protected-owner
  fail-closed decoding, and nested process-identity round trips;
- CLI collection wire-contract and nested Service State round-trip tests pass;
- focused health-evidence retention and protected-owner projection tests pass;
- exact process start-token and PID-reuse policy tests pass through the CLI
  compatibility re-export;
- strict workspace Clippy passes with warnings denied;
- formatting, architecture guard, guard fixtures, duplicate-definition scan,
  and diff checks pass;
- no process observation, browser, provider, runtime, install, staging,
  production, release, or GitHub CI effect was performed.

Material blockers: the full Service State aggregate still depends on job,
monitor, incident, challenge, provider, receipt, transaction, migration,
presentation-capacity, runtime-owner, authentication, recovery, and lifecycle
families. P204 still owns the shared validation and roadmap surfaces.

Next action: publish this checkpoint and reassess aggregate dependency depth.
Prefer another cohesive durable family over introducing an incomplete second
Service State aggregate.

## Checkpoint 8 | Site, Challenge, And Provider Policy Kernel

State transition: the crate now owns site policy, pacing, provider inventory,
and challenge records and enums, plus the deterministic interaction and
provider-decision kernel. One small interface now computes interaction risk,
pacing projection, provider selection, capability gaps, and challenge strategy
without Service State, persistence, provider execution, or runtime access.

Acceptance state and progress classification: this P1 deep-module packet is
accepted locally. The CLI access planner selects profiles, policies,
challenges, and providers from Service State, then calls the crate kernel.
Built-in named-site catalogs, URL matching, configuration precedence,
persistence, monitoring, HTTP and MCP projection, and provider or challenge
effects remain CLI adapters.

Evidence:

- five focused crate tests pass for record defaults and wire names, exact enum
  constants, standard/manual/hardened interaction decisions, sorted and
  deduplicated allow-list-aware provider selection, capability-gap ordering,
  and deny precedence;
- CLI collection wire-contract, stable site-policy serialization, and
  provider/challenge model tests pass through the compatibility facade;
- access-plan provider-fit, missing-capability, and pacing/risk tests pass
  against the extracted kernel;
- configured-entity overlay, built-in site-policy precedence, site-policy
  upsert defaults, and provider body-ID rejection adapter tests pass;
- strict workspace Clippy passes with warnings denied after production and
  test-only facade imports were separated;
- formatting, architecture guard, guard fixtures, duplicate-definition scan,
  and diff checks pass;
- no provider execution, credential, browser, runtime, install, staging,
  production, release, or GitHub CI effect was performed.

Material blockers: `ServiceJob` and `ServiceEvent` still depend on CLI-owned
request provenance, terminal outcome, and failure-recourse records.
`ServiceIncident` can move only after those dependencies are neutral or its
aggregate derivation seam is deliberately split. Receipt and transaction
families remain distributed across their owning CLI adapters. P204 still owns
the shared validation and roadmap surfaces.

Next action: publish this checkpoint. Extract request provenance, terminal
outcome, and failure-recourse records as the prerequisites for a cohesive
job, event, and incident model packet; keep request handling, failure
classification, and incident derivation in CLI adapters.

## Checkpoint 9 | Request And Outcome Prerequisites

State transition: the crate now owns request-provenance, terminal-outcome, and
failure-recourse records plus the deterministic failure classifier. The CLI
retains environment observation, ingress capture, native session attribution,
and failed-response decoration as effect-facing adapters.

Acceptance state and progress classification: this P1 prerequisite packet is
accepted locally. It removes the remaining CLI-owned model dependencies that
prevented `ServiceJob`, `ServiceEvent`, and `ServiceIncident` from moving as
one cohesive durable family. The terminal projection and failure classifier
are provider-free and deterministic; request capture still reads deployment
environment only in the CLI.

Evidence:

- all 55 service-model crate unit and integration tests pass, including new
  wire-contract, privacy-bound, failure-classification, and terminal-projection
  coverage;
- four CLI provenance-capture tests pass through the new adapter;
- three CLI response-decoration tests pass while the full pure classifier
  matrix now runs in the crate;
- Service collection wire-shape and nested-state round-trip tests pass;
- strict workspace Clippy passes with warnings denied;
- formatting, architecture guard, guard fixtures, duplicate-definition scan,
  and diff checks pass;
- no browser, profile, provider, runtime, install, staging, production,
  release, or GitHub CI effect was performed.

Material blockers: the canonical aggregate remains in the CLI. Job, event,
and incident records are now dependency-ready, while receipt, transaction,
migration, runtime-owner, authentication, recovery, lifecycle, and capacity
families remain distributed across their effect adapters. P204 still owns the
shared validation and roadmap surfaces.

Next action: publish this checkpoint, then extract `ServiceJob`,
`ServiceEvent`, and `ServiceIncident` together with only their deterministic
record-level helpers. Keep incident derivation, journal persistence, transport,
monitoring, and retention in CLI adapters.

## Checkpoint 10 | Job, Event, And Incident Records

State transition: the crate now owns canonical job, event, and incident
records, their lifecycle enums, and their stable wire-value constants. The CLI
keeps one compatibility re-export while Service State joins, incident
derivation, queue mutation, persistence, clocks, timeouts, retention,
monitoring, activity projection, and transport remain effect adapters.

Acceptance state and progress classification: this P1 durable-family packet is
accepted locally. The extraction closes the dependency chain opened in
checkpoint 9 without pulling the `ServiceState` aggregate or operational
derivation into the crate. High-fanout CLI consumers continue through the
existing `service_model` facade with no broad import churn.

Evidence:

- all 60 service-model crate unit and integration tests pass, including job,
  event, incident default and wire-contract coverage;
- three CLI record-contract tests and the nested Service State round-trip pass;
- six derived-view incident tests pass across browser, route, route-pool, and
  operator-metadata behavior;
- timeout terminalization, incident activity, persisted-incident backfill, and
  18 MCP resource projection tests pass;
- strict workspace Clippy passes with warnings denied;
- formatting, architecture guard, guard fixtures, duplicate-definition scan,
  and diff checks pass;
- no browser, profile, provider, runtime, install, staging, production,
  release, or GitHub CI effect was performed.

Material blockers: `ServiceState` still aggregates CLI-owned monitor, receipt,
transaction, migration, runtime-owner, authentication, recovery, lifecycle,
and capacity families. Moving the aggregate before those families are
classified would create a shallow crate or pull effects upward. P204 still
owns the shared validation and roadmap surfaces.

Next action: publish this checkpoint and inventory the remaining `ServiceState`
field families by provider-free closure. Select the next cohesive family with
the highest aggregate-unblocking value; do not move incident derivation or
other effect-facing joins.

## Checkpoint 11 | Monitor Records

State transition: the crate now owns the monitor record, target and state
enums, legacy defaults, and stable state values. Monitor scheduling, clocks,
probe execution, job admission, persistence, failure mutation, incident
derivation, and Service State joins remain CLI adapters.

Acceptance state and progress classification: this P1 record-family packet is
accepted locally. It removes another direct aggregate field dependency without
turning the model crate into a scheduler or monitoring runtime.

Evidence:

- all 63 service-model crate unit and integration tests pass, including three
  monitor default and wire-contract tests;
- CLI collection wire-contract, due-work filtering, upsert defaults, state
  update, failure reset, and MCP sorted-resource tests pass;
- strict workspace Clippy passes with warnings denied;
- formatting, architecture guard, guard fixtures, duplicate-definition scan,
  and diff checks pass;
- no monitor probe, browser, profile, provider, runtime, install, staging,
  production, release, or GitHub CI effect was performed.

Material blockers: the aggregate still contains operational receipts,
transactions, migrations, authority registries, runtime-owner state,
authentication and challenge-task state, presentation capacity, and flattened
forward-compatible fields. `BrowserCapabilityRegistry` is provider-free but
remains an explicitly advisory draft of opaque JSON arrays; moving its envelope
now would add little domain depth and could imply authority that the source
contract denies.

Next action: publish this checkpoint. Prefer the provider-free control-plane
and reconciliation snapshot records next, then the profile-lifecycle
authorization record closure. Defer the capability registry until its schema
or ownership graduates from advisory draft status.

## Checkpoint 12 | Operational Snapshot Records

State transition: the crate now owns the durable reconciliation and
control-plane snapshot records. The records remain deliberately passive:
status computation, health observation, timestamps, queue and browser counts,
capacity validation, Service State mutation, persistence, and transport stay
in CLI adapters.

Acceptance state and progress classification: this small P1 aggregate-
unblocking packet is accepted locally. No new semantic API was invented; the
only existing queue-capacity invariant belongs to the typed status projection
adapter and remains there.

Evidence:

- all 65 service-model crate unit and integration tests pass, including empty
  legacy decode and populated camel-case snapshot fixtures;
- nested Service State round-trip, status and collection response contract,
  reconciliation response contract, and runtime-census revision-isolation
  tests pass;
- strict workspace Clippy passes with warnings denied;
- formatting, architecture guard, guard fixtures, duplicate-definition scan,
  and diff checks pass;
- no reconciliation, health observation, browser, runtime, install, staging,
  production, release, or GitHub CI effect was performed.

Material blockers: the aggregate still stores multiple effect receipts and
transactions, focused-crate authority state, draft capability data, and
authentication or challenge execution state. The next lifecycle packet must
keep logical authorization separate from fresh physical proof and effect
settlement.

Next action: publish this checkpoint. Extract the profile-lifecycle
authorization record closure and its deterministic registration kernel only;
leave physical observations, proof construction, effect settlement, runtime
commands, and state mutation in CLI adapters.

## Checkpoint 13 | Profile Lifecycle Authorization Kernel

State transition: the crate now owns the lifecycle authorization, proof, and
minimal effect-receipt wire records plus deterministic authorization
registration. Registration validates the assurance floor and canonical target
set, preserves exact error codes, and is idempotent for an identical
authorization while rejecting conflicting reuse of the same identifier.

Acceptance state and progress classification: this P1 deep-module packet is
accepted locally. The CLI compatibility facade preserves existing internal
paths. Fresh daemon/browser observation, proof construction, grace-time checks,
policy fencing against current Service State, effect settlement, recovery,
clock access, repository mutation, and runtime commands remain CLI adapters.

Evidence:

- all 72 service-model crate unit and integration tests pass, including seven
  lifecycle wire, unknown-field, state, validation, canonicalization,
  idempotency, and conflict tests;
- all five CLI lifecycle authorization-to-effect boundary tests pass;
- the repository mutation test proves that profile draining and authorization
  remain atomically persisted through the extracted kernel;
- strict workspace Clippy passes with warnings denied;
- formatting, architecture guard, guard fixtures, duplicate-definition scan,
  and diff checks pass;
- no profile eviction, browser observation, browser close, runtime, install,
  staging, production, release, or GitHub CI effect was performed.

Delegation receipt:

- `/root/p205_lifecycle_impl`, requested `gpt-5.6-sol` at medium effort,
  completed the isolated new-file implementation; the primary reviewed and
  integrated it;
- `/root/p205_lifecycle_consumers`, requested `gpt-5.6-luna` at medium effort,
  completed the read-only consumer and focused-test inventory;
- `/root/p205_lifecycle_boundary`, requested `gpt-5.6-terra` at medium effort,
  returned no blocking findings in the closed-world authority-boundary review.

Material blockers: profile-lease records still depend on the principal
continuity and provenance seam; effect transaction families still encode
runtime joins and should not move as passive records. The canonical aggregate
therefore remains in the CLI.

Next action: publish this checkpoint. Resolve the principal-provenance seam
against the existing Lease Authority crate before moving profile-lease durable
records. Prefer direct reuse of the focused authority crate over duplicating
principal or lease concepts in the service-model crate.

## Checkpoint 14 | Profile Lease Durable Records

State transition: the crate now owns the principal-continuity recourse enum,
the six passive profile-lease record shapes, and their three schema constants.
The records reuse `ServicePrincipalProvenance` directly from Lease Authority,
so the extraction introduces neither a second authority vocabulary nor a
dependency cycle. The CLI modules remain compatibility facades.

Acceptance state and progress classification: this P1 aggregate-unblocking
packet is accepted locally. Projection from canonical claims and legacy state,
authentication, rejoin, renew, release, reconciliation planning and
application, time, process and runtime-owner joins, repository custody, and
event emission remain CLI adapters. Failure and error types also remain local
until their effect boundary is independently resolved.

Evidence:

- all 79 service-model crate unit and integration tests pass, including seven
  new exact wire-name, camel-case, nested-record, round-trip, and unknown-field
  tests;
- 67 focused CLI tests containing `profile_lease` pass across command, MCP,
  HTTP, routing, control-plane, recovery, resource, and lease adapter surfaces;
- strict workspace Clippy passes with warnings denied;
- formatting, architecture guard, guard fixtures, duplicate-definition scan,
  and diff checks pass;
- no lease mutation, profile recovery, browser, runtime, install, staging,
  production, release, or GitHub CI effect was performed.

Delegation receipt:

- `/root/p205_lease_record_inventory`, requested `gpt-5.6-luna` at medium
  effort, completed the read-only record and consumer inventory;
- `/root/p205_lease_authority_types`, requested `gpt-5.6-sol` at medium effort,
  verified dependency direction and the canonical authority-type seam;
- `/root/p205_lease_seam_review`, requested `gpt-5.6-terra` at high effort,
  independently approved direct reuse without a new authority vocabulary;
- `/root/p205_continuity_model`, requested `gpt-5.6-luna` at medium effort,
  implemented the isolated principal-continuity model and focused tests;
- `/root/p205_profile_lease_model`, requested `gpt-5.6-sol` at medium effort,
  implemented the isolated passive record family and focused tests; the
  primary reviewed and integrated both implementation packets.

Material blockers: the canonical `ServiceState` aggregate still contains
CLI-owned effect transactions, authority joins, migration behavior, and draft
capability data. Those families require a fresh dependency review rather than
being treated as passive records.

Next action: publish this checkpoint, then reassess the remaining aggregate
edges and select one closed provider-free family. Do not move the draft browser
capability registry or any runtime transaction merely to reduce line count.

## Checkpoint 15 | Browser Retirement Records

State transition: the crate now owns the passive inert-browser retirement
plan, terminal receipt, contamination report, and exact plan and receipt schema
constants. Planning against current browser state, process and runtime-owner
checks, evidence hashing, clocks, compare-and-swap application, repository
mutation, and command transport remain in the CLI adapter.

Acceptance state and progress classification: this small P1 dependency-
reduction packet is accepted locally. It removes another CLI-owned type from
the canonical aggregate without treating process observation or record
retirement as model authority.

Evidence:

- all 83 service-model crate unit and integration tests pass, including four
  new retirement wire, schema, camel-case, round-trip, and unknown-field tests;
- all 17 focused CLI tests containing `browser_retirement` pass, including the
  three exact inert-record retirement tests and the adjacent abandoned-browser
  effect-boundary tests selected by the filter;
- strict workspace Clippy passes with warnings denied;
- formatting, architecture guard, guard fixtures, duplicate-definition scan,
  and diff checks pass;
- the first crate attempt found one test-only `&&str` map-lookup error; the
  bounded fixture correction passed on the single rerun;
- no browser retirement, process signal, profile, runtime, install, staging,
  production, release, or GitHub CI effect was performed.

Delegation receipt:

- `/root/p205_remaining_family_audit`, requested `gpt-5.6-luna` at medium
  effort, ranked the remaining record families and approved the passive
  browser-retirement boundary while deferring its runtime checks;
- `/root/p205_aggregate_blockers`, requested `gpt-5.6-sol` at high effort,
  classified 29 of 44 aggregate fields as already downward-owned and 15 as
  remaining blockers, and rejected a partial or value-typed aggregate;
- `/root/p205_browser_retirement_model`, requested `gpt-5.6-luna` at medium
  effort, implemented only the isolated model record file and wire tests; the
  primary reviewed, corrected one fixture, and integrated it.

Material blockers: the aggregate still cannot move. The next smallest closed
record family is profile-policy migration, while recovery/reset receipts and
presentation capacity need slightly wider supporting closures. Runtime-owner,
abandoned-retirement, crash-regeneration, authentication, and challenge task
state require explicit separation from effect custody. The draft capability
registry must not be promoted into canonical model authority.

Next action: publish this checkpoint, then extract only the passive profile-
policy migration entry and report records. Keep legacy materialization, schema
stamping, aggregate mutation, and all migration I/O in the CLI.

## Checkpoint 16 | Profile Policy Migration Records

State transition: the crate now owns the passive profile-policy migration entry
and report records plus their schema constant. The records directly reuse the
crate-owned profile access mode. Legacy classification and materialization,
migration identifier hashing, report attachment, schema stamping, aggregate
mutation, compatibility decoding, staging, persistence, and filesystem I/O
remain in the CLI.

Acceptance state and progress classification: this small P1 aggregate-
unblocking packet is accepted locally. The compatibility facade preserves the
existing Service State path and downstream JSON consumers without moving codec
or migration authority into a record module.

Evidence:

- all 88 service-model crate unit and integration tests pass, including five
  profile-policy migration wire, schema, mode, round-trip, and unknown-field
  tests; the unknown-field fixture covers both the report and its nested entry;
- all 26 CLI Service State migration tests pass;
- the nested Service State round-trip and workstation access-axis migration
  tests pass through the compatibility facade;
- strict workspace Clippy passes with warnings denied;
- formatting, architecture guard, guard fixtures, duplicate-definition scan,
  and diff checks pass;
- no migration application, repository write, profile, browser, runtime,
  install, staging, production, release, or GitHub CI effect was performed.

Delegation receipt:

- `/root/p205_policy_migration_model`, requested `gpt-5.6-luna` at medium
  effort, implemented only the isolated passive record file and focused tests;
- `/root/p205_policy_migration_review`, requested `gpt-5.6-sol` at medium
  effort, found no blocking dependency, inventoried direct and downstream wire
  consumers, and identified the nested-entry unknown-field coverage added by
  the primary before integration.

Material blockers: the canonical aggregate still depends on presentation
capacity authority, recovery/reset receipts, runtime-owner authority, several
effect transaction families, authentication and challenge records, codec and
migration behavior, and the advisory capability registry.

Next action: publish this checkpoint. Review recovery and reset terminal
receipts as separate passive closures, then evaluate the presentation-capacity
kernel. Do not pull recovery execution or environment-derived capacity joins
into the model crate.

## Checkpoint 17 | Profile Recovery And Reset Receipts

State transition: the crate now owns the terminal profile-recovery and profile-
reset receipt records, their shared acquisition-state and reset-scope enums,
and their exact receipt schema constants. Recovery and reset plans, evidence
joins, parsing, sealing, verification, replay decisions, repository mutation,
runtime effects, acquisition retry, and command transport remain in the CLI
adapter behind the existing compatibility seam.

Acceptance state and progress classification: this P1 aggregate-unblocking
packet is accepted locally and moves two durable aggregate field types to the
canonical model module. The module interface stays typed and small; adding a
trait or port for one in-process adapter would create a hypothetical seam.
Wire compatibility gains locality in the model crate while effect semantics
retain locality in the CLI workflow.

Evidence:

- all 98 service-model crate unit and integration tests pass, including ten
  new receipt round-trip, enum wire-value, optional-field, camel-case, schema,
  and unknown-field tests;
- all 37 CLI profile recovery and reset workflow tests pass through the
  compatibility adapter;
- a new populated Service State aggregate test proves both receipt maps round
  trip with exact enum values and optional-field omission;
- the mixed-version recovery-artifact reader test passes;
- strict workspace Clippy passes with warnings denied;
- formatting, architecture guard, guard fixtures, duplicate-definition scan,
  and diff checks pass;
- no recovery, reset, repository write, profile, browser, runtime, install,
  staging, production, release, or GitHub CI effect was performed.

Delegation receipt:

- `/root/p205_recovery_receipt_model`, requested `gpt-5.6-luna` at medium
  effort, implemented only the recovery receipt model and focused fixtures;
- `/root/p205_reset_receipt_model`, requested `gpt-5.6-luna` at medium effort,
  implemented only the reset receipt model and focused fixtures;
- `/root/p205_receipt_seam_review`, requested `gpt-5.6-sol` at high effort,
  found no architecture blocker and identified the populated aggregate codec
  fixture and legacy optional-field readback added by the primary.

Material blockers: moving receipts does not move the recovery workflow. The
canonical aggregate still depends on presentation-capacity authority, runtime-
owner authority, effect transaction families, authentication and challenge
records, codec and migration behavior, and the advisory capability registry.
The final crate deletion test remains pending.

Next action: publish this checkpoint, then evaluate the presentation-capacity
authority as a deep provider-free kernel. Move only deterministic inventory,
admission, and state-transition behavior; keep environment-derived Service
State joins and provider/runtime effects in CLI adapters.

## Checkpoint 18 | Presentation Capacity Interface Freeze

State transition: the presentation-capacity deep-module seam is frozen before
parallel implementation. This checkpoint changes no runtime behavior. It
classifies every dependency as in-process and rejects both a callback or port
interface and a generic command/outcome wrapper.

Acceptance state and progress classification: this is blocker reduction for
the next P1 deep-kernel packet. The chosen interface combines a concrete,
immutable `PresentationCapacityObservations` envelope with explicit domain
methods for admission, bound admission, transition, release and dispatch,
binding reflection, quarantine, reconciliation, and projection. The common
caller remains one typed method call. The CLI is the sole adapter that derives
observations from Service State; the model module remains the sole owner of
capacity policy.

The observation envelope carries one entry per durable slot in inventory
order. Each entry binds the observed slot, route, and display identities and
contains only raw facts:

- whether an acquisition lease is current;
- whether a human controller is current;
- whether a non-controller viewer conflicts with staging;
- the live durable-handoff browser identities substantiated by current view
  streams;
- the authoritative browser identity resolved by the CLI route, display,
  browser, and pending-acquisition join.

The model converts those facts into limiting resources in the existing exact
precedence order. It owns constructor invariants, pressure-prefix admission,
protected reserves, browser exclusion, first-slot selection, bound observation
and recovery semantics, route-switch migration fencing, queue bounds, priority
aging, FIFO tie-breaking, transition fencing, scene generation, quarantine,
release and dispatch, binding reflection, reconciliation mutation, warning
ordering, projections, error strings, and durable serde compatibility.

The CLI retains inventory qualification from route, display, and pool records;
all Service State joins; provider and environment interpretation; process and
browser observation; repository custody; persistence; and runtime effects.
There is one in-process adapter, so no trait or port is introduced.

Compatibility freeze:

- preserve every current serde attribute, default, omission rule, enum spelling
  and order, error string, slot-vector ordering, and permissive decode behavior;
- preserve request clock advancement and error precedence;
- preserve the difference between conflict-free reserve counting during initial
  admission and all-warm-slot reserve counting during dispatch;
- preserve acquisition, controller, staging-viewer, then durable-handoff
  conflict precedence and minimum limiting-resource selection across slots;
- preserve bounded priority aging, insertion-order tie-breaking, and the exact
  transition graph;
- preserve bound-release behavior and route-switch destination custody;
- preserve the current release-and-dispatch quirk that advances the queue clock
  before a missing-slot result and clears a found slot before inventory-error
  dispatch suppression;
- preserve reconciliation fences for leased or non-warm/non-active slots.

Design comparison and model-choice receipt:

- `/root/p205_capacity_minimal_interface`, requested `gpt-6-astra` at high
  effort, maximized leverage through a three-entry command interface and found
  several hidden compatibility quirks;
- `/root/p205_capacity_flexible_interface`, requested `gpt-5.6-sol` at high
  effort, designed a sealed typed-operation kernel and a complete observation
  envelope;
- `/root/p205_capacity_common_interface`, requested `gpt-5.6-luna` at medium
  effort, optimized the ordinary caller and rejected a generic conflict trait;
- the primary selected the explicit-method and concrete-observation hybrid to
  avoid both broad outcome unpacking and operation-type interface growth.

Deletion test: after cutover, deleting the model module must force callers to
recreate durable wire compatibility, inventory validation, reserve arithmetic,
queue aging, exclusion, conflict precedence, bound and route-switch semantics,
transition fencing, quarantine, dispatch, and reconciliation. The CLI module
must contain only observation and Service State adapter logic, not a duplicate
decision path.

Next action: implement the provider-free capacity module and observation
records, then cut CLI callers over in bounded slices. Move records and policy
together; do not stop at a types-only extraction. Retain adapter-specific tests
for Service State joins and move pure behavior tests to the model interface.

## Checkpoint 19 | Presentation Capacity Deep Kernel

State transition: the provider-free model crate now owns the complete durable
presentation-capacity record family and deterministic kernel. The CLI capacity
module is an observation and inventory adapter: it qualifies Service State
records into ordered immutable observations, delegates policy to the model,
and retains no reserve arithmetic, queue ordering, conflict precedence,
transition graph, dispatch, or reconciliation mutation.

Acceptance state and progress classification: this P1 deep-kernel packet is
accepted locally. It is not a types-only extraction. The model owns serde and
default compatibility, constructor validation, ordinary and bound admission,
pressure prefixes, protected reserves, browser exclusion, request-dependent
conflict precedence, bounded queue aging, deterministic dispatch, transition
and scene fencing, binding reflection, quarantine, reconciliation, warnings,
and projections. The CLI owns route, display, browser, viewer, acquisition,
handoff, and provider inventory joins.

Compatibility evidence:

- all 117 service-model tests pass, including 19 presentation-capacity kernel
  tests covering serde, exact errors, reserves, pressure, queue clock and aging,
  admission versus dispatch counting, conflict precedence, bound recovery,
  route switching, transitions, quarantine, warnings, and reconciliation;
- five CLI adapter tests pass for authoritative binding, live-handoff
  substantiation, controller and viewer facts, qualified inventory, and a late
  controller fencing staging;
- all 61 presentation-focused, 99 service-health-focused, and 36
  configured-adapter-focused CLI tests pass;
- the production inventory outage fence test and all three route-switch tests
  pass;
- the exact CLI manifest compiles, strict workspace Clippy passes with warnings
  denied, formatting and diff checks pass, and the architecture guard plus its
  fixtures pass;
- the duplicate-definition and forbidden-upward-reference scans pass;
- no GitHub CI, browser, profile, provider, install, staging, production,
  release, or shared-runtime effect was performed.

Delegation and model-choice receipt:

- `/root/p205_capacity_minimal_interface`, requested `gpt-6-astra` at high
  effort for the frozen model files, implemented the kernel and its 19 tests;
- `/root/p205_capacity_flexible_interface`, requested `gpt-5.6-sol` at high
  effort, completed the read-only caller and mutation ledger; its later adapter
  implementation attempt was interrupted after it produced no coherent file,
  and no implementation claim from that attempt was trusted;
- `/root/p205_capacity_common_interface`, requested `gpt-5.6-luna` at medium
  effort, performed the closed-world compatibility audit that fixed the
  required Service State joins, optional-identity behavior, error ordering,
  and retained test set;
- the primary integrated the Service State adapter, caller cutover, retained
  join tests, and all acceptance evidence.

Transitional caller-removal ledger: authority fields remain public only while
inventory refresh, presentation lifecycle, abandoned-browser retirement,
resource inspection, and retained-state guards are converted to typed model
operations or immutable accessors. The known mutation owners are
`presentation_inventory.rs`, `presentation_inventory/production.rs`,
`presentation_lifecycle.rs`, `service_abandoned_browser_retirement.rs`, and
the inventory-failure path in `service_store.rs`. Test fixtures may use public
record construction until their owning production mutation is closed.

Deletion test: deleting the model module now removes every capacity record and
all capacity decisions. The CLI adapter contains only qualified inventory and
observation joins plus delegation functions. Recreating the deleted module
would require rebuilding serde compatibility, reserve and pressure policy,
queue aging, conflict precedence, bound and route-switch custody, transition
fencing, quarantine, dispatch, and reconciliation.

Material blockers: the capacity authority fields are still a transitional
cross-crate compatibility surface. The typed mutation closure above must land
before those fields become private. The canonical Service State aggregate,
codec and migration behavior, runtime-owner authority, effect transaction
families, authentication and challenge records, and capability registry remain
outside the model crate.

Next action: close external presentation-capacity mutation through typed
inventory, lifecycle, and retirement operations, then make authority fields
private behind immutable accessors. Do not broaden that packet into provider
effects or the canonical aggregate move.

## Checkpoint 20 | Capacity Mutation Closure Interface Freeze

State transition: the remaining public capacity-authority mutation surface is
classified and the closure interface is frozen before implementation. This
checkpoint changes no runtime behavior. `PresentationSlot` remains a public
durable record; the five `PresentationCapacityAuthority` fields become private
after every production mutator and fixture has crossed the typed seam.

Acceptance state and progress classification: this is blocker reduction for
the second capacity deepening packet. The selected interface exposes five
immutable accessors for config, slots, queued requests, queue clock, and the
admission error. It adds two separate inventory reconstruction operations, one
inventory-failure fence, six lifecycle operations, and one atomic browser-
retirement operation. It rejects a generic command enum, public mutable slice,
arbitrary mutation closure, unchecked parts constructor, and one generic
inventory replacement that would hide distinct error ordering.

The development inventory operation preserves this sequence: validate fresh
qualified inventory, inherit queue and clock, restore exact slot, route, and
display matches, append only explicitly observed retained acquisitions, sort,
then validate again. The production operation preserves a different sequence:
reconcile every prior slot in prior-vector order with explicit joined custody
observations, emit the existing exact binding or browser drift error, validate
the resulting inventory, then inherit queue and clock. Successful
requalification clears an old admission error and never truncates an inherited
queue merely because a new configured bound is smaller.

The lifecycle operations preserve current semantics without importing provider
effects: begin provisioning with provisioning, hard-maximum, then pressure
gate order; complete provisioning; quarantine failed provisioning with its
unconditionally appended obligation; begin cooldown; complete reclamation by
removing every matching slot; and quarantine failed reclamation with
deduplication but no sorting. Existing `quarantine_slot` and `transition_slot`
cannot substitute because their ordering and lease rules differ.

Browser retirement becomes one model operation that first rejects when any
matching slot has a lease and otherwise resets every matching browser slot to
warm idle while preserving route, display, scene generation, cleanup
obligations, queue, and clock. The CLI will run it on a cloned optional
authority at the current pre-mutation fence, then commit that prepared
authority only at the existing capacity-mutation point so later registry
validation cannot create a partial retirement.

Delegation and model-choice receipt:

- `/root/p205_capacity_mutation_minimal`, requested `gpt-6-astra` at high
  effort, produced the selected explicit interface and found the development
  versus production inventory error-order difference;
- `/root/p205_capacity_privacy_audit`, requested `gpt-5.6-luna` at medium
  effort, enumerated every production and fixture field access and confirmed
  serde can construct private authority fields without an unchecked API;
- `/root/p205_capacity_mutation_lifecycle`, requested `gpt-5.6-sol` at high
  effort, did not return a bounded result and was interrupted; no claim from
  that worker is accepted;
- the primary compared the proposals against current source and retained the
  smallest interface that closes every known production mutation.

Implementation slices:

1. add provider-free observation records, typed operations, exact behavior
   tests, and immutable accessors while fields remain transitional;
2. cut development and production inventory, lifecycle, store failure, and
   retirement mutation owners over without moving service or provider joins;
3. convert read-only production consumers and fixtures to accessors and valid
   constructors, then privatize all five authority fields in the same slice;
4. run focused model, inventory, lifecycle, retirement, retained-state,
   configured desktop, health, and route-switch tests plus strict local gates.

Hard stops: preserve byte-for-byte production custody errors, `None == None`
pending-binding behavior, constructor error order, lifecycle obligation order,
retirement atomicity, permissive serde decoding, and current panic behavior for
an internally missing provisioning slot. Do not move provider calls, resource
cleanup, clocks, cooldown selection, Service State joins, or runtime effects
into the model crate.

Next action: implement the model-owned operations and tests as one isolated
write surface, then cut CLI mutation owners over in non-overlapping slices.

## Checkpoint 21 | Capacity Mutation Closure Accepted

State transition: the presentation-capacity authority is now sealed behind its
model interface. All five authority fields are private. Development and
production inventory reconstruction, inventory-failure fencing, elastic
lifecycle changes, and abandoned-browser retirement use typed model
operations. Read-only CLI consumers use immutable accessors, and test fixtures
construct valid authorities rather than reopening a mutable production seam.

Acceptance state and progress classification: the second presentation-
capacity deepening packet is accepted locally. This is outcome progress. The
CLI retains provider inventory interpretation, Service State joins, lifecycle
effect adapters, repository custody, and retirement commit ordering. The model
owns the previously duplicated deterministic mutation rules. Retirement is
prepared on a cloned authority and installed only at the existing commit point,
so a later registry failure cannot partially park capacity.

Evidence:

- all 125 service-model unit and integration tests pass, including the eight
  capacity reconstruction, lifecycle, failure-fence, and atomic-retirement
  tests added in this packet;
- all 57 presentation-focused, 36 configured-adapter-focused, 99 service-
  health-focused, 14 abandoned-browser-retirement, 13 retained-state, and
  three route-switch CLI tests pass;
- the exact production-inventory outage fence and retained-primary
  revalidation tests pass;
- the complete CLI test target compiles, strict workspace Clippy passes with
  warnings denied, formatting and diff checks pass, and the architecture guard
  plus its fixture suite pass;
- duplicate capacity definitions and forbidden upward model references remain
  absent;
- no GitHub CI, browser, profile, provider, install, staging, production,
  release, or shared-runtime effect was performed.

Delegation and model-choice receipt:

- `/root/p205_capacity_mutation_minimal`, requested `gpt-6-astra` at high
  effort, implemented the selected model operations and focused tests;
- `/root/p205_capacity_privacy_audit`, requested `gpt-5.6-luna` at medium
  effort, converted the bounded read-only caller and fixture set to accessors
  and valid constructors;
- `/root/p205_capacity_mutation_lifecycle`, requested `gpt-5.6-sol` at high
  effort, returned no accepted result during the initial interface-design
  attempt, then completed the later bounded lifecycle and retirement caller
  cutover after the interface was frozen; the primary accepted only that later
  implementation and reran its affected tests;
- the primary completed inventory and store cutover, private-field fixture
  repair, integration review, and all acceptance gates.

Material blockers: the canonical `ServiceState` aggregate and codec remain in
the CLI. Its unresolved downward ownership edges are runtime-owner authority,
abandoned-retirement and crash-regeneration transactions, authentication and
challenge records, and the advisory capability registry. The runtime-owner
edge needs an explicit ownership decision because pure profile-identity
mechanics belong in Lease Authority while Service State joins and runtime-owner
orchestration remain CLI adapters. Authentication and challenge records still
depend on their control-plane crates and must not be copied into a second
canonical model. The capability registry remains advisory and must not be
promoted merely to make the aggregate movable.

Next action: publish this accepted checkpoint, synchronize the branch with
current `origin/main`, and rerun changed-surface validation. Then freeze one
remaining aggregate-dependency ownership matrix, beginning with the runtime-
owner boundary. Choose the canonical owner and adapter seam before moving
another record family; do not start a bug-fix, runtime effect, provider action,
or capability-registry promotion.

## Checkpoint 22 | Current-Main Synchronization And Next Gate

State transition: P205 first synchronized with `origin/main` at `fb616aee` by
merge commit `67f672dd`, then consumed the concurrent main advance through
`06972a5e` by merge commit `f5083df7`. The branch is zero commits behind
current main. The shared Cargo manifests and lockfile merged without a
conflict. No bug-fix issue or validation lane moved into P205 custody; the
architecture lane only consumes integrated main as its dependency baseline.

Acceptance state and progress classification: Checkpoint 21 remains accepted
after synchronization. This is blocker reduction for the next architecture
packet. The post-merge model crate and presentation adapter tests pass, the
architecture guard and fixtures pass, and strict workspace Clippy and
formatting pass. The validation selector classifies the whole long-running
branch as broad because it sees all P205 history; the bounded post-merge rerun
uses the exact current packet plus workspace compilation and linting. GitHub CI
remains explicitly skipped.

The next gate is not another passive-record extraction by line count. The
remaining aggregate dependencies need one ownership matrix with these ordered
decisions:

1. split the provider-free runtime-owner value and transition kernel from
   Service State joins, repository custody, runtime adoption, and effects;
2. decide whether that kernel belongs to Lease Authority because it owns pure
   principal and profile-identity mechanics, or to the service-model crate
   because `ServiceState` persists the registry wire record;
3. classify abandoned-retirement and crash-regeneration transaction records
   separately from their process and cleanup effects;
4. reuse canonical authentication and challenge-control record owners rather
   than copying their records into a second model;
5. keep the advisory capability registry outside canonical authority until a
   dedicated product decision admits it;
6. move the `ServiceState` aggregate and persisted codec only after every
   field has one downward canonical owner.

Material blockers: runtime-owner records and pure transitions are currently
interleaved with CLI-only `ServiceStateRepository`, runtime-adoption mode, and
principal-provenance dependencies. That boundary must be frozen before code
moves. Authentication and challenge records depend on their existing control-
plane crates, while the effect transaction families still mix durable records
with host observation and mutation sequencing.

Next action: produce the runtime-owner ownership matrix and minimum interface
freeze as the next bounded P205 packet. Use separate read-only architecture,
minimal-interface, and compatibility-review workers if parallelism materially
reduces the decision time; retain one primary writer and one review/rework
cycle. Do not implement until the canonical owner, dependency direction,
deletion test, and retained compatibility fixtures are explicit.

## Checkpoint 23 | Runtime-Owner Kernel Ownership Freeze

State transition: the runtime-owner boundary is frozen for implementation.
`agent-browser-lease-authority` is the canonical owner of the complete
provider-free runtime-owner custody kernel. The service-model crate may embed
that canonical registry, and the CLI may load, observe, orchestrate, and
persist it. This keeps dependencies directed downward and matches Lease
Authority's existing ownership of principal provenance, profile identity,
claims, fencing, custody, and authenticated transitions.

Canonical Lease Authority surface:

- the existing runtime-owner wire and value family: profile owner state and
  owner, transfer request/proposal/attachment, authority claim and binding,
  receipt attestation and owner attestation, transition kind and receipt,
  rollback snapshot and transition record, runtime lifecycle and cleanup
  states and record, registry and principal binding, reverse-transfer request,
  and typed failure code and error;
- the current provider-free constructors, validation, hashing, exact identity
  comparisons, receipt construction, registry observations, principal-binding
  checks, and registry transition methods;
- `BrowserAdoptionMode` as the same three-variant `snake_case` wire enum,
  removing the only upward type dependency from the kernel; and
- a pure registry lookup for the current effect-capable session binding,
  including its existing terminal-history filtering.

Retained CLI adapter surface:

- repository snapshot helpers, Service State joins, sidecar overlay and
  persistence, and transaction commit ordering;
- action-string admission policy and user-facing error presentation;
- runtime lifecycle authority, process and boot observation, runtime adoption
  orchestration, profile synchronization, cleanup, browser, provider, and
  other host effects.

Dependency direction after the move:

```text
CLI adapters -> agent-browser-service-model -> agent-browser-lease-authority
           \-------------------------------> agent-browser-lease-authority
```

The service-model crate already depends on Lease Authority. Lease Authority
must not acquire a dependency on the service-model crate or the CLI. A new
runtime-owner crate and a split record/transition ownership model are rejected:
both add a boundary without eliminating a dependency, while split ownership
would require a cycle, duplicated wire records, or translation between two
canonical authorities.

Compatibility freeze:

- preserve every existing serde case convention, unknown-field rule,
  default, optional-field omission, and serialize-only record;
- preserve `RuntimeOwnerRegistry::is_empty()` exactly: it considers owners and
  principal bindings, not lifecycle records;
- preserve conservative `Unknown` lifecycle and cleanup defaults;
- preserve generation, revision, compare-and-swap, pending-transfer, replay,
  abort, reversal, claim, attestation-hash, and error-order semantics;
- preserve the intentional distinction between the guarded relaunch binding
  lookup and the current effect-capable binding lookup, which filters terminal
  cleanup-satisfied history;
- preserve primary-state and runtime-owner/lifecycle sidecar precedence and
  wire compatibility; extraction does not remove either durable surface; and
- preserve action gate ordering before stream broadcast, browser recovery, or
  any other effect.

Encapsulation disposition: current production adapters directly mutate owner,
principal-binding, lifecycle, and revision fields. The extraction must not
make those fields unrestricted public merely to compile. Freeze the exact
caller ledger, introduce the minimum typed operations or bounded accessors
needed by those adapters, and close the transitional surface in the same
packet. Any access that cannot yet be closed must be explicitly enumerated as
a temporary compatibility debt before the packet can be accepted.

Deletion and acceptance tests:

1. remove all CLI-owned canonical runtime-owner record and transition
   definitions; a transitional CLI module may contain re-exports and adapters
   only;
2. prove exactly one workspace definition for each moved record and for
   `BrowserAdoptionMode`, and reject CLI, repository, clock, process, runtime-
   adoption, and service-model imports from the Lease Authority module;
3. in a standalone Lease Authority test, execute owner generation 7 to an
   observation-only proposal, commit generation 8, replay without another
   increment, and reverse to generation 9; assert claims, principal-binding
   generations, and byte-equivalent serialized replay receipts;
4. retain the legacy-default, lifecycle round-trip, principal-binding,
   cooperative transfer, attestation, replay, abort, orphan adoption, legacy
   revocation, reversal, mismatch, manual preservation, terminal-history,
   repository persistence, fixture-corpus, and gate-order coverage at the
   appropriate kernel or CLI layer; and
5. run Lease Authority tests, focused runtime-owner and Service State adapter
   tests, the Lease Authority and service-model architecture guards, strict
   workspace Clippy, and formatting. GitHub CI remains skipped.

Implementation slices:

1. add the canonical Lease Authority module, provider-free tests, and
   architecture/deletion guards without changing runtime behavior;
2. reduce the CLI runtime-owner module to compatibility re-exports plus
   repository, action-admission, and error-presentation adapters;
3. cut the service-model aggregate field and CLI callers to the canonical
   types, preserving sidecar overlay and persistence ordering; and
4. close direct registry mutation through the smallest typed interface, run
   the frozen local gates, publish the packet, and stop before the next
   aggregate dependency family.

Delegation and model-choice receipt:

- `/root/p205_runtime_owner_placement` used `gpt-6-astra` at high effort for
  the canonical ownership, dependency, and deletion-test decision;
- `/root/p205_runtime_owner_minimal` used `gpt-5.6-sol` at high effort for the
  smallest behavior-preserving cutover and caller seam;
- `/root/p205_runtime_owner_compat` used `gpt-5.6-luna` at medium effort for
  the serde, fixture, dependency, deletion-guard, and regression ledger; and
- the primary reconciled those read-only reports against current source and
  retained one bounded implementation sequence. No worker received Git,
  runtime, provider, browser, install, or production-effect custody.

Progress classification: this checkpoint is blocker reduction and an
implementation-ready architecture decision. It is not acceptance of the
runtime-owner extraction and does not move the final `ServiceState` aggregate.
The next action is the four-slice runtime-owner kernel implementation above.
No bug-fix issue, GitHub CI, runtime effect, or other aggregate dependency
family enters this packet.

## Checkpoint 24 | Runtime-Owner Canonical Placement Implemented

State transition: the provider-free runtime-owner value and transition kernel
now has one canonical source in `agent-browser-lease-authority`. The CLI
runtime-owner module is a compatibility facade containing typed re-exports,
repository snapshot joins, action admission classification, error formatting,
and CLI-layer compatibility tests. `BrowserAdoptionMode` also moved to Lease
Authority, and the CLI adoption module re-exports it. The CLI-owned
`ServiceState` field now names the Lease Authority registry directly.

Deletion and dependency evidence:

- the CLI no longer defines `ProfileOwner`, `RuntimeOwnerRegistry`,
  `OwnerTransferRequest`, `RuntimeLifecycleRecord`, `BrowserAdoptionMode`, or
  the registry transition implementation;
- the Lease Authority architecture guard now requires the runtime-owner
  module, rejects those duplicate CLI definitions, and rejects repository,
  Service State, runtime-adoption, process, or filesystem dependencies in the
  kernel;
- the guard accepts either the active or intentionally disabled focused
  workflow filename, preserving the current-main CI-disable decision without
  dispatching or re-enabling CI; and
- the existing Service State primary record and runtime-owner/lifecycle
  sidecars remain unchanged in shape and ordering.

Local evidence:

- all 110 Lease Authority tests pass, including the standalone owner generation
  7 to commit generation 8, byte-identical replay, and reverse generation 9
  custody test;
- all 24 CLI runtime-owner tests pass against the extracted kernel, including
  legacy defaults, lifecycle wire compatibility, principal binding, commit,
  replay, abort, adoption, reversal, terminal-history, repository persistence,
  fixture corpus, and action-gate ordering;
- the Lease Authority architecture guard and its mutation-fixture self-test
  pass; formatting and diff checks pass for the implemented slice; and
- no GitHub CI, runtime, browser, profile, provider, install, staging,
  production, or release effect occurred.

Delegation and model-choice receipt:

- `/root/p205_runtime_owner_core` used `gpt-5.6-terra` at high effort for the
  isolated Lease Authority module, minimum provider-free tests, and export;
- `/root/p205_runtime_owner_cli_facade` used `gpt-5.6-sol` at high effort for
  the deletion of duplicate CLI ownership and preservation of the adapter and
  compatibility-test layer; and
- the primary moved the aggregate field and adoption enum reference, extended
  the structural guard, integrated both write surfaces, and reran the local
  gates.

Acceptance state and progress classification: canonical placement, dependency
direction, duplicate deletion, and compatibility-facade cutover are accepted.
This is outcome progress, but the complete Checkpoint 23 packet is not yet
accepted. The initial cross-crate cutover temporarily exposes the registry
revision and owner, principal-binding, and lifecycle maps because production
adapters still mutate them directly. That surface is the exact remaining
encapsulation blocker; it is not a final public contract.

Next action: inventory only production direct mutations, add the smallest
named Lease Authority operations and immutable projections that preserve each
existing transaction boundary, cut those mutation owners over in disjoint
slices, and privatize the four registry fields. Tests may use constructors or
fixture builders but must not keep raw production mutation seams open. Stop
before abandoned-retirement, crash-regeneration, authentication, challenge,
or capability-registry ownership work.

## Checkpoint 25 | Runtime-Owner Encapsulation Interface Freeze

State transition: the complete production direct-access ledger is classified,
and the minimum interface required to privatize all four registry fields is
frozen. Repository locks, clone/apply/publish transactions, profile joins,
host observations, route naming policy, and whole-registry Service State
replacement remain CLI responsibilities.

Immutable Lease Authority projections:

- `revision() -> u64`;
- `owners() -> &BTreeMap<String, ProfileOwner>`;
- `principal_bindings() -> &BTreeMap<String,
  RuntimeOwnerPrincipalBinding>`; and
- `lifecycle_records() -> &BTreeMap<String, RuntimeLifecycleRecord>`.

The borrowed maps preserve deterministic iteration, exact durable keys,
indexing, counts, and existing joins without exposing mutable authority.
Existing semantic lookups such as `owner()` remain preferred where sufficient.

Named mutation surface:

1. one pure `apply_lifecycle_transition` entrypoint whose typed mutation and
   result vocabulary mirrors the existing CLI lifecycle intent and transition
   variants;
2. explicit `boot_epoch` observations for launch and replacement mutations,
   plus a CLI-computed canonical-route verdict for terminal profile migration;
3. `rotate_registered_principal_authority`, preserving the current validated
   removal and revision increment before the absent or non-ready owner result,
   and the second increment when the replacement bind succeeds;
4. `apply_runtime_reset_terminalization`, preserving one revision increment,
   the orphaned owner, terminal and satisfied lifecycle posture, and
   deduplicated evidence without adding a new generation or identity check;
5. `restore_lifecycle_records`, replacing the sidecar projection without a
   revision increment; and
6. `persistence_projection_without_lifecycle_records`, producing the current
   owner-sidecar/primary-state projection without mutating revision.

The lifecycle error remains typed but renders the existing byte-for-byte CLI
diagnostics. Owner transfer errors remain distinguishable from static
lifecycle rejections. Principal rotation and runtime-reset failures also use
small typed errors; they do not become generic edit closures.

Compatibility constraints:

- preserve saturating revision increments, current checked versus saturating
  generation behavior, first-error ordering, replay and no-op behavior, and
  the existing one- or two-increment lifecycle sequences;
- preserve partial in-memory candidate mutation before caller-owned rollback
  where it exists today;
- preserve ordered maps, serde defaults and omissions, sidecar precedence,
  and `is_empty()` ignoring lifecycle history;
- sidecar hydration replaces lifecycle records only and must not begin using
  the lifecycle sidecar's recorded registry revision as authority;
- runtime-reset filesystem preflight remains before the named registry
  mutation; and
- no mutable-map getter, mutable row getter, generic edit closure, production
  fixture setter, or public revision setter is admitted.

Whole-registry replacement remains valid only at existing transaction publish
boundaries: lifecycle authority clone/apply/publish, abandoned-browser
retirement commit, health reconciliation exact-before replacement, and durable
owner-sidecar precedence followed by lifecycle overlay. Those are Service
State custody operations, not registry mutation interfaces.

Implementation sequence:

1. add immutable projections and the pure lifecycle mutation vocabulary;
2. move lifecycle mutation bodies and their bootstrap/store helpers verbatim,
   injecting boot epoch and the route-policy verdict from the CLI;
3. add principal rotation, runtime-reset, and persistence operations and cut
   their three adapter owners over without changing transaction ordering;
4. mechanically convert remaining production reads, then fixtures and tests;
5. make `revision`, `owners`, `principal_bindings`, and `lifecycle_records`
   private and strengthen the architecture guard to reject direct CLI access,
   registry literals, public mutable getters, and public fields; and
6. run the frozen runtime-owner, lifecycle, profile lease/recovery, store,
   migration, health, retirement, Service State serialization, architecture,
   formatting, and strict Clippy gates. GitHub CI remains skipped.

Delegation and model-choice receipt:

- `/root/p205_owner_mutation_api` used `gpt-6-astra` at high effort to classify
  every production mutation, freeze the typed operation seam, and preserve
  revision and transaction semantics;
- `/root/p205_owner_projection_audit` used `gpt-5.6-luna` at medium effort to
  inventory read-only and fixture access, choose the minimal borrowed
  projections, and define the final structural guard; and
- the primary reconciled both reports against the lifecycle, principal
  rotation, runtime-reset, and service-store source before freezing this
  interface.

Progress classification: this checkpoint is blocker reduction and an
implementation-ready deep-interface decision. Checkpoint 24 remains accepted;
runtime-owner encapsulation remains open until all four fields are private and
the local gate set passes.

## Checkpoint 26 | Runtime-Owner Encapsulation Accepted

State transition: the runtime-owner custody kernel is now fully encapsulated
in Lease Authority. `revision`, `owners`, `principal_bindings`, and
`lifecycle_records` are private. CLI production code uses immutable
projections and named mutations; no mutable map getter, mutable row getter,
generic edit closure, registry literal, or revision setter remains.

The pure lifecycle state machine moved behind
`apply_lifecycle_transition`. All fifteen existing lifecycle intents and ten
outcomes retain their prior error strings and ordering, revision behavior,
partial-mutation behavior, bootstrap and rekey rules, and replay semantics.
The CLI still owns repository locking, clone/apply/publish, boot observation,
route naming, profile synchronization, and error presentation. It supplies
boot identity and a computed canonical-route verdict as observations.

The other direct mutation owners now use named Lease Authority operations:

- registered-principal rotation preserves the old-binding removal and first
  revision increment even when no ready owner remains, then preserves the
  second increment on successful bind;
- runtime-reset terminalization preserves the CLI's filesystem preflight and
  effect order, one revision increment, orphaned owner state, terminal and
  satisfied lifecycle state, and deduplicated evidence;
- lifecycle sidecar restoration and persistence projection preserve exact
  precedence and perform no revision mutation; and
- whole-registry replacement remains only at the existing Service State
  transaction publish boundaries.

Test-only malformed and legacy registry setup now uses a `cfg(test)` serde
wire fixture/editor in the CLI facade. It reconstructs through the production
decoder when an edit ends and does not exist in normal builds. This preserves
coverage without admitting a production `from_parts` or mutable accessor.

Structural evidence:

- the Lease Authority guard requires private registry fields, immutable
  projections, one canonical runtime-owner module, and no upward adapter
  dependencies;
- it rejects public registry fields, named or renamed mutable-reference
  getters, direct CLI map or revision access, duplicate canonical types,
  `BrowserAdoptionMode` duplication, and CLI registry literals; and
- the guard and every mutation-fixture self-test pass against the current
  tree.

Local acceptance evidence:

- all 116 Lease Authority tests pass;
- all 31 runtime-owner, 21 runtime-lifecycle, 148 profile-focused, and 43
  service-store-focused CLI tests pass;
- the CLI all-target check passes;
- the Lease Authority architecture guard and self-test pass;
- workspace formatting and strict workspace Clippy with warnings denied pass;
- the first focused-test attempt encountered an sccache transport disconnect;
  the documented cache-off retry passed, so this was infrastructure noise and
  not a product failure; and
- no GitHub CI, runtime, browser, profile, provider, install, staging,
  production, or release effect occurred.

Delegation and model-choice receipt:

- `/root/p205_owner_lifecycle_kernel` used `gpt-6-astra` at high effort to
  extract the lifecycle state machine and then the named principal, reset, and
  persistence operations with focused regression tests;
- `/root/p205_owner_projection_audit` used `gpt-5.6-luna` at medium effort for
  the first bounded production read-only cutover and all-target verification;
- `/root/p205_owner_mutation_api` used `gpt-6-astra` at high effort for final
  field privatization, test-only fixture reconstruction, remaining caller
  conversion, and structural guard closure; and
- the primary froze the interfaces, integrated the disjoint slices, reviewed
  the final structural state, and reran the full crate and strict workspace
  gates.

Acceptance state and progress classification: the complete Checkpoint 23
runtime-owner packet is accepted. This is outcome progress and removes the
runtime-owner downward ownership blocker from the final aggregate. The
canonical `ServiceState` aggregate remains in the CLI because abandoned-
retirement and crash-regeneration transaction records, authentication and
challenge records, and the advisory capability registry still require their
separate ownership decisions.

Next action: publish this accepted checkpoint, synchronize with current main
if it advanced, and freeze the ownership matrix for abandoned-retirement and
crash-regeneration durable transaction records without moving process,
cleanup, or repository effects. Do not begin authentication, challenge,
capability-registry, bug-fix, or runtime work in that packet.

## Checkpoint 27 | Post-Encapsulation Current-Main Synchronization

State transition: after publishing Checkpoint 26, the branch fetched current
`origin/main` at `bea09376` and found thirteen integrated commits. They contain
P212, P213, and P214 desktop/challenge contract work plus shared roadmap,
runbook, and lane-catalog updates. Merge commit `99089337` incorporated that
baseline without conflict. P205 is now zero commits behind current main; no
bug-fix source or runtime effect entered architecture custody.

Acceptance evidence: all 116 Lease Authority tests, the Lease Authority
architecture guard and mutation self-test, and strict workspace Clippy pass at
the exact merged head. The integrated desktop/challenge source did not change
the accepted runtime-owner interface or reopen its private registry fields.
GitHub CI remains skipped.

Progress classification: this is blocker reduction and exact-head
revalidation. Checkpoint 26 remains accepted. The next bounded architecture
gate remains the ownership matrix for abandoned-retirement and crash-
regeneration durable transaction records. Their host process, cleanup,
repository, and provider effects remain CLI adapters.

## Checkpoint 28 | Effect-Transaction Record Ownership Freeze

State transition: the abandoned-browser-retirement and crash-regeneration
durable families are assigned to `agent-browser-service-model`. They are
provider-free Service State transaction records, not Lease Authority custody
records. The CLI retains aggregate joins, repository compare-and-swap,
process and clock observation, provider operations, cleanup, and effect
sequencing.

Abandoned-retirement first packet:

- move the plan, terminal projection, transaction, receipt, exit evidence,
  exit failure, recourse, plan-schema constant, and resource-retirement policy
  data/defaults to Service Model;
- reuse Service Model's canonical process identity and browser health plus
  Lease Authority's lifecycle enums;
- keep observations, reservations, aggregate planning, identity rechecks,
  profile-claim fences, reservation/finalization, and live effects in CLI;
- replace the policy's inherent environment loader with a CLI free function,
  preserving all variable names, bounds, and fallbacks; and
- preserve field declaration order because the sealed plan identifier hashes
  serialized plan bytes with `plan_id` cleared.

Compatibility freeze: the six retirement wire records remain camelCase with
unknown fields denied. The policy remains camelCase, defaulted, and permissive
to unknown fields. Preserve optional fields, integer widths, adjacent-tagged
recourse encoding, error precedence, PID-sorted descendant closure,
census-order-independent seals, revision expectations, and replay ordering.
Reservation validates seal and expiry before replay. Finalization returns an
existing receipt before checking fresh evidence and does not gain an expiry
check. No sidecar, migration, or new schema is admitted.

Crash-regeneration second packet:

- move phase/state, stable identities, evidence, transaction, redacted status,
  request, operation, phase receipt, status-schema constant, validation,
  phase ordering, receipt application, and pure begin/resume, receipt,
  interruption, and ready-completion transitions to Service Model;
- retain the effect trait, coordinator, Service State map lookup/CAS, receipt
  persistence, process/display/Guacamole/browser operations, and error
  rendering in the CLI; and
- preserve camelCase and deny-unknown-field records, snake_case enums, status
  redaction and schema fields, exact errors, phase order, identity comparisons,
  interruption replay, and generic Service State persistence.

Deletion gates for both packets require exactly one canonical definition,
direct Service Model types in `ServiceState`, re-export-only compatibility
where temporarily needed, and no repository, native, filesystem, process,
clock, environment, provider, or runtime imports in their model modules. The
retirement packet must retain all fourteen focused CLI tests and add model
serde/hash fixtures. The crash packet must add model phase/receipt/transition
tests and retain CLI repository interruption, replay, and effect-order tests.

Delegation and model-choice receipt:

- `/root/p205_owner_mutation_api` used `gpt-6-astra` at high effort to audit
  the retirement family, sealing/replay compatibility, storage boundary, and
  smallest record-only extraction; and
- `/root/p205_owner_projection_audit` used `gpt-5.6-luna` at medium effort to
  audit crash regeneration, status/schema callers, pure phase transitions,
  and the repository/effect adapter boundary.

Progress classification: this is blocker reduction and freezes two ordered
implementation packets. The next action is the abandoned-retirement record
and policy extraction only. Do not combine it with crash regeneration,
authentication, challenge, capability-registry, bug-fix, or runtime work.

## Checkpoint 29 | Abandoned-Retirement Records Accepted

State transition: Service Model now canonically owns the abandoned-browser
retirement plan, terminal projection, transaction, receipt, exit evidence,
exit failure, recourse, schema constant, and resource-retirement policy data
and defaults. `ServiceState` names the canonical transaction directly. The CLI
retains observation, reservation, aggregate planning and publication,
identity rechecks, profile-claim fencing, environment loading, repository
transactions, and live effects.

Compatibility evidence:

- a frozen unsigned-plan fixture preserves exact serialized field order and
  SHA-256 `7623ecef1ccaa39d6a4aa2dc83e2331906f53f3f269ef8f50c4b40eb7fdb6e10`;
- strict retirement records round-trip and reject unknown fields, while the
  policy retains defaults and permissive unknown-field decoding;
- the CLI algorithm body from its result boundary onward is unchanged, so
  sealing, expiry/replay order, revision expectations, identity rechecks,
  profile fences, finalization replay, and effect sequencing remain intact;
- Service Model passes 115 unit and fourteen integration tests; all fourteen
  retirement, 39 resource, and 43 store CLI tests pass;
- the Service Model architecture guard and mutation-fixture self-test pass,
  workspace formatting passes, and strict workspace Clippy passes; and
- no dependency, external schema, sidecar, migration, crash-regeneration,
  GitHub CI, runtime, browser, profile, provider, install, staging,
  production, or release effect was introduced.

The structural guard now requires one model-owned retirement family and
rejects retained CLI definitions plus native, repository, filesystem,
process, clock, environment, provider, or runtime imports in the model module.

Delegation and model-choice receipt: `/root/p205_owner_mutation_api` used
`gpt-6-astra` at high effort for the record extraction, environment-loader
adaptation, fixed wire/hash tests, deletion guard, and focused validation. The
primary reviewed the final boundary and ran strict workspace Clippy.

Acceptance state and progress classification: the abandoned-retirement
record-only packet is accepted. This is outcome progress and removes that
downward aggregate dependency without moving its effectful aggregate
algorithms. Next action: implement the separately frozen crash-regeneration
model and phase-transition packet. Do not combine it with authentication,
challenge, capability-registry, bug-fix, or runtime work.

## Checkpoint 30 | Crash-Regeneration Model And Transition Kernel Accepted

State transition: `agent-browser-service-model` now canonically owns the
crash-regeneration phase and state enums, stable identities, private evidence,
durable transaction, redacted status, request, operation, phase receipt,
status schema constant, validation, phase ordering, status projection, and
pure begin/resume, receipt, interruption, and ready-completion transitions.
`ServiceState` stores the canonical transaction directly. The CLI retains the
effect trait, repository lookup and compare-and-swap, persistence, effect
sequencing, and user-facing error composition.

Compatibility and invariant evidence:

- all existing camelCase records, snake_case enums, phase order, operation
  identifiers, redaction, error strings, identity comparisons, replay
  behavior, and generic Service State persistence remain intact;
- the model transition seam now validates requests and phase receipts itself,
  so a non-CLI caller cannot create an empty-identity transaction or apply an
  invalid receipt merely by bypassing coordinator prevalidation;
- the CLI preserves the original no-op fast path after the final phase. A
  mutation-count assertion proves entering `Ready` does not perform an extra
  repository mutation;
- all 118 Service Model unit tests and fourteen crate integration tests pass;
  all six crash-regeneration CLI adapter tests and 41 focused `service_model`
  CLI tests pass;
- the Service Model architecture guard requires the canonical module, public
  interface exports, direct aggregate type, unique definitions, schema
  constant, and no upward or adapter imports. Its missing-module,
  missing-export, indirect-type, forbidden-import, and duplicate-definition
  mutation fixtures pass;
- Service API/MCP parity, generated service client contract checks, JavaScript
  client type checks, formatting, diff hygiene, and strict workspace Clippy
  pass; and
- GitHub CI remains skipped. No runtime, browser, profile, provider, install,
  staging, production, or release effect occurred.

Review and delegation receipt: `/root/p205_owner_projection_audit` was reused
for the bounded implementation and guard packets with requested
`gpt-5.6-luna` medium routing. `/root/p205_crash_standards_review` used requested
`gpt-5.6-luna` medium routing and returned no hard or judgment-call findings.
`/root/p205_crash_spec_review` used requested `gpt-5.6-sol` medium routing and
identified two accepted partial findings: public transition validation and
direct model transition coverage. The primary repaired both, independently
found and repaired the extra final repository mutation, and reran the affected
and final gates. The runtime did not expose independently verifiable effective
model or effort metadata. This consumed the packet's one review/rework cycle.

Acceptance state and progress classification: the crash-regeneration packet is
accepted. This is outcome progress and removes the second frozen effect-
transaction dependency without moving repository or host effects. Next action:
review P211's cold shutdown and requalification contracts against the remaining
P205 aggregate boundary, then freeze one consolidated ownership packet for the
authentication-run, challenge-task, and advisory capability-registry fields.
Do not begin bug-fix, CI, runtime, provider, browser, install, or production
work in that packet.

## Checkpoint 31 | Final Embedded-Owner And P211 Compatibility Freeze

State transition: the remaining aggregate dependencies now have one ordered
ownership decision. P211's cold shutdown, cold install, and current-boot
presentation requalification introduce no new Service State field, codec case,
transition, or projection. Their provider-neutral controllers accept effects or
explicit observation inputs and return ephemeral receipts. P211 retains those
contracts and their adapters; P205 must preserve the existing aggregate wire,
Lease Authority records, presentation records, and unknown-field round trip,
but must not invent a speculative bulk-shutdown transition or P211 field.

Final embedded-owner decisions:

1. `BrowserCapabilityRegistry` and the exact profile-compatibility row matcher
   move to Service Model as passive advisory data and a pure helper. Existing
   JSON-shaped rows, permissive decoding, camelCase field names, defaults,
   `generatedAt`, and `is_empty` behavior remain exact. CLI keeps path/body
   validation, clocks, collection mutation, access planning, launch selection,
   preflight, and every routing or effect decision. Canonical storage does not
   promote the registry into operational authority.
2. A focused new `agent-browser-authentication-control` crate becomes the
   canonical owner of the current provider-free authentication-run domain from
   `cli/src/native/authentication_run.rs`: stable binding, state and action
   enums, redacted observations and receipts, response-only effect traits,
   deterministic transition budget, replay fencing, receipt validation,
   cancellation, and exact-target verification. It has no Service State,
   repository, browser, credential, network, provider, transport, or platform
   dependency.
3. Service Model owns the durable Service authentication envelope and its pure
   start, reservation, completion, cancellation, replay, and redacted status
   transitions. It embeds the canonical authentication-control run and the
   canonical challenge-consumer admission receipt. CLI retains command parsing,
   forbidden-field checks, current principal/tab/policy joins, timestamps,
   repository custody, credential retrieval, browser input, provider watches,
   and effect sequencing.
4. `agent-browser-challenge-control` remains the canonical owner of challenge
   task requests, phases, outcomes, receipts, consumer evidence, and consumer
   admission. Service Model owns only the durable Service challenge envelope
   and pure envelope transitions. The existing full provider-free receipt stays
   byte/value-compatible as opaque JSON inside that envelope because its
   canonical receipt currently contains static profile vocabulary and is
   serialize-only. Any authority decision must first decode the existing
   `ChallengeConsumerEvidence` type and fail closed on malformed evidence;
   generic JSON never grants authority. A typed full-receipt migration is not
   admitted by P205.
5. Lease Authority remains the direct owner of `ServicePrincipalRegistry`,
   `LeaseAuthorityState`, and `RuntimeOwnerRegistry`. Existing CLI modules may
   re-export those types during cutover, but the final aggregate imports their
   canonical owners directly.

Dependency direction after the remaining packets:

```text
agent-browser-authentication-control    agent-browser-challenge-control
                 \                         /
                  agent-browser-service-model
                              ^
                              |
                         CLI adapters
```

Service Model's dependency allowance expands only for the two provider-free
control crates. Neither control crate may import Service Model or the CLI.
Challenge Control's existing downward dependency on Desktop Services remains
unchanged and does not grant Service Model any desktop effect authority.

Compatibility and deletion gates:

- require exactly one canonical definition for every moved record, enum,
  constant, and transition; reject retained CLI definitions after each packet;
- require direct canonical types in `ServiceState` and forbid CLI, repository,
  filesystem, process, clock, browser, provider, credential, network, and
  transport imports from model/control modules;
- retain representative old Service State JSON for empty and populated
  capability, authentication-run, and challenge-task maps, including unknown
  top-level field round trips and absent-empty serialization;
- retain exact schema strings, enum casing, field casing and order where hashed,
  unknown-field policy, defaults, stable IDs and hashes, replay/error ordering,
  redaction, transition budgets, and consumer-admission failure semantics; and
- after P211 rebases onto the integrated P205 result, compile its three pure
  controller modules unchanged and retain the stale-owner shutdown-close
  regression. P205 adds no P211-specific model surface.

Delegation and model-choice receipt: `/root/p205_p211_compatibility` used
requested `gpt-5.6-sol` high routing for the cross-plan compatibility audit and
found no P205 blocker. `/root/p205_auth_challenge_ownership` used requested
`gpt-5.6-terra` high routing for the canonical-owner and dependency decision.
The primary performed the deterministic capability and remaining-field
inventory locally after the runtime rejected a third worker thread. Effective
runtime model and effort metadata were not independently exposed.

Progress classification: this is blocker reduction and closes the last
ownership ambiguity before the aggregate move. Implementation order is the
advisory capability record, authentication-control domain, Service
authentication envelope, Service challenge envelope, then the canonical
aggregate/codec and remaining pure decisions. The next packet is only the
capability record and matcher. Do not combine it with authentication,
challenge, aggregate, bug-fix, CI, runtime, provider, browser, install, or
production work.

## Checkpoint 32 | Advisory Capability Registry Accepted

State transition: Service Model now canonically owns the passive
`BrowserCapabilityRegistry`, its exact `is_empty` contract, and the pure
profile, host, and executable compatibility-row matcher. `ServiceState` names
the canonical registry directly. The CLI retains a narrow compatibility
re-export plus all config overlay, path/body validation, timestamping,
collection mutation, access planning, preflight, launch selection, and routing
decisions.

Compatibility and authority evidence:

- camelCase fields, permissive unknown outer fields, opaque nested JSON values,
  default empty arrays, serialized `generatedAt: null`, nonempty handling for
  every collection and any present timestamp string, and exact case- and
  whitespace-sensitive matching remain unchanged;
- a populated model fixture proves nested arbitrary JSON round trips while
  unknown outer fields remain accepted and ignored exactly as before;
- the structural guard requires the model module, public exports, one canonical
  definition and matcher, and the direct aggregate type. Mutation fixtures
  reject a missing module, missing export, indirect aggregate type, duplicate
  CLI record, and duplicate CLI matcher;
- all 121 Service Model unit tests and fourteen crate integration tests pass;
  all twenty `browser_capability` CLI tests and 41 focused `service_model` CLI
  tests pass;
- Service API/MCP parity, generated service client contract checks, JavaScript
  client type checks, formatting, diff hygiene, and strict workspace Clippy
  pass; and
- the move creates no new dependency and does not change the existing advisory
  data's runtime use. No GitHub CI, runtime, browser, profile, provider, install,
  staging, production, or release effect occurred.

Delegation and model-choice receipt: the primary reused the warm-context
`/root/p205_owner_mutation_api` worker for the three-file record extraction
after the runtime rejected a new third thread. The worker changed only the
model module, crate exports, and CLI compatibility seam, then reported the
model tests. The primary independently added and ran the architecture guard,
reviewed the ownership and authority boundary, and ran all CLI and final gates.
No new model route was allocated, and effective runtime model and effort
metadata were not independently exposed.

Acceptance state and progress classification: the advisory capability packet
is accepted. This is outcome progress and removes the last passive aggregate
record owned only by the CLI. Next action: create the separately frozen
provider-free `agent-browser-authentication-control` crate and reduce
`cli/src/native/authentication_run.rs` to a compatibility re-export. Do not
combine that packet with the Service authentication envelope, challenge task,
aggregate, bug-fix, CI, runtime, provider, browser, install, or production
work.

## Checkpoint 33 | Authentication Control Domain Accepted

State transition: the provider-free authentication-run control domain now has
one canonical owner in the new `agent-browser-authentication-control` workspace
crate. The CLI `authentication_run` module contains only a crate-local wildcard
re-export. Service authentication custody, command parsing, current-state joins,
repository mutation, browser and credential effects, provider observation, and
response composition remain CLI adapters and were not changed in this packet.

Compatibility and boundary evidence:

- the complete former 1,834-line CLI domain, including all seventeen unit
  tests, moved unchanged after normalizing only cross-crate visibility from
  `pub(crate)` to `pub`; an exact normalized source diff is empty;
- schema and field names, serde casing and unknown-field policy, redaction,
  receipt validation, replay and error ordering, transition budgets,
  cancellation, and exact-target verification therefore remain identical;
- the new crate has only `serde` as a product dependency and `serde_json` as a
  test dependency. Its architecture guard uses a positive allowlist and rejects
  ordinary, table-form, build, target-qualified, and aliased dependencies;
- the same guard requires every canonical record, enum, trait, and schema
  constant exactly once, rejects CLI duplicates, forbids upward and effectful
  imports, and requires the CLI facade to contain only the exact compatibility
  re-export. Mutation fixtures cover missing owners, missing schema, unknown or
  aliased dependencies, process imports, missing facade, duplicate ownership,
  and extra facade logic;
- all seventeen authentication-control crate tests and ten focused CLI
  authentication-run tests pass, with 3,233 unrelated CLI tests filtered;
  formatting, diff hygiene, the architecture guard and its mutation fixtures,
  and strict workspace Clippy also pass; and
- no Service authentication envelope, Service State aggregate, challenge,
  bug-fix, GitHub CI, runtime, browser, profile, provider, credential, install,
  staging, production, or release effect occurred.

Delegation and model-choice receipt: `/root/p205_auth_control_extract` used the
requested balanced `gpt-5.6-terra` high route for the bounded six-file Rust and
manifest extraction, returned passing focused and strict checks, and made no
commit. The primary owned the disjoint architecture-guard and plan surfaces,
verified exact normalized source equivalence, and integrated the result. A
fresh `/root/p205_auth_control_review` pass used requested `gpt-5.6-sol` medium
routing and found two P2 guard gaps: a dependency denylist and a facade check
that permitted extra logic. Both findings were accepted, repaired with positive
constraints and mutation fixtures, and passed the reviewer's closed-world
rechecks. Runtime-reported effective model metadata was not independently
exposed.

Acceptance state and progress classification: the authentication-control
packet is accepted as outcome progress. It removes the largest remaining
provider-free domain from the CLI and unblocks the separately bounded Service
authentication envelope. Next action: move only the durable Service
authentication record and its pure start, reservation, completion,
cancellation, replay, and redacted projection decisions into Service Model.
Keep command parsing, current principal/tab/policy joins, clocks, repository
custody, credentials, browser input, provider watches, and effect sequencing in
the CLI; do not combine this packet with challenge, aggregate, bug-fix, CI,
runtime, provider, browser, install, or production work.

## Checkpoint 34 | Service Authentication Envelope Accepted

State transition: Service Model now owns the canonical durable
`ServiceAuthenticationRunRecord`, its private pending-effect fence, typed start
input and decision, completion and error vocabulary, redacted projection, and
the pure start, liveness, reservation, completion, cancellation, and empty-map
decisions. `ServiceState.authentication_runs` uses that record directly and its
serde omission predicate calls the crate-owned decision. The CLI no longer
defines any of those records or decisions.

Compatibility and boundary evidence:

- start remains a two-phase pure decision so an exact idempotent replay is
  returned before challenge admission, preserving the previous error and
  effect ordering while the CLI retains challenge authority;
- current principal, tab-handle, policy, and lease joins, clocks, repository
  custody, credentials, browser input, provider watches, network work, and
  effect sequencing remain CLI adapters. The model accepts explicit timestamps
  and a typed challenge-consumer admission receipt only;
- successful and failed authentication transitions persist through the same
  pending-effect fence, cancellation still uses the canonical authentication
  transition, and the projection exposes only redacted identifiers and state;
- the architecture contract requires every canonical definition, decision,
  schema constant, aggregate field type, export, and active serde predicate
  exactly once. Its positive dependency parser rejects ordinary, quoted,
  table-form, build, target-qualified, and aliased dependencies, including
  quoted alias keys. Mutation fixtures also prove that commented or earlier
  field attributes cannot satisfy the `authentication_runs` predicate check;
- all 127 Service Model unit tests, fourteen Service Model integration tests,
  nine focused Service authentication CLI tests, and 41 focused Service Model
  CLI tests pass. The architecture guard and mutation fixtures, Service API and
  MCP parity, generated-client contract and type checks, formatting, diff
  hygiene, and strict workspace Clippy pass; and
- the changed-surface selector was run from Checkpoint 33. Its Lease Authority
  and desktop-services architecture recommendations pass. Its unrelated CDP
  architecture check cannot start because this repository currently contains
  `.github/workflows/ci.yml.disabled` rather than the check's expected
  `.github/workflows/ci.yml`; this packet did not change CDP or workflow files
  and did not rewrite that separately owned state.

Delegation and model-choice receipt: `/root/p205_service_auth_envelope` used the
requested `gpt-5.6-sol` high route for the bounded model API design. The worker
was interrupted after it had not produced timely edits, but its first two-phase
API patch arrived at interruption and was preserved; the primary completed the
CLI cutover, tests, guard, and validation. A fresh
`/root/p205_service_auth_review` pass used requested `gpt-6-astra` high routing
and found four P2 guard bypasses across its initial and closed-world readback:
quoted dependency names, quoted alias keys, commented serde attributes, and an
attribute borrowed from an earlier field. All four were repaired with
discriminating negative fixtures inside the same bounded review cycle. Strict
Clippy additionally required both large start-decision payloads to be boxed;
the focused tests prove that indirection does not change replay or creation
semantics. Runtime-reported effective model metadata was not independently
exposed.

Acceptance state and progress classification: the Service authentication
envelope is accepted as outcome progress and removes another direct durable
owner and pure decision cluster from the CLI. No bug-fix, GitHub CI, runtime,
browser, profile, provider, credential, install, staging, production, or
release effect occurred. Next action: freeze and extract only the Service
challenge envelope. Do not combine it with aggregate and codec closure.

## Checkpoint 35 | Service Challenge Envelope Interface Freeze

State transition: the final embedded CLI owner is bounded before edits. The
1,058-line `service_challenge_task` module currently mixes five responsibilities:
command parsing, live Service State joins, repository and clock custody, the
durable challenge envelope, and deterministic envelope transitions. This
packet moves only the last two provider-free responsibilities into Service
Model. Challenge Control remains the canonical owner of challenge requests,
profiles, execution receipts, consumer evidence, and admission decisions.

Frozen Service Model ownership:

- `SERVICE_CHALLENGE_TASK_SCHEMA_VERSION`, the two registered downstream intent
  constants, `ServiceChallengeTaskState`, `ServiceChallengeTaskSummary`,
  `ServiceChallengeTaskRecord`, its private pending-effect record and kind, and
  the typed start, resume, projection, and error vocabulary;
- deterministic hash and ID derivation, exact idempotent start, status and
  principal ownership, operation replay, deadline validation, provider-free
  resume execution, cancellation, redacted projection, summary, empty-map
  omission, and typed conversion of the opaque receipt into
  `ChallengeConsumerEvidence` before admission; and
- a two-phase resume seam: model preparation preserves replay, terminal-state,
  timestamp, and deadline ordering; the CLI then supplies the exact current tab
  join; model completion validates that handle, executes the deterministic
  Challenge Control transition, and commits the next record. The full
  Challenge Control receipt remains value-compatible opaque JSON in the
  durable envelope.

Frozen CLI ownership:

- raw JSON command parsing, forbidden fields, required strings, registered
  profile and fixture decoding, digest syntax checks, and public error strings;
- current tab lookup, controlled-lease and principal joins, trace matching,
  effective site-policy lookup, repository load and mutation, timestamps,
  action dispatch, JSON response serialization, and test fixture assembly;
- consumer-admission orchestration in its existing order: task and principal,
  completed state, current retained handle, supplied handle, effective policy,
  typed receipt decode, then Challenge Control admission. Pure receipt decode
  and admission move behind a Service Model helper, but no generic JSON value
  grants authority; and
- no browser, desktop, provider, credential, network, filesystem, process,
  transport, install, or runtime effect moves into Service Model.

Required compatibility evidence:

1. Freeze the current empty and populated Service State wire shape, schema and
   enum casing, field order, defaults, unknown-field rejection, opaque receipt,
   omission of empty task maps, stable hashes and IDs, and secret-free
   projections.
2. Preserve start error and replay ordering; resume operation replay before
   terminal, timestamp, deadline, current-handle, profile, and execution
   checks; cancellation replay before terminal and timestamp checks; and the
   exact consumer-admission failure order.
3. Require one canonical owner and direct `ServiceState` type for every moved
   definition and decision. Extend the positive dependency and active serde
   predicate guards with discriminating mutation fixtures.
4. Run Service Model tests, focused Service challenge CLI tests, affected
   authentication and navigation consumer tests, Service Model focused tests,
   formatting, strict workspace Clippy, diff hygiene, architecture guards, and
   local Service API and client parity. GitHub CI remains explicitly excluded.

Critical path and worker boundary: the P205 owner freezes the API, integrates
the CLI adapter, owns shared manifests and guards, interprets compatibility,
and publishes the checkpoint. One balanced implementation worker may write
only the new Service Model challenge module and its unit tests. A fresh strong
reviewer receives a frozen read-only diff after local focused tests. Neither
worker receives repository, plan, forge, runtime, browser, provider, install,
or effect custody; workers may not spawn children.

Progress classification: this interface freeze is blocker reduction, not
acceptance. The next and only implementation outcome is the Service challenge
envelope extraction. Aggregate and codec closure remains a later packet.

## Checkpoint 36 | Service Challenge Envelope Accepted

State transition: Service Model now owns the canonical durable Service
challenge record, state, private pending-effect fence, registered intent
constants, typed start, resume, cancel, error and projection vocabulary, and
all provider-free envelope decisions. `ServiceState.challenge_tasks` uses the
canonical record and crate-owned empty-map predicate directly. Service Status
and resource projections call the crate-owned summary directly. The CLI module
is reduced from 1,058 to 851 lines and no longer defines a durable challenge
record or duplicates a pure envelope transition.

Compatibility and boundary evidence:

- start still joins and validates the current retained handle before hashing
  or idempotent replay. The model preserves the exact hashing tuple, stable
  24-hex ID suffix, schema, camelCase wire shape, defaults, unknown-field
  rejection, opaque receipt, and operation and idempotency redaction;
- resume preparation preserves task, principal, operation replay, terminal,
  timestamp, and deadline ordering before the CLI performs current-tab lookup.
  Completion reproduces the exact selected-handle lease, principal, identity,
  and trace predicate before deterministic Challenge Control execution;
- cancellation preserves replay before terminal and timestamp checks and still
  performs no deadline or current-handle validation. Resume and cancellation
  retain the parsed RFC3339 offset exactly;
- consumer orchestration remains CLI-owned in its prior order. Only the final
  opaque receipt decode and typed Challenge Control admission decision moved
  behind the model seam. Malformed evidence fails closed, and typed invalid
  evidence retains the distinct no-detail error string;
- projection key names and transition-count semantics, nullable terminal
  fields, opaque receipt value, effect-pending flag, and summary counts remain
  exact. Repository creation, raw parsing, clocks, current-state and policy
  joins, dispatch, and response serialization remain CLI adapters; and
- the architecture contract requires the module, canonical definitions,
  exported public vocabulary, three constants, ten pure decisions, direct
  aggregate type, and active serde predicate exactly once. Negative fixtures
  cover missing ownership and exports, CLI duplicates, indirect aggregate
  types, duplicate decisions, and commented or borrowed predicates.

Validation evidence:

- 136 Service Model unit tests and fourteen integration tests pass, including
  nine new challenge-envelope unit tests;
- six focused challenge-task CLI tests pass through the repository runner,
  including durable dispatch replay and closed request fields. A direct Cargo
  filter first exhausted the default test stack in the state-heavy dispatch
  fixture; the prescribed focused runner supplied the repository's 16 MiB test
  stack and passed the same six tests;
- three authentication admission tests, two challenge-navigation tests, the
  shared navigation-admission test, 41 focused Service Model CLI tests, 49
  Service Status tests, and 39 Service Resources tests pass;
- Service API and MCP parity, generated-client contract and type checks,
  formatting, strict workspace Clippy, diff hygiene, the architecture guard,
  and all guard mutation fixtures pass; and
- no GitHub CI, runtime, browser, profile, provider, credential, install,
  staging, production, or release effect occurred.

Delegation and model-choice receipt: `/root/p205_challenge_seam_inventory`
used requested `gpt-5.6-sol` high routing for a read-only owner, caller, and
error-order inventory. `/root/p205_challenge_model_extract` used requested
`gpt-5.6-terra` high routing and wrote only the new Service Model module and
its unit tests; the primary owned exports, CLI cutover, aggregate references,
guards, validation, and publication. Fresh `/root/p205_challenge_review` used
requested `gpt-6-astra` high routing and found one P2 compatibility drift:
resume and cancellation normalized nonzero RFC3339 offsets to UTC. The finding
was accepted and repaired with nonzero-offset regression assertions. The
runtime did not independently expose effective model metadata.

Acceptance state and progress classification: the Service challenge envelope
is accepted as outcome progress. Every previously embedded authentication and
challenge owner named by Checkpoint 31 now has its canonical provider-free
owner. The next action is a separate aggregate and codec closure freeze: move
the canonical `ServiceState`, compatibility codec, revision access, and
unknown-field preservation, then close remaining pure decisions before facade
deletion. Do not combine that freeze with runtime, bug-fix, CI, or release work.

## Checkpoint 37 | Aggregate Dependency Closure Freeze

State transition: aggregate movement is split from dependency closure so the
first packet has one falsifiable outcome. The `ServiceState` definition has
nine fields whose canonical types already live below the CLI but whose paths
still pass through CLI compatibility modules. Moving the aggregate while those
paths remain would either create an upward dependency or silently duplicate
types. This packet changes only those type and serde-predicate paths.

Frozen direct-owner replacements:

| `ServiceState` field | Current CLI path | Canonical path |
| --- | --- | --- |
| `presentation_capacity` | `super::presentation_capacity::PresentationCapacityAuthority` | `agent_browser_service_model::PresentationCapacityAuthority` |
| `profile_policy_migration` | `super::service_state_migration::ProfilePolicyMigrationReport` | `agent_browser_service_model::ProfilePolicyMigrationReport` |
| `service_principals` | `super::service_principal::ServicePrincipalRegistry` | `agent_browser_lease_authority::ServicePrincipalRegistry` |
| `profile_lease_reconcile_receipts` | `super::service_profile_lease::ProfileLeaseReconcileReceipt` | `agent_browser_service_model::ProfileLeaseReconcileReceipt` |
| `profile_recovery_receipts` | `super::service_profile_acquisition::RecoveryReceipt` | `agent_browser_service_model::RecoveryReceipt` |
| `profile_reset_receipts` | `super::service_profile_acquisition::ProfileResetReceipt` | `agent_browser_service_model::ProfileResetReceipt` |
| `profile_lifecycle_authorizations` | `super::service_profile_lifecycle::ProfileLifecycleAuthorization` | `agent_browser_service_model::ProfileLifecycleAuthorization` |
| `profile_lifecycle_effect_receipts` | `super::service_profile_lifecycle::ProfileLifecycleEffectReceipt` | `agent_browser_service_model::ProfileLifecycleEffectReceipt` |
| `browser_retirement_receipts` | `super::service_browser_retirement::BrowserRetirementReceipt` | `agent_browser_service_model::BrowserRetirementReceipt` |

The `service_principals` omission predicate moves to the same Lease Authority
path. Existing CLI compatibility re-exports remain temporarily available to
their own adapters and callers; this packet does not delete them, move
`ServiceState`, change field visibility, or change any record, default, codec,
revision, transition, projection, repository, clock, process, or effect logic.

Acceptance requires zero CLI-module type or predicate path inside the
`ServiceState` definition, an architecture guard that rejects any reintroduced
`super::` owner there, unchanged empty and populated aggregate serialization,
the focused Service Model and affected adapter tests, formatting, strict
workspace Clippy, and diff hygiene. GitHub CI and runtime effects remain
excluded. After acceptance, the next packet may freeze the aggregate move and
method disposition against a dependency-closed definition.

Progress classification: this is blocker reduction. It must land as a clean
published checkpoint before aggregate movement starts.

## Checkpoint 38 | Aggregate Dependency Closure Accepted

State transition: the `ServiceState` definition now names every embedded owner
directly. The nine frozen fields use `agent_browser_service_model` or
`agent_browser_lease_authority` without passing through a CLI compatibility
module, and the principal omission predicate uses the same direct Lease
Authority path. There is no remaining `super::` path in the aggregate
definition.

The architecture contract now extracts the exact Rust `ServiceState`
definition, requires each of the nine canonical field paths, requires the
direct principal omission predicate, and rejects any reintroduced `super::`
owner path. Its mutation fixtures prove both a field-type indirection and a
serde-predicate indirection fail. The cutover also removed the now-unused
`ServicePrincipalRegistry` CLI re-export and made the test-only
`ProfileLifecycleAuthorization` import test-only. No record, field visibility,
default, wire name, omission rule, transition, projection, revision, codec,
repository, clock, process, or effect behavior changed.

Acceptance evidence:

- the Service Model crate passed 136 unit tests and 14 integration tests;
- the focused `service_model` lane passed 41 tests;
- the focused `service_state` lane passed 89 tests;
- the architecture contract and all mutation fixtures passed;
- Service API/MCP parity passed for 66 browser controls, 26 Service tools,
  19 Service resources, 101 native Service actions, and 118 Service-request
  actions;
- generated Service client contract and JavaScript type checks passed;
- the changed-surface selector completed and retained the broad local route;
- formatting, strict workspace Clippy with `-D warnings`, and diff hygiene
  passed; and
- no GitHub CI, runtime, browser, profile, provider, credential, install,
  staging, production, or release effect occurred.

Delegation and model-choice receipt: `/root/p205_aggregate_codec_inventory`
used requested `gpt-5.6-sol` high routing for a read-only aggregate, codec,
revision, caller-seam, and blocker inventory at the last clean implementation
head. It made no edits or effects. The inventory confirmed 19 current
`ServiceState` methods, one direct upward cycle through the abandoned-browser
retirement claim predicate, a mixed pure/effectful migration file, and a
drift-prone manually maintained known-key list. The runtime did not
independently expose effective model metadata.

Acceptance state and progress classification: aggregate dependency closure is
accepted as blocker reduction and published separately from the aggregate
move. The next packet must freeze the aggregate interface and method
disposition before implementation. It must introduce one crate-owned
`ServiceState`, compatibility codec, revision interface, and unknown-field
owner while retaining repository locks, replay, file replacement, process
observation, and effect custody in CLI adapters. The upward retirement
predicate moves with lease admission; `service_state_migration.rs` must be
split rather than moved wholesale.

## Checkpoint 39 | Canonical Aggregate And Persisted Codec Interface Freeze

State transition: the aggregate cutover is frozen as one atomic ownership
packet. Rust does not permit the existing 19 inherent methods to remain in the
CLI after `ServiceState` becomes a foreign crate type. A re-export therefore
cannot precede method disposition, and a second wrapper, newtype aggregate, or
permanent extension trait would fail the one-aggregate rule and deletion test.

The Service Model crate will add one `service_state` module that owns the exact
44-field `ServiceState` declaration, its 19 existing inherent methods, their
pure helper closure, ordinary persisted compatibility decoding, deterministic
prepared-state encoding, version stamping, revision decisions, and flattened
unknown top-level field bag. The CLI will remove its aggregate definition and
inherent implementation in the same packet and temporarily re-export the
crate-owned type from `native::service_model` so caller paths do not obscure
the ownership move.

Frozen crate interface:

- `SERVICE_STATE_SCHEMA_VERSION` and
  `LEGACY_SERVICE_STATE_SCHEMA_VERSION`;
- `ServiceState` with exact field order, serde names, defaults, omission
  predicates, flattened unknown fields, and skipped entity-source projection;
- a typed `ServiceStateCodecError` that preserves the exact unsupported-state,
  unsupported-profile-lease, JSON, invariant, and revision-exhaustion
  identities at the CLI adapter;
- `decode_persisted_service_state_json`, which alone performs persisted
  version validation, legacy profile-policy materialization, and current
  version stamping;
- `prepare_service_state_for_persistence`, which performs the same pure
  materialization and stamping without observation or I/O;
- `encode_prepared_service_state_pretty`, which serializes exactly one
  prepared state deterministically and appends exactly one newline;
- `validate_service_state_invariants`, which remains an explicitly invoked
  pure check and is not silently added to ordinary reads or writes; and
- revision operations that return the current revision, prepare a cloned
  checked successor before the mutator runs, and compare payload equality
  while ignoring only the revision. A public generic mutation closure or
  revision setter is forbidden.

All 19 current inherent methods move with the aggregate. Their helper closure
includes profile-freshness overlay, built-in policies, readiness derivation,
incident derivation and ordering, stale-session classification, tab-handle
derivation, source projections, and job ordering. The lease-admission methods
will consume a model-owned pure abandoned-retirement claim predicate. The
derived-view closure will consume model-owned `route_pool_target_string` and
`route_pool_entry_matches_display` helpers, with existing CLI remote-view
callers redirected to the same canonical helpers. The terminal-state check
will name the Service Model owner directly. No upward `super::` edge may remain
in the moved module.

Compatibility access is explicitly temporary. The fields that are currently
public remain public. These current CLI-private fields become documented-hidden
public fields only for the recorded cutover ledger:

`schema_version`, `state_revision`, `profile_lease_schema_version`,
`presentation_capacity`, `profile_policy_migration`, `service_principals`,
`lease_authority`, `profile_lease_reconcile_receipts`,
`profile_recovery_receipts`, `profile_reset_receipts`,
`profile_lifecycle_authorizations`, `profile_lifecycle_effect_receipts`,
`browser_retirement_receipts`, `abandoned_browser_retirements`,
`crash_regeneration_transactions`, `protected_browser_owner_observations`,
`runtime_owner_registry`, `authentication_runs`, `challenge_tasks`, and
`unknown_fields`.

The architecture contract must recognize exactly that migration-only list,
reject any new documented-hidden aggregate field, and retain a final P4 gate
that requires the list to be empty before field privacy is accepted. This is
not a permanent public interface.

Repository ordering is frozen independently of the codec. The CLI repository
still loads the baseline, prepares the checked successor revision before
calling the mutator, evaluates no-op equality before derived-view refresh,
revalidates even a no-op against the durable current revision, replays a stale
candidate under exclusive custody, prepares sidecar and primary payloads once,
and commits those exact prepared bytes without reserialization. Filesystem and
process locks, bounded-stack threads, transaction journals, recovery, stale
replay, sidecar extraction and replacement, and effect custody remain CLI
adapter implementation.

The following surfaces remain distinct:

- persisted decode uses the Service Model compatibility codec;
- transport `Value` decode remains a raw bounded-stack CLI adapter and does
  not materialize persisted legacy policy;
- staged migration remains CLI orchestration around model-owned pure helpers,
  process and boot observation, placeholder construction, contamination
  evidence, backup and recovery artifacts, and explicit application; and
- ordinary encode does not run staged-migration invariants or host
  observation.

The staged migration's manually maintained known-key list is already missing
canonical fields and can overwrite newly derived values through its successor
preservation branch. This packet freezes, rather than silently repairs, that
baseline. P205 will not claim universal staged unknown-field fidelity from the
ordinary aggregate flatten. Any correction requires its own witness and
coordination with the bugfix lane; it is not part of this mechanical ownership
cutover.

Acceptance requires exactly one `struct ServiceState` and one inherent
`impl ServiceState`, both in Service Model; zero CLI aggregate or inherent-impl
definitions; no model import of CLI, filesystem, process, runtime, provider,
HTTP, MCP, dashboard, or clock acquisition; unchanged ordinary legacy and v2
decode; exact schema-rejection identities; deterministic pretty bytes with one
newline; top-level unknown scalar, object, array, and null round trips;
unchanged populated aggregate wire output; unchanged source-provenance
omission; unchanged derived views; unchanged retirement admission; unchanged
route matching; and unchanged repository no-op, overflow, stale replay,
prepared-payload, and sidecar behavior.

Hard stops are a duplicate aggregate, foreign inherent implementation,
generic JSON or callback mutation surface, unledgered public field, host
observation in the codec, changed revision or replay ordering, automatic
cross-record validation on normal writes, altered lifecycle sidecar
projection, staged unknown-field semantic change, or an unbounded CLI facade.

Delegation and model-choice receipt: `/root/p205_aggregate_interface` used
requested `gpt-5.6-terra` high routing to design the minimum aggregate, codec,
visibility, and retained-adapter interface. `/root/p205_method_disposition`
used requested `gpt-5.6-sol` high routing to inventory all 19 methods, their
relative caller risk, helper closure, and ordered test witnesses.
`/root/p205_aggregate_challenge` used requested `gpt-6-astra` high routing for
an adversarial review and found three blocking corrections to the initial
sequence: the foreign inherent-impl rule, the remote-view route-matcher upward
edge, and the exact repository revision/no-op/replay ordering. All workers were
read-only and performed no build, test, runtime, CI, forge, or publication
effect. The runtime did not independently expose effective model metadata.

The primary accepted the challenge corrections and the exact method/helper
inventory. It retained one atomic aggregate ownership packet rather than the
method worker's proposed post-aggregate method packets because a foreign
inherent implementation cannot provide an intermediate compiling state. The
existing methods and fields are explicitly migration compatibility surface,
not the final interface; typed transition and projection replacement plus
field privacy remain mandatory P4 closure. The primary also rejected the
interface worker's proposed staged known-key repair from this packet because
it would combine a behavior correction with the mechanical ownership move.

Progress classification: this packet completes P2 aggregate ownership and a
substantial P3 pure-decision closure together because the language ownership
rule makes that the smallest compilable deep-module move. Facade deletion and
private-field closure remain separate P4 outcomes.

## Checkpoint 40 | Canonical Aggregate And Persisted Codec Accepted

State transition: Service Model now owns the only `ServiceState` declaration,
the only inherent implementation, the 44-field aggregate, all nineteen
existing aggregate methods, ordinary persisted compatibility decoding,
deterministic prepared-state encoding, explicit invariant validation, revision
successor and no-op decisions, and the flattened unknown top-level field bag.
The CLI no longer declares or implements the aggregate. Its temporary
`native::service_model` compatibility path re-exports the crate-owned type
while repository, staged-migration, process, transport, and effect custody
remain CLI adapters.

The repository cutover preserves the frozen ordering. It prepares a checked
successor before the mutator, compares the payload while ignoring only the
revision, revalidates no-ops against durable state, replays stale candidates
under exclusive custody, and commits the exact prepared primary and sidecar
payloads. Persisted decode uses the model compatibility codec; raw transport
decode and staged migration remain distinct CLI paths. The known staged
migration key-list defect remains frozen for the bugfix lane and was not
silently changed.

Aggregate helper closure is canonical. Abandoned-retirement profile fencing
and remote-view route matching moved below the CLI. Built-in policy inventory,
default profile-seeding URL selection, and inactive-lease classification now
have one Service Model owner; the three identical CLI helpers were deleted.
`LeaseState::is_inactive()` is the shared typed decision. The exact twenty
migration-only fields remain `#[doc(hidden)] pub` under the recorded ledger;
this checkpoint does not claim final field privacy.

The architecture contract now requires the aggregate module, exactly one
`ServiceState` declaration and inherent implementation, all direct owner paths,
the codec and revision interface, the exact twenty-field hidden ledger, the
crate exports, no upward or `super::` imports, and a CLI re-export without a
second aggregate or implementation. Mutation fixtures reject a missing module,
duplicate CLI aggregate, foreign CLI implementation, missing re-export,
additional hidden field, missing codec, missing revision accessor, upward
import, and the earlier owner and serde-predicate mutations.

Acceptance evidence:

- all 150 Service Model unit tests and fourteen integration tests pass;
- the focused `service_model` lane passes 41 tests and the focused
  `service_state` lane passes 89 tests;
- five prepared-transaction tests pass, including no-op revision preservation,
  stale no-op replay, adjacent-revision convergence, and competing-writer
  serialization; the persisted revision probe also passes independently;
- Service API/MCP parity passes for 66 browser controls, 26 Service tools,
  19 Service resources, 101 native Service actions, and 118 Service-request
  actions;
- generated Service client contract and JavaScript type checks pass, and all
  route-confusion no-launch gates pass;
- the changed-surface selector retains the broad local route; formatting,
  strict workspace Clippy with `-D warnings`, diff hygiene, the architecture
  contract, and all mutation fixtures pass; and
- no GitHub CI, runtime, browser, profile, provider, credential, install,
  staging, production, or release effect occurred.

Delegation and model-choice receipt: `/root/p205_aggregate_challenge` used the
requested strong `gpt-6-astra` high route to implement the crate aggregate,
codec, method closure, and model tests. `/root/p205_method_disposition` used
the requested reliable `gpt-5.6-sol` high route for the CLI ownership cutover.
`/root/p205_aggregate_interface` used the requested balanced
`gpt-5.6-terra` high route for the migration invariant adapter.
`/root/p205_aggregate_guard` used the requested fast `gpt-5.6-luna` medium
route for the architecture guard and mutation fixtures; the primary found and
closed two guard gaps by counting all hidden fields and adding the missing
negative mutations. `/root/p205_privacy_audit` used the requested reliable
`gpt-5.6-sol` high route for a read-only direct-access audit. The primary
integrated every diff, repaired test-only facade visibility, removed the three
duplicate helpers, and independently ran the acceptance gates. Runtime-reported
effective model metadata was not independently exposed.

Acceptance state and progress classification: P2 is accepted and the aggregate
portion of P3 is accepted as outcome progress. P4 remains open. The privacy
audit found 351 external aggregate literals, approximately 350 of them test
fixtures, plus high-fan-out runtime-owner, presentation-capacity, principal,
lease, recovery, lifecycle, retirement, authentication, and challenge mutation
clusters. The next bounded packet will close trivial codec metadata and direct
revision reads, migrate the one production aggregate literal, and add the
smallest immutable projections. It will not attempt the full twenty-field
privacy flip in one change.

## Checkpoint 41 | First Privacy Preparation Packet Accepted

State transition: all production CLI reads of the hidden Service State revision
now use `state_revision()`. The runtime-health adapter uses the immutable
`profile_policy_migration()` projection, leaving only the model's canonical
materialization assignment as production field access. The sole production
`ServiceState` struct literal was replaced by the typed
`ConfiguredServiceStateInput` and `ServiceState::from_configured_entities`
interface. That constructor owns defaulting and configuration-source marking,
so future field privacy will not force the configuration adapter back into the
aggregate implementation.

The packet deliberately leaves test-only direct revision writes and aggregate
literals unchanged. They remain compatibility debt for the final fixture and
privacy migration, not production precedent. A first default-plus-assignment
rewrite was rejected after strict Clippy exposed the shallow shape; the primary
replaced it with the typed constructor rather than suppressing the lint. No
hidden field became private yet because Rust would invalidate every remaining
external aggregate literal at once.

Acceptance evidence:

- all 152 Service Model unit tests and fourteen integration tests pass;
- the focused `service_state` lane passes 89 tests, and the configured snapshot
  witness passes again after the typed constructor replaced the interim shape;
- fourteen abandoned-browser-retirement tests, two profile-reset tests, three
  profile-diagnosis tests, and three runtime-health tests pass;
- formatting, strict workspace Clippy with `-D warnings`, diff hygiene, the
  architecture contract, and all mutation fixtures pass; and
- no GitHub CI, runtime, browser, profile, provider, credential, install,
  staging, production, or release effect occurred.

Delegation and model-choice receipt: `/root/p205_revision_literal_cutover` used
the requested fast `gpt-5.6-luna` medium route for the bounded revision-read and
configuration-literal inventory and cutover. `/root/p205_migration_projection`
used the requested balanced `gpt-5.6-terra` high route for the immutable model
projection and install adapter. The primary rejected the intermediate field
assignment shape, added the typed configuration input and constructor, extended
the architecture contract and negative fixture, and independently ran every
acceptance gate. Runtime-reported effective model metadata was not independently
exposed.

Acceptance state and progress classification: the first P4 preparation packet
is accepted as outcome progress. Production aggregate construction, revision
inspection, and profile-policy migration inspection now cross explicit model
interfaces. P4 remains open because approximately 350 test fixtures and the
record-kernel, authority, lifecycle, repository-sidecar, and presentation
mutation clusters still use migration-only field access. The next bounded
outcome is the Service challenge task map kernel, followed separately by the
authentication run map kernel.

## Checkpoint 42 | Service Challenge Task Map Kernel Accepted

State transition: the canonical `ServiceState` aggregate now owns challenge
task lookup, summary, start, status, resume, and cancellation. These methods
compose the existing record-level decisions while owning map replay and
insertion semantics. Replay remains non-mutating, start inserts only a newly
created task, resume resolves the current Service tab handle only after replay
has been ruled out, and cancellation inserts only the newly cancelled record.
The model now reports the formerly adapter-local missing-handle failure through
the stable `challenge_task_service_tab_handle_missing` CLI identity.

The CLI continues to own command parsing, timestamps, repository custody, the
exact-current-handle preparation for start, task projection, and consumer
admission joins. Production CLI code has zero direct access to
`ServiceState.challenge_tasks`; four remaining direct accesses are confined to
the existing test module and remain part of the final fixture/privacy packet.
The architecture contract requires all six aggregate methods and rejects a
production CLI direct-map access, with negative fixtures for both regressions.

Acceptance evidence:

- all 157 Service Model unit tests and fourteen integration tests pass;
- the focused Service challenge adapter lane passes four tests, the focused
  resource lane passes 39 tests, and the focused status lane passes 49 tests;
- formatting, strict workspace Clippy with `-D warnings`, diff hygiene, the
  changed-surface selector readback, the architecture contract, and all
  architecture mutation fixtures pass; and
- no GitHub CI, runtime, browser, profile, provider, credential, install,
  staging, production, or release effect occurred.

Delegation and model-choice receipt: `/root/p205_challenge_state_kernel` used
the requested balanced `gpt-5.6-terra` high route for the aggregate mutation
kernel and model tests. `/root/p205_challenge_cli_cutover` used the requested
capable `gpt-5.6-sol` high route for the higher-fan-out adapter cutover. The
primary reviewed the combined ordering and error semantics, strengthened the
architecture contract and negative fixtures, and independently ran every
acceptance gate. Runtime-reported effective model metadata was not
independently exposed.

Acceptance state and progress classification: the Service challenge task map
kernel is accepted and is outcome progress toward P4 field privacy. The next
bounded outcome is the Service authentication run map kernel. P4 remains open
for that kernel and the crash, lease, authority, lifecycle, repository-sidecar,
presentation, fixture-migration, privacy, facade-deletion, and final
measurement gates.

## Checkpoint 43 | Service Authentication Run Map Kernel Accepted

State transition: the canonical `ServiceState` aggregate now owns immutable
authentication-run lookup, two-stage start replay and insertion, effect
reservation, site observation, provider-watch preparation, verification,
site-action completion, credential-delivery completion, challenge completion,
and cancellation. A narrow `ServiceAuthenticationRunStateError` preserves
not-found, envelope, and underlying authentication-transition domains so the
adapter retains every existing context-specific error identity.

The two-stage start boundary preserves the required authority order: the
adapter proves the current Service tab and exact binding, the aggregate resolves
replay, the adapter performs challenge-consumer admission only for a new run,
and the aggregate inserts only a successfully completed record. All other
production mutations preserve caller ownership before the typed transition.
Observation and watch preparation retain their liveness ordering, verification
does not gain a liveness check, and transition failures that previously
produced a persistable record still return that post-transition record.

Production CLI code now has zero direct access to
`ServiceState.authentication_runs`. Four remaining direct accesses are test
fixtures and remain part of the final fixture/privacy packet. The architecture
contract requires all eleven aggregate methods, rejects production CLI direct
map access, and now proves that the Service Model crate has exactly one inherent
`ServiceState` implementation globally.

Acceptance evidence:

- all 164 Service Model unit tests and fourteen integration tests pass;
- the focused Service authentication adapter lane passes nine tests, and the
  broader authentication-run filter passes ten tests including the closed,
  secret-free request contract;
- formatting, strict workspace Clippy with `-D warnings`, diff hygiene, the
  changed-surface selector readback, the architecture contract, and all
  architecture mutation fixtures pass; and
- no GitHub CI, runtime, browser, profile, provider, credential, install,
  staging, production, or release effect occurred.

Delegation and model-choice receipt: `/root/p205_auth_state_kernel` used the
requested high-capability `gpt-6-astra` high route for the multi-transition
aggregate API and model tests. `/root/p205_auth_cli_cutover` used the requested
workhorse `gpt-5.6-sol` high route for the single-module adapter cutover.
`/root/p205_auth_kernel_review` used the requested balanced `gpt-5.6-terra`
high route for an independent semantic review. The reviewer found no behavior
regression and identified the still-public compatibility field as a high-risk
encapsulation gap. That finding is accepted into the existing final P4
fixture/privacy gate rather than misclassified as closed here: approximately
350 external aggregate fixtures still prevent the field from becoming private
in this packet. The primary corrected the initially proposed second inherent
implementation boundary, strengthened the global guard, reconciled the review,
and independently ran every acceptance gate. Runtime-reported effective model
metadata was not independently exposed.

Acceptance state and progress classification: the Service authentication run
map kernel is accepted and is outcome progress toward P4 field privacy. The
next bounded outcome is the crash-regeneration transaction map kernel. P4
remains open for that kernel and the lease, authority, lifecycle,
repository-sidecar, presentation, fixture-migration, privacy, facade-deletion,
and final measurement gates.

## Checkpoint 44 | Crash-Regeneration Transaction Map Kernel Accepted

State transition: the canonical `ServiceState` aggregate now owns immutable
crash-transaction lookup, deterministic status projection, begin or replay,
phase-receipt application, interruption, ready settlement, and clone-only
redaction for public Service Status. The typed transitions compose the existing
provider-free crash-regeneration decisions and preserve the adapter's exact
missing-first and compare-and-swap ordering. Failed identity, revision, boot
epoch, stable-identity, phase, and receipt checks do not mutate the stored
transaction.

The CLI coordinator retains request validation, repository custody, phase
selection, effect execution, receipt validation, failure wording, and the
already-ready short circuit. The status adapter retains closed-tab compaction
and response-size enforcement while obtaining both the status vector and the
redacted aggregate through model-owned projections. Production CLI code has
zero direct access to `ServiceState.crash_regeneration_transactions`; eight
remaining direct accesses are test-only and remain part of the final
fixture/privacy packet. The architecture contract requires all seven aggregate
methods and rejects production direct-map access.

Acceptance evidence:

- all 171 Service Model unit tests and fourteen integration tests pass;
- the focused crash-regeneration coordinator lane passes six tests and the
  focused Service Status lane passes 49 tests, including public redaction;
- formatting, strict workspace Clippy with `-D warnings`, diff hygiene, the
  changed-surface selector readback, the architecture contract, and all
  architecture mutation fixtures pass; and
- no GitHub CI, runtime, browser, profile, provider, credential, install,
  staging, production, or release effect occurred.

Delegation and model-choice receipt: `/root/p205_crash_state_kernel` used the
requested high-capability `gpt-6-astra` high route for the aggregate
compare-and-swap kernel and model tests. `/root/p205_crash_cli_cutover` used the
requested workhorse `gpt-5.6-sol` high route for the coordinator cutover. A
third economical projection worker was attempted twice but the collaboration
runtime rejected it at the thread limit; the primary completed that two-file
projection cutover, strengthened the architecture contract, reconciled the
combined semantics, and independently ran every acceptance gate.
Runtime-reported effective model metadata was not independently exposed.

Acceptance state and progress classification: the crash-regeneration
transaction map kernel is accepted and is outcome progress toward P4 field
privacy. The next bounded outcome is Lease Authority mutation closure. P4
remains open for that closure and the remaining receipt, authority, lifecycle,
repository-sidecar, presentation, fixture-migration, privacy, facade-deletion,
and final measurement gates.

## Checkpoint 45 | Lease Authority Mutation Closure Accepted

State transition: Service Model now owns the current-claim projection, exact
release, recovery, and revocation replay lookups, and the corresponding three
state-bound mutation entry points. The mutation methods bind the existing
Lease Authority kernel directly to the aggregate's principal registry and
authority state. They do not introduce a second verification path, key-loading
path, signing oracle, clock, filesystem dependency, or runtime effect.

The CLI Lease Authority adapter retains command parsing, authenticated-release
authorization, replay-first ordering, error-context mapping, repository
custody, and response construction. It now crosses typed `ServiceState`
methods for replay and mutation instead of reaching into the embedded
authority state. The Service resource projection obtains the current claim
through the same aggregate seam. Production CLI code has zero direct field
access to `ServiceState.lease_authority`; the existing immutable
`lease_authority()` projection remains available to read-only authority joins.
Test fixtures still construct the documented-hidden field and remain part of
the final fixture and privacy packet.

Acceptance evidence:

- all 173 Service Model unit tests and fourteen integration tests pass;
- the focused Lease Authority adapter lane passes three tests and the focused
  Service resource lane passes 39 tests;
- all 116 Lease Authority crate unit tests and its architecture contract pass;
- formatting, strict workspace Clippy with `-D warnings`, diff hygiene, the
  changed-surface selector readback, the Service Model architecture contract,
  and all architecture mutation fixtures pass; and
- no GitHub CI, runtime, browser, profile, provider, credential, install,
  staging, production, or release effect occurred.

Delegation and model-choice receipt: `/root/p205_lease_state_kernel` used the
requested high-capability `gpt-6-astra` high route for the state-bound
authority interface and provider-free tests. `/root/p205_lease_cli_cutover`
used the requested workhorse `gpt-5.6-sol` high route for the adapter and
resource cutover. The primary preserved the existing key-loading and
verification boundary, strengthened the architecture contract and negative
fixtures, reconciled both worker diffs, and independently ran the acceptance
gates. Runtime-reported effective model metadata was not independently
exposed.

Acceptance state and progress classification: Lease Authority mutation
closure is accepted and is outcome progress toward P4 field privacy. The next
bounded outcome is exact receipt-map closure for profile lease reconciliation,
profile recovery, and profile reset. P4 remains open for those receipts and
the remaining principal, lifecycle, repository-sidecar, presentation,
fixture-migration, privacy, facade-deletion, and final measurement gates.

## Checkpoint 46 | Exact Receipt-Map Interface Freeze

State transition: the three remaining exact receipt maps are frozen as one
receipt-access packet before implementation. Service Model will own typed
lookup, replay matching, and terminal receipt recording for profile lease
reconciliation, profile recovery, and profile reset. The packet removes all
production direct access to those maps without moving repository custody,
effect orchestration, process or boot observation, filesystem cleanup, or
response assembly into the provider-free crate.

The frozen interface contains family-specific replay identities rather than a
generic map accessor or callback mutator. Lease reconciliation replay matches
the idempotency key and authenticated principal only, returns a clone with
`replayed` set, and leaves the stored receipt unchanged. Recovery replay
matches recovery ID, plan ID, principal, profile, producer build identity, and
the applied terminal result. Reset replay additionally matches scope and the
optional target service. Missing keys return no receipt; exact mismatches
retain the existing lease authority-mismatch, recovery-conflict, and
reset-conflict identities. Record methods derive their map key from the typed
receipt and preserve the current replacement behavior without adding a new
failure point or revision change.

The CLI retains the existing order and effect boundaries:

- lease reconciliation keeps current-principal validation before replay, then
  seal, expiry, boot-epoch, and lease compare-and-swap validation before its
  ordered state transitions and receipt recording;
- recovery keeps plan verification and snapshot replay before its external
  acquisition retry, then repeats replay under repository custody before
  postcondition validation, stale-reference repair, and receipt recording; and
- reset keeps plan verification and snapshot replay before preconditions, then
  repeats replay and preconditions under repository custody before its scoped
  mutation and receipt recording.

Hard stops are a generic receipt-map interface, mutable receipt reference,
new conflict after partial mutation, changed revision source, moved acquisition
retry, moved lock deletion, model-side boot acquisition, compensation or retry
change, or a claim that receipt access alone completes the later multi-record
lifecycle extraction. The three fields remain documented-hidden until the
approximately 350 external aggregate fixtures are migrated; eliminating their
production callers is necessary but not sufficient for final field privacy.

Delegation and model-choice receipt: `/root/p205_receipt_kernel_design` used
the requested high-capability `gpt-6-astra` high route for the consequential
boundary and replay semantics. `/root/p205_receipt_cli_audit` used the
requested workhorse `gpt-5.6-sol` high route for the production-call and test
witness inventory. Both workers were read-only, received no Git, build, test,
forge, CI, or runtime custody, and could not spawn children. The primary owns
the frozen interface, integration, guards, validation, and acceptance.

Acceptance state and progress classification: this is an interface freeze,
not implementation acceptance. It removes ambiguity from the next bounded
packet but does not itself advance a P4 implementation criterion. The packet
exits only when all production direct accesses to the three receipt maps are
gone, provider-free replay tests and affected CLI witnesses pass, and the
architecture guard rejects regression.

## Checkpoint 47 | Exact Receipt-Map Closure Accepted

State transition: Service Model now owns exact replay and terminal-record
custody for profile lease reconciliation, profile recovery, and profile reset.
The new borrowed identity inputs preserve each family's deliberately different
matching contract. The typed replay error keeps Lease Authority mismatch,
recovery receipt conflict, and reset receipt conflict distinct while the CLI
retains their established contextual messages.

All three record methods derive the map key from the typed receipt and retain
the previous replacement behavior. They do not increment Service State or
runtime-owner revisions, add a late conflict after partial mutation, or perform
serialization or effects. Lease replay marks only its returned clone. Recovery
status obtains one immutable receipt while preserving the unauthenticated
`not_found` response on a miss.

The CLI has zero production direct access to
`profile_lease_reconcile_receipts`, `profile_recovery_receipts`, or
`profile_reset_receipts`. Lease authority validation and replay order, recovery
plan verification and two-stage replay around acquisition, reset verification
and two-stage replay, postcondition checks, stale-reference repair, lock-file
handling, event append, response assembly, and both revision sources remain
unchanged. Direct accesses below test modules remain fixture debt for the final
aggregate privacy packet.

Acceptance evidence:

- all 178 Service Model unit tests and fourteen integration tests pass;
- five focused lease-reconciliation tests pass;
- all 37 profile recovery and reset module tests pass;
- the receipt codec round trip and prepared no-op revision witnesses pass;
- formatting, strict workspace Clippy with `-D warnings`, diff hygiene, the
  Service Model architecture contract, and all architecture mutation fixtures
  pass; and
- no GitHub CI, runtime, browser, profile, provider, credential, install,
  staging, production, or release effect occurred.

Delegation and model-choice receipt: `/root/p205_receipt_kernel_design` used
the requested high-capability `gpt-6-astra` high route first for the frozen
boundary and then for the two-file model implementation and provider-free
tests. `/root/p205_receipt_cli_audit` used the requested workhorse
`gpt-5.6-sol` high route first for the exact caller and ordering inventory and
then for the two-file CLI cutover. The primary froze the interface, implemented
the architecture guard and negative fixtures, reviewed the combined replay
ordering, and ran the complete acceptance batch. Runtime-reported effective
model metadata was not independently exposed.

Acceptance state and progress classification: exact receipt-map closure is
accepted and is outcome progress toward P4 field privacy. The next bounded
outcome is Service Principal registry closure, followed by the larger
runtime-owner and repository-sidecar authority boundary. P4 remains open for
those authority surfaces, the lifecycle and presentation transitions,
fixture migration, field privacy, facade deletion, and final measurement.

## Checkpoint 48 | Service Principal Registry Interface Freeze

State transition: the Service Principal registry boundary is frozen as nine
typed `ServiceState` methods before implementation. Four immutable projections
expose only the registry revision, one exact principal, one exact capability,
and ordered capability records. Two snapshot-bound methods own capability
authentication and current-authority validation. Two typed transitions
delegate registration and revision-guarded rotation to the existing Lease
Authority kernel. One pure projection constructs a read-only Lease Authority
view from the aggregate's principal registry and lease state internally.

The interface preserves the existing distinctions. Authentication retains its
missing-token, unique-digest, active-capability, profile, and active-principal
error order and does not add a provenance requirement. Current-authority
validation remains stronger: it requires registered-capability provenance,
exact capability revision, and exact principal and profile identity.
Capability iteration retains `BTreeMap` value order and includes revoked
records because existing status and migration consumers apply different
filters. Registration and rotation retain staged all-or-nothing registry
behavior, schema initialization, current metadata, exact replay and conflict
rules, compare-and-swap precedence, and registry revision semantics.

The CLI retains capability generation and private-file custody, repository
mutation, event append, cleanup, process and boot observation, runtime-owner
binding, profile-path resolution, HTTP and MCP transport mapping, and browser
or daemon effects. Registration remains private capability file, registry
transition, current-owner binding, event append, and persistence. Rotation
remains private capability file, active-work fencing, registry compare-and-swap,
owner-binding rotation, event append, and persistence. Lease authorization
schema validation remains before repository loading, while exact physical
profile and runtime-owner checks remain after the aggregate authorization join.
Signing and verification remain CLI-invoked Lease Authority operations because
their existing implementation loads trust-key filesystem state; Service Model
must not acquire that effect merely to hide two fields.

The caller audit found 42 production direct accesses across seven CLI files
and no repository sidecar. This packet removes those accesses without adding a
registry getter, mutable record reference, generic callback, registry
replacement, serialized authority field, or envelope revision mutation. It
does not claim extraction of the larger principal-continuity, subordinate work
lease, legacy migration, or daemon recourse algorithms. Those pure decisions
join sessions and runtime-owner state and remain a separately reviewable
follow-up rather than being obscured behind this access closure.

Hard stops are changed authentication or rotation precedence, provenance
homogenization, uniqueness strengthening in legacy projections, moved secret
or filesystem custody, moved process observation, combined registry and
runtime-owner atomicity, changed error strings, or a broad principal-registry
reference. Documented-hidden field visibility remains until external aggregate
fixtures are migrated.

Delegation and model-choice receipt: `/root/p205_receipt_kernel_design` used
the requested high-capability `gpt-6-astra` high route for the authority
boundary, corrected nine-method interface, and semantic hard stops. Its
pre-edit review found and prevented the transitive trust-key filesystem edge.
`/root/p205_receipt_cli_audit` used the requested workhorse `gpt-5.6-sol` high
route for the 42-access production inventory, order and effect classification,
test witnesses, and disjoint write plan. Both were read-only and had no build,
test, Git, forge, CI, runtime, or child-agent custody. The primary selected
registry access closure now and retained full continuity extraction as an
explicit later outcome.

Acceptance state and progress classification: this is an interface freeze,
not implementation acceptance. It makes the authority boundary executable but
does not by itself advance a P4 implementation criterion. Exit requires zero
production direct `service_principals` access, provider-free kernel tests,
affected CLI witnesses, strict local quality gates, and an architecture
regression guard.

## Checkpoint 49 | Service Principal Registry Closure Accepted

State transition: Service Model now owns nine typed Service Principal registry
operations: registry revision, exact principal and capability projections,
ordered capability iteration, capability authentication, current-authority
validation, registration, rotation, and construction of a read-only Lease
Authority view from the current aggregate. The interface exposes no registry
reference or mutable record and preserves the Lease Authority kernel's
existing authentication, provenance, replay, conflict, compare-and-swap, and
revision semantics.

The CLI has zero production direct access to `ServiceState.service_principals`.
Capability generation and private-file custody, repository mutation, event
append, runtime-owner binding, physical profile checks, signing and
verification, key loading, HTTP and MCP error mapping, and browser or daemon
effects remain adapter responsibilities. The model-owned
`lease_authority_view()` is a pure borrowed join and does not load trust keys
or perform an effect.

The initial caller inventory found 42 direct accesses in seven native CLI
files. Independent review then found one additional production MCP access and
a guard blind spot: the production-source scan discarded every file after its
first early `#[cfg(test)]` item. The MCP caller now uses the aggregate
authentication method. The guard now removes only test-gated items and
test-only files, retains later production code, and has both a positive
test-only-access fixture and a negative production-leak-after-test-item
fixture. This closed the false-green path rather than recording the first
passing guard as acceptance.

Acceptance evidence:

- all 184 Service Model unit tests and fourteen integration tests pass;
- 82 focused provider-free CLI witnesses pass across principal continuity,
  Lease Authority adapter joins, profile lease registration and rotation,
  profile acquisition and recovery, HTTP bearer authority, MCP capability
  authority, and migration safety;
- all 116 Lease Authority tests and its architecture contract pass;
- Service API and MCP parity, generated service-client contract and type
  checks, and the service-collection no-launch smoke pass;
- formatting, strict workspace Clippy with `-D warnings`, diff hygiene, the
  Service Model architecture contract, its mutation fixtures, and the
  changed-surface selector readback pass; and
- no GitHub CI, runtime, browser, profile, provider, credential, install,
  staging, production, or release effect occurred.

Delegation and model-choice receipt: `/root/p205_receipt_kernel_design` used
the requested high-capability `gpt-6-astra` high route for the corrected
nine-method boundary and provider-free tests. `/root/p205_receipt_cli_audit`
used the requested workhorse `gpt-5.6-sol` high route for the production
cutover. `/root/p205_principal_guard_fixture` used the economical
`gpt-5.6-luna` medium route for the positive guard fixture, and
`/root/p205_principal_test_routes` used `gpt-5.6-luna` low for focused test
selection. The primary integrated and corrected their work, added the residual
MCP cutover and witness, fixed the production-source guard, and independently
ran every acceptance gate. Independent privacy review found the MCP and guard
defects; closed-world verification was limited to those accepted findings.

Acceptance state and progress classification: Service Principal registry
access closure is accepted and is outcome progress toward P4 field privacy.
The next bounded outcome is the pure Principal Continuity kernel: state-bound
session authority, subordinate work-lease transitions, continuity recourse,
and legacy migration decisions with boot epoch supplied explicitly by the CLI.
Runtime-owner mutation and repository-sidecar custody remain a later boundary
rather than being exposed through a broad registry getter.

## Checkpoint 50 | Principal Continuity Kernel Interface Freeze

State transition: the next provider-free packet is frozen as three canonical
records and five `ServiceState` methods. Service Model will own
`PrincipalContinuityDecision`, `LegacyPrincipalMigrationDisposition`, and
`LegacySessionPrincipalMigrationPlan`, plus aggregate entry points for current
authenticated session-work authority, continuity recourse, legacy migration
planning, session work-lease binding, and tab work-lease binding.

The session binding method receives the observed boot epoch explicitly as an
`Option<String>`. The CLI compatibility wrapper obtains that observation at
the existing call boundary and contains no copied decision. Tab binding needs
no host observation. The packet preserves the current SHA-256 work-lease ID,
24-character lowercase digest prefix, saturating revision, exact error codes
and messages, lexical expiry comparison, deterministic ordering, and current
authority and owner-binding predicates.

Decision precedence remains frozen. Session-work authority requires registered
session provenance, nonterminal lease, present future expiry, a nonempty work
lease with positive revision, exactly one matching registered owner binding,
and current capability authority. Continuity distinguishes stale capability,
missing or stale owner binding, mismatched owner principal, unproven holder,
foreign holder, retained same-principal holder, stale same-principal session,
and ready owner without a session in that order. Legacy labels never become
authority, and a unique verified migration candidate remains observation-only
until an explicit later commit path.

The CLI retains boot observation, repository mutation, event append,
runtime-owner mutation, capability files, transport mapping, and every browser
or daemon effect. Rejoin ordering remains owner binding or refresh, session
binding, session lease and observation update, ordered tab binding, then lease
projection. The packet does not combine those steps into an atomic aggregate
transition or begin runtime-owner sidecar closure.

Hard stops are a broad principal or runtime-owner getter, model-side process or
boot observation, callbacks, timestamp normalization, new authority checks,
changed ambiguity or provenance behavior, changed errors, runtime-owner or
envelope revision mutation, repository ordering changes, or merged session and
tab transaction semantics.

Delegation and model-choice receipt: `/root/p205_receipt_kernel_design` reused
the requested high-capability `gpt-6-astra` high route for the consequential
five-method interface, semantic precedence, and pure/effect boundary.
`/root/p205_receipt_cli_audit` reused the requested workhorse
`gpt-5.6-sol` high route for exact caller, ordering, and focused-witness
inventory. Both workers are read-only; the primary owns the freeze,
implementation split, integration, validation, and acceptance.

Acceptance state and progress classification: this is an interface freeze, not
implementation acceptance. It removes ambiguity from the next bounded packet
but does not advance P4 until the canonical records and decisions move, CLI
production callers use the aggregate seam, focused model and adapter witnesses
pass, and the architecture guard rejects copied continuity logic.

## Checkpoint 51 | Principal Continuity Kernel Accepted

State transition: Service Model now owns the three canonical continuity and
legacy-migration records plus five aggregate methods for authenticated retained
session work, continuity recourse, legacy migration planning, session work
binding, and tab work binding. The CLI Service Principal module is a thin
compatibility facade: four functions delegate directly, while session binding
obtains the current boot epoch and supplies it to the provider-free transition.
All production principal, owner, session, tab, hashing, sorting, error, and
mutation decisions moved out of that facade.

The kernel preserves current authority and owner-binding predicates, error
precedence and messages, NUL-delimited SHA-256 work-lease IDs with 24-character
lowercase prefixes, saturating work revisions, deterministic holder and plan
ordering, migration ambiguity, and the exact session and tab field deltas. It
does not mutate the principal registry, runtime-owner registry, Service State
envelope revision, repository state, or any browser or daemon resource.

Boot observation remains CLI-owned and is read immediately before delegation.
It therefore precedes the pure validation inside the model rather than
occurring between validation and mutation as it did inside the former CLI
implementation. The observation is an infallible read returning
`Option<String>`; closed-world review confirmed that errors, IDs, revisions,
and state mutation remain unchanged. No model-side process, filesystem, clock,
or boot dependency was introduced.

Acceptance evidence:

- all 191 Service Model unit tests and fourteen integration tests pass,
  including seven new continuity, migration, mutation-delta, ordering,
  authority, wire, boot-input, saturation, and rejection-invariance tests;
- 39 focused CLI witnesses pass: eight Service Principal tests, all 28 profile
  lease tests, and exact health, control-plane, and daemon caller tests;
- formatting, strict workspace Clippy with `-D warnings`, diff hygiene, the
  Service Model architecture contract, its mutation fixtures, and the
  changed-surface selector readback pass;
- the first integrated model run failed only because the new synthetic fixture
  used a non-SHA-256 profile identity; one bounded repair replaced it with the
  Lease Authority's valid digest shape, after which the complete model lane
  passed; and
- no GitHub CI, runtime, browser, profile, provider, credential, install,
  staging, production, or release effect occurred.

Delegation and model-choice receipt: `/root/p205_receipt_kernel_design` used
the requested high-capability `gpt-6-astra` high route for the model kernel,
seven provider-free tests, and the one fixture repair. The primary independently
ran the complete model lane. `/root/p205_receipt_cli_audit` used the requested
workhorse `gpt-5.6-sol` high route for the 471-line CLI deletion and thin
facade. The primary fixed test-only imports, integrated the architecture guard,
and ran all CLI and quality gates. `/root/p205_privacy_audit` performed one
closed-world comparison against `ec6d00f2` and returned no findings.

Acceptance state and progress classification: Principal Continuity is accepted
and is outcome progress toward P4 field privacy. The next bounded outcome is a
runtime-owner and repository-sidecar interface freeze. That packet must
separate pure owner projections and transitions from filesystem sidecar
restoration, transaction ordering, process observation, and runtime effects
before any implementation begins.

## Checkpoint 52 | Runtime Owner Persistence And Authority Interface Freeze

State transition: the remaining runtime-owner surface is classified into three
separate contracts before implementation. Repository persistence projection
and restoration is the first packet. Purpose-specific immutable authority
projections follow as a second packet. Rollback-sensitive lifecycle and
principal-binding transitions remain a third packet that requires its own
freeze after every caller's partial-mutation behavior is classified. P205 will
not make the aggregate field private or expose a whole-registry escape hatch
until those contracts replace their production callers.

The closed-world inventory found 202 production direct runtime-owner field
dereferences across 35 CLI files, plus 286 test-only dereferences. Only six
production dereferences belong to repository serialization and restoration,
all in `service_store.rs`. The rest join runtime authority with diagnostics,
status, profiles, principals, boot and process observations, lifecycle
transactions, reconciliation, browser actions, or external effects. Treating
that mixed surface as one cutover would obscure authority ordering and make a
passing build weaker evidence than the current behavioral contracts.

The first implementation packet freezes these provider-free model types and
aggregate methods:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RuntimeOwnerPersistenceSnapshot {
    registry: agent_browser_lease_authority::RuntimeOwnerRegistry,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuntimeOwnerPersistenceRestore {
    pub owner_registry: Option<RuntimeOwnerPersistenceSnapshot>,
    pub lifecycle_records: Option<BTreeMap<String, RuntimeLifecycleRecord>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeOwnerPersistenceParts {
    pub owner_registry: RuntimeOwnerPersistenceSnapshot,
    pub lifecycle_registry_revision: u64,
    pub lifecycle_records: BTreeMap<String, RuntimeLifecycleRecord>,
}

impl ServiceState {
    pub fn restore_runtime_owner_persistence(
        &mut self,
        input: RuntimeOwnerPersistenceRestore,
    );

    pub fn runtime_owner_persistence_parts(
        &self,
    ) -> RuntimeOwnerPersistenceParts;

    pub fn strip_runtime_lifecycle_for_persistence(&mut self);
}
```

The transparent snapshot has exactly the existing registry wire shape but no
registry getter, `Deref`, mutable projection, callback, or transition method.
Repository envelopes retain their schema strings and parse behavior. The CLI
selects an owner or lifecycle sidecar exactly when its current schema string is
nonempty; the model receives that selection as `Some`. A present empty owner
snapshot replaces embedded authority, and present empty lifecycle records
clear embedded lifecycle evidence. Restoration applies the selected owner
snapshot first and selected lifecycle records second. The historical lifecycle
sidecar revision remains ignored and cannot advance owner authority.

The persistence projection returns owned copies of the owner registry without
lifecycle records, the original registry revision, and the original lifecycle
records. It leaves the source aggregate unchanged. Stripping changes only the
lifecycle records on the prepared clone and preserves owners, principal
bindings, registry revision, Service State revision, and unknown fields. The
repository must preserve this preparation order:

```text
clone, migrate, refresh derived views, remove builtins,
project persistence parts, serialize lifecycle payload,
strip lifecycle records from the prepared clone,
serialize primary state, handoff payload, and owner payload
```

Filesystem paths, sidecar envelopes, reads, absent-file behavior, parse and
serialization errors, bounded-stack workers, repository locks and
compare-and-swap, transaction recovery, temporary files, replacement order,
rollback, and transaction-marker custody remain CLI responsibilities. The
owner sidecar still replaces the embedded registry before the lifecycle
sidecar replaces lifecycle records. Commit visibility remains handoff, owner,
lifecycle, then primary, with the transaction marker cleared last.

First-packet witnesses are the no-sidecar, owner-only, lifecycle-only, both,
and explicit-empty restoration matrix; conflicting lifecycle replacement
without authority revision change; source-preserving projection; exact strip
delta; and transparent existing-wire round trip. The focused CLI witnesses are
`durable_runtime_owner_registry_survives_a_legacy_state_writer`,
`lifecycle_sidecar_preserves_new_evidence_without_breaking_legacy_registry_readers`,
and `four_file_service_state_commit_is_atomic_at_every_write_and_rename_boundary`,
plus the existing transaction-recovery and stale-revision tests touched by the
cutover. The architecture guard must reject production repository field access,
snapshot getters or mutable escape hatches, and copied lifecycle strip or
restore logic.

The second packet may expose exact owner, binding, lifecycle, revision,
authority-currentness, and session-binding projections, but no whole-registry
getter or general iterator is admitted by this checkpoint. The third packet
must preserve each caller's existing transaction semantics. In particular,
staged lifecycle transitions discard kernel failures, reconciliation sometimes
retains ordered partial transitions, capability rotation may retain removal of
the former binding, reset terminalization follows filesystem effects, and CLI
intent conversion supplies current boot and route-policy observations at the
existing boundary.

Hard stops are changed sidecar precedence, schema admission, historical
revision handling, serialization or commit error order, partial-mutation
behavior, observation placement, filesystem custody, runtime effects, a broad
registry accessor, or a claim of global field privacy after only the six
repository accesses close.

Delegation and model-choice receipt: `/root/p205_receipt_kernel_design` used
the requested high-capability `gpt-6-astra` high route for the persistence,
projection, transition, and effect boundary. `/root/p205_receipt_cli_audit`
used the requested workhorse `gpt-5.6-sol` high route for the production-only
202-access inventory, test separation, repository ordering, focused witnesses,
and hidden-coupling audit. Both were read-only and had no build, test, Git,
forge, CI, runtime, or child-agent custody. The primary reconciled their
sequencing recommendations by selecting the six-access repository seam first,
then immutable projections, then transition closure.

Acceptance state and progress classification: this is an interface freeze and
does not itself advance a P4 implementation criterion. The first packet exits
only when `service_store.rs` has zero production direct access to the field,
the model and repository witnesses pass, and the architecture guard rejects
both copied persistence logic and an authority escape hatch.

## Checkpoint 53 | Runtime Owner Persistence Boundary Accepted

State transition: Service Model now owns the opaque runtime-owner persistence
snapshot, selected restoration input, owned persistence projection, and three
aggregate operations for restore, projection, and lifecycle stripping. The CLI
repository retains sidecar selection, schema envelopes, parsing,
serialization, filesystem paths, transaction recovery, locking,
compare-and-swap, temporary files, replacement, rollback, and marker custody.
All six production repository dereferences of the aggregate field are removed.

The transparent snapshot preserves the existing owner-registry JSON body while
keeping the registry field private and providing no inherent method, getter,
conversion, `Deref`, mutable view, callback, or transition surface.
Restoration applies the selected owner snapshot before selected lifecycle
records. `None` retains embedded state while an explicit empty value replaces
it. Lifecycle-sidecar historical revision remains non-authoritative. The owned
projection preserves the source aggregate, carries the original revision and
lifecycle records, and strips lifecycle records only from its owner snapshot.
The separate strip operation changes only lifecycle evidence on the prepared
clone and is idempotent.

Repository load still selects sidecars from nonempty schema strings, overlays
owner authority before lifecycle evidence, and retains the missing-primary fast
return. Preparation still migrates, refreshes derived views, removes builtins,
projects and serializes lifecycle evidence, strips the prepared clone, then
serializes primary state, handoffs, and the owner sidecar. Commit and recovery
still write and replace handoff, owner, lifecycle, and primary artifacts in
that order, with primary as the final visibility point and the transaction
marker cleared last.

Acceptance evidence:

- all 196 Service Model unit tests and fourteen integration tests pass,
  including five new tests spanning eight restoration selections, explicit
  empty values, precedence and revision preservation, owned projections,
  exact and idempotent stripping, transparent wire compatibility, and
  lifecycle override without authority advancement;
- the three exact repository witnesses for legacy-writer owner survival,
  lifecycle-sidecar compatibility, and four-file atomicity pass;
- four additional save/load, persisted-revision, recovery-idempotence, and
  concurrent-revision replay witnesses pass;
- formatting, strict workspace Clippy with `-D warnings`, diff hygiene, the
  Service Model architecture contract, its mutation fixtures, and the
  changed-surface selector readback pass; and
- no GitHub CI, runtime, browser, profile, provider, credential, install,
  staging, production, or release effect occurred.

One initial local Cargo command incorrectly combined workspace and package
selection, so Cargo ran the broad CLI suite instead of the intended model
crate. That non-gate run encountered one unrelated route-host failure and then
the known default-stack overflow. No source repair was inferred from it. The
correct model-manifest command and all scoped acceptance witnesses passed under
the repository wrapper.

Independent review found no semantic, wire, precedence, revision, ordering, or
custody defect. It found one blocking architecture-guard omission: a public
snapshot field or a getter placed after a private helper could evade the first
guard. The primary completed the single allowed review repair by rejecting a
public registry field, all inherent snapshot implementations, selected
conversion traits, and persistence-method calls outside `service_store.rs`.
Public-field and helper-before-getter fixtures now fail closed alongside the
existing getter, `Deref`, direct-field, copied-strip, and foreign-caller
fixtures.

Delegation and model-choice receipt: `/root/p205_receipt_kernel_design` used
the requested high-capability `gpt-6-astra` high route for the three model
types, three aggregate methods, and provider-free matrix. `/root/p205_receipt_cli_audit`
used the requested workhorse `gpt-5.6-sol` high route for the six-access
repository cutover and exact ordering preservation. The primary implemented
and hardened the architecture guard, ran all local acceptance gates, and
reconciled the independent `/root/p205_privacy_audit` review finding.

Acceptance state and progress classification: runtime-owner repository
persistence closure is accepted and is outcome progress toward P4 field
privacy. The next bounded outcome is purpose-specific immutable runtime-owner
projection closure. It must reduce the read-only production surface without a
whole-registry getter or general iterator and without moving process, boot,
browser, filesystem, or provider effects.

## Checkpoint 54 | Immutable Runtime Owner Projection Interface Freeze

State transition: the next runtime-owner packet is frozen as six
purpose-specific immutable projections. It closes all eighteen production
direct field expressions in `install.rs`, `service_boot_epoch.rs`,
`service_diagnostics.rs`, `service_profile_diagnosis.rs`,
`service_status_projection.rs`, `service_resources.rs`, and
`service_resources/retained_tree.rs`. Test-only fixture access remains visible
debt for the final aggregate fixture migration and does not justify a mutable
production escape hatch.

The model owns these borrowed or owned projection records:

```rust
pub struct RuntimeLifecycleAuthoritySummary {
    pub registry_revision: u64,
    pub owner_count: usize,
    pub record_count: usize,
    pub lifecycle_state_counts: BTreeMap<String, usize>,
    pub cleanup_obligation_state_counts: BTreeMap<String, usize>,
}

pub struct RuntimeLifecycleBootEpochObservation {
    pub logical_browser_id: String,
    pub boot_epoch: Option<String>,
}

pub struct ProfileRuntimeAuthority<'a> {
    pub registry_revision: u64,
    pub owner: Option<&'a ProfileOwner>,
    pub principal_binding: Option<&'a RuntimeOwnerPrincipalBinding>,
}

pub struct RuntimeControlPlaneAuthority<'a> {
    pub attestation: Option<RuntimeOwnerAttestation>,
    pub current_owner: Option<&'a ProfileOwner>,
    pub lifecycle: Option<&'a RuntimeLifecycleRecord>,
}

pub struct RuntimeLaneAuthority<'a> {
    pub owner: Option<&'a ProfileOwner>,
    pub lifecycle: Option<&'a RuntimeLifecycleRecord>,
}

pub struct RuntimeResourceLane<'a> {
    pub browser_id: &'a str,
    pub lifecycle: &'a RuntimeLifecycleRecord,
    pub browser_pid: Option<u32>,
    pub tab_count: usize,
}
```

`ServiceState` owns exactly six methods:

```rust
pub fn runtime_lifecycle_authority_summary(
    &self,
) -> RuntimeLifecycleAuthoritySummary;

pub fn runtime_lifecycle_boot_epoch_observations(
    &self,
) -> Vec<RuntimeLifecycleBootEpochObservation>;

pub fn profile_runtime_authority(
    &self,
    profile_identity_digest: &str,
) -> ProfileRuntimeAuthority<'_>;

pub fn runtime_control_plane_authority(
    &self,
    session_id: &str,
    browser_id: &str,
) -> Result<RuntimeControlPlaneAuthority<'_>, String>;

pub fn runtime_lane_authority(
    &self,
    profile_identity_digest: &str,
    browser_id: &str,
) -> RuntimeLaneAuthority<'_>;

pub fn runtime_resource_lanes(&self) -> Vec<RuntimeResourceLane<'_>>;
```

The summary preserves registry revision, owner and lifecycle counts, and every
serialized lifecycle and cleanup state label in sorted maps, including the
existing `unknown` fallback and omission of zero-count labels. Install and
status retain repository loading, failure JSON, runtime-health observation,
and response assembly. Resource summary reads the same typed counts and
defaults absent owned, transferring, satisfied, and unknown labels to zero.

Boot observations include exactly lifecycle rows with a process group or
package-launch digest, retain lifecycle-map order, use the record's embedded
logical browser ID, and preserve an absent boot epoch. The CLI retains current
boot observation, cross-resource merging, assessment, and final finding sort.

Profile authority performs independent exact owner and principal-binding
lookups by canonical profile digest and supplies one same-snapshot registry
revision. It does not hide a binding without an owner, repair a generation
mismatch, or make a missing record an error. Live process observation, Chrome
lock inspection, diagnosis ID and JSON assembly remain CLI-owned.

Control-plane authority delegates `attestation_for_session` without changing
its ambiguity or receipt-serialization errors. It then chooses the first owner
in registry map order matching both attested owner ID and generation and
independently looks up lifecycle evidence by the supplied browser ID. Current
boot observation, process hashing, package-launch digest calculation, and
effect-authority response assembly remain in the adapter.

Lane authority returns independent owner and lifecycle options. Abandoned-lane
classification must continue to report owner missing before lifecycle missing,
while retained-tree verification preserves its separate `?` exits. It adds no
validation, generation filtering, or joined absence state.

Resource rows preserve lifecycle-map key order and retain the map key
separately from the embedded logical browser ID. Each row joins only the
browser PID and tab count for that exact key. The CLI keeps process samples,
budgets, activity, RSS aggregation, disposition, GC policy, ancestry, and
retirement effects. Public `runtimeLanes` still serializes only the lifecycle
records in registry order.

Model witnesses cover empty and complete summaries, every lifecycle and
cleanup variant, `u64::MAX` revision, boot inclusion and missing epoch, key and
embedded-ID mismatch, independent profile and lane presence combinations,
binding mismatch visibility, session ambiguity and exact ID plus generation
selection, resource joins and ordering, and full-state nonmutation. Focused CLI
witnesses preserve install and status lifecycle JSON, diagnosis identity and
trace revision, attestation custody, boot findings, resource summary and lane
order, missing-evidence reason precedence, and retained-tree acceptance.

The architecture guard must require the six types and methods, reject direct
production field access in the seven files even after a test-gated item, and
reject a registry return, mutable reference, generic owner or lifecycle
iterator, callback, `Deref`, conversion, renamed getter, or persistence
snapshot workaround. No test-only mutation helper is promoted into production.

Hard stops are changed filtering or map order, collapsed missing states,
changed error identity, binding or generation strengthening, malformed-record
hiding, substituted revision, process or boot observation movement, altered
runtime-health order, resource-policy movement, or any registry handle.

Delegation and model-choice receipt: `/root/p205_receipt_kernel_design` used
the requested high-capability `gpt-6-astra` high route for the exact projection
boundary, borrowed-record semantics, order and error invariants, and effect
separation. `/root/p205_receipt_cli_audit` used the requested workhorse
`gpt-5.6-sol` high route for the production-only eighteen-expression inventory,
caller mapping, existing witnesses, and two coverage gaps. Both were read-only
and had no edit, build, test, Git, forge, CI, runtime, or child-agent custody.

Acceptance state and progress classification: this is an interface freeze and
does not itself advance a P4 implementation criterion. Exit requires zero
production direct runtime-owner field access in all seven files, provider-free
projection tests, affected adapter witnesses, and a guard that rejects every
broad access route above.

## Checkpoint 55 | Immutable Runtime Owner Projections Accepted

State transition: Service Model now owns six purpose-specific immutable
runtime-owner projections for lifecycle summary, boot evidence, profile
authority, control-plane authority, lane custody, and resource rows. The seven
selected CLI files have zero production direct access to the aggregate field
and no production helper accepts a raw registry. The CLI retains repository
loading, runtime-health observation, current boot and process observation,
profile-path canonicalization, filesystem lock inspection, hashing, JSON
assembly, resource policy, ancestry, GC classification, and all effects.

Implementation corrected the frozen inventory from eighteen to nineteen
production expressions. One status provenance helper appears after an early
test-gated module and was missed by the first read-only count. The production-
aware architecture scan retained that later item and forced its conversion to
the resource-row projection. This is evidence for the guard's item-aware test
removal rather than a broader scope expansion.

Lifecycle summaries preserve exact registry revision, owner and record counts,
serialized lifecycle and cleanup labels, sorted count maps, unknown fallback,
and omitted zero-count categories. Boot projections preserve their inclusion
predicate, lifecycle-map order, embedded logical ID, and absent or empty epoch.
Profile projections retain independent exact owner and principal-binding
lookups plus one same-snapshot revision. Control-plane projections delegate
session attestation and its errors, then retain the first map-ordered owner
matching both ID and generation and the independent browser-key lifecycle row.
Lane custody retains separate missing states. Resource rows preserve the map
key separately from the embedded ID and join only exact-key browser PID and tab
count.

Install and status now consume typed summaries rather than raw registries.
Resource output still serializes only lifecycle records for `runtimeLanes`, in
registry key order. Missing owner still precedes missing lifecycle in abandoned
lane protection, and retained-tree verification retains its separate early
returns. Diagnosis identity and trace use the same projected registry revision.
No projection exposes a registry, mutable reference, callback, generic
iterator, conversion, persistence snapshot, or model-side observation.

Acceptance evidence:

- all 203 Service Model unit tests and fourteen integration tests pass,
  including seven new projection tests for every lifecycle and cleanup label,
  maximum revision, boot filtering and order, independent missing and mismatch
  states, exact attestation errors and selection, terminal-history delegation,
  resource joins, map-key order, and aggregate nonmutation;
- 25 focused CLI witnesses pass across boot evidence, all seven control-plane
  diagnostics tests, install and status lifecycle summaries, four profile
  diagnosis outcomes, resource summary and cleanup accountability, abandoned
  and retained custody, and the two new embedded-ID and runtime-lane ordering
  regressions;
- formatting, strict workspace Clippy with `-D warnings`, diff hygiene, the
  Service Model architecture contract, mutation fixtures, and changed-surface
  selector readback pass; and
- no GitHub CI, runtime, browser, profile, provider, credential, install,
  staging, production, or release effect occurred.

The selector conservatively classified this packet as broad because it changes
`install.rs` and the repository guard scripts. The packet did not alter
workstation provisioning, payloads, privileges, Guacamole assets, or runtime
effects. Exact install and status lifecycle witnesses plus strict workspace
quality gates were used instead of the unrelated comprehensive workstation
lane, consistent with this plan's focused local validation boundary.

Independent review found no projection-semantic, ordering, revision, caller,
or effect-custody defect. It found one blocking guard omission: a free function
or trait implementation inside the new projection module could expose the raw
registry without changing a checked aggregate signature. The primary completed
the single review repair by making production projection code data-only and
rejecting any raw registry reference, public function, or implementation in
that module. Free-function and impl escape fixtures now fail closed alongside
direct-field, raw-parameter, mutable, iterator, persistence, and post-test-item
fixtures.

Delegation and model-choice receipt: `/root/p205_receipt_kernel_design` used
the requested high-capability `gpt-6-astra` high route for six model records,
six aggregate methods, and seven provider-free tests. `/root/p205_receipt_cli_audit`
used the requested workhorse `gpt-5.6-sol` high route for the nineteen-access
cutover across seven files and the two coverage-gap witnesses. The primary
implemented and hardened the guard, ran all local acceptance gates, and
reconciled the independent `/root/p205_privacy_audit` review finding.

Acceptance state and progress classification: immutable runtime-owner
projection closure is accepted and is outcome progress toward P4 field
privacy. The next bounded outcome is rollback-sensitive runtime-owner
transition classification and interface freeze. It must distinguish staged
all-or-nothing transitions from intentionally retained partial mutations and
must not move host observation or effects into the model.

## Checkpoint 56 | Atomic Runtime Lifecycle Transition Interface Freeze

State transition: the remaining runtime-owner surface is classified as 177
production direct expressions across 27 files plus 288 fixture-only
expressions. The first transition packet is limited to the two accesses in
`RuntimeLifecycleAuthority::transition`: clone the current registry, apply one
typed lifecycle intent to the clone, and assign only the successful result.
After this packet, 175 production expressions remain explicit debt.

The item-aware audit also found that the architecture checker's source scanner
currently counts six `service_health.rs` test-module expressions as production
because lifetime syntax confuses its general item stripper. This does not
weaken the targeted first-packet guard, but final aggregate privacy must either
correct that parser case or use an equivalent exact production classifier
before claiming zero.

The model interface is one inherent aggregate method using the existing Lease
Authority vocabulary:

```rust
pub fn apply_runtime_lifecycle_transition_atomically(
    &mut self,
    intent: agent_browser_lease_authority::RuntimeLifecycleIntent,
) -> Result<
    agent_browser_lease_authority::RuntimeLifecycleTransition,
    String,
>;
```

The method clones the current registry, calls its existing
`apply_lifecycle_transition`, assigns the staged registry only after success,
and returns the exact kernel transition. It adds no validation, error mapping,
revision bump, persistence, retry, observation, callback, or registry handle.
On any error, the complete aggregate remains unchanged even when the kernel
mutated the staged registry before failing. On success, only the registry
changes exactly as the kernel defines; the Service State envelope revision
remains repository-owned.

The CLI splits its current helper into intent preparation and application.
`prepare_lifecycle_intent` converts the CLI intent into the Lease Authority
intent and retains current-boot observation plus canonical route policy. The
ordinary repository closure performs that conversion inside each mutation
invocation, then calls the aggregate atomic method. Direct reconciliation
helpers retain their existing registry-oriented application path because some
of them intentionally preserve ordered partial mutations.

Transition variant checks remain after the repository commit. An unexpected
variant therefore retains the current behavior: the transition is already
committed before the caller returns its outcome-mismatch error. The packet must
not move those checks inside the aggregate merely to manufacture broader
rollback.

Excluded surfaces require separate freezes:

- terminal replacement with profile synchronization validates a UTF-8 path
  before mutation, validates profile identity inside mutation, updates the
  profile path, and assigns the transitioned registry as one cross-field join;
- principal rotation deliberately retains removal and revision advancement
  before replacement may fail or return false;
- reconciliation and health helpers can retain an earlier lifecycle transition
  when a later transition fails;
- legacy revocation converts transition failure to absence;
- runtime reset follows validated records and filesystem cleanup; and
- principal binding can succeed before a later session join fails.

Model witnesses must compare successful registration, transfer, and close with
the raw kernel result; prove complete rollback for ambiguity, historical-row
removal, profile mismatch, and transfer rejection; preserve explicit boot
inputs, pending transfer, principal bindings, lifecycle evidence, unrelated
aggregate fields, envelope revision, and maximum registry revision. Focused
CLI witnesses use the in-memory repository whose mutation closure retains
partial state, proving rollback belongs to the aggregate rather than repository
infrastructure.

The architecture guard must require the exact aggregate method, reject a
callback, mutable-registry argument or result, registry return, iterator,
persistence-snapshot bypass, and direct clone or assignment in the ordinary
`transition` body. It must not claim the entire lifecycle file is closed or
reject the deferred direct helpers.

Hard stops are changed error strings, kernel mutation order, current-boot or
route-policy observation placement, profile validation precedence, reset
effect order, outcome-mismatch commit behavior, repository rollback
assumptions, or partial-mutation retention.

Delegation and model-choice receipt: `/root/p205_receipt_kernel_design` used
the requested high-capability `gpt-6-astra` high route to classify atomic,
cross-field, partial, sequential, and effect-separated transitions and freeze
the aggregate API. `/root/p205_receipt_cli_audit` used the requested workhorse
`gpt-5.6-sol` high route for the item-aware 177-production and 288-fixture
inventory, the exact two-access first cut, ordering, and focused witnesses.
Both were read-only and had no edit, build, test, Git, forge, CI, runtime, or
child-agent custody.

Acceptance state and progress classification: this is an interface freeze and
does not itself advance a P4 implementation criterion. Exit requires aggregate
rollback and success parity, the ordinary CLI transition cutover, exact focused
witnesses, and a targeted guard without obscuring the 175 deferred production
accesses.

## Checkpoint 57 | Atomic Runtime Lifecycle Transition Accepted

State transition: Service Model now owns the ordinary all-or-nothing runtime
lifecycle transition. The aggregate clones its hidden registry, delegates the
exact typed intent to the Lease Authority kernel, and commits the staged
registry only after success. The ordinary CLI transition prepares its intent
inside each repository mutation invocation and delegates to that aggregate
method. This removes the two direct registry expressions frozen at Checkpoint
56 and leaves 175 production expressions across 27 files plus 288 test-only
expressions as explicit migration debt.

The accepted method adds no validation, error mapping, observation, retry,
revision update, persistence work, callback, registry handle, or outcome
filter. Success returns the exact kernel transition and changes only the
registry. Every kernel error preserves the complete Service State aggregate,
including unrelated profiles, tabs, unknown fields, principal bindings,
pending transfer state, lifecycle evidence, and the envelope revision. CLI
boot observation and canonical route-policy conversion remain in the adapter.
Caller outcome mismatch checks remain after repository mutation and therefore
retain their existing post-commit behavior.

Acceptance evidence:

- all 208 Service Model unit tests and fourteen integration tests pass,
  including five new model witnesses for registration, transfer, close,
  explicit boot inputs, revision saturation, raw-kernel success parity, and
  rollback after ambiguity, historical-row removal, profile mismatch, and
  transfer rejection;
- all twenty focused `runtime_lifecycle` CLI tests pass, including a new
  non-rollback in-memory repository witness that first proves the raw kernel
  mutates owner state and revision before its ambiguity error, then proves the
  aggregate-backed CLI transition preserves the complete pre-call state;
- formatting, strict workspace Clippy with `-D warnings`, diff hygiene, the
  Service Model architecture contract, its mutation fixtures, and changed-
  surface selector readback pass; and
- no GitHub CI, runtime, browser, profile, provider, credential, install,
  staging, production, or release effect occurred.

The independent closed-world review accepted the implementation semantics but
found two blocking proof gaps. First, the guard checked clone, apply, and commit
tokens across the whole aggregate source instead of the named method. Second,
the existing CLI failure witness did not prove its selected kernel error
mutated before failing. The single allowed review repair now brace-extracts and
exact-normalizes the method signature and body, adds negative fixtures for
receiver, callback, persistence, registry, iterator, error-mapping, missing-
commit, and misplaced-body escapes, and adds the mutation-before-error CLI
witness. Closed-world verification passed both accepted findings.

Delegation and model-choice receipt: `/root/p205_receipt_kernel_design` used
the high-capability `gpt-6-astra` high route for the aggregate implementation
and five model witnesses. `/root/p205_receipt_cli_audit` used the workhorse
`gpt-5.6-sol` high route for the CLI cutover and exact production inventory.
The primary hardened the architecture contract, ran all local acceptance
gates, and reconciled one bounded independent review and repair cycle.

Acceptance state and progress classification: atomic runtime lifecycle
transition closure is accepted and is outcome progress toward P4 field
privacy. The next bounded outcome is the two-access terminal-replacement
profile synchronization join. It must preserve CLI path and canonical-route
preflight before intent preparation, aggregate profile validation before the
kernel, cross-field rollback, and post-commit caller outcome checks.

## Checkpoint 58 | Terminal Replacement Profile Sync Interface Freeze

State transition: the next cross-field runtime-owner packet is frozen to
`RuntimeLifecycleAuthority::transition_terminal_replacement_with_profile_sync`.
It contains two production registry expressions and one direct profile-path
mutation in a single repository closure. No other one of the 175 remaining
production registry expressions belongs to this packet. Acceptance will leave
173 production expressions across the same classified runtime-owner surface.

The aggregate interface is:

```rust
pub fn apply_runtime_lifecycle_transition_with_profile_sync_atomically(
    &mut self,
    profile_id: &str,
    user_data_dir: String,
    intent: agent_browser_lease_authority::RuntimeLifecycleIntent,
) -> Result<
    agent_browser_lease_authority::RuntimeLifecycleTransition,
    String,
>;
```

The CLI retains UTF-8 path conversion before repository mutation. Inside each
mutation invocation it retains immutable profile lookup plus embedded-ID,
canonical route-profile, and nonblank-ID validation before preparing the
kernel intent. This preserves the current error precedence:

1. `runtime_lifecycle_profile_path_invalid`;
2. `runtime_lifecycle_profile_record_missing`;
3. `runtime_lifecycle_profile_record_sync_rejected`; and
4. the exact Lease Authority kernel error.

Only after those adapter preflights may the CLI prepare the kernel intent,
including current-boot observation and canonical route policy, and call the
aggregate. The aggregate independently requires profile existence and rejects
a mismatched or blank embedded profile ID, then clones the registry and applies
the exact kernel intent. On success it updates only the selected profile's
`user_data_dir`, assigns the staged registry, and returns the exact transition.
On any error it preserves the complete aggregate. It does not interpret paths
or route names, observe the host, restrict intent or result variants, update
the envelope revision, map errors, retry, persist, or expose a registry.

The two callers retain their transition-variant checks after repository
mutation. A profile-migration or same-profile replacement outcome mismatch is
therefore still post-commit and must not become aggregate rollback. The packet
must not call the ordinary atomic transition before its fallible profile join.

Model witnesses must prove same-profile activation and cross-profile migration
match the raw kernel plus exactly one path update; missing, mismatched, and
blank profiles precede malformed kernel errors; kernel rejection and a
mutation-before-error kernel path roll back both fields; boot `Some` and
`None`, principal bindings, unrelated profiles, unknown fields, maximum
registry revision, and envelope revision remain exact; and a successful
nonreplacement intent returns its real variant and commits, preserving the
caller's post-commit mismatch contract.

Focused CLI witnesses retain terminal activation, canonical profile migration,
noncanonical or incomplete evidence rejection, collision rejection, and path
plus lifecycle assertions. New precedence witnesses must prove invalid paths
fail before repository mutation where the platform supports that input,
missing, mismatched, and noncanonical profiles fail before intent preparation,
a valid profile plus rejected lifecycle intent retains its old path, and the
two existing caller outcome checks remain outside the repository mutation.

The architecture guard must require the exact aggregate signature and ordered
profile validation, staged registry, exact kernel call, profile update,
registry assignment, and transition return. It must reject callbacks, registry
or persistence handles, mutable references, iterators, error mapping, model
observation or path interpretation, result-variant filtering, direct registry
clone or assignment in the CLI helper, and direct profile-path assignment
there. It must require CLI preflight before intent preparation while allowing
the immutable preflight reads.

Hard stops are changed path, missing-profile, or sync-rejection precedence;
intent preparation before CLI preflight; moved boot or route observation;
changed kernel order or error; registry-only commit; result-variant rollback;
envelope revision ownership; or inclusion of reconciliation, principal
rotation, legacy revocation, reset, filesystem, process, or runtime effects.

Delegation and model-choice receipt: `/root/p205_receipt_kernel_design` used
the high-capability `gpt-6-astra` high route to derive the cross-field atomic
contract, current ordering, interface, witnesses, and hard stops.
`/root/p205_receipt_cli_audit` used the workhorse `gpt-5.6-sol` high route to
refresh the item-aware inventory at 175 production and 288 test-only
expressions and identify this two-access helper as the smallest coherent next
packet. Both audits were read-only and performed no edit, build, test, Git,
forge, CI, runtime, or child-agent action.

Acceptance state and progress classification: this is an interface freeze and
does not itself advance a P4 implementation criterion. Exit requires exact
cross-field success parity and rollback, the one-helper CLI cutover, focused
precedence witnesses, and a targeted architecture guard. The next independent
candidate after acceptance is the two-access process-exit legacy revocation
packet, which intentionally retains revocation when a later session lookup
fails and therefore must not reuse this atomic join blindly.

## Checkpoint 59 | Terminal Replacement Profile Sync Accepted

State transition: Service Model now owns the terminal-replacement lifecycle
and profile-path cross-field commit. The CLI retains UTF-8 path conversion,
immutable profile lookup, canonical route policy, nonblank profile validation,
current-boot observation, and route-policy intent preparation in their prior
order, then delegates the prepared typed intent and owned path to the aggregate.
The packet removes the two frozen direct registry expressions and the direct
CLI profile-path assignment. The classified production registry debt is now
173 expressions; seven remain in `runtime_lifecycle.rs` outside this helper.

The aggregate repeats only provider-free profile existence and identity
validation, stages the Lease Authority transition, updates the selected path
only after kernel success, assigns the staged registry, and returns the exact
transition. Missing profile and profile-identity failures preserve their exact
strings and precede kernel validation. Kernel failures preserve both fields and
the complete aggregate even when the staged kernel mutated before returning an
error. Successful nonreplacement intents deliberately commit and return their
actual transition variant, preserving the two callers' existing post-commit
outcome-mismatch behavior.

Acceptance evidence:

- all 212 Service Model unit tests and fourteen integration tests pass,
  including four new profile-sync tests covering the activation and migration,
  boot `Some` and `None`, ordinary and maximum revision parity matrix; profile
  presence and identity error precedence; ambiguity and historical-row-removal
  rollback; unrelated owner, binding, profile, tab, unknown-field and envelope-
  revision preservation; and successful nonreplacement behavior;
- all twenty-one focused `runtime_lifecycle` CLI tests pass, including route-
  policy and invalid-path preflight plus complete profile and registry rollback
  through the non-rollback in-memory repository;
- formatting, strict workspace Clippy with `-D warnings`, diff hygiene, the
  Service Model architecture contract, expanded mutation fixtures, and
  changed-surface selector readback pass; and
- no GitHub CI, runtime, browser, profile, provider, credential, install,
  staging, production, or release effect occurred.

The first focused CLI run failed because the new rollback witness used a
synthetic profile ID that the unchanged canonical route predicate correctly
rejected before reaching the intended kernel ambiguity. The fixture was
corrected to the existing canonical `rdp-guac-route-a-viewer` form. The exact
affected witness and then the complete focused lifecycle group passed. This was
a fixture attribution, not a product repair or retry of the failed semantics.

The architecture contract now exact-matches the aggregate signature and
ordered validation, stage, kernel, path, registry, and return body. Its CLI
guard brace-extracts the one helper, requires path conversion before repository
mutation, profile lookup then route and nonblank preflight before intent
preparation, and aggregate delegation last. Negative fixtures reject callback,
error mapping, result filtering, direct registry or path mutation, omitted
nonblank validation, early intent preparation, and path conversion moved into
the mutation. The primary inspected and reconciled the complete candidate;
the cumulative independent review and rework allowance was already consumed
at Checkpoint 57, so no second broad review or rework loop was opened.

Delegation and model-choice receipt: `/root/p205_receipt_kernel_design` used
the high-capability `gpt-6-astra` high route for the atomic cross-field method
and four provider-free tests. `/root/p205_receipt_cli_audit` used the workhorse
`gpt-5.6-sol` high route for the exact helper cutover and focused precedence
and rollback witnesses. The primary implemented the architecture contract,
diagnosed the one fixture failure, and ran all local acceptance gates.

Acceptance state and progress classification: terminal replacement profile
synchronization is accepted and is outcome progress toward P4 field privacy.
The next bounded outcome is process-exit legacy revocation. That packet must
preserve health recording before revocation and the intentional retained
revocation when the later session lookup fails; it is not an all-or-nothing
transition.

## Checkpoint 60 | Process-Exit Legacy Revocation Interface Freeze

State transition: the next partial-mutation packet is frozen to the two direct
registry expressions in
`persist_process_exited_browser_health_in_repository`. Acceptance removes the
session-binding lookup and mutable legacy-revocation call from
`control_plane.rs`, leaving 171 classified production expressions. This packet
must preserve direct retained-registry mutation. Reusing either atomic helper
would be a correctness defect because the Lease Authority kernel may revoke an
owner before a later lifecycle join fails.

The aggregate interface is purpose-specific and nontransactional:

```rust
pub fn revoke_process_exited_session_owner(
    &mut self,
    session_id: &str,
) -> Option<agent_browser_lease_authority::ProfileOwner>;
```

The method resolves `binding_for_session(session_id)`, converts binding errors
or absence to `None`, rejects observation-only bindings, and applies the exact
`RevokeLegacyOwner` intent directly to the retained registry with all five
claim identities. Kernel failure becomes `None` without rollback. Only the
exact `LegacyOwnerRevoked` transition returns its owner; an unexpected variant
also returns `None` after retaining any kernel mutation. The method adds no
session lookup, authentication, health policy, cleanup, observation,
persistence, callback, generic intent, registry handle, principal mutation, or
envelope revision update.

The CLI retains its exact order:

1. observe timestamp and boot epoch before repository mutation;
2. read prior browser and live daemon PID/CDP state;
3. construct and annotate process-exited browser health;
4. append the health-change event;
5. authenticate retained session work at the observed time;
6. call the aggregate revocation method;
7. clone the registered session after revocation;
8. orphan any display allocation and remove browser, identity, observation,
   tab, and session edges regardless of revocation result; and
9. only when both owner and session exist, reinsert the cleared registered
   session, refresh derived views, and return the owner.

Authentication failure, missing or ambiguous binding, observation-only
binding, and every revocation-domain failure remain absence, not caller-visible
errors. Repository construction, load, save, or mutation failures remain the
only errors from the inner function, and its outer convenience wrapper still
suppresses those to `None`. A successful revocation followed by missing session
capture retains the revocation and continues cleanup. Current authentication
makes that branch effectively unreachable in this synchronous path, but its
ordering and defensive semantics remain frozen.

Model witnesses must prove exact success parity; missing, ambiguous, and
observation-only bindings without mutation; pending transfer, invalid retained
evidence, and exhausted generation failure behavior; lifecycle ambiguity after
revocation retains the orphaned owner, incremented generation, and revision;
historical-row removal followed by profile mismatch remains retained;
principal bindings, unrelated aggregate fields, maximum registry revision, and
envelope revision remain exact; repeated invocation is inert; and revocation
does not depend on a Service State session record.

The three existing process-exited health witnesses remain required. A new CLI
witness must introduce ambiguous matching lifecycle rows into the registered-
session fixture, prove the owner mutation occurred before the swallowed
lifecycle failure, and prove the health event plus browser, tab, and session
cleanup still commit without restoring the session. Repository-error behavior
must remain covered at its existing adapter seam.

The architecture guard must require the exact aggregate signature and direct
binding, effect-capable gate, direct retained-registry kernel call, exact result
selection, and absence mapping. It must reject staging, cloning, atomic-helper
reuse, callbacks, generic intents, registry or persistence handles, session or
authentication preconditions, error exposure, and principal cleanup. In the
CLI function it must require health event before authentication, aggregate
revocation before session lookup, and operational cleanup afterward, while
rejecting both direct registry expressions even after test-gated source items.

Hard stops are rollback after kernel failure or session absence, stronger or
different binding selection, principal-binding cleanup, health-event movement,
changed absence/error mapping, session lookup before revocation, envelope
revision mutation, or movement of clock, boot, PID/CDP, event, repository,
display, browser, tab, session, or derived-view custody into the model.

Delegation and model-choice receipt: `/root/p205_receipt_kernel_design` used
the high-capability `gpt-6-astra` high route to derive the intentional partial-
mutation contract, minimal interface, witnesses, and hard stops.
`/root/p205_receipt_cli_audit` used the workhorse `gpt-5.6-sol` high route to
audit the two exact accesses, ordering, swallowed error families, existing and
missing witnesses, CLI-owned effects, and post-packet count. Both were
read-only and performed no edit, build, test, Git, forge, CI, runtime, or child-
agent action.

Acceptance state and progress classification: this is an interface freeze and
does not itself advance a P4 implementation criterion. Exit requires exact
retained partial-mutation behavior, the one-function CLI cutover, the malformed-
lifecycle cleanup witness, and a targeted architecture guard.

## Checkpoint 61 | Process-Exit Legacy Revocation Accepted

State transition: Service Model now owns the purpose-specific process-exit
session-owner revocation. The method resolves the exact effect-capable session
binding and applies `RevokeLegacyOwner` directly to the retained registry.
Binding and kernel errors become absence without rollback, and only the exact
legacy-owner transition returns an owner. The control-plane adapter no longer
reads or mutates the registry directly. The two frozen production expressions
are removed, leaving 171 classified production expressions.

Health event construction and recording, authentication, later session lookup,
display orphaning, operational browser/tab/session cleanup, optional registered-
session restoration, observations, repository custody, and derived-view
refresh remain CLI responsibilities in their prior order. A lifecycle error
after owner revocation still retains the orphaned owner, advanced generation,
and registry revision while returning absence; cleanup still commits. A later
missing-session join likewise cannot undo revocation. The now-unused CLI
`revoke_legacy_owner_in_registry` wrapper had no remaining caller and was
deleted rather than retained as a second transition path.

Acceptance evidence:

- all 216 Service Model unit tests and fourteen integration tests pass,
  including four new revocation tests covering raw-kernel success parity,
  session-record independence, inert repeat, missing and ambiguous bindings,
  observation-only authority, invalid evidence, pending transfer, generation
  exhaustion, ordinary and maximum registry revision, retained ambiguity and
  historical-row-removal failures, principal bindings, unrelated aggregate
  state, and envelope revision;
- all sixteen focused `process_exit` CLI tests pass, including the three prior
  process-exited health witnesses and the new malformed-lifecycle case proving
  retained owner revocation plus committed health, browser, tab, and session
  cleanup;
- formatting, strict workspace Clippy with `-D warnings`, diff hygiene, the
  Service Model architecture contract, expanded mutation fixtures, and
  changed-surface selector readback pass; and
- no GitHub CI, runtime, browser, profile, provider, credential, install,
  staging, production, or release effect occurred.

The architecture contract exact-matches the nontransactional aggregate method,
including direct retained-registry mutation and absence mapping. It brace-
extracts the process-exit adapter before string stripping so unrelated Rust
lifetime syntax cannot hide the target function, requires health event,
authentication, aggregate revocation, later session lookup, and cleanup order,
rejects direct registry access, and rejects reintroduction of the superseded
CLI wrapper. Mutation fixtures cover staging, generic intent expansion, direct
CLI registry access, early session lookup, late health event, and wrapper
reintroduction. The primary inspected and reconciled the complete candidate;
no second broad review or rework loop was opened after the cumulative allowance
used at Checkpoint 57.

Delegation and model-choice receipt: `/root/p205_receipt_kernel_design` used
the high-capability `gpt-6-astra` high route for the nontransactional aggregate
method and four provider-free tests. `/root/p205_receipt_cli_audit` used the
workhorse `gpt-5.6-sol` high route for the two-access adapter cutover, shared
registered-session fixture, and malformed-lifecycle witness. The primary
removed the superseded wrapper, implemented the architecture contract, and ran
all local acceptance gates.

Acceptance state and progress classification: process-exit legacy revocation
closure is accepted and is outcome progress toward P4 field privacy. The next
bounded outcome must come from the remaining classified surface without
combining atomic, retained-partial, sequential, and effect-separated semantics.

## Checkpoint 62 | Existing Immutable Authority Projection Cutover Freeze

State transition: the next packet is frozen to four production direct registry
reads across two immutable adapter decisions. No new Service Model API is
required. `RuntimeResourceReconciler::classify` will replace owner and lifecycle
map reads with one `runtime_lane_authority` projection.
`authorize_lease_effect_in_repository` will replace owner and principal-binding
map reads with one `profile_runtime_authority` projection. Acceptance leaves
167 classified production expressions.

Runtime reconciliation must destructure the projected owner before lifecycle
so missing owner retains `runtime_lifecycle_owner_unproven` precedence over
`runtime_lifecycle_record_unproven`. Every earlier browser, process identity,
profile path, and canonical digest check and every later process-group, launch
digest, owner readiness, browser, process, generation, lifecycle, and cleanup
check remains unchanged. The projection returns the exact two independent map
selections and deliberately does not validate or zip them.

Lease-effect authorization retains schema validation, repository load, Lease
Authority authorization, profile lookup, profile resolution, and canonical
digest computation before projection. Its match semantics remain exact:

- absent claim generation plus absent owner passes this owner check even if an
  orphan binding exists;
- present generation plus present owner requires matching generation, ready
  owner, and binding generation, profile, principal, and capability identity;
  and
- every other presence combination returns
  `lease_authority_owner_generation_stale`.

The packet must not add provenance, pending-transfer, or current-binding
predicates and must not move signing, trust-key, repository, profile resolution,
or error presentation into the model. The projection's additional immutable
revision read is ignored and has no behavior or effect.

Existing provider tests already prove independent owner, binding, and lifecycle
options plus unvalidated mismatch preservation. Focused adapter witnesses retain
the exact closing package-browser ownership case and diverged owner/principal-
binding rejection. New narrow cases must prove reconciliation missing-owner
precedence when both are absent, lifecycle-missing after an owner is present,
and the lease owner-generation presence matrix, especially absent owner and
generation with an otherwise present binding.

The architecture guard must reject production direct registry access in both
files after item-aware test exclusion while allowing fixture-only access. It
must require the purpose-specific projection calls and reject a raw registry
parameter, getter, persistence snapshot, generic iterator, zipped missing
states, or additional authority predicate.

The two workstation maintenance reads remain outside this packet. The primary
adjudicated the audit disagreement as follows: `runtime_resource_lanes` does
enumerate every lifecycle map key, so its `browser_id` map keys can derive both
tracked count and missing-key membership without a new model API. However that
cutover needs a focused missing-count join witness and must use the map key, not
the embedded lifecycle browser ID. It will be frozen separately rather than
piggybacked onto this adapter packet.

Hard stops are changed error precedence, stronger authority checks, owner or
lifecycle option collapse, embedded-ID substitution, new model API, observation
or effect movement, or expansion into other reconciliation, lease mutation, or
workstation owner-selection paths.

Delegation and model-choice receipt: `/root/p205_receipt_kernel_design` used
the high-capability `gpt-6-astra` high route to verify exact projection parity,
semantic traps, and the zero-new-API boundary. `/root/p205_receipt_cli_audit`
used the workhorse `gpt-5.6-sol` high route for the exact four-read inventory,
ordering, witness audit, and safe split from workstation maintenance. Both were
read-only and performed no edit, build, test, Git, forge, CI, runtime, or child-
agent action.

Acceptance state and progress classification: this is an interface freeze and
does not itself advance a P4 implementation criterion. Exit requires both
adapter cutovers, exact missing-state and generation-presence witnesses, and
zero production direct registry access in the two target files.

## Checkpoint 63 | Existing Immutable Authority Projection Cutover Accepted

State transition: runtime reconciliation and lease-effect authorization now
consume the existing purpose-specific immutable Service Model projections.
`RuntimeResourceReconciler::classify` obtains owner and lifecycle independently
through `runtime_lane_authority`, while preserving missing-owner precedence over
missing-lifecycle. Lease-effect authorization obtains owner and principal
binding through `profile_runtime_authority`, while preserving the exact owner
generation presence matrix and all existing binding predicates. The four
frozen production registry expressions are removed, leaving 167 classified
production expressions.

The architecture contract now requires both projection calls and rejects
production direct registry access in both adapters after item-aware test
exclusion. Its mutation fixtures prove that a test module cannot hide later
production access. The selected Lease Authority architecture gate initially
reported a pre-existing false positive for the Service Model-owned
`RuntimeOwnerPersistenceParts.lifecycle_records` field. The guard now excludes
only the exact `persistence.lifecycle_records` projection in `service_store.rs`;
its self-test accepts that typed projection while continuing to reject direct
registry map access.

Acceptance evidence:

- all five focused runtime-reconciliation tests pass, including both new
  missing-state precedence witnesses;
- all four focused lease-authority adapter tests pass, including the new
  absent-generation, absent-owner, orphan-binding witness;
- all 116 Lease Authority crate tests pass;
- formatting, strict workspace Clippy with `-D warnings`, diff hygiene, the
  Service Model architecture contract and mutation fixtures, and the Lease
  Authority architecture contract and self-test pass;
- the changed-surface selector reports its conservative broad local route; and
- no GitHub CI, runtime, browser, profile, provider, credential, install,
  staging, production, or release effect occurred.

Delegation and model-choice receipt: `/root/p205_reconciliation_cutover` used
the requested fast `gpt-5.6-luna` medium route for the reconciliation cutover
and its two focused witnesses. `/root/p205_receipt_cli_audit` used the requested
workhorse `gpt-5.6-sol` high route for the lease-effect cutover and orphan-
binding witness. Both workers stayed inside their exact file scopes and ran no
build, Git, forge, CI, runtime, or child-agent action. The primary reconciled
both diffs, repaired the independently surfaced Lease Authority guard false
positive, and ran every local acceptance gate.

Acceptance state and progress classification: immutable authority projection
cutover is accepted and is outcome progress toward P4 field privacy. The next
bounded outcome is the separately identified workstation maintenance two-read
projection cutover. It must derive tracked and missing lifecycle counts from
the `runtime_resource_lanes` map-key projection, not from the embedded lifecycle
browser ID, and must add the focused join witness before removing those reads.

## Checkpoint 64 | Workstation Cleanup-Obligation Projection Freeze

State transition: the next packet is frozen to the two production lifecycle-
map reads inside `reconcile_runtime_maintenance`. No new Service Model API is
required. A private workstation helper will consume the existing
`runtime_resource_lanes` projection once, report its row count as
`trackedCount`, and derive `missingCount` from process-backed Service browser
keys absent from the projected lifecycle map-key set. Acceptance leaves 165
classified production runtime-owner expressions.

The projection's `RuntimeResourceLane.browser_id` is the lifecycle map key and
is authoritative for this join. The helper must not substitute
`lane.lifecycle.logical_browser_id`, because retained compatibility state can
preserve a different embedded identifier. It must count lifecycle-only rows as
tracked, ignore browsers without durable process identity when computing
missing obligations, count a process-backed browser with no lifecycle map key,
and retain an ordinary process-backed tracked browser as non-missing. A single
focused pure witness will cover all five cases, including a map key that differs
from the embedded lifecycle browser ID.

Process lifecycle reconciliation, Service State repository retry and mutation
custody, process garbage collection, retained-state pruning, resource response,
JSON names and values, install generation collection, backoff, receipts,
filesystem operations, units, and runtime effects remain unchanged. The
architecture guard must require the purpose-specific projection and reject
production direct registry access in `workstation_install.rs` after item-aware
test exclusion. It must not reject fixture-only registry setup.

Hard stops are a new model API, embedded-ID membership, changed count semantics,
movement of process observation or install effects into Service Model, reuse of
a persistence snapshot or whole-registry handle, or expansion into the other
workstation owner-selection and mutation paths.

Delegation and model-choice receipt: the mechanical single-file implementation
is assigned to `/root/p205_reconciliation_cutover` on its existing requested
fast `gpt-5.6-luna` medium route. The worker may edit only
`cli/src/workstation_install.rs`, add the one pure helper and focused witness,
and run targeted formatting without builds, Git, forge, CI, runtime, or child-
agent actions. The primary owns the architecture guard, integration, local
acceptance, checkpoint disposition, commit, and publication.

Acceptance state and progress classification: this is an interface freeze and
does not itself advance a P4 implementation criterion. Exit requires zero
production direct runtime-owner registry access in the frozen maintenance
function, exact map-key count behavior, the focused witness, and the targeted
architecture contract.

## Evidence And Exit

| Requirement | Evidence | Current state |
| --- | --- | --- |
| One provider-free model crate | workspace manifest, crate manifest, architecture guard | canonical aggregate and Service Principal registry boundary accepted |
| Stable compatibility | frozen current fixtures and byte or value-equivalent canonical outputs | ordinary persisted codec and current aggregate wire accepted; staged known-key correction remains outside this packet |
| One canonical aggregate | no duplicate `ServiceState` or durable record owners | accepted at Checkpoint 40; field-privacy ledger remains open |
| Deep module interface | pure policy decisions and record contracts through one crate seam | aggregate methods, helper closure, configured input, revision and migration projections, Service challenge and authentication map kernels, crash-regeneration transaction kernel, Lease Authority mutation closure, exact receipt-map closure, Service Principal registry closure, Principal Continuity kernel, runtime-owner persistence boundary, immutable runtime-owner projections and two adapter cutovers, ordinary atomic runtime lifecycle transition, terminal profile-sync cross-field transition, and process-exit partial legacy revocation accepted; remaining transition closure remains open |
| CLI adapters remain adapters | repository, process, browser, transport, and provider imports absent from crate | twenty-one model families, capacity mutation closure, and authentication-control boundary accepted |
| Focused correctness | crate tests and affected CLI adapter tests | immutable authority adapter cutover accepted through Checkpoint 63 |
| Build acceleration | comparable baseline and candidate focused-loop receipts | 171.01-second cold CLI baseline and 4.08-second crate loop recorded; broader claim pending |
| Shared validation wiring | P204 commits contained in `origin/main`; merged into P205 at `d10e7c17` | changed-surface selector and focused lane used locally |
| Runtime effect | no runtime, browser, profile, provider, install, staging, production, or release effect | required none |

Every material checkpoint records the transition, acceptance state, progress
classification, evidence, blockers, worker receipts, and next action. Local
focused checks are required; GitHub CI dispatch, monitoring, and acceptance are
out of scope by explicit user direction.

## Non-Goals

- No change to user-facing service behavior, wire names, or authority semantics.
- No replacement of typed durable records with `serde_json::Value`.
- No extraction of filesystem persistence, transaction locks, runtime-owner
  observation, browser launch, HTTP, MCP, dashboard, or provider effects.
- No new public command, flag, environment variable, or runtime workflow.
- No installed-runtime, browser, profile, provider, staging, production, or
  release mutation.
- No GitHub CI dispatch, retry, or active waiting.
- No independent rewrite of P197 or P204 shared planning sections.

## Stop Condition

Stop the affected packet before any runtime effect, provider or credential use,
production or staging mutation, release, edits to P204's active shared sections,
unreconciled duplicate model, weakened compatibility contract, or a fourth
attempt at the same failed approach. Preserve a clean published checkpoint and
the exact unresolved dependency rather than hiding it behind a shallow facade.
