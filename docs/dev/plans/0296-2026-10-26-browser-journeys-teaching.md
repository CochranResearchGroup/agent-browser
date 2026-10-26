# P296: Dependable browser journeys and narrated teaching

Product lane: PL-RECIPES
Status: OPEN
Spec: https://github.com/CochranResearchGroup/agent-browser/issues/466
Integration branch: recipes/spec466-integration
Integration base: 5770d693b23d51c9932574fa68e012a98029ce73
Validated source tip:34d400f0ebcb4915cc93746e46586e69d37e5573; latest documentation checkpoint follows on recipes/spec466-integration / draft PR480.

## Custody and scope

Reuse clean, integrated, candidate-free duplicate-403 checkout for integration and profile-proof checkout for worker473; retain former branches. Seven registered checkouts remain. Preserve dirty Auth checkout, separate Auth owner and all pinned candidates. No production install, provider changes or private-media access is implied.
Primary owns integration and shared contracts. Worker473 owns dedicated teaching evidence implementation; overlapping public projections serialize by explicit agreement.
Stage1 issue453 and draft454 remain historical planning inputs; their former P295 serial conflicts with canonical standalone Auth P295. This P296 owns implementation of spec466 without overwriting Auth or creating a competing Stage1 program.

## Execution ledger

| Ticket | Blockers | State | Owner | Branch / checkout | Evidence |
| --- | --- | --- | --- | --- | --- |
| 467 | none | validated | root | recipes/spec466-integration / duplicate-403 | Reopened regression repairedff812bf0f; model2/MCP151/strictClippy/fmt PASS34d400f0e; reclosure pending |
| 468 | 467 | running | implement_468 | platform/task-recovery-468 / spec466-468 | base42875e629; artifact gate repair pending |
| 469 | 467 | running | implement_469 | challenge/native-assistance-469 / spec466-469 | base42875e629; independent source/native fixture work |
| 470 | 468,469 | pending | unassigned | pending | none |
| 471 | 470 | pending | unassigned | pending | none |
| 472 | 471 | pending | unassigned | pending | none |
| 473 | none | validated | implement_473 | recipes/teaching-evidence-473 / profile-proof | worker ea0e2f42ab59; integration67780a8b1; synthetic cases and shared docs validation pass; tracker CLOSED with explicit source/synthetic receipt |
| 474 | 473 | validated | implement_473 | recipes/teaching-draft-474 / profile-proof | worker243b7405; integration25dd69054; combinedMCP151/strict/docs/client PASS; closure pending |
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


## Current worker and shared-contract custody

The nine registered checkouts comprise preserved foreign/pinned lanes plus P296
integration, existing teaching worker and two necessary temporary auxiliaries.
Admissions468/469 are serialized under the common Git admission lock; exact
receipts remain in common Git metadata. These auxiliaries must complete governed
clean closeout after exact integration. No foreign checkout or pinned candidate
is reclaimed.

Root writes central MCP/catalog/HTTP/module lists, root scripts and shared docs.
468 owns connection/task-operation/retained-only host/workflow and additive
operation model;469 owns Native capture/OCR and pure classification;474 owns
SiteGuide teaching/playbook/client/dedicated docs. Root applies shared snippets
into worker branches serially.467 artifact repair adds one pure service-model
artifact module/lib declaration, explicitly reconciled with468's disjoint model
additions before either integration.

## Reopened467 artifact publication regression

Source-only briefing gates passed, but successful ordinary commands advance
session activity/expiry. Comparing entire pre/post sessions incorrectly refused
artifact publication.467 reopened; no dependent integration/closure relies on
that incomplete artifact claim. New public owner-model seam retains a completed
artifact under exact task/tab/target/process context, tolerating activity/expiry
and peer changes without replay or paths/contents.

Red: cargo-safe test -p agent-browser-service-model --test task_artifact_references
failed legitimate activity/peer progress. Green passed after logical identity
fencing. Second red exposed replaced-target acceptance; green2/2 follows exact
target/process fencing. CLI publisher now delegates to that owner-model method,
preserving original operation ID in completion-refusal errors. Combined MCP,
strict Clippy/format and documentation qualification remain pending.

## Integrated474 source checkpoint

Worker243b7405cd3983eef71500bb370fea95b81590bc integrated as25dd69054.
Public JSON-RPC source red/green covers useful typed teaching inspection,
invented-reference refusal, ordinary BILL compatibility, no transport/peer/state
mutation and inert draft execution. Worker strict Clippy/fmt passed. Integration
teaching-draft/evidence JS tests pass; shared pointers/root script applied.
The implementing agent interpreted real decoded frames from the fixed synthetic
caption-led A/V and authored four useful routines, date/refunds parameters,
outcomes/recovery and source-linked uncertainty. Reference-tone audio is not
narration; actual spoken/portal qualification remains478. Combined gates pending.


## Requalified467 and474 integration

At34d400f0ebcb4915cc93746e46586e69d37e5573:
- Public owner-model artifact regression PASS2 after both recorded reds.
- Full MCP source set PASS151, including teaching inspection/compatibility,
  ordinary task briefing and exact no-effect/peer/control checks.
- Strict workspace Clippy PASS and cargo-safe workspace format check PASS.
- Teaching draft/evidence and briefing client tests/types PASS; package exports PASS.
- Docs build PASS38 routes; changed documentation links PASS.
- API/MCP parity PASS. Handoff docs initially exposed that the compacted skill
  had moved its explicit opaque-path rule into the reference. Restored readiness,
  durable-link and raw-provider prohibition in the main skill; focused handoff
  docs and parity both PASS, main skill9980 bytes.
No user-scoped skill publication or runtime installation occurred.

Root independently viewed the date/result frames: May1–May31, checked refunds,
USD,125.00/5 and240.00/8 example rows, with captions distinguishing future policy
and verification. This supports the concrete synthetic draft; it does not turn
reference-tone audio into narration or validate future-date execution.

467 may be reclosed with its repaired publication invariant and explicitly scoped
source evidence.474 may be closed with meaningful agent-authored draft evidence.
475 becomes ready only from this validated integration checkpoint;468/469 remain
owned running work. Post-batch broad review/remediation counts remain0/0.
