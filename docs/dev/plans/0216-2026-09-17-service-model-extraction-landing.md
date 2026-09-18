# Plan 0216 | Service Model Extraction Landing

Date: 2026-09-17

Plan version: 3

State: OPEN

Consolidation: required

Product lane: PL-PLATFORM

Lane: P216

Work item: `CochranResearchGroup/agent-browser#178`

Predecessor: [Plan 0205](0205-2026-09-16-service-state-model-crate-extraction.md),
closed at `1ff20161`

Branch: `platform/p205-service-model-crate` with inherited P205 custody

Worktree: `/home/ecochran76/workspace.local/agent-browser-p205`

Target: `main`

Integration: one protected pull request after final local qualification;
operator-disabled GitHub CI remains disabled and must not be restored,
dispatched, retried, or monitored

## Objective

Land the accepted provider-free Service State model extraction from exact
checkpoint `1ff20161` without resuming expression-by-expression privacy work.
Prove the bounded outcome against issue #178, reconcile current `origin/main`
once, run one final local qualification batch, and integrate through one pull
request.

Success means the extracted crate and its deep model interfaces enter canonical
`main` with truthful evidence. It does not mean every Service State field is
private or every compatibility facade is deleted.

## Current State

At admission, `platform/p205-service-model-crate` is clean at
`1ff20161da1108dc3b5e9ad9fc35e4b26486cb67`, pushed to its matching remote,
78 commits ahead and zero behind `origin/main@bea09376`. Issue #178 is open and
no pull request exists.

The published closeout and admission commit is
`909a7c1dc51994ffd45ee855bcaef0bdd3e4b56d`. At the Plan 0216 version 2
amendment, the worktree is clean, the local and remote P216 branch tips match,
`origin/main` remains `bea09376`, the branch is 79 commits ahead and zero
behind, issue #178 remains open, and no pull request exists.

Plan 0205 accepted the canonical `agent-browser-service-model` crate, aggregate
and persisted codec, provider-free records, pure transitions and projections,
CLI adoption, architecture enforcement, and focused test-loop evidence.
Checkpoint 69 removed seven direct registry expressions and passed four new
model tests, eight focused CLI tests, formatting, strict workspace Clippy,
architecture guards, mutation fixtures, diff hygiene, and changed-surface
selection.

No GitHub CI, browser, profile, provider, credential, installed-runtime,
development-runtime, staging, production, or release effect is in flight.

The residual ledger contains 155 classified production runtime-owner field
expressions plus fixture migration and facade-deletion debt. Those counts are
diagnostic inventory, not P216 progress metrics or landing gates.

P211 is an urgent dependent consumer, not part of the P216 implementation
scope. Its published branch is clean at `1d325fa0` and currently changes ten
files. Three files overlap the P216 branch: `cli/src/main.rs`,
`cli/src/native/control_plane.rs`, and `docs/dev/active-lanes.yaml`. A
read-only three-way merge preview found one source conflict in the
`control_plane.rs` stale-owner shutdown test. The product semantics are
compatible: P211 makes explicit daemon shutdown close the browser despite
stale ownership metadata, while P205/P216 moves runtime-owner access behind
the canonical Service Model and fixture seams. P211 has not yet edited the
deferred Service State, installer, profile-acquisition, or remote-view join
surfaces.

## Consolidated Batch

1. Map each issue #178 acceptance statement to current source and durable local
   evidence. Record a blocker only when the current checkpoint contradicts the
   issue, not because optional privacy debt remains.
2. Record one P211 consumer handoff from the final P216 candidate: the
   canonical crate and adapter seams P211 should consume, the three current
   overlaps, and the known stale-owner shutdown-test reconciliation. Do not
   merge, cherry-pick, or implement P211 product work in P216.
3. Reconcile current `origin/main` once. Preserve P211 and P215 source custody;
   resolve only actual integration conflicts and shared planning projections.
4. Repair at most two blocking defects demonstrated by the acceptance map or
   reconciliation. Do not continue opportunistic extraction, field privacy,
   facade deletion, test multiplication, or architecture-guard expansion.
