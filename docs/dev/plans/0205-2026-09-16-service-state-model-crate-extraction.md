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

Current `origin/main` is the source baseline. The 11,000-line
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

P202 is integrated and closed. P197 is clean and published on its challenge
branch. P204 owns the current CI workflow, validation selector,
`scripts/ci/rust-tests.sh`, `package.json`, `AGENTS.md`, `ROADMAP.md`,
`RUNBOOK.md`, and `docs/dev/active-lanes.yaml` transition. P205 will not edit
those shared surfaces until P204 publishes an integration checkpoint. This
plan and issue #178 are the durable P205 authorities meanwhile.

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
5. Add the dedicated crate compartment and changed-surface selection after
   P204 releases those files, then measure the focused edit and test loop.

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
- After P204 integration, add the dedicated crate compartment and validation
  selector route without weakening the comprehensive lane.
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

## Evidence And Exit

| Requirement | Evidence | Current state |
| --- | --- | --- |
| One provider-free model crate | workspace manifest, crate manifest, architecture guard | eleven families accepted; aggregate pending |
| Stable compatibility | frozen current fixtures and byte or value-equivalent canonical outputs | pending |
| One canonical aggregate | no duplicate `ServiceState` or durable record owners | pending |
| Deep module interface | pure policy decisions and record contracts through one crate seam | first deep kernel accepted; aggregate interface pending |
| CLI adapters remain adapters | repository, process, browser, transport, and provider imports absent from crate | eleven packets accepted |
| Focused correctness | crate tests and affected CLI adapter tests | eleven packets accepted |
| Build acceleration | comparable baseline and candidate focused-loop receipts | 171.01-second cold CLI baseline and 4.08-second crate loop recorded; broader claim pending |
| Shared validation wiring | P204 integrated before P205 edits its owned files | dependency pending |
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
production or staging mutation, release, shared-file edit still owned by P204,
unreconciled duplicate model, weakened compatibility contract, or a fourth
attempt at the same failed approach. Preserve a clean published checkpoint and
the exact unresolved dependency rather than hiding it behind a shallow facade.
