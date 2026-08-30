# Plan 0142 | Service State Concurrency And Client Recourse Reliability

Date: 2026-08-29

State: OPEN

Execution state: `slice_f_revision_cas_checkpoint_complete_stress_and_lifecycle_matrix_next`

Lane: P142

Source baseline: `3ee177181fe0b9946c85d657588e8ed4dac7c767`

Branch: `main`

Target: `main`

Integration model: direct to `main` through cohesive, validated checkpoints.
No pull request is required for this one-maintainer repository. Any installed
runtime or production effect remains behind a separate exact gate.

Authority: PLAN, SOURCE, DOCUMENTATION, PROVIDER-FREE FIXTURES, AND ISOLATED
DEVELOPMENT-RUNTIME VALIDATION ARE IN SCOPE. THIS PLAN DOES NOT AUTHORIZE
PRODUCTION SERVICE RESTART, PROCESS TERMINATION, STATE REPAIR, PROFILE OR
PROVIDER USE, DUPLICATE BROWSER LAUNCH, ROUTE CREATION, OR CANDIDATE
INSTALLATION.

Depends on:

- Plan 0130 access-plan owner reuse coherence;
- Plan 0132 terminal-owner supersession route coherence;
- closed Plan 0134 crash epoch and profile lifecycle coherence;
- open Plan 0137 profile acquisition, recovery, and lifecycle reliability;
- `docs/dev/notes/0140-2026-08-29-plan-0137-custody-and-slice-j-handoff.md`;
  and
- `docs/dev/notes/0141-2026-08-29-plan-0137-slice-j-no-effect-candidate-preflight.md`.

## Executive Decision

Agent Browser will treat Service State contention and lifecycle ownership
conflicts as first-class service outcomes, not opaque strings that require a
client to guess whether retry, reuse, recovery, or escalation is safe.

The implementation will make two coordinated changes:

1. shorten and instrument the Service State mutation critical path so normal
   concurrent clients do not lose work to a one-second process-mutex timeout;
   and
2. return a shared structured recourse object that tells every client what is
   known about effects, whether retry is allowed, whether exact reuse was
   proven, whether a sealed recovery plan exists, and which inspection
   locator to use before another mutation.

Increasing the timeout alone is not an acceptable repair. Blind retry is not
an acceptable client policy. A raw lifecycle owner error must never be
translated into reuse unless a current access plan supplies the exact browser
and session route.

This plan is a reliability prerequisite for Plan 0137 Slice J. It does not
replace that plan's acquisition and recovery architecture or widen its
production authority.

## Goal

Make concurrent Agent Browser clients receive timely, effect-safe, and
machine-readable outcomes when Service State or managed lifecycle ownership
prevents an operation.

The system must:

- identify which lock phase timed out and how long the request waited;
- distinguish a request that provably had no effect from one whose effect is
  uncertain because completion persistence failed;
- preserve crash-safe and multi-process Service State consistency;
- remove avoidable read and preparation work from the exclusive mutation
  interval;
- prevent one file-lock waiter from obscuring process-mutex ownership;
- classify lifecycle-owner conflicts through the access and recovery plane;
- offer exact reuse only when current evidence supplies compatible
  `browserId` and `sessionName` hints;
- return a sealed recovery plan when bounded repair is available;
- return an actionable hard blocker when neither reuse nor recovery is safe;
- keep CLI, HTTP, MCP, jobs, traces, generated clients, dashboard, help,
  README, docs site, and skills aligned; and
- prove that clients do not respond to either failure by launching a duplicate
  profile lane.

## Non-Goals

- This plan does not weaken lifecycle admission, profile capability checks,
  principal isolation, or exact-route requirements.
- It does not make a retry safe merely by adding an idempotency key after an
  effect may already have occurred.
- It does not delete, rewrite, unlock, or garbage-collect retained state as a
  contention remedy.
- It does not terminate a process because it held or may have held a lock.
- It does not infer browser reuse from a matching profile name, a raw owner
  error, a ready-looking projection, or a remembered session route.
- It does not fold the recovery plane, job store, lifecycle registry, and
  Service State repository into one new monolith.
- It does not automate provider navigation or mutate Google Ads, Amazon,
  SoyLei, Odollo, or another tenant during source acceptance.
- It does not authorize the Plan 0137 production candidate installation.

## Current Control Record

At planning time:

- `HEAD` and `origin/main` are both
  `6cf3b4e5a7d1d5ec11008c9e9e0b62befa856b0d`;