5. Run one final local qualification batch matched to the complete branch diff.
   Reuse valid checkpoint evidence when its covered source is unchanged.
6. Publish one pull request, review only the bounded landing contract, merge it,
   verify canonical `main`, close issue #178 when its acceptance is satisfied,
   and close this plan. P211 then reconciles its existing branch and worktree
   onto the integrated Service Model boundary before beginning its deferred
   overlapping implementation.

## Scope

Included:

- issue #178 acceptance mapping;
- one read-only P211 compatibility preview and exact consumer handoff;
- one current-main reconciliation;
- blocking integration repairs only;
- existing Service Model and CLI architecture contracts;
- one final local validation batch;
- pull-request publication, protected integration, canonical-main readback,
  issue disposition, and concise closeout documentation.

Deferred unless separately admitted:

- the remaining 155 direct runtime-owner expressions;
- full `ServiceState` field privacy;
- bulk fixture conversion;
- deletion of compatibility surfaces that do not duplicate the canonical
  aggregate or model decisions;
- additional crate extraction, API redesign, performance optimization, or
  generalized projection work.
- P211 shutdown, cold-install, presentation-requalification, profile-release,
  trusted-single-user acquisition, remote-view, or documentation
  implementation.

## P211 Dependent-Consumer Sequencing

P216 lands before P211 begins its deferred Service State, installer,
profile-acquisition, presentation-inventory, or remote-view join work. P211 may
retain and validate its already published lane-owned modules, but it must not
create a second Service State model seam or continue into overlapping adapters
until the P216 merge commit is available on `origin/main`.

The P216 candidate handoff to P211 must identify:

- the exact integrated Service Model commit and canonical crate paths;
- the aggregate, codec, transition, and projection APIs relevant to shutdown,
  ownership release, current-boot requalification, and remote-view admission;
- any required CLI adapter that intentionally remains outside the pure model;
- the three presently overlapping files and the expected semantic resolution
  for `control_plane.rs`; and
- any demonstrated missing API that would otherwise force P211 to recreate a
  model decision in the CLI.

The expected `control_plane.rs` reconciliation preserves both outcomes: P211's
terminal explicit-shutdown behavior and P205/P216's canonical runtime-owner
access and fixture conventions. `cli/src/main.rs` module registration and the
active-lane catalog are coordination merges, not reasons to combine the lanes.

P216 may repair a demonstrated issue #178 blocker inside its existing two-
commit allowance. A model API needed only for unfinished P211 behavior remains
P211 work after reconciliation unless the acceptance map proves the omission
also violates issue #178. P216 completion does not wait for P211 completion,
and P211 continues in its existing admitted worktree after reconciling the
integrated P216 boundary.

## Gate 1 Acceptance Map And P211 Consumer Handoff

Gate 1 was evaluated against issue #178, current source at `909a7c1`, the
published Plan 0205 evidence through `1ff20161`, and the current P211 checkpoint
`1d325fa0`.

