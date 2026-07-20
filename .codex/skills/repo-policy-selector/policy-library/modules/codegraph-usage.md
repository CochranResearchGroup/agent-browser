---
id: codegraph-usage
title: Codegraph Usage
summary: Use an available indexed codegraph before code exploration or edits so agents start from structural code context instead of ad hoc text search alone.
tags:
  - code
  - graph
  - analysis
  - refactor
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
- Treat the codegraph as a discovery and impact-analysis aid, not as proof that a change is correct. Verify behavior with source reads, targeted tests, type checks, linters, browser checks, or runtime smoke as appropriate.
- Prefer codegraph lookups over broad manual grep loops for symbol, flow, caller/callee, and architecture questions. Use text search or direct file reads to confirm details the index does not cover.
- Account for index freshness. After editing code, wait for the index to refresh or use direct source reads and validation instead of assuming the graph reflects the newest file state.
- Keep secrets, credentials, private logs, and unrelated runtime data out of indexed codegraph inputs or persisted analysis artifacts.
- If codegraph tooling is unavailable, stale, or not indexed for the target repo, proceed with ordinary repo inspection and state the fallback in the handoff when it affects confidence.

## Adoption Notes

Use this module when a repo contains code that agents edit, review, trace, or refactor and an indexed codegraph is available or expected in the working environment.

Keep exact commands, MCP tool names, sibling checkout paths, index refresh commands, and service health checks repo-local unless they generalize across many codegraph-backed repos.