- the primary worktree is clean;
- foreign worktrees and the untracked Plan 0138 note remain outside this
  plan's custody;
- Plan 0137 remains open and its Slice J no-effect preflight is blocked on
  current state compatibility and presentation prerequisites;
- the observed production Service State was approximately 2.9 MiB and held
  179 sessions, 53 owners, and 52 handoffs;
- seven retained jobs failed with
  `service_state_lock_timeout: process mutation lock` during one eight-minute
  interval, spanning launch, remote-view open, and tab creation;
- later jobs succeeded, which is evidence of intermittent contention rather
  than a permanently poisoned mutex;
- one current Service read took approximately 1.52 seconds end to end, leaving
  little headroom under the one-second process-mutex wait budget;
- the lock implementation does not retain enough owner and phase telemetry to
  identify the exact historical holder action; and
- no runtime or provider state was mutated while preparing this plan.

The runtime observations above are incident evidence, not acceptance proof for
a future build. Every implementation checkpoint must use fresh fixtures or
fresh isolated-runtime receipts.

## Incident Model

### Service State process-mutation lock timeout

`cli/src/native/service_store.rs` currently serializes Service State access
through one process-wide mutex and an exclusive file lock. The default
process-mutex wait budget is one second. Mutation retains both locks while it
loads state, runs the mutation, prepares related registries, and commits
several durable files.

This creates a convoy: one operation may hold the process mutex while waiting
for or working under the file lock, and peer requests can time out without
knowing the holder, phase, expected duration, or effect boundary. Large state
and repeated whole-file persistence increase the critical interval.

The exact failed request can be in one of two materially different states:

- lock acquisition failed before its mutation began, so the request had no
  effect; or
- the action progressed and terminal job or transaction persistence later
  failed, so the effect is uncertain until current state is inspected.

The product does not currently express that distinction reliably enough for a
client to choose a safe next action.

### Lifecycle owner blocks direct launch

`ensure_managed_lane_launch_allowed` correctly rejects a managed launch when
an existing owner is not eligible for an explicit transition. The direct
launch guard runs before Chrome starts, so the blocked launch itself has no
browser effect.

The raw error
`runtime_lifecycle_existing_owner_requires_explicit_transition` does not say
which of these conditions applies:

- a healthy compatible browser is available for exact reuse;
- a terminal or inconsistent owner has one sealed Recovery Plan;
- the current evidence is stale and the access plan must be refreshed; or
- a genuinely live or ambiguous authority is a hard blocker.

In the Ads review incident, reuse was not offered and the client correctly
stopped without retrying or launching a duplicate lane. That is safe client
behavior, but the product should have returned the reason and the next safe
step as data instead of depending on prose or operator judgment.

## Client Outcome Contract

Every service operation failure covered by this plan will carry one shared
recourse object. Exact names remain subject to Slice A schema review, but the
semantic fields are frozen:

```json
{
  "code": "service_state_lock_timeout",
  "axis": "service_state",
  "phase": "process_mutex_wait",
  "effectState": "no_effect",
  "retryDisposition": "inspect_before_retry",
  "recommendedAction": "inspect_job_and_refresh_plan",
  "reuseAllowed": false,
  "recoveryPlan": null,
  "jobId": null,
  "traceId": null,
  "safeNextActions": [],
  "hardStops": []
}
```

Required semantics:

| Field | Contract |
| --- | --- |
| `code` | Stable machine-readable error code. The current raw code remains available for compatibility. |
| `axis` | The authority or subsystem that prevented progress, such as `service_state`, `lifecycle_owner`, `profile_lease`, or `presentation`. |
| `phase` | The last proven execution boundary. Unknown is explicit and never guessed. |
| `effectState` | `no_effect`, `effect_uncertain`, or `verified_effect`. |
| `retryDisposition` | `do_not_retry`, `inspect_before_retry`, `retry_same_request`, or `refresh_access_plan`. |
| `recommendedAction` | One stable action derived from the same classifier as access and recovery planning. |
| `reuseAllowed` | True only when exact compatible route hints are present in the current outcome. |
| `recoveryPlan` | A sealed Plan 0137 recovery object or null. Presence is not apply authority by itself. |
| locators | Safe job, trace, incident, access-plan, or recovery-status locators. No capability or private path is persisted. |
| `safeNextActions` | Bounded read or effect actions the current authority may take. |
| `hardStops` | Explicitly forbidden shortcuts, including blind retry or duplicate launch when applicable. |

