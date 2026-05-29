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
