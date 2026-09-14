# Plan 0181 | Lease-Authority Kernel Crate Extraction

Date: 2026-09-13

Plan version: 3

State: OPEN

Lane: P181

Product lane: PL-PLATFORM

Planning branch: `platform/lease-authority-crate-plan`

Branch: `platform/lease-authority-crate`

Target: `main`

Integration: merge

Work item: [CochranResearchGroup/agent-browser#99](https://github.com/CochranResearchGroup/agent-browser/issues/99)

Source baseline: `16d4fb22dfd8cf96cd7e65edfd0b66fe54953945`

Depends on:

- [ADR 0001](../../adr/0001-separate-profile-access-from-runtime-ownership.md), which requires a proven in-process seam before crate extraction;
- [Plan 0144](0144-2026-08-31-lease-authority-coordination-and-revocation-plan.md) and [issue #71](https://github.com/CochranResearchGroup/agent-browser/issues/71), which own unfinished public, effect-admission, and installed proof gates for the same authority;
- [Plan 0151](0151-2026-09-01-cdp-transport-crate-extraction-plan.md), the accepted procedural precedent for a real Rust crate seam; and
- [Plan 0174](0174-2026-09-12-ci-production-scale-and-process-identity-reliability.md), which owns current Cargo and provider-free test reliability.

Consolidation: required

Authority: SOURCE IMPLEMENTATION AND PROVIDER-FREE VALIDATION ONLY. This plan
authorizes the bounded crate extraction, repository documentation, work-item
and lane coordination, local builds, and provider-free tests. It does not
authorize runtime installation, protected-authority upgrade, browser or profile
use, provider effects, production state changes, release publication, or
destructive cleanup.

## Objective

Extract the canonical Lease-authority kernel into one deep
`agent-browser-lease-authority` Rust crate. The crate owns active claims,
monotonic revisions, fencing, authorization, receipts, signing and verification,
protected protocol custody, durable authority publication, and the pure
principal and profile-identity mechanics those decisions require.

The CLI remains the product adapter. It owns Service State joins, profile and
browser policy, runtime orchestration, executable dispatch, installation, and
public CLI, HTTP, MCP, generated-client, and dashboard integration.

The architecture outcome is mandatory. Build acceleration is a falsifiable
secondary acceptance axis, not a premise of the extraction:

1. architecture: callers cross one smaller interface and the prior owner is
   deleted without exposing secrets or creating a compatibility facade; and
2. build acceleration: the focused authority edit and test loop is measured
   comparably, with no complete-workspace regression. The plan may integrate a
   depth-only result only when architecture and correctness are accepted, but
   it must report a neutral or slower loop plainly and must not claim build
   acceleration.

## Current State

P181 is active on `platform/lease-authority-crate` through work item #99. The
published implementation baseline is
`16d4fb22dfd8cf96cd7e65edfd0b66fe54953945`; source checkpoint `b7cd01bd` is
published for draft PR #106. P0 through the P4 source migration are implemented
but not yet compile-validated. The architecture contract is green, Cargo
metadata reports the third workspace member, the old native owner is deleted,
and one private CLI adapter retains repository, Service State, and runtime-owner
joins. Exact label comparison preserves all 106 baseline authority and protocol
tests as 108 crate tests plus three CLI adapter tests, including five additional
principal and profile-identity tests.

The immutable baseline preparation and four candidate Cargo validation
attempts were not admitted because the Cargo wrapper reported host memory pressure; no Cargo
scope, rustc process, test, or benchmark sample started. The latest readback had
approximately 24 GiB available while the wrapper required approximately 30 GiB
to preserve its configured reserve and one build claim. Unrelated browser and
service processes are outside P181 cleanup authority. Native-Linux draft-PR CI
is therefore the first pending compile signal; local Cargo validation remains
required when admission becomes available. Issue #71 is currently closed in the
forge, but P181 has no authority to treat that tracker state as Plan 0144
acceptance or to reopen it. Its public, effect-admission, and installed gates
remain separate.

The candidate workspace contains the `agent-browser` binary crate plus the
`agent-browser-cdp` and `agent-browser-lease-authority` library crates. The
baseline Lease-authority kernel was a cohesive native module inside the binary
compilation unit; checkpoint `b7cd01bd` moves that owner into the new crate.

Current baseline evidence:

| Surface | Size | Tests | Recent change pressure |
| --- | ---: | ---: | ---: |
| authority domain and trust state | 5,250 lines | 22 | 25 commits in 30 days |
| protocol and durable store | 10,604 lines | 56 | 32 commits in 30 days |
| protected client | 1,873 lines | 7 | included below |
| endpoint custody | 752 lines | 13 | included below |
| Linux protected service | 1,547 lines | 8 | included below |
| complete extraction population | 20,026 lines | 106 | 38 directory commits in 30 days |

Fourteen native files contain 46 imports of the authority module. The current
focused command still targets the whole CLI test binary. There is no Lease
Authority Cargo package, test compartment, validation-selector route, or
comparable before-and-after focused-loop timing.

The historical 2026-09-01 cache-off isolated Cargo check selected eight jobs at
62.737 seconds and approximately 3.54 GiB maximum RSS. It calibrates the host,
not this extraction. Plan 0151's CDP timings are likewise precedent, not a
Lease Authority baseline.

The module passes the deletion test: removing it would force claim, fencing,
authorization, signing, custody, publication, and replay rules into callers.
Its extraction is not move-only. These upward dependencies prevent a valid
crate seam:

| Current dependency | Required treatment |
| --- | --- |
| authority to `ServiceStateRepository` | keep transactions in a CLI adapter |
| authority to Service State and runtime-owner joins | pass typed current evidence into cohesive authority operations |
| protocol to `service_principal` | move only pure registry, capability, rotation, authentication, and digest mechanics |
| protocol and client to `runtime_profile` helpers | move the exact name-validation and canonical-identity primitives once; keep runtime-profile orchestration in CLI |

Plan 0144 remains open through issue #71. P181 is a structural successor, not a
replacement. It cannot claim P144 public parity, effect-sink coverage, or
installed acceptance merely because the code compiles in a crate. P181 must
not activate concurrently with a P144 writer on the authority files.

## Consolidated Batch

This batch contains one bounded outcome:

- freeze the current authority interface and dependency inventory;
- deepen the four leaking dependencies in process;
- extract the authority and protected protocol into one crate;
- migrate consumers directly and delete the old owner;
- partition the 106 authority tests into a focused crate loop while retaining
  product integration tests in the CLI crate;
- add deterministic architecture, schema, security, and platform guards;
- measure focused, incremental, clean, and complete build paths; and
- update contributor architecture and validation guidance before integration.

Prerequisite deepening belongs in this batch because moving the current files
unchanged would preserve cycles and widen sensitive implementation details.
Unfinished P144 feature and live-proof work stays outside this batch.

## Domain And Seam Decision

`CONTEXT.md` defines the Lease-authority kernel as the singular authority for
active resource claims, monotonic revisions, fencing tokens, and effect
authorization. Historical events, terminal records, Service State projections,
access policy, and runtime ownership proof remain separate concepts.

### Included in `agent-browser-lease-authority`

- claim resource keys, modes, active state, transitions, revisions, and fencing;
- authorization, recovery, administrative, terminal, adoption, and launch
  intents and receipts;
- idempotency, replay, authority time, and expiry decisions;
- private signing, public verification, key rotation, trust generations, and
  anti-rollback rules;
- protected protocol envelopes, framed exchange, service dispatch, durable
  protected store, client, and endpoint custody;
- pure principal and capability registration, authentication, rotation, and
  digest mechanics required by authority decisions;
- the singular profile-name validation and canonical profile-identity digest
  primitives used by authority and CLI; and
- core, protocol, store, client, service, and custody interface tests.

### Retained in `agent-browser`

- `ServiceStateRepository` implementations and file-lock transactions;
- Service State joins, projections, compatibility migration, and persistence;
- access policy and Profile occupancy evaluation;
- browser, session, tab, route, retained-browser, and runtime-owner orchestration;
- runtime profile selection and discovery beyond the shared identity primitive;
- browser effect execution and recovery workflows;
- executable argument and environment dispatch in `main.rs`;
- workstation installation and systemd unit rendering; and
- public adapters and product integration tests.

### Interface constraints

Concrete Rust names remain an implementation decision, but these facts are
frozen:

1. The crate accepts typed intent and current evidence and returns typed
   decisions or receipts. It never accepts mutable `ServiceState` or a broad
   `ServiceStateRepository` interface.
2. Private claim maps, signing-key constructors, secret loaders, raw signing
   helpers, and mutable store internals remain behind the seam.
3. Public verification cannot initialize an authority domain or access private
   signing material.
4. User-home trust and root-owned protected stores remain distinct adapters
   with distinct custody rules.
5. The protected authority derives time, principal, process, and custody
   evidence rather than trusting caller projections.
6. Production CLI and in-memory test adapters make injected ports real.
7. The crate does not depend on `agent-browser`, CDP, browser engines, Tokio,
   Reqwest, image processing, dashboard code, or generated clients.
8. CLI consumers import the crate directly. No permanent or final
   `native::service_lease_authority` facade remains.

## Preserved Security And Correctness Invariants

- Only active claims authorize or block effects. Events remain history.
- Every mutation validates or advances the exact revision and fencing token.
- Durable publication precedes a successful mutation response.
- A stale publisher cannot reselect an older generation.
- Corrupt history may degrade history only; corrupt selected authority never
  falls back silently.
- Capability and secret material remains nonserializable where protected,
  redacted from debugging, and wiped on drop where applicable.
- Bearer authentication occurs before private signing-key use.
- Endpoint and process custody remains kernel-derived and fail closed.
- Linux-only client and service behavior remains target-gated. Unsupported
  platforms retain explicit fail-closed behavior.
- Schema versions, error codes, hashes, canonical identities, and replay results
  remain compatible unless a separately reviewed contract change is required.
- P144 public-adapter, effect-sink, and installed gates remain separately open.

## Critical Path

```text
P0 freeze seam and baseline
  -> P1 remove upward dependencies in process
  -> P2 extract core and pure identity mechanics
  -> P3 extract protected protocol and custody
  -> P4 migrate product adapters and delete old owner
  -> P5 wire validation, join, and validate
  -> P6 measure build feedback
  -> P7 review, document, integrate, and close
```

P0 through P4 have one critical-path source owner because their write surfaces
overlap. Validation tooling may proceed beside P2 through P4 after P0 freezes
the interface. Documentation follows the accepted interface.

## Delivery Sequence And Budget

### P0 | Freeze The Seam And Capture The Baseline

Owner: primary architecture agent.

Expected writes:

- one architecture inventory and red contract for the absent crate, forbidden
  upward imports, old-owner deletion, and direct crate imports; and
- a benchmark manifest recording toolchain, host, profile, jobs, cache, linker,
  target directory, command, and candidate identity.

Required evidence:

- current CodeGraph dependency map and exact extraction population;
- P144 work-item, branch, and writer reconciliation;
- current test and target-platform inventory;
- red architecture contract; and
- one smallest usable comparable focused-loop baseline before candidate
  measurement. The immutable baseline SHA and command must be frozen before
  source movement; actual execution may occur later from that exact SHA when
  the Cargo wrapper admits it.

Exit: the interface constraints, file population, exclusions, benchmark SHA and
commands, and P144 ordering are frozen. Stop if P144 has an overlapping active
writer or if comparable measurement requires product behavior changes. A
resource-gated, unadmitted waiter does not block source work when the immutable
baseline remains independently runnable, but the baseline preparation must pass
before any candidate measurement.

Evidence deadline: 90 active minutes after implementation begins.

### P1 | Remove Upward Dependencies In Process

Owner: one normal implementation worker, integrated by the primary.

Expected writes:

- narrow CLI evidence and transaction adapters;
- pure principal and capability mechanics separated from session and tab
  continuity logic; and
- one canonical owner for profile-name validation and identity digest.

Evidence:

- the authority core imports no Service State, Service State repository,
  runtime-owner orchestration, or runtime-profile orchestration;
- identity fixtures preserve symlink, nearest-existing-ancestor, versioned
  hash, and Windows normalization behavior; and
- current claim, signing, publication, replay, and principal tests pass.

Exit: ADR 0001's in-process seam is proven. Stop if this requires moving
browser or product policy.

### P2 | Extract The Core Crate

Owner: the same critical-path implementation worker.

Expected writes:

- workspace and crate manifests;
- `crates/agent-browser-lease-authority/` core modules;
- direct CLI imports; and
- moved core and pure identity tests.

Evidence:

- Cargo metadata reports the third workspace member;
- no crate source imports `agent-browser` paths;
- public visibility matches the frozen constraints; and
- core decisions are tested through the crate interface.

Exit: the core builds and tests independently. Any temporary integration seam
must be named and deleted by P4.

### P3 | Move Protected Protocol, Store, Client, Service, And Custody

Owner: critical-path implementation worker.

Expected writes:

- protocol, durable store, client, Linux service, and custody modules;
- target-gated dependencies and build configuration; and
- the corresponding moved tests.

Required invariant fixtures include:

- `publication_crash_before_selector_keeps_prior_generation_selected`;
- `stale_publisher_cannot_reselect_an_older_valid_generation`;
- `protected_state_round_trip_preserves_replay_without_persisting_the_bearer`;
- `corrupt_selected_authority_never_falls_back_to_an_older_generation`;
- `authenticated_acquire_derives_holder_identity_inside_the_kernel`;
- `service_identity_challenge_binds_nonce_domain_epoch_and_custody`.

The baseline inventory contains 106 tests, but it is not a promise that all
106 move. P0 freezes a one-for-one invariant ledger. Interface-level authority,
protocol, store, client, service, and custody tests move to the crate. Tests
that exercise Service State, runtime-owner bindings, repositories, or product
adapters remain in the CLI. In particular,
`effect_boundary_rejects_diverged_owner_principal_binding` remains a CLI
integration fixture unless its product dependencies are deliberately replaced
without weakening the invariant.

Exit: protected behavior is independently testable without exposing private
custody or signing implementation.

### P4 | Migrate Product Adapters And Delete The Prior Owner

Owner: primary integration agent.

Expected writes:

- `main.rs` Linux entry dispatch;
- Service State, profile recovery, acquisition, lease, access, and action-runtime
  adapters;
- retained CLI integration tests; and
- deletion of the old implementation and every temporary facade.

Evidence:

- the deletion test passes;
- no second principal, identity, signing, claim, or publication owner remains;
- Service State serialization and public behavior remain compatible; and
- every consumer imports the crate directly or uses an intentional CLI adapter.

Exit: one crate owns authority behavior and the CLI owns product integration.

### P5 | Validation Wiring, Candidate Join, And Validation

Owner: deterministic validation worker for execution; primary for disposition.

Before freezing the candidate, add the new crate test compartment,
validation-selector route, and architecture-contract wiring. Then run in
increasing cost order:

1. architecture contract self-test and current-tree check;
2. the complete mapped crate-side invariant selection;
3. the complete mapped retained CLI integration selection, including focused
   serialization, repository, acquisition, release, recovery,
   access-plan, launch, adoption, entrypoint, and IPC adapter tests;
4. validation-selector self-test for the new crate;
5. existing CDP architecture contract;
6. `pnpm validation:select -- --base <batch-baseline>`;
7. `scripts/ci/cargo-safe.sh fmt --all --manifest-path Cargo.toml -- --check`;
8. `scripts/ci/cargo-safe.sh clippy --workspace --manifest-path Cargo.toml -- -D warnings`;
9. `scripts/ci/rust-tests.sh`; and
10. target-platform CI, especially Linux custody and Windows compilation.

The candidate is frozen before broad validation. One failed broad run permits
one bounded repair pass and selective reuse of unaffected gates under policy
0042. No runtime installation or browser launch belongs to P181.

### P6 | Build-Acceleration Measurement

Owner: tools first, with an economical worker limited to receipt extraction.

Freeze baseline and candidate SHA, host, toolchain, eight Cargo jobs, WSL
admission and cgroup limits, test profile, environment, and concurrency. Use
separate worktrees and disposable target directories. Set
`AGENT_BROWSER_CARGO_CACHE=off` and `AGENT_BROWSER_CARGO_FAST_LINKER=off` for
causal comparison. Preserve the shared target directory.

Every `/usr/bin/time -v` receipt records exit status, wall time, maximum RSS,
command, SHA, target directory, test count, and Cargo timing artifact. A pair
with different toolchain, admission, job count, profile, or cache posture is
invalid and remains visible rather than being averaged.

#### Primary Focused Edit Loop

Prime one reusable isolated target per variant for each warm benchmark family.
Make one private,
benchmark-only token change in the authority implementation for each measured
run so Cargo observes a real edit. Alternate AB and BA order across five pairs.

- Baseline: wrapper-run CLI test binary filtered to Lease Authority.
- Candidate: wrapper-run `cargo test -p agent-browser-lease-authority`.
- Promotion target: the identical frozen crate-side invariant selection passes
  in both variants, candidate median is at most 50 percent of baseline, and no
  run breaches the existing per-scope memory limit. The retained CLI selection
  is correctness evidence and is timed separately from this focused comparison.

The 50 percent threshold is the frozen acceleration target. A candidate median
above 50 percent does not by itself block a depth-only integration, but it
prohibits an acceleration claim. Any focused-loop regression triggers the
bounded diagnosis below. Missing the target never permits discarding failures
or adding samples.

#### Downstream Invalidation Safeguard

Across three paired authority edits, run wrapper-governed
`cargo check --workspace`. Candidate median must not regress more than 10
percent and RSS must remain within the resource contract. A larger regression
requires dependency or crate-shape repair before any acceleration claim.

#### Cold Workspace Safety Pair

Run one cache-off, linker-off, `CARGO_INCREMENTAL=0` workspace check per SHA in
fresh disposable targets. This is directional safety evidence, not a clean-build
performance conclusion.

The frozen invocation ledger is 23 admitted compiling invocations at most:

- 4 unmeasured preparation invocations: one baseline and one candidate prime
  for each of the focused and downstream warm families;
- 18 measured invocations: 10 focused, 6 downstream, and 2 cold; and
- 1 replacement invocation only for a typed infrastructure failure. The
  invalid receipt remains preserved and the replacement repeats the same
  variant, family, and pair position.

There is no other priming, exploratory compile, or added sample inside this
packet. Report focused, retained-CLI, downstream, cold, complete-suite,
wall-time, and RSS axes separately.

An invocation that the Cargo wrapper never admits is an infrastructure receipt,
not a compile or benchmark sample. Preserve it, but do not count it as one of
the 23 compiling invocations. This distinction cannot be used after Cargo
admission or rustc startup.

Exit: the report states which loop improved, stayed neutral, regressed, or was
not comparably measured. A correctness regression blocks integration. A
focused-loop regression triggers one bounded diagnosis and then repair or a
recommendation not to land the split.

### P7 | Review, Documentation, Integration, And Closeout

Owner: primary agent.

- Run one fresh-context architecture and security review.
- Adjudicate findings once, then run one closed-world verification pass over
  accepted findings and critical regressions.
- Update Cargo guidance, `AGENTS.md` architecture and testing sections, inline
  module documentation, and the plan. Executable test-compartment and
  validation-selector changes belong to P5 and must already be validated.
- Update ROADMAP, RUNBOOK, work item, and active-lane projection with separate
  source, measurement, integration, and runtime claims.
- Publish a pull request from the exact validated branch and merge only after
  required CI and self-review pass.

Exit: remote `main` contains the accepted extraction through a merged pull
request, the lane and work item close truthfully, and no runtime claim is made.

### Complete Delivery Budget

- Maximum packets: eight, P0 through P7.
- Maximum attempts per packet: two.
- Maximum broad validation: one initial run and one repair run.
- Maximum review discovery passes: one.
- Maximum review and rework cycles: one.
- Maximum benchmark Cargo invocations: 23, comprising 4 preparation, 18
  measured, and 1 typed-infrastructure replacement invocation.
- Maximum active primary effort: 16 hours.
- Maximum aggregate worker effort: 24 hours.
- Checkpoint backstop: 30 active minutes or two checkpoints without outcome
  progress.
- First evidence deadline: 90 active minutes.

If complete proof cannot fit the remaining allowance, preserve the latest
coherent checkpoint and reframe before another build or worker attempt.
Packet renaming, worker replacement, or a successor does not reset the bounds.

## Worker Assignments

Execution bias: balanced. Maximum topology is the primary plus two active
workers. Nesting is prohibited.

| Role | Initial route | Write scope | Evidence and stop |
| --- | --- | --- | --- |
| primary architecture and integration owner | strongest available primary, currently `gpt-6-astra`, high | shared manifests, interface freeze, product adapters, plan, integration | owns seam, security, finding disposition, benchmark interpretation, and final claim |
| core extraction worker | `gpt-5.6-terra`, medium | new crate and one assigned source packet | one implementation attempt plus one compiler or test repair |
| adapter worker | `gpt-5.6-sol` or `gpt-5.6-terra`, medium | named CLI consumer files only | one implementation attempt plus one bounded repair |
| mechanical validation worker | `gpt-5.6-luna`, medium | architecture checker, runner, selector, benchmark harness, contributor guidance | one pass plus one deterministic repair |
| benchmark receipt worker | tools first; `gpt-5.6-luna`, low only if judgment-light extraction remains | read-only results | one complete measurement packet; rerun only typed infrastructure failure |
| independent reviewer | `gpt-6-astra`, high, fresh compact context | read-only | one discovery pass and one closed-world recheck |

P2 is serialized before P4. Product-adapter migration and validation wiring may
fan out only after the crate interface is frozen and their write sets are
disjoint. The primary owns root and CLI manifests, lockfile, module exports,
plan state, issue disposition, lane catalog, and integration.

### Model Optimization And Calibration

Routing minimizes total accepted milestone effort rather than raw call price or
maximum parallelism.

- Deterministic inventory, hashing, formatting, measurement, and counters use
  tools first.
- Mechanical work starts with `gpt-5.6-luna` and an exact verifier.
- Bounded implementation starts with `gpt-5.6-sol` or `gpt-5.6-terra` at
  medium reasoning.
- Architecture, custody, secret-handling, dependency cycles, and acceptance
  stay with `gpt-6-astra` at high reasoning.
- Escalation requires a named reasoning obstacle after one bounded attempt.
  Cargo admission, compiler imports, fixture failures, or missing authority are
  diagnosed as tool, code, environment, or governance conditions first.
- Ordinary work returns to the economical route after a specialist decision.

Each receipt records packet, agent handle, requested model and effort,
runtime-reported effective model and effort when available, context scope,
status, elapsed wall time, accepted result, defects, primary interventions, and
measured allocation or labeled proxy. This single delivery is a provisional
calibration sample only. It cannot establish general price or savings claims.
No synthetic model benchmark or automatic blanket reviewer is authorized.

## Planning Delegation Receipt

Three read-only, one-level workers informed version 1:

| Worker | Requested route | Result | Primary disposition |
| --- | --- | --- | --- |
| `/root/lease_seam_audit` | `gpt-6-astra`, high | complete | accepted the dependency splits, population, exclusions, security constraints, and P144 separation |
| `/root/lease_build_routing` | `gpt-5.6-sol`, medium | complete | accepted the invariant inventory, focused-loop milestone, comparable benchmark, packet graph, and economical routing |
| `/root/lease_plan_review` | `gpt-6-astra`, high | complete | reconciled the invocation ledger, moved-versus-retained test partition, acceleration acceptance rule, and validation-wiring sequence |

The runtime did not report effective model, token, elapsed, or cost metadata,
so those fields are unknown. Workers had no write, runtime, live-effect,
child-spawn, or acceptance authority. The primary reconciled all three reports
and retains every plan decision.

## Execution Delegation Receipt

| Packet | Worker | Requested route | Result | Primary disposition |
| --- | --- | --- | --- | --- |
| P0 architecture and validation wiring | `/root/lease_validation_wiring` | `gpt-5.6-luna`, medium | complete after one bounded repair | accepted the red contract, self-tests, test compartment, selector route, and 106-test ledger |
| P0 seam classification | `/root/lease_core_extract` | `gpt-5.6-terra`, medium | semantic inventory complete; later persistence turn hit account usage limit | accepted the 99-move/7-retain partition and four seam decisions; no source edits were attempted |
| P1 initial source route | `/root/lease_core_extract_sol` | `gpt-5.6-sol`, medium | partial profile/principal edits followed by model-capacity failure | preserved the partial files for specialist recovery; capacity failure was not treated as a reasoning result |
| P1 through P3 source recovery and extraction | `/root/lease_slice_recovery` | `gpt-6-astra`, high | complete under cheap structural checks | accepted exact principal/profile movement, private CLI adapter isolation, typed authenticated kernel API, protected-stack transfer, direct caller migration, and deletion of the old owner; Cargo validation remains with the primary |

The runtime did not report effective model, token, elapsed, or cost metadata.
The unavailable Terra route is an environment/account constraint, not a
reasoning failure. P1 may route once to the planned Sol alternative without
resetting its attempt or effort bounds.

## Risks And Stop Rules

| Risk | Prevention or stop rule |
| --- | --- |
| move-only extraction creates a shallow crate | P1 proves the in-process seam before crate creation |
| private maps or signing helpers become public | architecture contract rejects forbidden visibility and construction |
| profile identity is copied | one canonical implementation; fixture digests remain identical |
| P181 absorbs or races P144 | explicit dependency, separate ledgers, no simultaneous writer |
| circular Cargo dependency appears | crate never depends on CLI; no generic shared-types crate |
| fast tests lose product coverage | interface tests move; cross-seam adapter tests remain in CLI |
| target behavior regresses | Linux custody, Unix distinctions, Windows compilation, and unsupported paths stay gated |
| workspace builds slow despite a fast crate | downstream threshold and separate claims |
| benchmark noise creates a false claim | frozen environment, alternating order, invalid-pair rejection, bounded samples |
| model cost repeats mechanical work | tools first, economical workers, named escalation trigger |
| worker overlap creates reconciliation debt | one source writer; disjoint validation writer |
| host pressure invalidates results | all compilation uses `cargo-safe.sh`; no concurrent benchmark Cargo |

Hard stops:

- an active P144 writer owns planned authority source;
- the crate needs a dependency on `agent-browser`;
- the seam requires moving browser, access-policy, or runtime-owner semantics;
- a private secret or signing primitive must become public to compile;
- canonical profile identity would have two implementations;
- an existing schema, error, hash, replay, or custody invariant changes without
  a separately reviewed decision;
- the broad validation plus one repair still fails;
- the complete delivery or effort bound is exhausted; or
- work requires an installed, provider, production, credential, browser,
  profile, or destructive effect.

## Evidence And Exit

| Axis | Required evidence | Invalid substitutes |
| --- | --- | --- |
| seam | frozen dependency graph, no upward imports, one owner, deletion test | smaller files or more modules |
| correctness | one-for-one mapping of all 106 baseline tests into crate-side or retained CLI selections, zero lost invariants, complete provider-free suite | moving every fixture or focused green tests alone |
| security | private signing and custody, redaction, zeroization, corruption and endpoint tests | visibility alone |
| compatibility | identical schema, errors, hashes, identities, replay, and target behavior | successful compilation |
| acceleration | comparable focused, downstream, cold, and complete measurements | Plan 0151 timing or one warm run |
| CI | formatting, strict Clippy, architecture guards, selected checks, target CI | local state alone |
| custody | work item, plan, lane, branch, PR, remote SHA, and merged-main readback agree | chat or local branch |
| runtime | explicitly not claimed by P181 | source merge or tests |

P181 completes only when:

1. `agent-browser-lease-authority` is a workspace member with one deep
   interface and no upward dependency;
2. the old implementation and every transitional facade are deleted;
3. Service State and product workflows remain CLI adapters;
4. signing, trust, protocol, publication, replay, and custody invariants pass;
5. target-platform behavior remains explicit and green;
6. authority tests run independently and the complete provider-free suite is
   green;
7. comparable measurement reports the actual build result without overstating
   clean or workspace gains;
8. accepted review findings pass closed-world verification;
9. documentation and validation selection are current;
10. a merged pull request integrates the exact validated commit; and
11. P144's remaining gates retain their own truthful state.

If depth succeeds but the focused loop misses the 50 percent target without
regressing, P181 may integrate as an architecture-only result and records the
measurement explicitly. If the focused loop regresses, one bounded diagnosis
and repair attempt is required; an unresolved regression blocks integration.
In either case, a build-acceleration claim is prohibited unless the frozen
target passes.

## Next Action

Complete P0 by freezing the one-for-one invariant ledger and accepting the red
architecture contract. Then begin P1. Recheck P144 source custody immediately
before the first authority-source edit. The baseline preparation remains due
from the immutable baseline before candidate measurement.
