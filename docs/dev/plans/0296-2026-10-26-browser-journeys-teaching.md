# P296: Dependable browser journeys and narrated teaching

Product lane: PL-RECIPES
Status: OPEN
Spec: https://github.com/CochranResearchGroup/agent-browser/issues/466
Integration branch: recipes/spec466-integration
Integration base: 5770d693b23d51c9932574fa68e012a98029ce73
Integration tip:67780a8b1 plus the pending coordinator custody commit; source467 validation remains pending

## Custody and scope

Reuse clean, integrated, candidate-free duplicate-403 checkout for integration and profile-proof checkout for worker473; retain former branches. Seven registered checkouts remain. Preserve dirty Auth checkout, separate Auth owner and all pinned candidates. No production install, provider changes or private-media access is implied.
Primary owns integration and shared contracts. Worker473 owns dedicated teaching evidence implementation; overlapping public projections serialize by explicit agreement.
Stage1 issue453 and draft454 remain historical planning inputs; their former P295 serial conflicts with canonical standalone Auth P295. This P296 owns implementation of spec466 without overwriting Auth or creating a competing Stage1 program.

## Execution ledger

| Ticket | Blockers | State | Owner | Branch / checkout | Evidence |
| --- | --- | --- | --- | --- | --- |
| 467 | none | running | root | recipes/spec466-integration / duplicate-403 | public red/green pending |
| 468 | 467 | pending | unassigned | pending | none |
| 469 | 467 | pending | unassigned | pending | none |
| 470 | 468,469 | pending | unassigned | pending | none |
| 471 | 470 | pending | unassigned | pending | none |
| 472 | 471 | pending | unassigned | pending | none |
| 473 | none | validated | implement_473 | recipes/teaching-evidence-473 / profile-proof | worker ea0e2f42ab59; integration67780a8b1; synthetic cases and shared docs validation pass; tracker closure pending |
| 474 | 473 | pending | unassigned | pending | none |
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