The envelope must be additive and versioned. Older clients continue receiving
the string error and failed job state. New clients consume the structured
object without parsing prose.

## Frozen Recourse Matrix

| Condition | Effect state | Retry disposition | Required client action |
| --- | --- | --- | --- |
| process mutex was not acquired | `no_effect` | policy-selected bounded retry or inspect | respect server-provided disposition; do not invent backoff policy |
| file lock was not acquired before mutation | `no_effect` | policy-selected bounded retry or inspect | use the same request identity when retry is explicitly allowed |
| action may have completed but terminal persistence failed | `effect_uncertain` | `inspect_before_retry` | read exact job, trace, and current resource state before another mutation |
| current access plan proves one compatible owner and exact route hints | `no_effect` for blocked launch | `refresh_access_plan` or plan execution | reuse only the returned browser and session |
| sealed recovery plan is available | `no_effect` for blocked launch | `do_not_retry` | review or apply the exact recovery plan under its authority |
| inconsistent terminal owner has no recovery plan | `no_effect` for blocked launch | `do_not_retry` | report the exact blocker and preserve the lane |
| current live or ambiguous owner blocks replacement | `no_effect` for blocked launch | `do_not_retry` | coordinate or wait; never launch a duplicate lane |
| access or recovery evidence is stale | `no_effect` | `refresh_access_plan` | refresh once through the named controller and reclassify |

No adapter may hardcode `reuse_existing_browser` from the lifecycle error
alone. Reuse is a positive proof produced by current access planning.

## Service State Architecture

### Lock telemetry

One operation context will follow a Service State request across load,
mutation, transaction preparation, and commit. It records:

- operation kind and redacted request, job, and trace identifiers;
- process-mutex wait and hold durations;
- file-lock wait and hold durations;
- current phase: process-mutex wait, file-lock wait, load, derive, mutate,
  prepare, commit, or finalize;
- state revision and coarse size or record counts;
- success, timeout, stale revision, commit failure, and effect boundary; and
- the current in-process holder's safe operation metadata when available.

Telemetry must not contain capabilities, URLs, profile paths, page content,
provider data, or tenant-private payloads. It must be available in bounded
diagnostics and test receipts without turning the Service State file into a
high-churn log.

### Critical-section reduction

The implementation proceeds incrementally:

1. measure current wait and hold phases with deterministic fixtures;
2. keep immutable snapshot reads outside the process mutation mutex when the
   repository revision contract proves that safe;
3. perform pure validation, derivation, and serialization before exclusive
   commit when possible;
4. acquire locks in one documented order and avoid retaining the process mutex
   while merely waiting on an external file lock;
5. re-read and compare the durable revision at commit, rejecting stale
   prepared transactions before effect; and
6. preserve crash-safe ordering for Service State, lifecycle, owner, handoff,
   and transaction-ledger records.

If measurement proves that high-churn job and event persistence dominates the
critical interval, split only that bounded ledger behind an explicit schema,
revision, recovery, and projection boundary. A wholesale Service State rewrite
is outside this plan.

Timeout budgets become measured policy after critical-section repair. They
must not be raised first to conceal contention.

## Frozen Invariants

1. A timeout before mutation entry is provably zero-effect.
2. An uncertain terminal persistence outcome never becomes an automatic retry.
3. One request identity cannot produce two browser, tab, route, or recovery
   effects.
4. Multi-process file-lock safety remains authoritative across daemon and CLI
   processes.
5. Read-only status, trace, access-plan, and recovery-plan operations do not
   acquire mutation authority or persist derived entities.
6. Pure preparation may move before the exclusive commit only when a revision
   check detects stale inputs before effect.
7. Transaction commit remains crash-safe across every participating durable
   file, including interruption and rollback paths.
8. Unknown fields and newer schema records survive mixed-version reads and
   rollback.
9. Lifecycle owner errors remain fail-closed until the acquisition or recovery
   classifier proves one safe outcome.
10. `reuseAllowed=true` requires exact, mutually consistent `browserId` and
    `sessionName` hints from current evidence.
11. A Recovery Plan is sealed, revision-bound, and zero-effect until applied.
12. A hard blocker never becomes permission for force unlock, process kill,
    broad cleanup, or a duplicate profile lane.
13. All public adapters derive recourse from one shared domain model.
14. Job and trace persistence preserve both the original error text and the
    structured recourse outcome.
15. No telemetry or recourse object leaks a capability, local profile path,
    provider data, or raw remote-view route.

## Execution Graph

