# Roadmap

Updated: 2026-10-05

## Current direction

Agent Browser launches and operates browsers for existing clients. The operator handoff is a deep link directly to a Remote View desktop. Use the real production Remote View origin and its existing Authelia authentication. Prefer a chooser restricted to Agent Browser-owned desktops.

Keep the workflow simple: no copied Remote View service, dashboard detour, extra Open desktop click, or additional Agent Browser viewer-grant ceremony. Use the client's actual chosen profile, including AuraCall's default; do not invent a login requirement for a synthetic profile. Preserve existing profile data and authentication.

## Current work and boundaries

P223 remains paused in `platform/p220-remote-view-consumer`. This documentation cleanup neither resumes it nor claims its local implementation is merged, published, or fully accepted. RUNBOOK owns current execution status. The active plan index locates scope; existing client/provider behavior is reused within its proven scope.

The four-profile governance MCP policy pilot landed through PR #206. Use its compact cited packets and freshness checks instead of bulk policy ingestion.

## Historical material

[Legacy planning archive](docs/history/2026-10-05-legacy-planning/README.md) preserves earlier roadmaps, runbooks, plans, identifiers, and evidence. It is reference-only and does not define current direction. Archival does not close unrelated issues or PRs. Do not reinstate old requirements from an archived OPEN state.
