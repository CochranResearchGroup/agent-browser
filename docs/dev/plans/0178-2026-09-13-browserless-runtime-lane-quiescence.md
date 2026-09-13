# Plan 0178 | Browserless Runtime Lane Quiescence

Date: 2026-09-13

State: OPEN

Lane: P178

Product lane: PL-BUGFIX

Branch: `fix/plan-0240-quiesce-browserless-runtime-lanes`

Target: `main`

Integration: merge

Work item: [issue #84](https://github.com/CochranResearchGroup/agent-browser/issues/84)

Review: [PR #83](https://github.com/CochranResearchGroup/agent-browser/pull/83)

## Objective

Quiesce only proven browserless competing lanes in the old shared runtime host
before cooperative handoff, then require a stable Service State revision window
before the primary transfer begins.

## Current State

Concurrent work created the branch from `origin/main` during Plan 0177 cleanup.
Checkpoint `e80728b2` failed CI because `Duration` was not qualified. Published
checkpoint `24159acb` contains the narrow compile repair and is awaiting fresh
review evidence. The canonical worktree is back on clean `main`; the branch and
PR retain source custody without a dedicated worktree.

## Acceptance

- A lane with missing health evidence or a browser-bearing worker fails closed.
- Only exact browserless competing lanes close before handoff preparation.
- The revision quiet window rejects a continuing Service State writer.
- Focused tests, Rust format, workspace clippy, workstation fixtures, and exact
  changed-surface validation pass.
- Installed acceptance, browser closure, handoff, restart, and Service State
  mutation remain separately authorized effects.

## Stop Condition

Plan 0177 records custody only. Any further source repair, CI diagnosis,
installation, or runtime effect remains with issue #84 and PR #83.
