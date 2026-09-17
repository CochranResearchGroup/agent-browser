# Plan 0216 | Service Model Extraction Landing

Date: 2026-09-17

Plan version: 1

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

## Consolidated Batch

1. Map each issue #178 acceptance statement to current source and durable local
   evidence. Record a blocker only when the current checkpoint contradicts the
   issue, not because optional privacy debt remains.
2. Reconcile current `origin/main` once. Preserve P211 and P215 source custody;
   resolve only actual integration conflicts and shared planning projections.
3. Repair at most two blocking defects demonstrated by the acceptance map or
   reconciliation. Do not continue opportunistic extraction, field privacy,
   facade deletion, test multiplication, or architecture-guard expansion.
4. Run one final local qualification batch matched to the complete branch diff.
   Reuse valid checkpoint evidence when its covered source is unchanged.
5. Publish one pull request, review only the bounded landing contract, merge it,
   verify canonical `main`, close issue #178 when its acceptance is satisfied,
   and close this plan.

## Scope

Included:

- issue #178 acceptance mapping;
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

## Delivery Sequence And Budget

### Gate 1 | Acceptance Map

Inspect issue #178, the Plan 0205 evidence table, the branch diff, crate
ownership, compatibility facade, and architecture guard. Produce one compact
requirement-to-evidence table. Classify each requirement as proved, blocking,
or explicitly outside the issue.

Exit: every issue acceptance statement has exact source or validation evidence,
and any blocking defect has a bounded reproducer.

### Gate 2 | Reconciliation And Blocking Repair

Fetch `origin/main` once and reconcile it into the inherited branch if needed.
Repair only blockers from Gate 1 or actual merge conflicts. Preserve unrelated
P211 and P215 work and do not rewrite their roadmap, runbook, catalog, or source
sections independently.

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
| Canonical provider-free model crate | workspace manifests, crate source, architecture guard | implemented at `1ff20161`; final branch verification pending |
| One canonical aggregate and codec | Service Model aggregate and persistence contracts | implemented; final acceptance map pending |
| Deep transitions and projections | crate APIs, provider-free tests, CLI callers | implemented; final acceptance map pending |
| CLI remains the effect adapter | forbidden-import guard and current adapter boundaries | locally proved at `1ff20161`; final candidate rerun pending |
| Focused correctness | retained crate and affected CLI tests | passed at `1ff20161`; reconciliation impact review pending |
| Build acceleration evidence | Plan 0205 baseline and focused-loop measurements | recorded; no new benchmark authorized |
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

## Stop Condition

Stop with a clean published checkpoint if the acceptance map proves a material
issue #178 requirement is absent and cannot be repaired inside two bounded
commits, if current-main reconciliation exposes a real semantic conflict with
P211 or P215, or if protected integration requires restoring operator-disabled
CI. Do not convert any of those conditions into another open-ended extraction
campaign.
