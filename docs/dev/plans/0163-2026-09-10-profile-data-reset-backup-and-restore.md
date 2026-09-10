# Plan 0163: Profile-Data Reset, Backup, and Restore

Date: 2026-09-10

State: PLANNED

Consolidation: required

Lane: P157

Branch: plan/profile-permissions-and-request-provenance

Target: main

Integration: merge

Parent: [Plan 0161](0161-2026-09-09-first-class-profile-repair-and-reset-plan.md), W6

## Objective and authority

Deliver the destructive `profile_data` reset scope that Plan 0161 intentionally
leaves unavailable. One exact profile operation must create and verify a
restorable backup, reset only the selected physical profile, roll back a failed
apply, and restore the original profile under revision-fenced authority.

Planning, contracts, implementation, and disposable fixtures are authorized by
the standing Plan 0161 goal. This plan does not authorize use against a
production or tenant profile. Any live profile-data reset requires a separate,
action-specific operator instruction naming the profile and backup disposition.

## Current state

Plan 0161 exposes `profile_data` as a typed unavailable reset scope. Diagnose,
preserving repair, runtime reset, and authentication reset neither delete nor
replace browser data. No public operation currently claims backup, destructive
reset, rollback, or restore authority.

The successor starts only from the accepted Plan 0161 profile identity,
capability authentication, revision fencing, sealed-plan, receipt, trace, and
peer-preservation contracts. It must not reopen those contracts or weaken the
foreign-owner and ambiguous-process refusals.

## Consolidated batch

The batch contains four inseparable outcomes:

1. a content-addressed, permission-preserving backup with an exact source
   profile identity and manifest;
2. a sealed reset plan that refuses live, foreign, ambiguous, changed, or
   unbacked profile state before effects;
3. atomic reset with rollback when any postcondition fails; and
4. explicit restore with digest verification, revision fencing, idempotent
   replay, and exact peer preservation.

Credential-store deletion, provider actions, tenant workflows, broad runtime
cleanup, and automatic production application remain out of scope.

## Delivery sequence and budget

1. Freeze the backup manifest, physical-profile identity, exclusion rules,
   retained permissions, free-space check, and failure matrix. Stop if any
   profile content cannot be attributed to the selected physical identity.
2. Implement backup and restore at one repository transaction seam with
   disposable profiles. Stop after one architecture-level rollback failure and
   replan before another destructive implementation attempt.
3. Add the sealed reset planner and executor, then expose CLI, HTTP, MCP,
   generated-client, dashboard, schema, help, README, skill, and docs parity.
4. Run focused fixtures first. Run changed-surface qualification once after the
   candidate freezes. No live or production reset is part of this plan's
   default acceptance.

Source delivery is bounded to one coherent implementation batch and one
review/rework cycle. A second failed destructive acceptance cycle requires a
revised plan checkpoint before more effects.

## Worker assignments

The primary owns the destructive boundary, filesystem effects, transaction,
rollback, integration, and acceptance. Mechanical contract or dashboard work
may be delegated only after the schema freezes, with disjoint files and no
runtime authority. No worker may operate on a non-disposable profile.

## Validation

Disposable acceptance must prove successful backup and reset, byte- and
permission-correct restore, forced mid-apply failure rollback, insufficient
space refusal, symlink and path-escape refusal, live or ambiguous ownership
refusal, changed-revision refusal, replay without a second effect, and survival
of unrelated profiles and browser lanes. Receipts must contain digests and
bounded metadata, never cookies, credentials, or private page content.

## Evidence and exit

RUNBOOK.md remains the sole current execution table. Plan 0163 closes only when
the complete public surface is implemented and qualified on disposable data,
backup and rollback evidence is independently readable, documentation is
synchronized, and no production effect has occurred without separate explicit
authority. Until then, Plan 0161's `profile_data_unavailable` response remains
the correct public behavior.
