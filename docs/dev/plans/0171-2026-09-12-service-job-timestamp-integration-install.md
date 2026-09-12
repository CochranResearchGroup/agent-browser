# Plan 0171 | Service Job Timestamp Integration And Install

State: OPEN
Lane: P171
Roadmap: P171
Branch: maintenance/plan-0170-job-timestamp-ordering
Target: main
Integration: merge
Date: 2026-09-12

Current execution status: [RUNBOOK.md](../../../RUNBOOK.md)

## Objective

Reconcile the source-qualified lifecycle timestamp repair with current
`origin/main`, integrate it into the public fork, and install the exact
consolidated candidate without disturbing retained browsers or profiles.

## Current State

- Plan 0170 is source-qualified at `c075a632`.
- The branch is clean and three commits ahead but two behind `origin/main`;
  those target commits contain the retained-owner route repair now installed.
- Production generation `0.28.0-be907d46a2fb-53bd7b56e050` is accepted and
  steady, but does not have Git-bound proof of the Plan 0170 repair.

## Scope

- merge current `origin/main` into the clean repair lane;
- rerun focused timestamp, submit, timeout, formatting, and clippy gates;
- build one exact consolidated candidate;
- publish and integrate through the normal fork workflow;
- run transactional workstation dry-run/apply and verify exact installed
  executable, accepted transaction, units, multiplicity, and resources.

## Non-Goals

- no browser launch, navigation, profile mutation, provider call, route switch,
  cleanup, historical timestamp rewrite, upstream pull request, or formal
  release;
- no edits to the dirty main or Turnstile worktrees.

## Acceptance Criteria

1. The reconciled branch is clean and contains current `origin/main`.
2. Required focused tests, formatting, clippy, and candidate build pass.
3. The fork target contains the timestamp repair through its normal
   integration path.
4. A guarded workstation transaction accepts the exact built candidate.
5. Fresh doctor and resource reads prove one selected executable generation,
   no runtime-multiplicity issue, and zero readiness-impacting cleanup work.

## Execution Packet

- owner: primary agent;
- critical path: reconcile, validate, integrate, build, dry-run, apply,
  installed verification;
- write surfaces: this branch, fork refs/PR, production workstation selector,
  plan, active-lane catalog, runbook, and closeout receipt;
- bound: one reconciliation attempt, one source repair pass, one candidate
  build, one dry-run, and one apply;
- terminal condition: installed acceptance or one exact fail-closed blocker;
- authority classification: `inherited_authority`; the operator approved the
  recommended integration and installation path with “Ok go”.

Subagent status: `not_spawned`; current orchestration policy prohibits
delegation.

## Definition Of Done

All five acceptance criteria have current Git, build, transaction, and runtime
receipts, or the plan records the exact terminal blocker without widening
effects.