```text
P142-A contract and red fixtures
  ├─ P142-B structured recourse core → P142-C lifecycle classification
  └─ P142-D lock telemetry → P142-E read-path decontention
                                  ↓
                           P142-F commit-path repair
  P142-C + P142-F
          ↓
  P142-G public surfaces and guidance
          ↓
  P142-H provider-free and development acceptance
          ↓
  Plan 0137 Slice J prerequisite checkpoint
```

P142-B and P142-D may proceed independently after Slice A because their
primary write surfaces differ. P142-C joins the Plan 0137 acquisition model.
P142-F remains on the critical path because concurrency acceptance cannot
close before crash-safe commit behavior is proven. P142-G joins both branches
so every public surface describes the implemented, measured behavior.

## Execution Slices

### P142-A | Freeze the incident and outcome contracts

Objective: turn both observed failures into provider-free, redacted,
deterministic regression fixtures before changing behavior.

Deliverables:

- lock-holder, lock-contender, state-size, and effect-boundary fixtures;
- lifecycle matrices for live reusable, live non-reusable, terminal clean,
  terminal inconsistent, recovery-available, and hard-blocked owners;
- versioned recourse schema and compatibility rules;
- one source authority map for classification, persistence, projection, and
  rendering; and
- red tests proving the current job record exposes only a string error.

Acceptance:

- fixtures reproduce both exact error codes without launching Chrome;
- the lock fixture identifies whether mutation entry occurred;
- lifecycle fixtures prove no duplicate launch; and
- schema review resolves every field in the frozen recourse matrix.

### P142-B | Add the structured recourse core

Objective: establish one shared Rust domain model and durable job projection.

Deliverables:

- versioned failure, effect-state, retry-disposition, and recommended-action
  types;
- phase-aware lock-timeout classification;
- additive structured failure data on jobs, events, traces, incidents where
  applicable, HTTP, and MCP responses;
- preservation of the legacy string error; and
- generated TypeScript types plus a typed client error surface.

Acceptance:

- clients never parse error prose to choose a mutation;
- unknown recourse codes remain safely non-retryable;
- an effect-uncertain response always carries inspection locators; and
- adapter parity tests prove byte-equivalent semantics after normalization.

### P142-C | Route lifecycle blockers through acquisition and recovery

Objective: replace action-less lifecycle errors with current, exact recourse.

Deliverables:

- one classifier shared by access planning, profile acquisition, direct launch
  admission, and recovery planning;
- exact reuse hints only for compatible current owners;
- sealed Recovery Plan linkage for recoverable terminal inconsistencies;
- hard blockers for live, foreign, ambiguous, or unsupported states; and
- a refresh path for stale classification that performs no mutation.

Acceptance:

- all lifecycle matrix cases produce exactly one stable client outcome;
- the Ads-review case stops with a typed blocker when reuse is not offered;
- a healthy reusable owner returns exact route hints through the access plan;
- an inconsistent terminal owner returns a Recovery Plan only when its
  preconditions are fully proven; and
- no case retries direct launch or opens a duplicate lane.

### P142-D | Instrument lock ownership and effect boundaries

Objective: make contention attributable and budgets evidence-based.

Deliverables:

- bounded operation context and phase timings;
- current in-process holder metadata without private payloads;
- timeout diagnostics in job, trace, and doctor or status readback;
- counters or histograms suitable for isolated stress receipts; and
- tests for telemetry cleanup after success, panic, timeout, and cancellation.

Acceptance:

- every synthetic timeout identifies its wait phase and proven effect state;
- diagnostics identify an active holder class without exposing secrets;
- completed holders leave no false active-owner record; and
- telemetry overhead is measured and bounded.

### P142-E | Decontent snapshot and read paths

Objective: ensure ordinary reads do not queue behind mutation-only work when a
consistent immutable revision can be returned safely.

Deliverables:

- a documented read consistency contract;
- revision-bound immutable snapshot loading;
- shared or lock-minimized read behavior where platform semantics permit;
- no-write proofs for jobs, status, trace, access-plan, and doctor reads; and
- fallback behavior for unsupported or degraded locking platforms.

Acceptance:

- concurrent reads do not acquire the process mutation mutex;
- each response identifies one consistent Service State revision;
- a read racing a commit returns either the prior or next valid revision,
  never a partial transaction; and
- cross-platform unit tests preserve Windows and Unix behavior.

### P142-F | Shorten and harden mutation commit

Objective: reduce exclusive hold time without weakening transactional safety.

Deliverables:

- precomputed pure transaction preparation where valid;
- explicit lock ordering and no process-mutex convoy while waiting on an
  external lock;
