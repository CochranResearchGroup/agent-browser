# Plan 0201 | X Display Live Occupancy

Date: 2026-09-16

Plan version: 2

State: OPEN

Consolidation: required

Product lane: PL-PLATFORM

Lane: P201

Work item: `CochranResearchGroup/agent-browser#159`

Branch: `platform/p201-x-display-live-occupancy`

Target: `main`

Integration: merge through the protected `main` workflow after provider-free regression and changed-surface validation

Source baseline: `b11a7227cdbe73f5f06e2a12907e1148383f35d3`

## Objective

Reserve an X display only when current evidence shows a live filesystem or
abstract Unix listener or a live matching X process. Reclaim exact stale
socket residue safely and report a bounded per-display classification when the
configured range is exhausted.

## Current State

Issue #159 is open and source-complete through draft PR #166 at checkpoint
`42beec54`. The pre-repair focused run failed because both an ordinary file and
a dropped listener's stale socket inode were classified `ActiveSocket`; the
live filesystem listener control passed. The repair now derives socket
occupancy from exact filesystem or abstract addresses in `/proc/net/unix`,
classifies path residue separately, and revalidates the live-listener census
before removing only the exact display socket and lock paths.

All 11 focused X-display tests pass, including ordinary-file residue, stale
socket inode, live filesystem and abstract listeners, reused live non-X PID,
unknown socket observation, and bounded classification diagnostics. Formatting,
strict workspace Clippy, patch hygiene, the planning audit, and the
selector-required no-launch route-confusion gates pass. Exact-head CI,
integration, and closeout remain.

P190 and P197 are adjacent active lanes but neither owns or edits
`cli/src/native/cdp/chrome.rs`. P201 is the sole writer for that source file and
its focused tests. The shared roadmap, runbook, and active-lane catalog are
coordination projections only.

## Consolidated Batch

1. Add deterministic provider-free fixtures for an ordinary pathname, a stale
   filesystem socket inode, and a live filesystem listener. Capture the
   original failure before source repair.
2. Replace pathname existence with exact filesystem socket and listener
   evidence while retaining abstract socket and live lock-process evidence.
3. Revalidate and remove only the exact stale display socket and lock residue
   needed to admit the selected display. Unknown observation remains
   fail-closed.
4. Add a bounded exhaustion summary that identifies each scanned display's
   classification without unbounded output.
5. Run focused regressions, formatting, strict workspace Clippy, and every
   additional check selected from the complete changed surface.

## Scope And Effect Boundary

Expected source writes are limited to `cli/src/native/cdp/chrome.rs` and its
inline tests. This plan and its branch-local roadmap, runbook, and lane
projections are also in scope. Minimum existing user documentation may be
updated only if the repaired diagnostic has a documented contract.

This plan does not authorize a browser or X server launch, production or
development installation, Service State mutation, provider mutation, retained
profile access, broad `/tmp` cleanup, process termination, release, or any
other live effect. Tests may create and remove only their own disposable
temporary directory entries and Unix listeners.

## Delivery Sequence And Budget

- Attempt 1: red ordinary-file and stale-socket-inode fixtures plus a live
  filesystem listener control.
- Attempt 2: one evidence-based classifier and exact-residue cleanup repair
  with focused regression checks.
- Attempt 3: one bounded correction if changed-surface validation exposes a
  defect introduced by this packet.
- Maximum work-unit attempts: 3.
- Maximum review and rework cycles: 1.
- Reassess after two checkpoints or 30 active minutes without outcome
  progress.
- Overall effort ceiling: 180 active minutes through protected integration.

No subagent, runtime operator, browser worker, fixture worktree, benchmark
checkout, production install, or shared-runtime mutation is assigned.

## Worker Assignments

The P201 lane owner holds the critical path, plan, branch, fixtures, source
repair, validation, integration, and closeout. P201 is the sole writer for
`cli/src/native/cdp/chrome.rs`. P190 and P197 retain their existing disjoint
source custody.

## Evidence And Exit

Exit requires current evidence that:

- an ordinary file at the display socket pathname does not reserve the display;
- a stale filesystem socket inode is reclaimed only after live-listener
  absence is proven;
- a live filesystem or abstract socket listener remains unavailable;
- a live matching X lock process remains unavailable and unknown observation
  remains fail-closed;
- exhaustion reports a bounded per-display classification summary;
- focused regressions, formatting, strict Clippy, and all selected
  changed-surface gates pass; and
- the exact source checkpoint enters `main` through the linked pull request
  with applicable exact-head CI green.

| Requirement | Current evidence | State |
| --- | --- | --- |
| Original pathname conflation is red-capable | Pre-repair focused run: ordinary-file and stale-socket-inode fixtures failed as `ActiveSocket`; 4 controls passed | proven |
| Live listeners remain reserved | Live filesystem and abstract listener fixtures pass | focused pass |
| Exact stale residue is reclaimable | Ordinary file and dropped-listener socket inode are removed with their exact stale locks and become allocatable | focused pass |
| Unknown observations fail closed | Injected permission-denied socket census remains `Unknown` and preserves both paths | focused pass |
| Reused live non-X PID is not active X evidence | Live shell PID fixture classifies `StaleLockReusedPid` and preserves the foreign process | focused pass |
| Bounded exhaustion summary | Diagnostic fixture reports exact per-display names and stays below 1 KiB | focused pass |
| Required validation and integration | 11 focused tests, format, strict Clippy, patch hygiene, planning audit, and no-launch route-confusion gates pass at `42beec54`; exact-head CI and integration remain | local validation passed |

## Stop Condition

Stop before deleting any path outside the exact scanned display socket and
lock names or when live-listener absence cannot be proven. Stop before browser,
X server, Service State, provider, installed-runtime, or retained-profile
effects. If implementation requires a source surface owned by P190 or P197,
publish the dependency and reconcile writer custody before editing it.
