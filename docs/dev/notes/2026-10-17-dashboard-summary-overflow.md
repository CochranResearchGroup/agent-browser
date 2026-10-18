# Dashboard summary response overflow

Product lane: PL-BUGFIX
Disposition: active-input
Owning plan or work item: [issue 301](https://github.com/CochranResearchGroup/agent-browser/issues/301), UX301

The operator reported an empty rail, a 2 MiB dashboard summary ceiling error and missing-stream fallback. A read-only owner probe reproduces that exact error. Full status succeeds at about 6.9 MB and the owner retains 13 browser and 27 tab records; record counts alone do not prove every browser is live. Runtime lifecycle contributes about 1.5 MB, mostly detailed resource inventories and a duplicated reconciliation receipt outside the existing service-state summary budget.

Reuse the UX301 checkout and combined PR306 candidate. This incident owns service_status_projection and its tests, plus the required summary guidance. Preserve handoff-context enforcement and foreign checkouts. One 60-minute repair/qualification/promotion packet and one review pass cover this incident. Repair the dashboard projection without deleting authority, history, browser state or increasing the response ceiling. Keep health flags, resource totals and omission/detail metadata useful; full status must remain lossless. Qualify the combined source, isolated candidate and live preservation before calling the dashboard restored.

The real projector regression reproduces the exact response-ceiling error before the fix and passes after it. All 17 projection tests, 39 help tests, formatting, strict workspace Clippy, client contracts/types/helpers, policy wiring, handoff documentation and documentation build pass. Full lifecycle status remains lossless, live-tab data and degraded health flags survive summary projection, and omissions have explicit counts and a full-status route. At source seal, candidate qualification and installation were pending. Private probes and raw failures remain outside the repository.

## Installed restore

PR306 integrated as `9aeb71805325e522524643efc85b8e2e2e9b75d5`. Sealed source `d329cc81662d0f9a7afb9215083db1a053a740e8` entered canonical ancestry with identical trees. Candidate `candidate-56ccbdee0270f0f9-4131bd84bdda21c6` uses the production release profile; binary SHA256 is `4131bd84bdda21c61d0609cd3106c35375922a8ebe4d9e4f3609a03f5ce0384b`.

Exact sealed MCP/CLI negative checks prove missing creator context fails before task/profile creation. Isolated development publication passes runtime and provider doctors, three disposable launch cycles, and native Alpha/Beta/Alpha selection with physical input, independent CDP readback, peer preservation and zero remaining owned processes. An older fixture initially omitted the newly mandatory labels and was rejected before launch. A subsequent operator-focus proof rejection is retained separately; fresh authenticated bindings qualified the final native workflow without bypassing authority.

Transaction `upgrade-d23ea068-9ccf-4e85-b358-8b9b2d6683da` accepted production generation `0.28.0-4131bd84bdda-c06c6149b420`. Installed and supervised-host checksums match the sealed bytes; install doctor passes. Exact before/after evidence preserves the selected browser process, all nine managed targets observed live before upgrade, and the profile/handoff record key sets. The original native handoff callback returns 200 with a validated destination.

Production summary responses are approximately 809 KB, below the unchanged 2 MiB ceiling; lifecycle projection is about 1.7 KB. Two consecutive post-warmup samples contain live browser evidence. The actual installed inventory UI renders a live browser, current tab rows and retained handoffs, with no empty-rail or overflow message, no repeated Unknown creator rows and no JavaScript errors. The selected task reaches opened and supplies its iframe URL. Its document was isolated at the viewer boundary to avoid establishing human use; fresh public authenticated viewer delivery remains unverified.

Whole-summary requests still take about 3.6 seconds, and an initial presence sample can be unverified while observation refreshes. That wider latency/freshness work remains open in UX301. No account login, provider configuration change, authority/history deletion or broad process cleanup was performed. A fresh OS/resource readback is retained privately. Pinned candidate bytes and the checkout remain retained.
