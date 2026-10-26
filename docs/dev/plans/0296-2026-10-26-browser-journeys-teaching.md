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
| 467 | none | validated | root | recipes/spec466-integration / duplicate-403 | Reopened regression repairedff812bf0f; model2/MCP151/strictClippy/fmt PASS34d400f0e; tracker CLOSED after requalification |
| 468 | 467 | running | root | recipes/spec466-integration / duplicate-403 | worker045dd85a; integration96fad35a2; combined existing-profile HTTPS PASS; reopened borrowed-attachment regression |
| 469 | 467 | integrated | implement_469 | challenge/native-assistance-469 / spec466-469 | base42875e629; independent source/native fixture work |
| 470 | 468,469 | pending | unassigned | pending | none |
| 471 | 470 | pending | unassigned | pending | none |
| 472 | 471 | pending | unassigned | pending | none |
| 473 | none | validated | implement_473 | recipes/teaching-evidence-473 / profile-proof | worker ea0e2f42ab59; integration67780a8b1; synthetic cases and shared docs validation pass; tracker CLOSED with explicit source/synthetic receipt |
| 474 | 473 | validated | implement_473 | recipes/teaching-draft-474 / profile-proof | worker243b7405; integration25dd69054; combinedMCP151/strict/docs/client PASS; tracker CLOSED after source qualification |
| 475 | 474 | validated | implement_473 | recipes/teaching-review-475 / profile-proof | worker5e226f17+89da499b; source9436460eb gates PASS; tracker CLOSED |
| 476 | 475 | running | implement_473 | recipes/teaching-grilling-476 / profile-proof | base9436460eb; new grilling module consumes review.save |
| 477 | 475 | running | implement_468 | recipes/report-repeat-477 / spec466-468 | base9436460eb; fresh repeat and promotion |
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


475 admitted from785c4196e after live tracker/native blocker refresh. The prior
worker243 commit was cherry-picked as25dd; all21 changed blobs verified identical,
rather than claiming ancestry. Clean/pin-free/build-free inspection preceded
reuse; prior branch retained.467 and474 tracker closures read back successfully.
No new checkout for475.

468 actual owned HTTPS EOF red observed: one dropped acknowledged fill reply
caused two nonempty fills instead of one under the default transport retry.
Peer draft and PNG existed, fresh owned residue census zero. Earlier TLS/profile
setup failures are excluded.469 actual Native input/return ran, but OCR admission
failed before complete workflow; those runs are diagnostic until full corrected
Native/classification/admission qualification, with No-CDP posture separate.


## Integrated468 and475

468045dd85abede058a59903087686252355fc296f4 integrated as96fad35a2.
4755e226f172deb1f88fbf4193406654ad106b8cbc8 integrated asfa5433ec3.
Shared lib merge was automatic and preserves467 artifact module. Root applied
shared MCP/catalog/HTTP/native-module hunks within468 worker before its tests.
Integration Node operation/review/draft/evidence cases PASS; shared guides,
help, vocabulary, scripts and notes index now applied. Combined Rust/client/docs
gates remain pending, so no dependents unlock yet.

468 final source candidate SHA256 is
7e0f6e99398915328b642058e7cd8e1ba29e2df67e1c4c2c9db9a54148df9165.
Four lost successful replies covered screenshot plus fill/action/submission,
no-recapture hash/mtime, one application effect each, original readback before
owner replacement, isolated host restart, same drafts/target/artifact and
separate physical-loss intervention. Fresh owned OS residue0. Private receipt
recovery468-52de9c00-4fbb-4570-b092-081220304ac3.json; no raw state in Git.
Worker gates: MCP151, host14, manager49, adopted-original1, clients/types,
strict workspace Clippy and format PASS. Native and legacy/unscoped effects
retain their prior journals/contracts. Qualified existing-profile adoption
retains already-observed immutable process proof; no new PID trust or upgrade
adoption program. New receipt ID constraints skip excluded/unscoped requests.

475 gates: MCP152, strict Clippy/format and review/draft/evidence clients/types
PASS. Authenticated synthetic HTTP operator POST and receiver read are exact-
content bound; roles/proposals/unconfigured readers cannot approve. Concurrent/
stale versions, real killed save children, original-operation replay and private
derived-frame deletion preserve accepted/peer bytes. Evidence access is owner-
bound per reference and unchecked by default. Production receiver review wiring
and actual narrated portal remain separate.

469 remains running. Its manual focus/close repair uses verified owned-child
launch display after immutable identity checks. Historical006 orphan exposed
parent-census coverage limits; exact cleanup and all001–014 owned-resource audit
correct that record. Full No-CDP015 receipt passed the HTTPS workflow, but final
current/historical cleanup and source handoff are still pending. First-run setup
uses the existing empty-synthetic-profile smoke convention, not legal consent or
login. Candidate pins are immutable. No dependent470 readiness is inferred yet.

