# Plan 0191 | Repository Worktree And Lane Reconciliation

Date: 2026-09-14

State: OPEN

Lane: P191

Product lane: PL-PLATFORM

Branch: `platform/p191-repository-lane-reconciliation`

Target: `main`

Integration: merge

Work item: [issue #139](https://github.com/CochranResearchGroup/agent-browser/issues/139)

Source baseline: `4000b1d8f50b104c334857d698e4f10b77ecf4af`

Consolidation: required

## Objective

Restore one truthful repository portfolio before substantive development:
preserve recoverable dirty auxiliary evidence, retire only proven-integrated or
disposable worktrees, align plan and issue completion state with accepted
evidence, and register the surviving challenge lane against its actual branch,
plan, work items, and published checkpoint.

## Current State

The starting inventory contained the canonical checkout plus six auxiliary
worktrees. Five auxiliaries were integrated or detached benchmark checkouts
with no unique commits. Three of those five retained uncommitted task artifacts:
eight `.codex/wake/` lifecycle files in the integrated P181 checkout and one
disposable benchmark-token insertion in each detached P181 comparison tree.
The sixth auxiliary is the clean, remotely published challenge worktree at
`fec7fd8729679c1649c838ef5d317ea86179323f`; it owns Plan 0187, issue #127,
leaf issue #66, and draft PR #128.

The active-lane catalog still described integrated P180 and P182 through P185
branches as active or open custody and pointed P169 at the superseded
`feature/turnstile-desktop-challenge` branch and Plan 0169. P186 later accepted
the cumulative preserving-install chain and final runtime coherence, providing
the missing installed evidence for P180 and P183 through P185. P182 source is
integrated through PR #98, but its exact preserved Authentication Run has not
received the separately gated same-run acceptance and therefore remains open
without an active source checkout.

## Consolidated Batch

1. Preserve the exact dirty P181 task artifacts outside the product repository
   with recoverable archives, standalone patches, and content digests.
2. Remove the exact five integrated or disposable auxiliary worktrees after
   ancestry, remote-ref, and archive checks. Retain all named branch refs and
   exclude the challenge worktree.
3. Close P180 and P183 through P185 from merged source plus P186 cumulative
   installed acceptance. Keep P182 open as an operational acceptance record,
   but retire its integrated source custody from the active catalog.
4. Replace the stale P169 projection with Plan 0187 on
   `challenge/p169-control-plane` at its equal local and remote checkpoint.
   Record draft PR #128's current main conflict and failed Rust gate without
   rebasing, merging, retrying, or changing challenge code.

## Scope

- Exact worktree inventory, ancestry, remote custody, and removal receipts.
- Local user-scoped archive metadata for dirty auxiliary artifacts.
- Plans 0180, 0182, 0183, 0184, and 0185 completion or custody corrections.
- `ROADMAP.md`, `RUNBOOK.md`, `docs/dev/active-lanes.yaml`, and the notes index.
- Issue #95 closure and issue #139 campaign tracking.

## Non-Goals

- No source repair, ref deletion, challenge-branch integration, rebase, merge,
  build, install, browser operation, authentication retry, provider mutation,
  credential access, tenant effect, runtime effect, or formal release.
- No closure of Plan 0182 or issue #96 without same-run acceptance evidence.
- No activation of planned P190 implementation.

## Delivery Sequence And Budget

1. **Evidence preservation, 15 minutes.** Inventory the dirty artifacts, avoid
   exposing prompt contents, create one local archive and two exact patches,
   then verify their digests and members.
2. **Exact worktree closure, 15 minutes.** Recheck each HEAD as an ancestor of
   current `origin/main`, verify named remote custody, remove only the five
   listed auxiliaries, and confirm the surviving inventory.
3. **Ledger reconciliation, 30 minutes.** Update the plan, roadmap, runbook,
   notes index, active-lane catalog, and issue states from current evidence.
4. **Validation and integration, 30 to 60 minutes excluding CI queue time.**
   Run deterministic planning and lane audits, patch hygiene, and selected
   documentation checks; publish one issue-linked pull request and use one lazy
   CI readback.

The campaign ceiling is two hours of active work. Stop if a target has a unique
commit, an unarchived dirty artifact, unequal named remote custody, a changed
challenge checkpoint, or an audit finding that requires source or runtime
work. Such a finding becomes a separately bounded item and does not widen this
maintenance batch.

## Worker Assignments

- **Primary and coordination owner:** the current P191 session owns inventory,
  archive verification, exact checkout removal, shared-ledger edits, issue
  mutations, validation, integration, and the final evidence claim.
- **Challenge lane owner:** remains the existing top-level session represented
  by Plan 0187 and branch `challenge/p169-control-plane`. P191 does not edit its
  worktree or branch.
- **Other workers:** none. The reconciliation is a serialized shared-authority
  transition and no delegated exploration or implementation is required.

## Evidence And Exit

| Requirement | Evidence | State |
| --- | --- | --- |
| Dirty P181 artifacts preserved | Local archive and exact patches with recorded SHA-256 digests | pending |
| Integrated and disposable auxiliaries retired | Final `git worktree list --porcelain` contains only canonical P191 and challenge P169 checkouts | pending |
| Completion ledgers truthful | P180 and P183 through P185 closed; P182 remains open without source custody | pending |
| Challenge lane registered | Catalog reads Plan 0187 and remote-equal checkpoint `fec7fd87` on `challenge/p169-control-plane` | pending |
| Deterministic validation | Planning, active-lane, documentation, and patch checks pass | pending |
| Shared integration | Issue-linked pull request merges to `main`; final remote readback matches | pending |

Completion requires every row to be complete. A preserved local archive is a
recovery receipt, not product source. A clean worktree inventory does not close
P182's live acceptance or make P169 merge-ready.
