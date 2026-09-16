# Plan 0200 | Cargo Scope Descendant Lifetime

Date: 2026-09-16

Plan version: 2

State: OPEN

Consolidation: required

Product lane: PL-PLATFORM

Lane: P200

Work item: `CochranResearchGroup/agent-browser#102`

Branch: `platform/p200-cargo-scope-descendant-lifetime`

Target: `main`

Integration: merge through the protected `main` workflow after provider-free regression and changed-surface validation

Source baseline: `3863106e7ad482b318b8406e4a9f12dbc1c97aff`

## Objective

Keep Cargo admission accountable for every process in its exact user-systemd
scope after the Cargo or wrapper parent exits. Make test teardown converge on
an inactive scope and an unused temporary profile without terminating foreign
or retained browser processes.

## Current State

Issue #102 is claimed and in progress. Source checkpoint `fc2a3359` gives each
admitted invocation an exact scope identity, retains a dead-wrapper claim while
that scope remains active or cannot be observed safely, and stops only that
scope before releasing the claim. Success and nonzero Cargo exits use the same
bounded cleanup path.

The original provider-free fixture failed in 2.6 seconds because the success
path returned while its browser-like descendant remained alive. The repaired
fixture passes success, exit-23 failure, active orphan-scope accounting,
unavailable-systemd, temporary-profile residue, and foreign-process controls.
Opt-in success and exit-23 runs against disposable real user-systemd scopes
also pass and leave no P200 unit or process residue.

P190 is an adjacent active `PL-PLATFORM` lane. P200 is the primary writer for
`scripts/ci/cargo-safe.sh` and new descendant-lifetime fixtures. P190 owns its
candidate orchestration and current `scripts/ci/rust-tests.sh` changes. P200
will not edit P190's dirty files and will publish a checkpoint before any
dependent reconciliation.

## Consolidated Batch

1. Add one fast deterministic provider-free fixture that leaves a child,
   grandchild, or double-forked browser-like process in the exact admitted
   scope after its parent exits. Prove current behavior releases accountability
   while the descendant remains.
2. Minimize the failure and discriminate whether claim cleanup, systemd scope
   completion, or runner teardown owns the defect.
3. Keep the claim alive until the exact scope is empty, or conservatively keep
   the scope's residual resources in admission accounting with typed pressure
   recourse.
4. Add bounded exact-scope teardown for the fixture and runner paths needed by
   this repair. Preserve foreign, retained, and unrelated processes.
5. Prove success and failure paths leave the scope inactive and no process
   using the temporary profile. Run focused checks, then one complete
   changed-surface validation pass against the frozen batch.

## Scope And Effect Boundary

Expected write surfaces are `scripts/ci/cargo-safe.sh`, directly supporting
Cargo admission helpers, provider-free process fixtures, and the minimum CI or
developer documentation required for the changed behavior. This plan and its
branch-local roadmap, runbook, and lane projections are in scope.

This plan does not authorize production or development installation, Chrome
launch against a retained profile, Service State mutation, production browser
GC, retained-profile policy changes, foreign-process cleanup, supervisor
mutation, or release. Teardown may affect only the disposable exact test scope
and profile created by the P200 fixture.

## Delivery Sequence And Budget

- Attempt 1: red-capable exact-scope descendant fixture and minimized evidence.
- Attempt 2: one lifetime-accounting and bounded-teardown repair with focused
  regression checks.
- Attempt 3: one bounded correction if changed-surface validation exposes a
  defect introduced by this packet.
- Maximum work-unit attempts: 3.
- Maximum review and rework cycles: 1.
- Reassess after two checkpoints or 30 active minutes without outcome progress.
- Overall effort ceiling: 180 active minutes through protected integration.

No subagent, browser worker, runtime operator, benchmark checkout, fixture
worktree, provider operator, production install, or shared-runtime mutation is
assigned.

## Worker Assignments

The P200 lane owner holds the critical path, plan, branch, fixture, source
repair, validation, integration, and closeout. P190 remains the primary writer
for its candidate orchestration and current test-runner edits. P200 owns
`scripts/ci/cargo-safe.sh` and its new exact-scope fixture; no parallel P200
writer is assigned.

## Evidence And Exit

Exit requires current evidence that:

- one deterministic unattended command exercises the real wrapper and exact
  user-systemd scope, fails for the original lingering-descendant behavior,
  and passes after repair;
- admission accountability remains until the exact scope is empty, or residual
  scope resources participate conservatively in admission with typed recourse;
- success and failure paths leave the exact fixture scope inactive;
- no process retains the fixture's temporary profile path;
- teardown is sealed to the exact test-owned scope and profile and cannot
  terminate foreign or retained processes;
- ordinary successful and failed test paths need no manual `systemctl` cleanup;
- focused regression checks and every gate selected from the complete changed
  surface pass; and
- the published checkpoint enters `main` through the linked pull request with
  applicable exact-head CI green.

| Requirement | Current evidence | State |
| --- | --- | --- |
| Original descendant leak is red-capable | Pre-repair `node scripts/test-cargo-safe-capacity.js` failed with `success path left its browser-like descendant alive` | proven |
| Claim covers scope descendants | The deterministic stop adapter observes exactly one claim during exact-unit stop; a dead-owner active-scope claim blocks a new one-slot admission with `reason=concurrency_limit` | focused pass |
| Success and failure teardown converge | Exit 0 and exit 23 fixtures stop the exact generated unit and leave zero profile-path processes | focused pass |
| Foreign and retained processes remain ineligible | The unrelated control process remains alive across both teardown paths; no retained or installed profile is used | focused pass |
| Real scope becomes inactive | `AGENT_BROWSER_CARGO_REAL_SCOPE_TEST=1 node scripts/test-cargo-safe-capacity.js` passes success and exit-23 cases; fresh unit/process readback is empty | focused pass |
| Required batch validation and integration | At `fc2a3359`, Shellcheck, Node syntax, deterministic fixtures, real-scope fixtures, planning audit, diff hygiene, and the changed-surface selector pass. Protected exact-head CI, merge, and closeout remain | local pass |

The aggregate `pnpm test:wsl-cargo-safety` command reaches a pre-existing
static-entrypoint failure at
`scripts/test-lease-authority-crate-architecture.js:46:raw_compiling_cargo`.
The same command fails identically on canonical `main@3863106e`; that control
failure is not counted as P200 validation. The P200 capacity suite itself
passes independently.

## Stop Condition

Stop before broad process cleanup, production or development runtime mutation,
Service State changes, retained-profile changes, or any cleanup whose exact
scope ownership cannot be proven. If the repair requires editing a P190 dirty
surface, publish the P200 dependency and reconcile ownership before continuing.
Do not weaken host reserve, claim capacity, cgroup, profile, or foreign-process
interlocks to make the fixture pass.