| Issue #178 requirement | Exact evidence | Gate 1 disposition |
| --- | --- | --- |
| Canonical Service State records, pure derivations, wire invariants, and deterministic behavior live behind one coherent crate interface | `crates/agent-browser-service-model/src/service_state.rs` owns the sole aggregate, compatibility decoder, persistence preparation, deterministic encoder, transitions, and projections; the crate contains the canonical record modules exported by `src/lib.rs` | proved in source; final focused qualification pending |
| CLI callers consume the extracted module without a second model or duplicated decisions | `cli/src/native/service_model.rs` re-exports the canonical aggregate; the architecture contract requires exactly one `ServiceState` definition and inherent implementation and rejects the classified duplicated record and decision families | proved in source; final architecture rerun pending |
| Architecture guard rejects upward imports into CLI, runtime, browser, HTTP, MCP, filesystem, or platform providers | `scripts/dev/check-service-model-architecture.js` enforces the dependency allowlist, forbidden import paths, adapter modules, canonical aggregate, and caller cutovers; `scripts/dev/test-service-model-architecture.js` mutation fixtures currently pass | proved at Gate 1; final candidate rerun pending |
| Focused crate and affected CLI adapter tests pass locally | Plan 0205 Checkpoint 69 records four model witnesses, eight CLI witnesses, formatting, strict Clippy, architecture, mutation, diff-hygiene, and selector passes at `1ff20161` | valid historical evidence for unchanged source; one final branch qualification pending |
| Focused edit and test loop is measured against a baseline, with no unsupported acceleration claim | Plan 0205 records a 171.01-second cold CLI baseline, 4.08-second focused crate loop, 132-second affected CLI loop, and 0.44-second warm compatibility loop | measurement proved; claim limited to the cold pure-model feedback loop, not general workspace or CLI build time |
| Source enters `main` through the protected pull-request workflow | branch is published at `909a7c1`, 79 commits ahead and zero behind `origin/main@bea09376`; issue #178 is open and no pull request exists | incomplete; Gates 3 and 4 own qualification and integration |

No issue #178 source blocker was found. Full field privacy, the remaining 155
classified direct expressions, fixture migration, and optional facade deletion
remain outside the issue and P216.

The P211 consumer handoff is:

- Canonical aggregate and codec:
  `agent_browser_service_model::ServiceState`,
  `decode_persisted_service_state_json`,
  `prepare_service_state_for_persistence`, and
  `encode_prepared_service_state_pretty`.
- Durable mutation custody remains in the CLI adapter
  `ServiceStateRepository::mutate`; P211 must prepare external observations and
  effects outside that closure.
- Runtime ownership reads use `runtime_lifecycle_authority_summary`,
  `runtime_lifecycle_boot_epoch_observations`, `profile_runtime_authority`,
  `runtime_owner_binding_for_session`, `runtime_control_plane_authority`,
  `runtime_lane_authority`, and `runtime_resource_lanes`.
- Runtime ownership changes use
  `apply_runtime_lifecycle_transition_atomically`,
  `apply_runtime_lifecycle_transition_with_profile_sync_atomically`, or the
  existing typed lease claim operations. Runtime persistence uses
  `runtime_owner_persistence_parts`, `restore_runtime_owner_persistence`, and
  `strip_runtime_lifecycle_for_persistence`.
- Presentation requalification supplies observation in P211's adapter, then
  uses `PresentationCapacityAuthority::from_revalidated_inventory` and
  `reconcile_authoritative_bindings`; route and display compatibility uses
  `route_pool_target_string` and `route_pool_entry_matches_display`.
- Process discovery, browser closure, filesystem persistence, current-boot
  observation, route probing, and remote-view effects remain CLI adapter work.
- The current model has no bulk `shutdown` or `release_all` transition. If P211
  proves that one atomic model decision is required, it must add that bounded
  interface after reconciling onto integrated `main`; it must not recreate the
  decision through direct aggregate-field mutation.
- The known `cli/src/native/control_plane.rs` reconciliation keeps P211's
  `CloseBrowser` result for explicit shutdown with stale authority and keeps
  P205/P216's canonical runtime-owner API and test-fixture conventions.

Gate 2 fetched `origin/main`, the P216 branch, and the P211 branch once. Remote
`main` remains the admission commit `bea09376`, so no merge or rebase is
required in P216. The P211 preview remains read-only and is not integrated into
this branch.

## Candidate Freeze

The exact P216 integration candidate is
`6ff7bc0de0a10fab0afbb1bbf56c591a98ea118c`. It contains the accepted source
checkpoint `1ff20161` plus the P205 closeout, P216 admission, issue-acceptance
map, and P211 sequencing record. Gate 1 and Gate 2 found no source repair to
make, so the qualified Rust and JavaScript source tree is identical to
`1ff20161`.

`pnpm validation:select -- --base refs/remotes/origin/main` classified the
117-file inherited slice as broad. The final local qualification matched the
actual Service Model, CLI adapter, Lease Authority, workstation routing, and
service-contract surfaces:

