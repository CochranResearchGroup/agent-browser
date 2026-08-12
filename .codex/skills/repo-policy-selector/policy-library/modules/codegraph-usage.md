---
id: codegraph-usage
title: Codegraph Usage
summary: Use and keep current an expected codegraph before code exploration or edits so agents start from branch-accurate structural context instead of ad hoc text search alone.
tags:
  - code
  - graph
  - analysis
  - refactor
  - index
  - worktree
---

## Policy

- When a repo has an available codegraph or indexed code-intelligence service, consult it before making non-trivial code changes, architecture claims, trace analysis, or refactor plans.
- Prefer codegraph context, trace, callers, callees, impact, or file-index tools for structural questions such as:
  - where a symbol is defined
  - what calls or depends on a function, class, route, or component
  - how one behavior flows into another
  - what a refactor is likely to affect
  - which files make up an unfamiliar subsystem
- Use the repo's documented codegraph entrypoint when one exists, such as a sibling `../codegraph` checkout, local MCP tools, CLI wrapper, or indexed workspace service.
- Resolve the intended repository or worktree root and inspect current index status before relying on graph results. A sibling checkout's index is not proof that a fresh worktree or different branch is indexed.
- When a repo has already adopted, configured, or explicitly declared codegraph as an expected development surface, treat a missing index in a verified local worktree as routine derived-state maintenance: run the documented initialization workflow and verify the resulting index status. Do not require a fresh approval solely because the worktree is new.
- Do not assume automatic refresh applies to every project. The active checkout may have a live watcher while secondary projects, explicit-path queries, and fresh worktrees require explicit synchronization.
- When status or a staleness banner reports pending files, disabled auto-sync, an unwatched project, or a stale index, run the documented explicit sync once and re-check status. Do not wait repeatedly on a watcher that is absent or disabled.
- Treat the codegraph as a discovery and impact-analysis aid, not as proof that a change is correct. Verify behavior with source reads, targeted tests, type checks, linters, browser checks, or runtime smoke as appropriate.
- Prefer codegraph lookups over broad manual grep loops for symbol, flow, caller/callee, and architecture questions. Use text search or direct file reads to confirm details the index does not cover.
- After editing code, inspect the reported staleness or pending-sync state instead of guessing a delay. Use direct reads for specifically flagged files until synchronization is confirmed.
- Keep secrets, credentials, private logs, and unrelated runtime data out of indexed codegraph inputs or persisted analysis artifacts.
- Before initialization, confirm the target root and repo-local exclusions. Stop and ask when codegraph has not been established for the repo, the target or allowed input scope is ambiguous, repo policy reserves indexing for an operator, or initialization would create unexpected tracked-file changes.
- If initialization or one explicit sync still leaves codegraph unavailable or stale, proceed with ordinary repo inspection and report the exact failed status or staleness evidence in the handoff when it affects confidence.

## Adoption Notes

Use this module when a repo contains code that agents edit, review, trace, or refactor and an indexed codegraph is available or expected in the working environment.

Keep exact commands, MCP tool names, sibling checkout paths, service repair, and project-specific index exclusions repo-local. The reusable contract is initialize an expected missing index, explicitly sync when automatic refresh is absent, and verify current status.