## Combined Native integration checkpoint

469 worker ef57c4a06bc5a3503cc899cb52eb1c71fb552e2e integrated with468 and475. The only merge conflict was additive client exports; preserve all teaching and challenge entrypoints. Public client behavior checks pass for each. The attempted nonexistent client tsconfig command supplied no evidence; use the established explicit-file type gate.

Final owned Native015 attached and CDP-free workflows passed with independent HTTPS clearance/access and exactly one completion each. Cleanup reconciles all001–015:348 owned process generations absent, unknown0, fresh matching process scan0, Docker resources0, unit files0 and26 X11 bindings clear. Foreign ancestry preserved. Full historical P279 matrix and source/binary equivalence of external Remote View remain unclaimed. Combined-source gates remain pending.

## Combined source validation and next teaching admission

Source9436460eb8445814470238f051675cc56480a2da is clean and published through PR480. MCP153, host14, manual2, Native integration14, model manager49/artifacts2 and ChallengeControl60 pass. Strict workspace Clippy/format, full client suite, explicit teaching/challenge/task implementation types, policy11, changed links20 and docs40 routes pass. Narrow475 type correction89da499b integrated9436460eb; prior declaration-only check did not cover implementation. Tracker475 CLOSED.

Immutable debug candidate SHA256 e33fb4738a548fc714fee29c0ad79dd565cec3f3ec544d60b1810b60b16c2471 binds source9436460eb. Workers468/469 rerun only their existing owned synthetic HTTPS recovery and attached/CDP-free Native workflows against it. No installed/runtime production claim. A mistaken nonexistent documentation-check filename supplied no evidence; canonical check-documentation-links.js passes.

476 is ready from validated475 and reuses its clean candidate-free profile-proof checkout at source9436460eb. Pre-reuse inspection found no pinned candidates or active builds. Worker owns a new grilling client, its tests and dedicated documentation; it consumes existing review.save without editing teaching-review.js/d.ts. Root owns shared contracts/docs. Planned477 owns promotion/fresh-repeat changes in teaching-review and owned portal fixture, explicitly disjoint from476. No extra checkout, shared registry or Auth custody.

## Recovery requalification and parallel teaching contracts

Combined9436460eb candidate passed468 replay: four successful replies lost without duplicate PNG/fill/action/submission, isolated owner restart preserves task/target/drafts/artifact, and separate physical loss intervenes without navigation replay. Receipt recovery468-8a7d7fd4-8bbe-4d59-83b9-f6343c82bae6.json binds e33fb4738a548fc714fee29c0ad79dd565cec3f3ec544d60b1810b60b16c2471. Fresh owned process residue0; private fixture removed. Tracker468 CLOSED.469 run016 still qualifying.

477 ready from validated475; reuses clean candidate-free auxiliary468 on recipes/report-repeat-477 from9436460eb. No new checkout.477 owns teaching-review.js/d.ts promotion/repeat ports and owned HTTPS report fixture.476 exclusively owns additive optional clarification metadata in teaching.rs and teaching-draft.d.ts plus new grilling module. Both consume existing immutable SiteGuide save semantics; neither edits the other's files without coordinator serialization. Root remains sole shared catalog/output/package/general documentation writer.

## Native combined qualification remains incomplete

Run016 against combined9436460eb failed before challenge/viewer phase: public browser_fill refused browser_task_operation_process_unverified. Aggregate INCOMPLETE, not an uncertain input replay. Trusted app attachment retained immutable PID/start/executable/family proof and all attachment/profile/endpoint/desktop joins; original failure-time mode.reason was not preserved, so it is unknown. No product guard change or inferred mode proof. Bounded017 diagnostic captures existing public mode before fill and stops on unverified evidence.469 stays OPEN;470 remains pending. Separate468 existing-profile recovery qualification remains valid within its actual scope.

Fresh001–016 teardown audit:367 positively owned generations absent, unknown/private-process/Docker/unit residue0. Earlier failed run and its ambiguity remain in private receipts; no foreign signals. Full historical matrix remains unclaimed.

## Borrowed attachment regression reopened468

Run017 preserves mode=unverified, reason=retained_debugger_endpoint on the exact original app-provided process. Identity/family/complete argv passed; debugging flag recognition did not. It stopped before fill, ambiguous=false. No parser change is justified until argv shape is qualified;018 captures only token-shape diagnostics on a new owned fixture and stops before input if unverified.468 reopened; root owns the repair, worker468 now owns independently ready477. Do not infer a lost original identity or weaken its guard from this mode result.
