# Plan 0201 | X Display Live Occupancy

Date: 2026-09-16

Plan version: 1

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

Issue #159 is open and claimed by P201. The classifier currently treats any
pathname at `/tmp/.X11-unix/XN` as an active socket through `Path::exists()`.
An existing test encodes that behavior by writing a regular file and expecting
the display to remain reserved. The first implementation checkpoint will
replace that assumption with red-capable ordinary-file and stale-socket-inode
fixtures while preserving live filesystem listeners, abstract listeners, live
matching lock PIDs, and unknown observations as unavailable.

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
| Original pathname conflation is red-capable | Test update not yet run | pending |
| Live listeners remain reserved | Existing abstract-listener fixture; filesystem listener control pending | partial |
| Exact stale residue is reclaimable | Fixtures and repair pending | pending |
| Unknown observations fail closed | Existing classifier behavior; regression coverage to retain | partial |
| Bounded exhaustion summary | Repair and test pending | pending |
| Required validation and integration | Not yet run | pending |

## Stop Condition

Stop before deleting any path outside the exact scanned display socket and
lock names or when live-listener absence cannot be proven. Stop before browser,
X server, Service State, provider, installed-runtime, or retained-profile
effects. If implementation requires a source surface owned by P190 or P197,
publish the dependency and reconcile writer custody before editing it.