- `pnpm version:sync`, `git diff --check`, the Service Model architecture
  guard and mutation fixtures, and the Lease Authority architecture guard
  passed. Documentation links and policy wiring also passed after this
  receipt was recorded.
- Workspace formatting and strict workspace Clippy passed through
  `scripts/ci/cargo-safe.sh`.
- The Service Model crate passed 234 tests. The focused CLI `service_model`
  filter passed 41 tests, and all six exact CLI adapter witnesses named by the
  selector passed.
- The Lease Authority crate passed 116 tests.
- Route-confusion, workstation-install fixture, workstation host-provision,
  fresh-workstation VM harness, workstation Guacamole asset, Guacamole
  PostgreSQL durability, and route-specific RDP user-sync checks passed.
- Service API and MCP parity, service-client contract and type checks,
  no-launch service collections, and the focused `workstation_install`,
  `workstation_payload_status`, `service_access_plan`, and `service_health`
  Rust filters passed. The original sequential filter runner lost its output
  handle after reaching the final filter; the terminal `service_health`
  filter was rerun once for an exact receipt and passed all 99 selected tests.
- A final selector read against `HEAD` reported no uncommitted changed files
  and only diff hygiene.

The CDP and desktop-services crate suites were not repeated because their
source is unchanged in this slice; their workspace membership is covered by
strict workspace Clippy and the source-identical historical evidence. Browser
E2E, provider, GitHub CI, release-build, installed-runtime, development-runtime,
staging, production, and release checks remain excluded by plan. No browser,
profile, provider, credential, install, runtime, staging, production, or
release effect occurred.

The build-time claim remains deliberately narrow: the recorded cold pure-model
feedback loop improved from 171.01 seconds to 4.08 seconds. The affected cold
CLI loop remained 132 seconds and the warm compatibility loop was 0.44 seconds,
so P216 does not claim a general workspace or CLI build-time reduction.

## Delivery Sequence And Budget

### Gate 1 | Acceptance Map

Inspect issue #178, the Plan 0205 evidence table, the branch diff, crate
ownership, compatibility facade, and architecture guard. Produce one compact
requirement-to-evidence table. Classify each requirement as proved, blocking,
or explicitly outside the issue.

Add the bounded P211 consumer map from the current `1d325fa0` checkpoint. It is
a compatibility and sequencing check, not an expansion of issue #178. Record a
P216 blocker only if the current extracted boundary would force P211 to
duplicate an already canonical model decision; do not pre-design unfinished
P211 behavior.

Exit: every issue acceptance statement has exact source or validation evidence,
any blocking defect has a bounded reproducer, and P211 has an exact consumer
handoff against the candidate boundary.

### Gate 2 | Reconciliation And Blocking Repair

Fetch `origin/main` once and reconcile it into the inherited branch if needed.
Repair only blockers from Gate 1 or actual merge conflicts. Preserve unrelated
P211 and P215 work and do not rewrite their roadmap, runbook, catalog, or source
sections independently.

Do not merge or cherry-pick P211 into P216. Retain the read-only cross-lane
merge preview as coordination evidence. P211 reconciles after P216 enters
canonical `main`; the known stale-owner shutdown-test conflict is resolved in
P211, where its product semantics are owned.

Exit: the branch contains current main, has no unresolved conflicts, and has no
known issue-acceptance blocker.

### Gate 3 | One Local Qualification Batch

Run the changed-surface selector against the reconciled base, then execute the
smallest complete local set it requires for the actual touched source. At
minimum retain diff hygiene, Service Model architecture guard plus mutation
fixtures, focused Service Model tests, affected CLI adapter tests, formatting,
and strict workspace Clippy. Do not run GitHub CI, browser E2E, provider tests,
release builds, runtime smokes, or production checks.

Exit: every required local gate passes at one exact candidate commit, with
scope and exclusions recorded once.

### Gate 4 | Integration

