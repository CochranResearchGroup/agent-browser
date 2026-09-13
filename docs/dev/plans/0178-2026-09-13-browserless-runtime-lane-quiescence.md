# Plan 0178 | Browserless Runtime Lane Quiescence

Date: 2026-09-13

State: BLOCKED

Lane: P178

Product lane: PL-BUGFIX

Integrated branches: `fix/plan-0240-quiesce-browserless-runtime-lanes`,
`fix/plan-0240-quiesce-admission-claim`

Target: `main`

Integration: merge

Work item: [issue #84](https://github.com/CochranResearchGroup/agent-browser/issues/84)

Reviews: [PR #83](https://github.com/CochranResearchGroup/agent-browser/pull/83),
[PR #92](https://github.com/CochranResearchGroup/agent-browser/pull/92)

## Objective

Quiesce only proven browserless competing lanes in the old shared runtime host
before cooperative handoff, then require a stable Service State revision window
before the primary transfer begins.

## Current State

PR #83 merged the browserless lane quiescence repair as `ae426642` from source
checkpoint `f6b263f0`. Its complete fast and comprehensive Rust CI passed. A
subsequent candidate transaction showed that old-runtime status and close
commands also require the exact active runtime-admission transaction claim.
PR #92 merged that follow-up as `7e59ae35` from checkpoint `e33d34df`; focused
admission and quiescence tests, Rust format, Rust Quality, Dashboard, Service
Client, and Version Sync passed. Workstation Fixtures and comprehensive Rust
were still in progress at the one lazy readback and were not actively watched.

Both source checkpoints are ancestors of `origin/main`. Their clean worktree
and local and remote topic refs are retired. Source custody is complete, but
installed acceptance remains blocked by the production maintenance quarantine
under issue #76 and requires separate effect authority. No installation,
restart, handoff, browser closure, or Service State mutation occurred during
this closeout.

## Acceptance

- A lane with missing health evidence or a browser-bearing worker fails closed.
- Only exact browserless competing lanes close before handoff preparation.
- The revision quiet window rejects a continuing Service State writer.
- Focused tests, Rust format, workspace clippy, workstation fixtures, and exact
  changed-surface validation pass.
- Installed acceptance, browser closure, handoff, restart, and Service State
  mutation remain separately authorized effects.

## Stop Condition

Do not retry installation or runtime handoff from this source closeout. Issue
#84 remains BLOCKED until production maintenance quarantine issue #76 is
resolved and a separately authorized installed acceptance passes. A new source
defect requires a fresh branch from current `origin/main`.
