# Roadmap

Updated: 2026-10-05

## Current direction

[The product contract](docs/dev/policies/0054-browser-product-contract.md) owns the value proposition: complete browser work, hand it to a person, and continue afterward with less effort and fewer failures.

Use an existing client profile, a managed persistent profile, or a disposable profile as the task requires. Remote View owns desktop operation and the native viewer; Agent Browser owns browser operation and their integration. The operator handoff opens the native desktop directly through existing Remote View authentication, including production Authelia. Prefer an Agent Browser-owned desktop chooser.

Prioritize less plumbing, useful continuity, seamless human assistance, predictable operation and clear results. Each substantial change names its concrete workflow improvement and validates it. Current implementation gaps belong in RUNBOOK rather than becoming permanent product rituals.

## Current work and boundaries

[P223 version 6](docs/dev/plans/0223-2026-10-04-finish-agent-browser-acceptance.md) is the open browser-workflow plan. It remains paused at the operator checkpoint. Its scope is now published independently of the unmerged implementation in `platform/p220-remote-view-consumer`. RUNBOOK owns current evidence and the next bounded packet; this re-anchoring does not resume, publish or accept runtime changes.

The four-profile governance MCP policy pilot landed through PR #206. Use its compact cited packets and freshness checks instead of bulk policy ingestion.

## Historical material

[Legacy planning archive](docs/history/2026-10-05-legacy-planning/README.md) preserves earlier roadmaps, runbooks, plans, identifiers, and evidence. It is reference-only and does not define current direction. Archival does not close unrelated issues or PRs. Do not reinstate old requirements from an archived OPEN state.
