# Dashboard summary response overflow

Product lane: PL-BUGFIX
Disposition: active-input
Owning plan or work item: [issue 301](https://github.com/CochranResearchGroup/agent-browser/issues/301), UX301

The operator reported an empty rail, a 2 MiB dashboard summary ceiling error and missing-stream fallback. A read-only owner probe reproduces that exact error. Full status succeeds at about 6.9 MB and the owner retains 13 browser and 27 tab records; record counts alone do not prove every browser is live. Runtime lifecycle contributes about 1.5 MB, mostly detailed resource inventories and a duplicated reconciliation receipt outside the existing service-state summary budget.

Reuse the UX301 checkout and combined PR306 candidate. This incident owns service_status_projection and its tests, plus the required summary guidance. Preserve handoff-context enforcement and foreign checkouts. One 60-minute repair/qualification/promotion packet and one review pass cover this incident. Repair the dashboard projection without deleting authority, history, browser state or increasing the response ceiling. Keep health flags, resource totals and omission/detail metadata useful; full status must remain lossless. Qualify the combined source, isolated candidate and live preservation before calling the dashboard restored.

The real projector regression reproduces the exact response-ceiling error before the fix and passes after it. All 17 projection tests, 39 help tests, formatting, strict workspace Clippy, client contracts/types/helpers, policy wiring, handoff documentation and documentation build pass. Full lifecycle status remains lossless, live-tab data and degraded health flags survive summary projection, and omissions have explicit counts and a full-status route. Candidate qualification and installation are pending. Private probes and raw failures remain outside the repository.
