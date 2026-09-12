# Plan 0171 | Service Job Timestamp Integration And Install

State: OPEN
Lane: P171
Roadmap: P171
Branch: maintenance/plan-0170-job-timestamp-ordering
Target: main
Integration: merge
Date: 2026-09-12
Plan version: 2

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
- repair the comprehensive CI runner's shared-target race after the first PR
  run proved that its two parallel lanes can replace a still-running test
  executable.

## Non-Goals

- no browser launch, navigation, profile mutation, provider call, route switch,
  cleanup, historical timestamp rewrite, upstream pull request, or formal
  release;
- no edits to the dirty main or Turnstile worktrees.

## Version 2 Change Note

Pull request 43's first comprehensive Rust run failed in both parallel lanes
because they shared one Cargo target directory. While one lane executed
`agent_browser-*`, the other rebuilt and unlinked that exact executable; the
failures consistently reported `(deleted)` or `No such file or directory`.
This is a deterministic CI isolation defect also present on current main, not a
timestamp assertion failure. Version 2 spends the plan's one source repair pass
on separate per-lane Cargo target directories and adds a static contract test.

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

## Checkpoint P0171-C01 | 2026-09-12

State transition: `active -> integration_ready`.

Progress classification: `blocker_reduction`; current fork repairs merged
cleanly, three focused tests plus formatting and clippy pass, and the optimized
candidate built successfully.

Evidence: candidate source commit `fbeca748`; executable SHA-256
`d49bfb95425af4498acb75b7516ad224112e923237c78045ecc7cdb0e9507146`;
44,141,200 bytes. The 12-minute build remained inside the governed one-job
Cargo cgroup.

Acceptance state: criteria 1-2 pass. Fork integration and transactional
installed verification remain.

## Checkpoint P0171-C02 | 2026-09-12

State transition: `integration_ready -> repair_active`.

Progress classification: `blocker_reduction`; PR 43's first run passed Version
Sync, Rust Quality, Dashboard, Service Client, and all Workstation Fixtures.
The broad Rust job failed only because concurrent comprehensive lanes shared a
Cargo target directory: eleven tests could not reopen or spawn the executable
after the peer lane replaced it. The timestamp-specific tests remained green.

Repair: isolate the native and support lanes under `comprehensive-native` and
`comprehensive-support`, then rerun the static runner contract and PR gates.
