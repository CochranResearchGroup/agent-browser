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

## Evidence And Exit

| Requirement | Evidence | Current state |
| --- | --- | --- |
| One provider-free model crate | workspace manifest, crate manifest, architecture guard | first family accepted; aggregate pending |
| Stable compatibility | frozen current fixtures and byte or value-equivalent canonical outputs | pending |
| One canonical aggregate | no duplicate `ServiceState` or durable record owners | pending |
| Deep module interface | snapshot, transition, and projection interface tests plus deletion test | planned |
| CLI adapters remain adapters | repository, process, browser, transport, and provider imports absent from crate | first packet accepted |
| Focused correctness | crate tests and affected CLI adapter tests | first packet accepted |
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