Open one pull request from the inherited branch to `main`. Review the issue
acceptance map and exact candidate, not the residual privacy count. Apply at
most one bounded repair cycle for accepted blocking findings. Merge through the
protected workflow without restoring or dispatching GitHub CI, then verify the
merge commit contains the candidate and close issue #178 if satisfied.

Exit: canonical `main` contains the extraction, the issue and plan states match
the integrated outcome, and no runtime or release effect occurred.

### Bounds

- Overall active-time ceiling: 180 minutes.
- Maximum source repair commits after admission: 2.
- Maximum broad review passes: 1.
- Maximum rework cycles: 1.
- Maximum plan checkpoints: 2, candidate freeze and integrated closeout.
- Single-agent execution by default. Do not create parallel audit workers.
- One helper is allowed only for a concrete, disjoint mechanical task that is
  already defined by a reproduced blocker.
- Run formatting and strict Clippy once at the completed candidate boundary,
  not after each small edit.
- Stop and report if landing requires new product semantics, runtime effects,
  CI restoration, a third repair commit, or expansion into residual privacy
  debt.

## Worker Assignments

The fresh primary agent owns the acceptance map, branch reconciliation,
blocking repairs, validation interpretation, Git, forge operations, and final
claim. Use the ordinary workhorse model for document reconciliation, exact
mechanical conflict resolution, and deterministic local checks. Escalate to the
strongest available model only if a reproduced semantic blocker crosses the
Service Model and Lease Authority boundary.

Do not use subagents for duplicate inventory, alternative-plan generation,
status polling, CI watching, or independent re-reading of the same source. If
one disjoint helper is justified, record its exact files, evidence, and stop
condition before assignment. The primary must inspect its diff.

## Evidence And Exit

| Requirement | Evidence | Admission state |
| --- | --- | --- |
| Canonical provider-free model crate | workspace manifests, crate source, architecture guard | verified at candidate `6ff7bc0d` |
| One canonical aggregate and codec | Service Model aggregate and persistence contracts | verified by the acceptance map and final local qualification |
| Deep transitions and projections | crate APIs, provider-free tests, CLI callers | verified by 234 crate tests, 41 focused CLI tests, and six exact adapter witnesses |
| CLI remains the effect adapter | forbidden-import guard and current adapter boundaries | architecture and mutation guards pass at `6ff7bc0d` |
| Focused correctness | retained crate and affected CLI tests | final local qualification passes at `6ff7bc0d` |
| Build acceleration evidence | Plan 0205 baseline and focused-loop measurements | recorded; no new benchmark authorized |
| P211 dependent-consumer readiness | current P211 branch, read-only merge preview, exact candidate handoff | three overlaps, one known test conflict, and the post-merge reconciliation contract are recorded against `6ff7bc0d` |
| Canonical integration | protected PR merge and `origin/main` readback | incomplete |
| GitHub CI | none | explicitly excluded by operator direction |
| Runtime or production effects | none | explicitly excluded |

## Non-Goals

- No expression-count burn-down.
- No new freeze and acceptance pair for each caller cutover.
- No new generalized registry getter, iterator, callback, or mutable escape.
- No comprehensive suite, GitHub CI, browser launch, provider call, credential
  use, install, staging, production, or release action.
- No new worktree unless the existing P205 worktree becomes unavailable and
  policy 0052 admission is re-established.
- No independent rewrite of P204, P211, or P215 source or shared planning
  sections.
- No absorption of P211's urgent shutdown, cold-install, or remote-view fixes
  into issue #178 or the P216 pull request.

## Stop Condition

Stop with a clean published checkpoint if the acceptance map proves a material
issue #178 requirement is absent and cannot be repaired inside two bounded
commits, if current-main reconciliation exposes a real semantic conflict with
P211 or P215, if the extracted boundary would force P211 to duplicate a
canonical model decision and the missing issue #178 interface cannot be
repaired within the existing allowance, or if protected integration requires
restoring operator-disabled CI. A mechanical P211 post-merge reconciliation is
not a P216 blocker. Do not convert any of those conditions into another
open-ended extraction campaign.
