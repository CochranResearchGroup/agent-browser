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

## Evidence And Exit

| Requirement | Evidence | Current state |
| --- | --- | --- |
| One provider-free model crate | workspace manifest, crate manifest, architecture guard | nineteen families accepted; aggregate pending |
| Stable compatibility | frozen current fixtures and byte or value-equivalent canonical outputs | pending |
| One canonical aggregate | no duplicate `ServiceState` or durable record owners | pending |
| Deep module interface | pure policy decisions and record contracts through one crate seam | presentation-capacity kernel accepted; aggregate interface pending |
| CLI adapters remain adapters | repository, process, browser, transport, and provider imports absent from crate | nineteen families plus capacity mutation closure accepted |
| Focused correctness | crate tests and affected CLI adapter tests | capacity closure accepted through Checkpoint 21 |
| Build acceleration | comparable baseline and candidate focused-loop receipts | 171.01-second cold CLI baseline and 4.08-second crate loop recorded; broader claim pending |
| Shared validation wiring | P204 commits contained in `origin/main`; merged into P205 at `d10e7c17` | available; local use pending |
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
