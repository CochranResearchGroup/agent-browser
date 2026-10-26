# P296: Dependable browser journeys and narrated teaching

Product lane: PL-RECIPES
Status: OPEN
Spec: https://github.com/CochranResearchGroup/agent-browser/issues/466
Integration branch: recipes/spec466-integration
Integration base: 5770d693b23d51c9932574fa68e012a98029ce73
Integration tip:72b62dc94; source467 validation remains pending. Draft PR480 records publication.

## Custody and scope

Reuse clean, integrated, candidate-free duplicate-403 checkout for integration and profile-proof checkout for worker473; retain former branches. Seven registered checkouts remain. Preserve dirty Auth checkout, separate Auth owner and all pinned candidates. No production install, provider changes or private-media access is implied.
Primary owns integration and shared contracts. Worker473 owns dedicated teaching evidence implementation; overlapping public projections serialize by explicit agreement.
Stage1 issue453 and draft454 remain historical planning inputs; their former P295 serial conflicts with canonical standalone Auth P295. This P296 owns implementation of spec466 without overwriting Auth or creating a competing Stage1 program.

## Execution ledger

| Ticket | Blockers | State | Owner | Branch / checkout | Evidence |
| --- | --- | --- | --- | --- | --- |
| 467 | none | validated | root | recipes/spec466-integration / duplicate-403 | 135 MCP tests, strict workspace Clippy and format check PASS at72b62dc94; tracker closure pending |
| 468 | 467 | pending | unassigned | pending | none |
| 469 | 467 | pending | unassigned | pending | none |
| 470 | 468,469 | pending | unassigned | pending | none |
| 471 | 470 | pending | unassigned | pending | none |
| 472 | 471 | pending | unassigned | pending | none |
| 473 | none | validated | implement_473 | recipes/teaching-evidence-473 / profile-proof | worker ea0e2f42ab59; integration67780a8b1; synthetic cases and shared docs validation pass; tracker CLOSED with explicit source/synthetic receipt |
| 474 | 473 | running | implement_473 | recipes/teaching-draft-474 / profile-proof | base72b62dc94; result pending |
| 475 | 474 | pending | unassigned | pending | none |
| 476 | 475 | pending | unassigned | pending | none |
| 477 | 475 | pending | unassigned | pending | none |
| 478 | 468,469,476,477 | pending | unassigned | pending | requires real operator narration and scoped portal |

## Validation and bounded review

Readiness requires validated integration evidence, separately from tracker closure. Each ticket records exact commits, commands and limitations here. Rust compilation uses cargo-safe; changed Rust requires fmt and strict workspace Clippy. CI stays disabled.
One broad Standards/Spec review after the ticket batch, then one bounded TDD remediation pass. Proposed fixed review base is integration base above; user answer pending. No broad review has run; no postreview remediation has run.

## Ticket467 seam and initial evidence

Public seams: MCP browser_command task_briefing and CLI mcp task-briefing. Compare identical exact-ID projections, no transport creation, no peer/owner-state mutation, honest missing provenance and ambiguous lane exclusion. This catches read-only reconstruction failures; it does not qualify current page state or physical media.

Baseline public red: installed `agent-browser --json mcp task-briefing unknown` returned unsupported command (exit1). Focused source test launched through rust-tests; resource admission delayed execution, so no source-test red claimed. Source green pending.

Client protocol red/green: `node scripts/test-task-briefing-client.js` failed before the helper implemented HTTP transport, then passed exact encoded identity, mismatch/no-retry and invalid-ID/no-request cases. `pnpm test:task-briefing-client` passed protocol plus JavaScript type checks.
First Rust focused run passed1 test at an earlier intermediate source state; enriched evidence and HTTP adapter changes require a fresh run before validation. Resource admission is preserving the host reserve.

## Ticket473 integration receipt

Worker ea0e2f42ab595bf7c11d4f598d7ade31b2ba5463 integrated serially as67780a8b1. Dedicated source, protocol tests and notes only; shared documentation supplied by coordinator. Integration reran `node scripts/test-teaching-evidence.js` and `node scripts/qualify-teaching-speech-owner.mjs /tmp/agent-browser-spec466-implementation/speech-owner473-pin/index.mjs`: PASS. Actual Speech HTTP/Saved/Core and FFmpeg synthetic conversion, deterministic recognition and Remote View contract fixture. The frozen external artifact SHA256 is466969f5c0406bf48e6a915dd934e0bccd36ea5c947b528c20a58dfc63525b9a. Speech checkout source/build equivalence unclaimed; owner checkout798023f9625e03cc69693f2d7222c69eeb32d48c is dirty and preserved. Native Stop sequence source8d73717d1f32465891dab648200c765b5ff859e8 is separate from caller gap attribution. Real narration/native protected entry/recognition qualification stays pending for478.

Ticket467 follow-up source run exposed HTTP result serialization mismatch (compile error), corrected at the existing JSON writer adapter. Fresh full MCP tests and strict workspace Clippy queued through resource admission; no successful compile claimed for that failed attempt.

Integration gates: docs build PASS37 routes; policy wiring PASS11; changed documentation links PASS; Remote View handoff docs PASS; full service-client provider-free contracts/types/exports/examples PASS. API/MCP parity initially found the existing main skill already exceeded its10000-byte cap at baseline11108 bytes. Moved unchanged advanced continuity and diagnostic guidance to its linked CLI/service reference; main skill9968 bytes, parity PASS. No user-scoped skill publication occurred.

Tracker473 closed after remote branch publication and draft PR480, retaining explicit real-media/native qualification limits. Worker474 admitted serially by reusing its clean original checkout at72b62dc94; no new worktree. Root467 strict Rust validation still pending resource capacity.

Source compilation resource receipt: default14GiB claim waited persistently while host MemAvailable stayed roughly26GiB. Replaced those waiting commands before execution with `AGENT_BROWSER_CARGO_MEMORY_CLAIM_KIB=8388608 AGENT_BROWSER_CARGO_MEMORY_HIGH=7G AGENT_BROWSER_CARGO_MEMORY_MAX=8G` and the same cargo-safe runners. This matches an8GiB admission claim to an8GiB hard cgroup limit while preserving the default16GiB host reserve; no process cleanup or reserve reduction. Prior exact-package unit compilation observed roughly6GiB compiler RSS. Full MCP tests and strict workspace Clippy admitted; outcomes pending.

## Ticket467 validated source receipt

At source commit72b62dc943f6576b334e037e7e30ee313b617f64: full MCP public tests PASS135 (including exact-ID CLI/MCP briefing, unchanged peer/owner state, no transport creation, retained progress/artifacts and original reconciled uncertainty); strict workspace Clippy PASS; cargo-safe format check PASS. Matching8GiB hard cgroup/admission claim retained16GiB reserve and succeeded. Client protocol/types, provider-free full client suite, parity and documentation gates passed as recorded above. No installed/native qualification claimed.

Fresh admission inventory still has seven registered checkouts. Auth owner independently advanced its own checkout to auth/stripe-acceptance-464 at9ba0edc1987782f360361ea8deacc03324bbe12b; preserve it. Dashboard/mode checkouts have pinned candidates; enrollment has a retained candidate and active-build claim. Root dirty Auth work remains preserved. Teaching474 uses its already admitted profile-proof checkout. Two necessary source-worker auxiliaries for newly ready468/469 will be bound to P296, with exact clean integration and governed removal as terminal obligation; they do not gain runtime/install custody. No foreign checkout is reclaimed.
