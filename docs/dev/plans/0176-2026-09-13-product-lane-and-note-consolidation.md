# Plan 0176 | Product Lane And Note Consolidation

Date: 2026-09-13

State: INTEGRATION_READY

Consolidation: required

Lane: P176

Product lane: PL-PLATFORM

Branch: `platform/product-lanes-note-consolidation`

Target: `main`

Integration: merge through the repository review workflow

## Objective

Define stable product ownership for multi-agent Agent Browser development and
route the repository's reusable notes into that model without rewriting or
discarding historical evidence.

## Current State

The repository has 253 notes, many numbered execution plans, and a machine
readable active-lane catalog. Plans and branches provide short-lived delivery
custody, but there is no stable product taxonomy above them. Related work can
therefore reuse plan IDs, compete for shared source surfaces, or treat field
notes as an implicit backlog.

The current collision is concrete: canonical P173 is the closed expired-session
repair, while the Turnstile branch also labels its CAPTCHA roadmap P173. Plan
0161 is closed profile-platform work but its authentication-reset capability can
be mistaken for ownership of the broader automated-authentication product.

Graphiti discovery was healthy in group `agent_browser_main`. It confirmed the
service-owned lifecycle direction and development-runtime isolation history but
contained no current five-lane taxonomy. Current plans, notes, Git custody, and
repo policies therefore govern this change.

## Consolidated batch

1. Define five stable product lanes for bug fixes, credential management and
   automated authentication, anti-automation challenge handling, reusable
   automation recipes, and architecture/infrastructure.
2. Define ownership, dependencies, shared-surface arbitration, worktree limits,
   branch prefixes, and merge ordering for multi-agent development.
3. Add one notes index that routes current reusable evidence without moving or
   editing historical notes.
4. Assign current catalog entries to product lanes and teach agents to select a
   product lane before opening substantive work.
5. Wire the taxonomy into the roadmap and current runbook.

## Non-Goals

- Renumbering or editing the active Turnstile branch from outside its worktree
- Absorbing the active Plan 0240 branch or its untracked note
- Implementing authentication, CAPTCHA, recipe, runtime, or browser behavior
- Moving, deleting, or bulk-editing historical notes
- Changing production runtime, profiles, credentials, browsers, or tenant state
- Adding a new issue tracker or changing formal release policy

## Delivery sequence and budget

1. Inventory current plans, notes, worktrees, and Graphiti history.
2. Freeze lane names, ownership, dependency direction, and WIP limits.
3. Add the product-lane authority, notes index, roadmap/runbook wiring, agent
   guidance, and catalog projection in one documentation batch.
4. Run planning, lane, link, and patch-hygiene checks; publish one reviewable
   branch.

Time bound: 60 minutes. Review/rework bound: one correction cycle. No build,
browser launch, provider operation, runtime reconciliation, or installation is
needed for this documentation-only milestone.

## Worker assignments

- Primary owner: current Codex agent.
- Parallel workers: none; current orchestration policy prohibits delegation and
  the governing files form one tightly coupled decision surface.
- Write scope: this plan, product-lane authority, notes index, roadmap, runbook,
  AGENTS guidance, and active-lane product assignments.

## Evidence and exit

| Requirement | Evidence | Exit state |
| --- | --- | --- |
| Five stable product lanes | `docs/dev/product-lanes.md` | Defined with outcomes and boundaries |
| Plan 0161 placement | Platform lane text and notes index | Platform owns aggregate; auth consumes target reset |
| Notes consolidation | `docs/dev/notes/README.md` | Current evidence routed; historical files preserved |
| Multi-agent controls | Lane WIP, writer, overlap, and merge rules | Explicit and reviewable |
| Repository wiring | ROADMAP, RUNBOOK, AGENTS, active catalog | All canonical surfaces aligned |
| Validation | Planning audit, active-lane audit, link checks, `git diff --check` | This plan adds no finding; legacy audit debt remains visible |

The plan becomes integration-ready when the complete documentation batch is
committed and published with passing local checks. It closes only after merge
to `main` and a clean canonical readback. Implementing any product feature is a
separate bounded plan in its assigned product lane.

## Checkpoint P0176-C01 | 2026-09-13

State transition: `open to integration_ready`.

The complete taxonomy and note-routing batch is committed at `ce0ac47f` and
published on `platform/product-lanes-note-consolidation`. Plan 0176 passes its
planning contract, the runbook remains below 200 lines, remote-view guidance
checks pass, and patch hygiene is clean. The repository-wide active planning
audit still reports 37 pre-existing legacy findings; none belongs to Plan 0176.

The Turnstile branch overlaps ROADMAP, RUNBOOK, and active-lane documentation.
This platform contract lands first; that owning lane must merge current `main`,
renumber its conflicting CAPTCHA roadmap, and reconcile its lane projection
before review. The active Plan 0240 worktree and note remain untouched.

Next action: review and merge this two-commit documentation packet, then require
a clean canonical active-lane readback before closing Plan 0176.
