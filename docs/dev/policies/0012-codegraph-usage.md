# Policy | Codegraph Usage

## Policy

- Use CodeGraph before non-trivial source-code exploration, architecture
  claims, flow tracing, refactor planning, impact analysis, or code edits.
- Prefer CodeGraph structural tools for questions such as:
  - where a symbol is defined
  - what calls or depends on a function, class, route, component, or command
  - how one behavior reaches another
  - what a refactor is likely to affect
  - which files make up an unfamiliar subsystem
- Prefer CodeGraph lookups over broad manual grep loops for symbol, flow,
  caller/callee, dependency, and architecture questions.
- Use text search or direct file reads for literal strings, comments, log
  messages, documentation prose, generated artifacts, or details that
  CodeGraph does not cover.
- Treat CodeGraph as a discovery and impact-analysis aid, not as proof that a
  change is correct.
- Verify behavior with direct source reads, targeted tests, type checks,
  linters, browser checks, runtime smokes, or release gates appropriate to the
  touched surface.
- Account for index freshness. After editing source files, do not immediately
  assume CodeGraph reflects the new file contents.
- If CodeGraph tooling is unavailable, stale, or not initialized for this repo,
  proceed with normal repo inspection and state the fallback in the handoff
  when it affects confidence.
- Keep secrets, credentials, private logs, browser profiles, auth state, and
  unrelated runtime data out of indexed CodeGraph inputs or persisted analysis
  artifacts.
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

## Repo-Local Entry Points

- This repo has a `.codegraph/` index in the working tree. Treat it as
  workstation-local state, not a source file to edit or commit.
- Use the CodeGraph MCP tools when exposed in the current agent session:
  `codegraph_context`, `codegraph_search`, `codegraph_trace`,
  `codegraph_callers`, `codegraph_callees`, `codegraph_impact`,
  `codegraph_node`, `codegraph_explore`, `codegraph_files`, and
  `codegraph_status`.
- For architecture or feature-area questions, start with `codegraph_context`,
  then use one focused `codegraph_explore` call for source bodies when needed.
- For flow questions, start with `codegraph_trace` rather than rebuilding the
  path manually from search results.
- For refactor planning, use `codegraph_search`, `codegraph_callers`, and
  `codegraph_impact` before editing shared symbols.
- If `.codegraph/` is missing or the MCP server reports that the repo is not
  initialized, ask before running an indexing command.

## Adoption Notes

This policy adopts the shared `codegraph-usage` module with the current
agent-browser CodeGraph index and MCP workflow. The policy supplements the
existing validation rules; it does not replace Rust, dashboard, docs, client,
or live-service checks.
Keep exact commands, MCP tool names, sibling checkout paths, service repair, and project-specific index exclusions repo-local. The reusable contract is initialize an expected missing index, explicitly sync when automatic refresh is absent, and verify current status.