- revision compare-and-swap at the exclusive commit boundary;
- bounded persistence for high-churn records when measurement justifies it;
- crash, interruption, stale revision, rollback, and replay fixtures; and
- measured timeout and contention budgets after the repair.

Acceptance:

- realistic state-size burst tests complete without process-mutation lock
  timeouts at the frozen concurrency target;
- stale prepared transactions fail before effect and can be re-planned;
- interruption at every durable write boundary converges on restart;
- multiple processes cannot commit conflicting revisions; and
- a failed commit leaves one exact recovery obligation or none.

### P142-G | Align clients, dashboard, help, docs, and skills

Objective: make the safe response obvious and consistent without source
knowledge.

Deliverables:

- CLI error rendering and `cli/src/output.rs` help;
- README behavior and troubleshooting guidance;
- repository and shared Agent Browser skill guidance;
- docs-site service, profiles, troubleshooting, and remote-view guidance where
  applicable;
- generated client helper and typed error examples;
- dashboard selected-job and selected-record actions; and
- inline documentation on lock, effect-state, and lifecycle classification
  boundaries.

Acceptance:

- exact error-code guidance is present on every maintained operator surface;
- the client-facing rule is conditional: reuse only when offered, otherwise
  inspect Recovery Plan or stop on the exact blocker;
- the lock-timeout rule distinguishes zero effect from uncertain effect;
- dashboard actions remain disabled when the recourse object does not grant
  them; and
- no example recommends process cleanup, force unlock, blind retry, or a
  duplicate profile lane.

### P142-H | Complete provider-free and isolated acceptance

Objective: prove the full contract before any production candidate work.

Required proof:

- focused Rust unit, transaction, lifecycle, and service-contract tests;
- service request metadata, schema, HTTP, MCP, generated-client, dashboard,
  help, docs, and skill parity;
- deterministic multi-thread and multi-process contention tests;
- realistic-state burst and latency receipts with the frozen thresholds;
- crash and restart convergence at each transaction boundary;
- three disposable development-runtime browser launch, URL read, close, and
  residue cycles;
- an isolated lifecycle conflict that proves exact reuse, recovery, and hard
  blocker branches without provider navigation; and
- a fresh OS process and resource census after development acceptance.

P142-H closes only when all acceptance criteria have evidence bound to one
integrated commit and development runtime identity. Production remains a
separate Plan 0137 Slice J gate.

## Write Surfaces

Expected Rust ownership:

- `cli/src/native/service_store.rs` and focused persistence helpers;
- service job, event, trace, incident, and response models;
- `cli/src/native/runtime_lifecycle.rs`;
- access-plan, profile-acquisition, and recovery classifiers;
- HTTP and MCP service adapters;
- `cli/src/native/service_contracts.rs`; and
- focused Rust and provider-free fixture modules.

Expected contract and client ownership:

- `docs/dev/contracts/service-request.v1.schema.json` and relevant service
  observability schemas;
- service contract metadata and code generators;
- generated `packages/client` JavaScript and TypeScript surfaces;
- service client and adapter-parity tests; and
- dashboard service workspace, job, incident, and trace projections.

Expected user-facing ownership:

- `cli/src/output.rs`;
- `README.md`;
- `skills/agent-browser/SKILL.md`;
- the shared `agent-browser-service` skill and its relevant references;
- `docs/src/app/` service, profile, and troubleshooting pages; and
- inline source documentation.

Before implementation starts, reconcile these surfaces against
`docs/dev/active-lanes.yaml` and every worktree. If another active lane owns an
overlapping file, split the slice or wait for a recoverable checkpoint. Do not
copy or overwrite foreign changes.

## Validation Strategy

Use the repository Cargo safety wrapper for every Cargo command that can
compile code on WSL.

The exact focused commands will be frozen in P142-A after test seam discovery.
The minimum validation families are:

```bash
scripts/ci/cargo-safe.sh fmt --manifest-path cli/Cargo.toml -- --check
scripts/ci/cargo-safe.sh clippy --manifest-path cli/Cargo.toml -- -D warnings
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml service_store -- --test-threads=1
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml runtime_lifecycle -- --test-threads=1
pnpm test:service-api-mcp-parity
pnpm test:service-client
pnpm test:dashboard-inspector-actions
pnpm validation:select -- --base <last-known-green-commit>
```

The acceptance note must record:

