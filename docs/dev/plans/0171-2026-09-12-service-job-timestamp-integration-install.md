# Plan 0171 | Service Job Timestamp Integration And Install

State: CLOSED
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
The next run proved the Rust repair, then exposed a separate frozen-fixture
omission: four already-registered profile repair/reset MCP tools were absent
from the no-launch allowlist. Synchronizing that allowlist is fixture
maintenance, not an additional runtime behavior change.

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

## Checkpoint P0171-C03 | 2026-09-12

State transition: `repair_active -> validation_active`.

Progress classification: `blocker_reduction`; the repaired comprehensive Rust
test step passed in PR run `34706467255`, proving the shared-target race closed.
Its subsequent no-launch smoke found the frozen MCP allowlist omitted the
already-registered `service_profile_repair_*` and `service_profile_reset_*`
contracts and the already-registered profile diagnosis resource template. Add
those frozen inventory entries in their actual order and rerun the smoke plus
PR gates.

## Checkpoint P0171-C04 | 2026-09-12

State transition: `validation_active -> closed`.

Progress classification: `verified_outcome`; PR 43 merged to `origin/main` as
`40da27f75f7ff1116d7e92e463182c8ab644783b`. Final CI run `34707619741`
passed Version Sync, Rust Quality, Dashboard, Service Client, Workstation
Fixtures, the isolated comprehensive Rust suite, and no-launch service smokes.

Exact candidate: the integrated tree built under the governed Cargo cgroup
with cache disabled in 4m25s. Its 43,910,400-byte executable has SHA-256
`2c185ec7ccd691deaf0ee59412d3d31485ab0d8ad464cc02775785fb81be621d`.
The initial cache-enabled attempt failed before product compilation because
the sccache compiler-probe diagnostic included its inherited environment; its
credential sanitizer is blacklist-based and did not suppress every sensitive
variable shape. The output is intentionally not reproduced. Cache-off is the
accepted build fallback; hardening the sanitizer remains a separate security
follow-up.

Installed acceptance: guarded transaction
`upgrade-1fcb7d57-671f-4cf9-afc6-1bcb45293e1a` accepted revision 13 and selected
generation `0.28.0-2c185ec7ccd6-f318ad66074f`. The installed executable hash
matches the candidate exactly. Admission drain is off; runtime multiplicity is
`steady_current` with one executable generation, one runtime host, one
dashboard process, zero legacy daemons, and no multiplicity issues. Resources
report zero candidates and zero unknown or transferring cleanup obligations.

Acceptance state: criteria 1-5 pass. Dashboard operator-journey convergence and
one default-profile lease warning remain nonblocking; no browser launch,
navigation, profile mutation, provider call, route switch, or cleanup occurred.
