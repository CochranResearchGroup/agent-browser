# Plan 0170 | Service Job Lifecycle Timestamp Ordering

State: CLOSED
Date: 2026-09-12
Branch: `maintenance/plan-0170-job-timestamp-ordering`
Target: `main`

## Objective

Guarantee nondecreasing public Service Job lifecycle timestamps when the host
wall clock moves backward during one request, without changing monotonic
deadline enforcement or service effects.

## Current State

- Last30days Plan 0068 observed a successful `tab_handle_release` job whose
  `startedAt` preceded `submittedAt` by approximately 1.881 seconds.
- `ControlRequest` already records a monotonic submission instant for elapsed
  time and deadlines.
- Public `submittedAt`, `startedAt`, and `completedAt` are sampled separately
  from the adjustable wall clock and are persisted without ordering guards.
- The governing `control_plane.rs` blob is identical at the observed installed
  source revision, the indexed checkout, and current `origin/main`.

## Scope

- add a provider-free deterministic regression for backward wall-clock input;
- clamp display/audit lifecycle timestamps to the preceding lifecycle boundary;
- preserve monotonic deadline and timeout behavior unchanged;
- run focused control-plane tests plus required Rust format and clippy gates;
- record source qualification separately from any installed-runtime rollout.

## Non-Goals

- no production install, service restart, browser action, provider call,
  profile mutation, timeout change, or Last30days refresh;
- no rewriting of historical Service State timestamps;
- no change to event ordering beyond the affected job lifecycle timestamps.

## Acceptance Criteria

1. A backward candidate wall timestamp resolves to the prior lifecycle
   boundary.
2. A later candidate timestamp remains unchanged.
3. Running jobs persist `startedAt >= submittedAt`.
4. Terminal jobs persist `completedAt >= startedAt >= submittedAt`.
5. Focused tests, Rust formatting, and clippy pass without retry.

## Execution Packet

- owner: primary agent;
- worktree: isolated branch from current `origin/main`;
- write surface: control-plane implementation/tests, plan, active-lane catalog,
  runbook, and closeout note;
- validation tier: focused Rust plus required Rust quality gates;
- terminal condition: source-qualified repair or one exact blocker;
- overall effort ceiling: one implementation attempt and one repair pass;
- authority classification: `inherited_authority`; the operator requested
  diagnosis, repair, and testing of the observed timeout-related defects.

Subagent status: `not_spawned`; current orchestration policy prohibits
delegation.

## Checkpoint P0170-C01 | 2026-09-12

State transition: `diagnosed -> source_qualified`.

Progress classification: `outcome_progress`; public job lifecycle timestamps
are nondecreasing across backward wall-clock adjustments while monotonic
deadlines remain unchanged.

Evidence:

- deterministic regression first failed to compile because the ordering helper
  did not exist, then passed after the repair;
- the helper preserves a later RFC 3339 candidate and clamps an earlier
  candidate to its preceding lifecycle boundary;
- running persistence uses submission as its floor; terminal persistence uses
  the recorded start, or submission when no start exists;
- focused ordering, normal submit, and timed-out job tests pass;
- Rust formatting and workspace clippy with warnings denied pass;
- the first test launch was infrastructure-failed before product compilation:
  the Cargo slice had 947 of 1,024 tasks and `sccache`/`rustc` could not spawn
  threads despite 39 GiB available memory. The bounded one-job, cache-disabled
  run produced the real red/green evidence.

Acceptance reconciliation: criteria 1-5 are satisfied for source
qualification. Production build, installation, restart, and live browser
execution were not performed.

Graphiti write status: `graphiti_write_pending`. Job
`084f242a-977a-4017-a113-6f9a5e62b1a3` failed once with a retryable
`TimeoutError` before creating an episode. Preserve the failed job and retry
the intended Plan 0170 source-qualification summary at the next closeout;
do not enqueue a duplicate episode.

Next action: review and integrate this isolated branch. Qualify the timestamp
ordering in an installed candidate only as part of the next consolidated
Agent Browser build rather than spending a standalone production replacement.
