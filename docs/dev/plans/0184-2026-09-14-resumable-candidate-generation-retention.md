# Plan 0184 | Resumable Candidate Generation Retention

Date: 2026-09-14

State: OPEN

Lane: P184

Product lane: PL-BUGFIX

Branch: `fix/plan-0184-resumable-candidate-retention`

Target: `main`

Integration: merge

Work item: [issue #108](https://github.com/CochranResearchGroup/agent-browser/issues/108)

Consolidation: required

## Objective

Prevent unattended generation GC from deleting the exact candidate required by
the newest resumable census-blocked workstation transaction, while preserving
the existing bounded cleanup of superseded transaction history.

## Current State

P183 merged through PR #107 as `12848e34`. Its exact integrated candidate,
SHA-256 `e4beadf07f664f240131a64977a4af113a14b482dcdf61887a937e6ff6f806ea`,
passed migration preview. One preserving apply staged generation
`0.28.0-e4beadf07f66-0c92aecf29d1` and then stopped before selector or payload
mutation because three prior-boot browser rows lacked current process evidence.

Exact admitted-host Service reconciliation proved all three recorded PIDs
absent, removed the stale browser and tab projections, and preserved registered
owner records. The installed interlock had already run GC at 12:58:30Z and
deleted the candidate generation. Transaction
`upgrade-310bb0f1-e659-40c6-aab5-3062ad9c4489` remains at revision 3 in
`blocked_ambiguous_runtime` and advertises `resume`, but exact resume fails
before effect because the candidate directory is absent.

## Consolidated Batch

- Retain the newest transaction's old and candidate generations when its state
  is `BlockedAmbiguousRuntime`.
- Keep older census-blocked transaction files as metadata without multiplying
  retained generations.
- Prove both the pure retention decision and the workstation GC reference
  projection.
- Integrate one source packet, build one exact integrated candidate, inspect a
  fresh migration and runtime census, and perform one changed-source preserving
  apply. Do not reuse the irrecoverable candidate or fabricate its directory.

## Scope

- `cli/src/runtime_retention.rs` generation retention policy and tests.
- `cli/src/workstation_install.rs` GC reference regression only.
- Canonical P183 and P184 roadmap, runbook, and active-lane projections.
- One issue-linked PR and one integrated installed acceptance attempt.

## Non-Goals

- No retention of every historical blocked candidate.
- No bypass of immutable-generation hashes, transaction revision checks,
  runtime census, Service State migration, or supervisor takeover.
- No browser launch, authentication retry, credential change, tenant effect,
  profile deletion, broad process cleanup, or formal release.

## Test Plan

The red fixture must show that current `main` omits the latest blocked
candidate reference. The green fixture must retain that candidate and selected
rollback source. The existing 49-transaction fixture must continue converging
historical generations to the selected and previous healthy pair.

Run the focused retention module, workstation GC fixture, Rust format, workspace
clippy with warnings denied, patch hygiene, and validation selection from
`12848e34`. Use one lazy CI readback after publication.

## Source Validation Evidence

- The new retention regression failed on unmodified `main`, then passed with
  the repair alongside all nine retention module tests.
- The workstation GC reference fixture passed and proves both the candidate and
  selected rollback-source reasons.
- The full focused `workstation_install` Rust suite passed: 161 tests, zero
  failures.
- Rust format, workspace clippy with warnings denied, patch hygiene, and all six
  validation-selector JavaScript fixtures passed.

## Delivery Sequence And Budget

1. Freeze the interlock journal, transaction, generation inventory, and
   post-reboot Service reconciliation evidence, 15 minutes.
2. Add the red regression and implement newest-resumable retention, 30 minutes.
3. Run focused and selected validation once on a frozen source head, 45 minutes.
4. Commit, publish, review, and merge one issue-linked PR, excluding CI wait.
5. Build from the exact merge, run a fresh no-effect preview, then use at most
   one changed-source preserving apply and verify installed/runtime coherence.

Stop on a new protected-record removal, identity ambiguity, unowned live
browser, admission drain mismatch, candidate hash mismatch, or uncertain effect.

## Worker Assignments

The primary owns diagnosis, both Rust files, validation, integration, and the
runtime gate. No subagent is assigned. P181 must reconcile only if it changes
the same retention or workstation installer surfaces before P184 integrates.

## Evidence And Exit

Source exit requires the red-green regression, historical anti-bloat fixture,
format, clippy, selected validation, and merged issue-linked PR. Installed exit
requires an exact integrated candidate, reviewed no-effect preview, one accepted
transaction, matching installed SHA, one selected supervised runtime host,
coherent stream publication, no admission drain, and unchanged Last30Days
principal, capability, owner identity, and owner generation.