- commit and generated-client identities;
- fixture state size and record counts;
- concurrency, attempt, wait, hold, and latency distributions;
- timeout count by phase and effect state;
- crash-boundary and restart results;
- lifecycle recourse matrix results;
- duplicate-process and residue counts;
- development runtime identity and browser executable;
- exact skipped gates and why; and
- fresh git, active-lane, and OS process readbacks.

## Quantitative Gates

P142-A must freeze thresholds before optimization. At minimum:

- one realistic-state fixture at or above the observed 2.9 MiB class;
- one burst representing at least the observed mix of launch, remote-view, tab,
  and read requests;
- zero duplicate browser, tab, route, or recovery effects;
- zero unclassified failed operations;
- zero partial or cross-revision reads;
- zero process-mutation lock timeouts at the accepted target concurrency after
  P142-F; and
- bounded p95 and maximum lock hold times justified by the measured timeout
  budget.

If the target concurrency or latency threshold cannot be justified from a
reproducible fixture, the slice remains open. Raising a timeout does not satisfy
the gate.

## Execution Bounds And Checkpoints

- Maximum work-unit attempts before split or tactic change: 2.
- Maximum review and rework cycles per slice: 1.
- Maximum consecutive hardening checkpoints without acceptance movement: 2.
- Maximum fresh-context drift-discovery passes for the whole plan: 1.
- Checkpoint cadence: every material state transition and no later than three
  slices or 90 minutes of active implementation work.
- Retry controllers: the Service State transaction coordinator owns stale
  revision retries; the acquisition coordinator owns one access-plan refresh;
  the recovery coordinator owns idempotent recovery resume. No adapter or
  client creates its own retry loop.
- Every retry controller has a maximum of one immediate reclassification or
  replay attempt in provider-free acceptance. A second failure exits to a
  typed terminal outcome and durable evidence.
- Each checkpoint records state transition, acceptance state, progress
  classification, evidence, material blockers, and next action or stop reason.

Before implementation, change State to `OPEN`, refresh the source baseline,
and register P142 if execution uses a topic branch, secondary worktree, or
parallel writer. Planning alone does not reserve write custody.

## Rollback And Compatibility

- Structured recourse fields are additive. Removing them restores older
  presentation behavior without corrupting retained jobs.
- Existing string errors remain until a separately planned compatibility
  removal.
- New readers preserve unknown recourse fields and newer action values.
- Old readers continue to see failed job state plus the original error string.
- Lock-path changes land behind provider-free crash and multi-process tests.
- Any ledger split must provide forward, backward, rebuild, and rollback
  behavior before isolated runtime validation.
- A development-runtime regression rolls back only the development candidate
  and retains its diagnostic receipt.
- Production rollback, state restoration, process restart, or owner repair is
  not authorized by this plan.

## Client Guidance To Publish

The final user-facing guidance must communicate this rule:

> Do not retry either blocker blindly. For a Service State lock timeout, follow
> the returned effect state and retry disposition; inspect the exact job and
> current resource when effects are uncertain. For a lifecycle-owner blocker,
> reuse only when the current access plan returns exact browser and session
> hints. Otherwise review the returned Recovery Plan or stop on the exact hard
> blocker. Never launch a duplicate profile lane.

For the Ads review case specifically:

> Reuse was not offered. Agent Browser returned an inconsistent terminal-owner
> blocker, so the client correctly stopped without retrying or launching a
> duplicate lane. The updated contract will also return whether a sealed
> recovery plan exists and the exact safe next action.

## Completion Criteria

Plan 0142 may close only when:

1. both incident classes have deterministic provider-free regressions;
2. every covered failure carries structured phase, effect, retry, reuse,
   recovery, locator, and hard-stop semantics;
3. Service State reads and mutation preparation no longer create avoidable
   process-mutex contention;
4. realistic-state burst tests meet the frozen concurrency and latency gates;
5. crash, restart, stale revision, and multi-process tests preserve durable
   consistency;
6. every lifecycle matrix case chooses exactly one of reuse, recovery, refresh,
   or hard block;
7. no test or isolated runtime run creates a duplicate profile lane;
8. CLI, HTTP, MCP, job, trace, generated client, dashboard, help, README, docs
   site, shared skill, repository skill, and inline documentation agree;
9. isolated development acceptance and the final process census are clean;
10. the integrated commit and all required receipts are durable; and
11. Plan 0137 records P142 as satisfied before its Slice J production
    candidate gate proceeds.

Completion of this plan proves source and isolated-runtime readiness. It does
not prove the production installation, provider behavior, or tenant operation
complete.
