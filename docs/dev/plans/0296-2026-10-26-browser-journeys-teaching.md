# P296: Dependable browser journeys and narrated teaching

Product lane: PL-RECIPES
Status: OPEN
Spec: https://github.com/CochranResearchGroup/agent-browser/issues/466
Integration branch: recipes/spec466-integration
Integration base: 5770d693b23d51c9932574fa68e012a98029ce73
Validated source tip:fe6a3ddeef3b3a926aaa2165e6d9aa2b8025419c; latest documentation checkpoint follows on recipes/spec466-integration / draft PR480.

## Custody and scope

Reuse clean, integrated, candidate-free duplicate-403 checkout for integration and profile-proof checkout for worker473; retain former branches. Seven registered checkouts remain. Preserve dirty Auth checkout, separate Auth owner and all pinned candidates. No production install, provider changes or private-media access is implied.
Primary owns integration and shared contracts. Worker473 owns dedicated teaching evidence implementation; overlapping public projections serialize by explicit agreement.
Stage1 issue453 and draft454 remain historical planning inputs; their former P295 serial conflicts with canonical standalone Auth P295. This P296 owns implementation of spec466 without overwriting Auth or creating a competing Stage1 program.

## Execution ledger

| Ticket | Blockers | State | Owner | Branch / checkout | Evidence |
| --- | --- | --- | --- | --- | --- |
| 467 | none | validated | root | recipes/spec466-integration / duplicate-403 | Reopened regression repairedff812bf0f; model2/MCP151/strictClippy/fmt PASS34d400f0e; tracker CLOSED after requalification |
| 468 | 467 | validated | root | recipes/spec466-integration / duplicate-403 | worker045dd85a; integration96fad35a2; combined HTTPS + repaired Native019 PASS; tracker reclosed |
| 469 | 467 | validated | implement_469 | challenge/native-assistance-469 retained / auxiliary reused by471 | workeref57c4a0; combinedb7 Native019 attached/CDP-free PASS; tracker CLOSED |
| 470 | 468,469 | validated | root | recipes/spec466-integration; native-checkbox-470 branch retained | fe6a/ef948 source gates and actual037 bounded checkbox lane PASS; peer-preserving cleanup and940-generation census qualified; tracker CLOSED |
| 471 | 470 | running | implement_471 | challenge/single-round-image-471 / reused spec466-469 auxiliary | admitted from validatedfc607ad2c; source work only until separately pinned/scoped runtime |
| 472 | 471 | pending | unassigned | pending | none |
| 473 | none | validated | implement_473 | recipes/teaching-evidence-473 retained / auxiliary removed | worker ea0e2f42ab59; integration67780a8b1; synthetic cases and shared docs validation pass; tracker CLOSED with explicit source/synthetic receipt |
| 474 | 473 | validated | implement_473 | recipes/teaching-draft-474 retained / auxiliary removed | worker243b7405; integration25dd69054; combinedMCP151/strict/docs/client PASS; tracker CLOSED after source qualification |
| 475 | 474 | validated | implement_473 | recipes/teaching-review-475 retained / auxiliary removed | worker5e226f17+89da499b; source9436460eb gates PASS; tracker CLOSED |
| 476 | 475 | validated | implement_473 | recipes/teaching-grilling-476 retained / auxiliary removed | workerace432be; integratione39dc8dae; combinedb7a4a6a3f gates PASS; tracker CLOSED |
| 477 | 475 | validated | implement_468 | recipes/report-repeat-477 retained / auxiliary removed | worker0d83b36a7; merge4a68271a3; d270 gates + actual22a/dfe report PASS; tracker CLOSED |
| 478 | 468,469,476,477 | blocked | unassigned | no execution custody | validated prerequisites; missing actual recording/portal/task authority |

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

## Grilling integration and collapsed argv source repair

476 ace432be6786257fff7b87448a529141d530165f integrated e39dc8dae with agreed optional clarification and fresh-repeat claim contracts. Worker154 MCP, strict Clippy/fmt, public synthetic authenticated-answer/killed-child recovery and implementation/declaration types pass. Static labels remain inert; this is not physical human participation. Only task identifier bounds expand to4000; actual length100 example, synthetic length186 and oversized refusal are distinct from477 future actual long-profile evidence.

Root468 repair bcddaec4e follows qualified018 one-NUL-argument shape, not guessed missing identity. Public-mode RED unverified versus cdp, corrected minimal shape normalization GREEN6. Fixture-only observation commits04cf91573/8182a35a integrated685d28bdb/4d1ffad9d. Immutable identity and ordinary argv boundaries remain; collapsed renderer still refuses. All001–018 cleanup405 process generations absent, unknown/Docker/unit/private-process residue0,32 X11 bindings clear. Combined strict/MCP/client/docs source gates and actual attached/CDP-free Native replay remain pending before468/469 closure.

## Validated grilling and repaired source candidate

At clean sourceb7a4a6a3f366cb59500293cd579e793e5c580613, combined154 MCP, strict workspace Clippy/format, implementation/declaration types, all public clients and new grilling export, full service-client suite, policy11, changed links11 and docs41 routes pass.476 tracker CLOSED. Original source/media boundaries remain as in its receipt.

Immutable source-only candidate e1e2b25d93e875ecbbbc75e30fb69414172a14a8ecd262065e2218bc981ab59d binds that clean source.469 runs one full019 attached plus actual managed CDP-free Native qualification with mode preflight and diagnostic-only disabled.477 uses this binary with a separately pinned clean JS consumer source for owned HTTPS report acceptance. No production install or external/private portal authority.468 remains reopened until attached replay qualifies;470 stays pending.

## Requalified original task and Native assistance

Native019 candidateb7/e1e2b25d93e875ecbbbc75e30fb69414172a14a8ecd262065e2218bc981ab59d passes both attached and actual managed CDP-free OCR/input/viewer/takeover-return/independent HTTPS clearance and protected access, same original task/link/peer, exactly1 completion each. Mode preflight cdp/reason null proves the observed collapsed-argv defect is repaired without bypass. DIAGNOSTIC_ACCEPT122.69s, full historical matrix and external Native producer source/binary equivalence unclaimed. Fresh001–019438 owned PID/start generations absent, unknown/private-process/Docker/unit residue0,34 X11 bindings clear; foreign preserved.468 reclosed and469 CLOSED after qualification.

470 is ready from this validated integration source. Reuse clean auxiliary469 at8182a35a72df741d93895a4746627afa6c5737de, no pinned candidates/active builds; preserve old branch and create challenge/native-checkbox-470 fromb7a4a6a3f. Worker owns pure/current Native checkbox helper, dedicated tests/docs and existing owned fixture changes. Root owns shared catalog/output/package/general docs;477 owns teaching review and report fixture, disjoint. Scope is bounded passive observation and one policy-authorized registered checkbox attempt with exact original-operation retention and independent admission; unknown/stale/uncertain cases assist, never blind retry. Historical observation-age changes require current-seam reproduction first. No paused legacy provider execution or production/site benchmark. No new checkout.

## Current Native age characterization and local date-fill repair

470 private fixture setup failures020–023 are not age or product results: an external dependency symlink had disappeared, then both scoped forwarding layers needed /input. Exact locked dependency bytes restored offline without lifecycle scripts. Provider-free real Node/Python forwarding chain passes one valid move with unchanged context and rejects wrong desktop/generation and click/key at both layers. Native024 then dispatches nine move-only requests across3496ms, same original window/generation/process and zero button events. Historical750ms stale-transaction failure does not reproduce on this current seam; no timing change. Fresh001–024558 owned generations absent, unknown/private-process/Docker/unit residue0. Same-store Native capture/attempt journal implementation remains running; root applies only shared MCP wiring in worker checkout.

477's actual public RED proves browser_fill returns success but input[type=date] remains empty for requested2026-06-01, with zero report requests/commits/exports. Receipt report477-40a803b7-7ef6-455b-a834-bbc247052e6f.json preserves this separately from earlier HTTP harness failures. Actual source/review task IDs131 characters qualify the4000 task-ID bound; original474 media unchanged.

Root owns only local interaction.rs date-fill repair, focused persistent E2E coverage and shared help/docs. Native date setter stays in the exact resolved element realm, checks value/writability, emits input/change once and preserves ordinary text path. Isolated real-browser E2E GREEN1 validates correct date/events, invalid-date value preservation, read-only refusal and ordinary text. Initial direct Cargo test omitted the repository16MiB test-stack setting and aborted before a result; retry with RUST_MIN_STACK16777216 passed. Fresh OS readback excludes exact current ancestors and finds no owned date-fixture process or unknown match. Reviewed Chrome digest0ba9d6c54dbc1a46dfaa5f95d5975bc9b963030c8647880f5b6a537f8af7d2e8 verified. Strict/source gates and a new immutable binary are next before477 actual report requalification; no evaluate workaround, selector/launcher/download change.

## Parent review correction and report source plumbing

Parent475 independently reproduced echo-only reader falsely approving on ordinary public reopen. JS-only correction1d0d80dd9 sends binding fields, requiring independent decision/provenance; review/grilling and strong implementation/declaration checks pass.475 was reopened, corrected/published, then reclosed;477 already carries the matching fix1447b7331. No schema/media change or physical approval claim.

Date21dfd745d/91bea15c4f9b24846480d4178eed0dfa2189a3a439fc24bb442eafe7505e10a5 passes actual477 June value/input/change checks. Its report run then refuses before export because the runtime discarded already-qualified local attachment proof before the download adapter. Receipt report477-c27fd8c8-218f-47b2-88ac-281c39915a5c.json preserves report1/exports0 and cleanup0. Existing README already promises qualified existing-profile downloads; this is lost proof plumbing, not a new blob/data/remote download mode.

Root adds a private download-only process/endpoint slot populated solely by existing verified local attach checks. Download revalidates that original process and endpoint; browser_pid/lifecycle/launcher semantics remain unchanged. Missing/changed/unverified sources still refuse; no caller PID authority. Current provider-free runtime gate31 passed,8 browser tests explicitly ignored; actual report verification still required. An inherited-disabled date case independently failed successfulfill on a disabled fieldset, then passed using the browser's disabled-state predicate; invalid/read-only/ordinary-text/date-event cases stay green.

## Reviewed report integration

Worker0d83b36a77eea98fac058eeff2c2c2ae5651578d merged serially. Shared476 provider changes were identical and produced no Rust difference from22a; parent475 binding-only fix auto-reconciled. Runtime-tested review.js/d.ts and smoke script blobs equal dfe37e6416837d645d780680157d2c9805b3c619. Root retains the independent ordinary-review echo regression alongside477 promoted-review test. Merged review/grilling and strong implementation/declaration checks pass; shared docs gates pending.

Actual receipt report477-9bc83734-014a-4eac-9328-9bd49f9ded9b.json binds clean consumerdfe37e641 and binary22a75625932a21c1508739a95ef712b37215a7a8/d69e3ce33bd8a4dff6d70c7c826874dd434ce8cd239d3f5a8b9289cc0e586754. Wrong-May CSV rejected despite June UI/download; correct June artifact96a66dfd7fb0d1fcdd6525a004170b80042b46794e95f6d0c010ebd1ab64a3ed, actual lost successful submit reply produces submission1/export1/no replay. Only independent verifier proof plus operator event permits promotion; missing/changed proof revokes reuse. Changed Export control assists with exports0; peer/prior accepted/original template preserved. Fresh owned OS0 and private home removed. Earlier HTTP/proxy harness failures remain preserved without invented counters. Physical narration/private account/production installation remain478.

At22a root154 MCP, download-artifact4, runtime31 (8 explicitly ignored real-browser cases), date E2E1, strict Clippy/fmt, help/parity/handoff/link and docs41 gates pass. Source22a remains the runtime pin; later output/help/doc changes require their own applicable source gates, not a claim of a new installed runtime.

## Report repeat closed, real narrated journey input-gated

Shared sourced27092b68a1ba865ea52de7133290a82ace1f8cf passes154 MCP, strict workspace Clippy/format, review/grilling clients and strong implementation/declaration types, full service-client suite, parity/handoff/changed-link checks and42-route docs.477 tracker CLOSED after publication. Actual runtime proof remains separately pinned to22a/dfe consumer and the completed owned report receipt; no later install/physical claim.

478's four source dependencies are validated. Execution remains blocked on an actual operator-narrated recording/artifact selection and an authenticated reporting portal/account/task plus exact applicable effect/cleanup authority. Existing captioned/reference-tone fixtures and synthetic operator events cannot fulfill physical narration/review. The earlier user input request remains unanswered; no private recording archive is searched or provider/account effect inferred. Existing Auth449 capability is consumed only as required by the selected portal, not blocked wholesale by separate464 acceptance. Independent470–472 work continues; no final broad review/remediation has run (counts0/0).

## Ticket477 governed auxiliary closeout

Fresh inspection of worker0d83b36a77eea98fac058eeff2c2c2ae5651578d found clean tracked state, no pinned candidates and no active candidate builds. Its exact commit is an ancestor of the integration tip. Fresh OS readback found zero processes with this checkout as their working directory; the prior runtime receipt separately proves owned fixture cleanup. Governed operation worktree-closeout-7f1e129f-4586-4303-a1c9-83099ed3e087 previewed then committed removal of the exact auxiliary incarnation ff5d2e924e03c4c96f1a70a2c2b52c3d3fe1bee78982d50b51e08ed7e810d7e4. The recipes/report-repeat-477 branch, integrated source and private runtime receipts remain retained. No other checkout, branch or installed runtime was removed. Eight registered checkouts remain; Native470 retains its active auxiliary.

## Ticket470 shared guidance preparation

Root prepared uncommitted README, CLI help, main skill, CONTEXT vocabulary and package-test pointers for the worker's opt-in Native checkbox path. These depend on the worker's not-yet-integrated dedicated source/reference/test script. They are not a qualified publication checkpoint. The main skill remains within the existing10000-byte bound at9999 bytes; opaque handoff guidance remains unchanged. Worker reports provider-free policy/journal/transport/client checks passing, with clean candidate build and actual checkbox runtime still outstanding. No470 closure or471 admission follows from those partial checks.

Ticket477 closeout memory disposition: queued. Graphiti group agent_browser_main accepted one source-anchored job aeabc719-befb-4114-b246-836d684747ad, stable name p296-report-repeat477-qualified-20261027, from the curated report-repeat source receipt. Queue acceptance is not persistence or retrieval proof. The compact episode distinguishes synthetic qualification and retained worker branch from production, physical narration and unfinished tickets. No retry was attempted.

## Ticket470 source integrated, runtime pending

Clean workerfc21702e4f33606976581e0e9cbbbd941da57ed6 merged without conflicts asadd11438c. Worker final gates:155 MCP, strict workspace Clippy, format, service client, docs build,21 automatic assistance client scenarios, existing human assistance,16 Native transport/control, one locator and two owner-policy tests PASS. Root shared guidance is now supplied alongside the dedicated reference and test script. This is integrated source, not actual checkbox qualification. Root will build one clean immutable combined candidate, then the worker will qualify frozen namespace025 against that pin. Historical001–024 cleanup has558 recorded process generations absent, remaining0/unknown0, private processes/Docker/owned units0. No470 closure or471 admission yet.

## Ticket470 combined source qualification and immutable runtime candidate

Clean combined344d979e3d589160452b38303911e039e1c3201c passed cargo-safe format, strict workspace Clippy, all155 MCP tests, automatic and human challenge clients, strong implementation/declaration type checks, review-repeat/grilling and full service-client checks, API/MCP parity, handoff guidance, changed-link validation and docs build. Initial link invocation omitted required base and exited before checking; corrected command with basecfc9de779 passed. No code failure was concealed. Main skill9999 bytes passed parity. Candidate MCP help independently includes bounded attempts, uncertain-delivery no-replay and CDP-free human guidance.

The optimized Cargo ci build passed under the matched8GiB hard cgroup/admission claim without changing the16GiB reserve. Its immutable executable SHA256 is6fb10accc517132bde68a965ddc6419ff1d4cbea6e82e21c6e134918b727025e. Source was clean before pinning; frozen worker client and qualification script are byte-identical to integration. This candidate is not installed. Worker received explicit scope to run only fresh owned synthetic namespace025 after exact external pin, ownership, port/display and current runtime-policy checks. Actual checkbox/expectedVisual refusal/lost-reply acceptance remains outstanding; ticket470 stays OPEN and471 remains pending.

## Native025 qualification failure and bounded diagnostic gate

Actual combined344d979/6fb10acc run025 stopped at its first hCaptcha case: complete-window Native evidence classified unknown, no registered profile, helper assistance reason challenge_not_registered_checkbox, application inputEvents0 and no original input operation. Expected continued-status assertion failed. No checkbox acceptance or retry is claimed. The failing capture PNG/OCR was not retained, so the cause remains unproven; immediate post-staging capture without a paint boundary is only a hypothesis.

Fresh001–025 cleanup reconciled589 recorded process generations: remaining0/unknown0, positive private processes0, Docker0 and owned units0; exact025 display locks/sockets and VNC listeners absent. Root authorized a clean fixture-only readiness/evidence correction, then one fresh read-only diagnostic026 after source integration and exact preflight. The unchanged runtime pin remains344d979/6fb10acc. No classifier, threshold, input policy or timing-policy change, guessed input, production logging or checkbox/button/key effects are authorized by that diagnostic.470 remains OPEN;471 is pending.

## Native026 read-only fixture integration

Workera39515af4 and label followup1c018f8c0 integrated serially ascbf0d836a/f18e49cc6. Only three fixture scripts changed; all runtime Rust bytes remain identical to qualified344d979. Node syntax, Python AST, patch hygiene and frozen consumer byte equality passed. The initial bridge receipt omitted observe-only fields and preceded label changes; worker reran the actual two-handler provider-free check and root matched both complete current source hashes. Node and Python independently denied observe-only input, with owner forwards unchanged2 to2 and no Native effect.

Root authorized one fresh owned read-only026 diagnostic using unchanged344d979/6fb10acc runtime and frozen1c018f8c0 consumer. Both forwarding layers prohibit all input, result labels explicitly deny checkbox/input acceptance, and no full-lane attempt or classifier/policy change follows automatically. Actual captured diagnosis and cleanup remain pending.

## Native026 observed diagnosis, fixture result incomplete

Root independently viewed the exact private owned Native PNG and OCR receipt. The visible14px synthetic prompt “I am human” was recognized as “tam human” with minimum word score50.902313, below the registered classifier requirement; hCaptcha branding was observed separately. Unknown/refusal is correct for this evidence. No product classifier or confidence-threshold defect is proven, and no threshold is relaxed. No Native input occurred.

The diagnostic itself remains INCOMPLETE: it incorrectly expected image bytes in the public Native capture result, whose owner intentionally strips them. The scoped private bridge retained the actual26516-byte PNG and OCR. Fresh001–026 cleanup qualified620 recorded process generations absent, remaining0/unknown0, private processes/Docker/units0 and exact026 X11/VNC residue absent. Root approved only fixture private-bridge evidence saving and explicitly synthetic18px prompt legibility, followed by source review before one read-only027. Exact current-task HTTPS origin must also be verified independently; the scheme-hidden screenshot is not origin proof. No automatic full attempt follows.

Worker77aeed0a2 integrated as13c5928bb: two fixture scripts only, including18px synthetic prompt, private bounded PNG saving and independently addressed current-task HTTPS origin assertions. Root verified Node/Python syntax, hygiene, byte equality, unchanged runtime Rust, current hash-bound observe-only refusals at both handlers and forwards2 to2. The existing challenge fixture source creates HTTPS;027 must still prove the actual task origin. One fresh read-only027 is authorized against unchanged344d979/6fb10acc and frozen77aeed0a2 consumer, with input refused and no automatic attempt qualification. Result pending.

## Native027 read-only result and bounded legibility preparation

Read-only027 returned DIAGNOSTIC_ACCEPT with actualNativeInput:false and challengeInputDispatched:false, not checkbox acceptance. Exact addressed-task URL and independent app page URL both proved HTTPS and the same owned origin. At18px the pinned owner still recognized “1am human” with minimum word score30.880775; retaining unknown is correct. Private Native PNG/OCR and public classification were retained. Fresh001–027 cleanup qualified650 recorded process generations absent, remaining0/unknown0, no positive private processes, Docker resources, units or exact027 X11/VNC residue.

No product change or blind font retry follows. Root authorized bounded provider-free preparation on one disposable owned headless CfT page: prefer existing registered fixture typography; otherwise at most four recorded spacing/font variants, no auth/application/Native input. All outcomes and exact local Tesseract/model identity must remain attributable; local OCR is preparation, not pinned Native proof. Box/offset geometry and confidence criteria stay fixed. At most one declared synthetic candidate may be proposed with process/profile cleanup before one further read-only Native confirmation.14px/18px failures remain preserved; no production accuracy or automatic attempt is claimed.

## Narrow470 visual corroboration repair

The four-variant local preparation selected no exact-text candidate and stopped. Its current-profile census was zero, but initial PID/start lineage was not retained and its changed-scope runtime packet was checked only retrospectively. It is nonqualifying preparation, not Native acceptance; those limitations remain explicit. No additional font search is authorized.

Root verified current sealed registered locator source already supports high-confidence iam/lam/tam prompt tokens. The Native pure-text classifier does not connect to that visual evidence. Approved repair is Native-only Unknown-to-Checkbox corroboration using the same returned PNG/OCR, exact hCaptcha branding and whole prompt at unchanged70 confidence, plus unique existing registered checkbox geometry. Sealed profiles/normalizers remain byte-unchanged; no new glyph rewriting, extra capture, debugger, focus, input or provider authority. Established denial/ambiguous/image classifications and actual low-score026/027 cases remain refused.

Worker owns narrow browser_native_control adapter/evidence metadata, desktop_locator test module only, dedicated docs and fixture selection of already-recorded Georgia18. Public NativeControl capture RED must fail before implementation; opt-in retained observation and low-score/missing-brand/missing-or-duplicate-box/wrong-frame/denial negatives follow. Ordinary470 feature TDD does not consume the reserved one postreview remediation pass. Clean source/gates then one combined candidate/read-only confirmation are required before further attempt qualification.

## Native visual repair source integration, combined gates pending

Worker7b64f929a integrated asf46d3f2cf; observed-state followup7523d9382 as46e389319; spatial-brand followupb574b2ea3 as57c71b43f. Public Native capture RED-to-GREEN and expanded actual-registered-locator matrix passed; Native control/HTTP16 passed. New v2 requires observed owner OCR, strong prompt and exact spatially matched hCaptcha brand, unique same-returned-frame pixels and complete fresh context. Unavailable/missing OCR state, prior low-score captures, wrong frame, partial/duplicate/blank geometry, denial/image/ambiguity and unrelated distant branding remain refused. Pure v1 and sealed locator/profile/corpus bytes are unchanged.

Strict client acceptance of the one declared v2 rule required its own observed RED-to-GREEN; unknown rules remain refused. Root reran23 automatic scenarios, human assistance, strong implementation/declaration types and service-client checks PASS. Initial docs/parity/link/handoff checks passed after7b integration; final docs checks follow current tip. No actual Native input or automatic acceptance is claimed. Root shared guidance is prepared and one clean combined candidate must pass full source gates before one read-only confirmation.

Preparation cleanup readback found zero readable exact browser/helper/profile matches, with six unreadable generations predating the preparation preserved as unknown/foreign. No global executable absence or reconstructed initial lineage is claimed. Exact model digest and engine version from actual027 match local preparation; this corrects the earlier model-identity limitation without promoting local screenshots or Native executable/source equivalence.

## Resumed470 qualification custody and reused final source gates

User explicitly resumed implement-spec after the handoff. Root takes remaining470 qualification; other pending image tickets stay blocked by validated integration readiness. Worker remains frozen cleanb574. Combined clean0adf8dfa9c66e5df89db04ecf9531e5abb094922 passed fmt, strict workspace Clippy, MCP155, updated client23/human/strong types/service-client, final parity/handoff/links/docs and optimized ci build. Immutable executable SHA256f08fe91f78545b393a872988e7189383da49450940fa9185c4e58e2acecbbe65 is not installed. No rebuild is needed for unchanged source.

Root obtained full current implementation/runtime packets and refreshed the650-generation Native historical census with remaining0/unknown0/positive owned resources0. One read-only028 is released under existing owned synthetic authority after exact candidate/external source/browser pins, frozen consumer bytes, absent namespace, free displays7000/7001 and VNC62200/62201, and nonoverlapping isolated subnet checks. Both input denial layers remain active. No full automatic attempt or new Native acceptance follows from source readiness. Actual028 result and cleanup pending.

## Native028 positive actual visual corroboration

Read-only028 against immutable0adf/f08 returned DIAGNOSTIC_ACCEPT. Root independently verified actual hCaptcha Checkbox classification, explicit corpusv2 registered visual rule, engine/model identity and unchanged70 floor. The pinned owner observed joined prompt score95.239861 and exact brand88.276283; original low-confidence cases remain refused. Application inputEvents0, no input dispatch or protected access is claimed. Fresh001–028 census qualified680 recorded generations absent, unknown0 and positive owned runtime resources0; exact X11 locks/sockets and kernel TCP listeners were absent. An auxiliary TCP-connect readback was interrupted and replaced by direct kernel listener inspection, not promoted to success.

Root released one fresh automatic029 under a current runtime packet for the frozen470 acceptance. Initial planned port62210 was occupied, so preflight stopped before namespace/effects and preserved the foreign socket; fresh port62300/62301 passed. Displays7010/7011 and isolated nonoverlapping subnet were checked. Exact0adf/f08 and external pins/consumer bytes remain unchanged. Actual automatic result and cleanup pending;470 stays OPEN.

## Native029 primary failure and fresh030 observability

Full029 stopped at the first hCaptcha attempt. Actual v2 classification succeeded, but the helper returned awaiting_operator/native_input_uncertain_or_refused. The saved original is uncertain, retryAllowed:false and siteOutcome unverified; owner policy/provider refs were retained. Independent application counters were inputEvents0, cleared:false and browser/backend access0. No provider input intent/ack receipt exists, so its absence is not claimed as complete delivery proof. The precise backend reason was swallowed by the client and remains unproven. The parent missing diagnostic file is secondary. Fresh001–029 cleanup qualified710 recorded generations absent, no positive owned runtime resources and exact X11/kernel listeners absent.

Fixture-only commits9230c782e/7b339612f now retain bounded public input errors and returned responses plus before/after original-operation readbacks, preserving the original thrown error and unchanged product/client behavior. Worker copy is frozenfa4afe4d1. Node syntax/hygiene passed; runtime Rust/client bytes remain identical to0adf, so no rebuild. Root released one fresh030 owned instance after current policy/pin/namespace/display/port/subnet preflight. It does not replay the029 original or relax any input guard. Actual result and cleanup pending.

## Native030 exact activity-admission failure

Fresh030 reproduced the first automatic hCaptcha refusal. The new fixture captured exact public error browser_session_activity_admission_conflict. Before input the original was absent; afterwards it was uncertain/noRetry with owner site/provider refs retained. Application input0 and no protected access remain independently recorded. No029 original was replayed. Root requested a narrow source-grounded activity ownership repair with public regression from the worker, preserving pending-before-focus and foreign/competing activity exclusion. No bypass or input-policy relaxation is authorized. Fresh Native historical census and exact030 X11/kernel-listener readback are retained privately; no new runtime starts before source/gates and a new candidate.

Source diagnosis corrected the initial nested-activity hypothesis: focus admission compares the current complete session document to effects.expected. Native policy evidence publication advances the owner document; refreshing the host cache left the existing effects baseline stale. Approved bounded repair adds exact expected-before atomic owner publication returning its exact post-own-update snapshot, then updates the same effects operation from that proof before exact retained-only focus. Arbitrary fresh reloads, foreign context/epoch/target changes and retirement remain rejected. Worker owns narrow Native host/task-operation mutation plus production-consumed adapter regression; no shared model/store relaxation or new caller bypass. Actual030 is the public RED; persistent provider-free RED/GREEN and a new qualified candidate are required before another Native run. Source/gates pending.

## Owner-publication repair integrated and source-qualified

Worker461e5fd69ee3099a3de0a24417a1dd6e68bac7a3 integrated as5a9f53ebdc09b1c591604a191999bc3e673ae1b0. Persistent public Native-host adapter RED reproduced the real SQLite/effects activity conflict after own policy publication; GREEN and foreign-before/after-target and cooperative-epoch negatives passed with original pending/budget retained and no launch/input. The owner mutation atomically checks exact before-state and pending retained context, returns an unforgeable process-local exact after-state proof, and the same effects adapter reseeds only from that proof before retained-only focus. Store/model/retirement fences remain unchanged.

Clean combined5a9f passed fmt, strict workspace Clippy, MCP155 and optimized ci build. Byte-identical client/docs gates remain valid from the preceding slice. Immutable executable SHA256dffa4eaa051d391f16cba647b0e8bc016c9e1588b2d6e2220c91dbce929f5ffb is not installed. Root released one fresh full031 against that pin and frozen461e consumer after fresh740-generation cleanup and exact external/resource checks. Displays7030/7031, VNC62500/62501 and nonoverlapping owned subnet were checked; no prior uncertain original is replayed. Actual031 result and cleanup pending;470 remains OPEN.

## Native031 visual-fence refusal and separate assignment obligation

Fresh031 on repaired5a9/dffa passed the former activity-conflict point and forwarded exactly one click intent. Actual Native owner returned HTTP400 visual_observation_conflict with no delivery acknowledgement; application input/access remained0 and original uncertain/noRetry. The fixture effectsAcknowledged:false annotation is not independent provider zero-effect proof. Do not replay this original or infer frame drift. Actual prior OCR exposes distinct rendered capture and sourceCapture hashes; which digest the pinned input API requires remains a contract question under read-only source discovery. No hash guard is relaxed.

Child cleanup retained taskClosed:false and assignment-cleanup failure, although final child process observations and outer resources were absent. Root fresh001–031 census independently qualified770 recorded Native generations absent, unknown0, no positive owned runtime resources; exact031 X11/kernel listeners were absent. This settles the resource readback, not the retained assignment disposition, which remains under diagnosis. No new runtime until that obligation and digest contract are resolved.

## Native031 retained disposition and frame diagnostic

Authoritative Remote View source confirms expectedVisual uses the cropped capture SHA256, not sourceCapture. OCR and guard share the screenshot and GdkPixbuf crop path; binary/source equivalence remains unclaimed. No hash substitution or guard relaxation is justified. Exact031 acquired assignment and baseline peer remain active in the offline private pool store, with empty release replay records. Root recorded retain/quarantined disposition privately; process absence does not establish logical release. Do not restart or reuse that namespace to manufacture release evidence.

Root added fixture-only private original OCR PNG plus same-region pre-input and post-refusal capture evidence. The original input intent and expectedVisual remain unchanged and forward at most once; extra calls are observations. Python syntax and patch hygiene passed. Initial032 invocation refused dirty tracked source before namespace creation or runtime effects; source must be frozen before qualification. Existing5a9/dffa runtime is unchanged and needs no rebuild.470 stays OPEN; final review/remediation counts remain0/0.

## Native032 partial success and separate fixture mismatch

Frozen fixturea09254e25 with unchanged5a9/dffa runtime acknowledged exactly one hCaptcha click and independently verified clearance/protected access. Extra pre-input observation changes timing; this is partial actual success, not proof the prior visual conflict was repaired. Turnstile then classified correctly but refused registered_target_unqualified before an input original. Its retained high-confidence glyph bounds and actual checkbox differ vertically by three pixels under the fixture's shared prompt transform; bounded family-specific fixture alignment is under qualification, with sealed locator/profile/confidence unchanged.

032 remains INCOMPLETE. Its later OCR overwrote the first image label, so cross-scene image comparison cannot establish a visual-conflict cause. Root changed private evidence names to include capture hashes, preserving future frames separately. Fresh001–032 census qualifies800 recorded generations absent, unknown0, positive owned resources0. Child exact process cleanup qualified, but subsequent appAbsenceFence failed and blocked assignment release; acquired and baseline032 records are retained offline/quarantined separately. No logical release or470 closure follows from resource absence. No new runtime before source and cleanup diagnosis.

## Native033–035 exact behavior and cleanup diagnosis

033 refused a new unreadable unanchored process tree before challenge input. The process and parent were absent from its initial baselines; later foreign ancestry is advisory and does not reconstruct contemporaneous proof. No ownership exception was added.033 cleanup fully qualified, released its acquired assignment and preserved its peer. A fresh034 baseline was admitted independently; initial occupied port62800 stopped preflight before effects and foreign listeners were preserved.

034 passed every470 behavioral scenario: both checkbox families, passive wait, access denial, image intervention, budget, changed frame, missing provider, expiry and lost-reply no-replay. It remained INCOMPLETE because exact process cleanup25.53seconds plus independent app absence6.38seconds exceeded one shared30second wrapper. Its offline assignment records are retained/quarantined without a release claim. Fresh001–034 census qualified850 recorded generations absent with no positive owned resources.

Worker2074700a integrated ase8f0203a0 separates those two existing cleanup stages, each retaining30seconds and the parent180second reserve. Positive exact completion and no prior errors are required before the absence stage; all existing assignment-release fences remain. Actual extracted orchestration checks pass success, unqualified exact, prior-error, late exact and late absence cases. Existing15 cleanup safety tests pass. No ownership, signal, retry or visual guard is weakened.

035 proved cleanup qualified and released its acquired assignment while preserving its peer. Its hCaptcha attempt safely refused because the exact original capture and pre-input frame differ only within Chrome toolbar bounds48,46 through588,79: the expanded sign-in pill collapsed and the URL bar widened. The owned app pixels remained unchanged. Original input stayed uncertain/noRetry with application0. This is actual frame change, not a crop-hash alias or wrong digest. Bounded static-fixture visual readiness before the client's original attempt is under review; no refresh or replay of that original is authorized.470 remains OPEN and471 stays dependency-gated.

## Bounded readiness and Native036 persistence failure

Worker43ac19ed integrated as8fc392f3b; rootc64681b5e adds exact read-only capture_region routes in both fixture bridges. Qualifier readiness requires stable full-window pixels/exact Native target for three seconds, at most12captures/15seconds, before any classified original or attempt. Stable, transient reset, target change, partial/registered observation, finite failure and deadline cases pass with injected readers/clocks; root independently checked stable and changed-target cases. Original input guards and product/runtime bytes remain unchanged. Sourcec64681b5e is frozen and published.

036 stable readiness and actual hCaptcha/Turnstile/passive/access-denied cases passed. Image-transition setup then failed public observation completion with browser_task_operation_completion_unavailable, before any application click. The MCP wrapper hides the underlying finish/store failure, so cause is under read-only diagnosis rather than assumed journal capacity. This run is INCOMPLETE; no blind retry follows. Cleanup qualified, acquired assignment released and peer preserved. Fresh001–036 census qualified910 recorded generations absent, unknown0 and no positive owned runtime resources.470 remains OPEN; review/remediation counts0/0.

## Ticket470 qualified closeout and471 admission

The [curated470 source receipt](../notes/0296-native-checkbox-470-source-receipt.md) records sourcefe6a/ef948, full bounded037 acceptance, exact cleanup and limitations. Tracker470 is verified CLOSED. Graphiti memory disposition queued: jobd1f36629-5327-4b55-a659-584457d6063e accepted the source-anchored closeout; this is not persistence/retrieval proof and was not retried.

Fresh fetched inventory has eight registered checkouts. Preserve dirty original Auth and all unrelated lane heads. The existing470 auxiliary is clean5984dd10f, with no pinned candidates or active candidate builds; advisory inspection records incarnation877044c62029f5e6424c56ce5315f483878556d9588ff66515d3b2cea689d7ad. Reuse it on challenge/single-round-image-471 from validatedfc607ad2c, preserving its old branch. No new checkout or cleanup effect is required. Parent P296 retains custody and eventual governed removal obligation.

Worker471 owns the bounded single-round image implementation, dedicated client/reference/tests and fixture source. Root owns shared README/help/main skill/CONTEXT, ledger/catalog, service schemas/generated contracts and runtime execution; interface overlaps require explicit serialization before edits. Reuse current owner stores, Native input and sealed pure visual prior art; no retired provider ownership, fake model success, universal puzzle/audio coverage or paused-lane resumption. Actual runtime remains separately scoped after clean source qualification.472 stays blocked;478 remains input-blocked. Final broad review/remediation counts0/0.

## Approved471 bounded interface

471 source work extends existing native_capture/native_input parameters with a separate typed static-image observation and imageAttempt containing only original observation ID plus candidate IDs. The owner derives coordinates from current Native raster/OCR and reuses existing pure visual-round/provider decisions. Generic checkbox configuration grants no image authority: the approved static profile requires exact provider configRef agent-browser:remote-view-native-image-static-3x3:v1, existing captcha_solve capability and owner-verified SitePolicy membership, all policy-bound. Shared capability/schema enums need no expansion on this design.

The approved NEW image profile is one static3x3 English bus-selection round, at most four tiles plus Verify, five clicks/fifteen pointer events/zero keys, one attempt and a60second evidence/deadline bound for actual agent image reasoning. Checkbox10seconds and sealed profiles stay unchanged. New observations or family names cannot reset taskwide consumed attempts; image-open after a spent checkbox remains intervention. Dynamic, partial, ambiguous or unsupported puzzles are outside471.472 separately owns refreshed rounds. Owner current-frame/control/context fences and uncertain-delivery no-replay remain mandatory.

Runtime qualification will use root reasoning from an actual private Native PNG to an untrusted digest-bound tile-ID proposal, then the public owner seam and independent HTTPS oracles. A source test double may exercise selectors but cannot establish actual model selection. Root retains shared schemas, package commands and user documentation; worker owns adapter/kernel/client/dedicated reference/fixture changes. This section approves bounded source design only, not implementation acceptance or runtime effects.

471 coordinate contract correction: root initially described OCR bounds as crop-relative after inspecting PNG cropping. Worker identified the existing absolute-desktop locator contract. Exact Remote View tools/desktop_ocr.py parse_tsv adds region.x/y to word bounds and unions absolute line bounds; the owner returns that result unchanged. A direct provider-helper nonzero-region check passed. New image geometry must subtract the region for raster-local matching and add it exactly once for Native input. No existing checkbox bug or binary/source equivalence is claimed. This correction occurred before a geometry-policy change or runtime effect.

## Completed teaching auxiliary governed closeout

While471 source work continued, root inspected the completed profile-proof auxiliary at cleanace432be6786257fff7b87448a529141d530165f. No pinned candidates or active candidate builds remained. Git cherry proves its sole unmatched commit is patch-equivalent in the published integration branch; no exact-ancestor claim is made. Fresh readable-cwd projection found no positive process match; unreadable entries were not promoted to global absence proof. The active471 Cargo target is independent of this checkout.

Governed operation worktree-closeout-31f54fa3-3d86-4979-8fd7-41711d88a7ee previewed and committed removal of exact incarnationab2bf7a2bc6f309cf8ae61e98afe936cc91baa0f50a4a4ddc840410bd2e53931. The recipes/teaching-grilling-476 branch and its exact commit remain retained. Registered population fell from eight to seven; the active471 auxiliary, dirty original Auth and other lane checkouts were preserved. Terminal receipt and supporting inspection/readbacks remain in user-scoped coordination artifacts.

Teaching auxiliary closeout memory disposition: queued. Group agent_browser_main accepted source-anchored jobab285cfc-2a21-4e7b-bc41-686b4448f187 from the canonical ledger. Queue acceptance is not persistence/retrieval proof; no retry was attempted.

## Ticket471 source integrated, actual image selection pending

Worker clean3f38aa3c26d371b65949eddb879f35f925d0e97e integrated as1b2d70914. Separate typed image observation/input, explicit static provider membership, owner-derived tile/Verify points, original frame/control/task fences, same taskwide attempt budget, durable ACK/uncertain effect accounting, safe static completion diagnostics, and dedicated typed client/harness/reference are present. Worker gates: full MCP160, Native integration21, image focus5, checkbox policy/focus2, strict workspace Clippy, format, service client and dedicated JS/declaration checks PASS. These are source/provider-free gates, not model selection or actual Native acceptance.

Root supplied shared metadata, user guidance, package command and exact-target fixture hooks. The image wire allows only two through five single-click actions with the original visual/control guard; both actual extracted bridge guards passed bounded/malformed/foreign/read-only negatives. Image acceptance additionally requires a real Native owner acknowledgement, independent application checks and exact cleanup. Each of four selection phases uses a fresh nonce and actual private PNG; root will publish an atomic digest-bound untrusted candidate-ID response after image inspection. The private helper contains no classifier or oracle answers. Combined source gates and immutable pin are pending; no actual Native image run or471 closure is claimed.472 stays blocked and final review/remediation remain0/0.

## Native038 first actual image result

Combined cleanb3f7a04e138f1b867e6355388fd2a01444214ecc passed root format/strict workspace Clippy, new image client/types and old checkbox/human clients, docs/parity/handoff/links, then optimized ci build. Immutable executable SHA2561244c2a779d9ef188e90ea959f52592599a6eac05d22c13e95c15de4b4e1b34d is not installed. Actual038 stopped before a selection proposal or any input, returning native_image_workflow_unavailable_observe_original. Private store retains the registered capture original pending with empty evidence; no native_input original exists. Initial original-notfound handling therefore succeeded. Actual OCR and raster match the supported complete grid, with no locator defect established. No semantic tile proposal was submitted and no original was replayed.

038 cleanup qualified, acquired assignment released and baseline peer preserved. Fresh001–038 census qualified970 recorded generations absent, unknown0 and no positive owned resources. Exact underlying public capture/completion error was erased by the client fallback. Root adds qualifier-only bounded error recording for all workflow actions before preserving the original throw; no production client, Native binary, policy, detector or guard changes. Syntax and hygiene passed. A new scoped original is required to observe that cause; unchanged b3/1244 executable and source gates are reused, with separately pinned harness source.471 remains OPEN and472 blocked.

## Native039 actual visual proposal and frame refusal

Fresh039 used unchangedb3/1244 runtime and separately pinnedac6 fixture. Original lookup was accepted, registered Native capture completed and root inspected the actual new PNG. Root selected the three visually observed buses and atomically submitted only nonce/task/observation/frame/puzzle-bound candidate IDs. One four-click owner intent reached Native, which returned visual_observation_conflict without delivery acknowledgement; application selection/Verify/input remained0 and original is uncertain/no-replay. The fixture effectsAcknowledged:false annotation is not independent provider zero-effect proof.

Actual original and pre-input images differ by8642 RGB pixels only in Chrome toolbar bounds48,46 through588,79; puzzle pixels are unchanged. This proves another real frame change, not a wrong image digest or guessed candidate.038's earlier capture-completion failure remains unexplained and retained.039 cleanup qualified, acquired assignment released and peer preserved. Root preserves occupied63400 listener and uses a different free pair; no old original was replayed.

Read-only source trace shows actual retained input focus uses tab selection, Page.bringToFront, native window focus and maximization, not the internal native_focus command. No production focus change is justified. Root changes only owned-fixture preparation to bring primary forward and maximize after each static scene, then run the unchanged bounded pixel readiness before classification. Original expectedVisual,60second image bound, OCR/profile and source binary remain unchanged. One fresh diagnostic is needed to test this bounded preparation hypothesis.

## Native040 rejected preparation hypothesis and exact public focus seam

040 CDP-only bring-to-front/maximize did not settle the transition. Root inspected a fresh PNG and submitted a fresh nonce-bound visual proposal; one four-click intent again reached Native but returned visual_observation_conflict/no acknowledgement, application0. Actual frames differ by8666 RGB pixels only in the same Chrome toolbar rectangle. Original remains uncertain/no-replay.040 cleanup qualified, acquired assignment released and peer preserved; no old proposal or original was replayed.

Source trace distinguishes the missing step: focus_for_view's Native helper maps/raises windows, sets X11 core input focus and root active-window state. CDP-only preparation and Remote View activate are not equivalent to it. Existing public browser_tab select validates ownership and invokes this exact complete focus pipeline, including when selecting the already-current tab. Root therefore adds that existing exact owned-tab selection before bounded readiness/classification. No new endpoint, direct provider-window mutation, production focus change, timing extension or guard relaxation is introduced. This source-backed preparation still requires fresh runtime proof.

## Native041 outcome and explicit image-region scope extension

041 selected the owned current tab before readiness, but its original and pre-input frames again differed only in Chrome toolbar bounds48,46 through588,79 (8641 RGB pixels). Root's fresh actual-image proposal reached one four-click intent; Native refused the original frame, application0, no acknowledgement and no replay. Cleanup qualified, assignment released and peer preserved. Source paths converge on the same focus implementation, but optional helper execution depends on runtime PID/display state and no per-focus receipt proves actual execution. No different focus implementation or new production focus repair is claimed.

Current v1 image policy requires complete_window evidence. Merely passing a crop would violate that contract.471 now adds an explicit IMAGE-ONLY fresh-capture mode: imageCaptureScope complete_puzzle_region plus a declared region, persisted typed captureScope complete_puzzle_region. Legacy observations default only to complete_window and retain region-equals-window validation. The owner must derive complete prompt/brand/nine aligned whole cells/Verify from the same Native pixels/OCR within the ROI; ROI remains inside exact full window/display. Native process, window, generation, geometry, control, policy,60second bound and cumulative budget remain unchanged. Original ROI and hash stay immutable through model selection/input; no old guard is clipped or refreshed.

Generic partialWindow and classification.completeWindow remain honest. A registered complete puzzle region is separate typed evidence, never whole-window clearance; independent HTTPS clearance and protected access remain required. Client captureRegion explicitly requests scope before the original and retains the recorded ROI for subsequent observations. Checkbox/full-window behavior stays unchanged. Source RED rejected the new partial-region mode as expected; owner/client implementation, compatibility/containment/partial-puzzle regressions and fresh combined gates are required before a NEW binary/runtime. Root owns metadata/docs/fixture viewport extent; worker owns typed core/client/reference/tests. This extends camera evidence scope without adding input coordinates, effects or retry authority.

## Explicit image-region source integrated, new runtime pending

Worker clean7fcd29f8994c8bdb17be77b3c81a9381f1c2e449 integrated asa9460b782. Request/response scope enums, compatibility default, strict declared-region capture, owner-derived complete-puzzle proof, unchanged whole-window fences, scoped geometry digest, same-ROI after-observation and explicit typed client captureRegion are present. Generic partial-window classification remains honest and cannot establish clearance. Worker Native22, MCP161, image6, old checkbox policy/focus2, strict Clippy/format and client/types pass. No actual regional input or clearance is claimed.

Root qualifier supplies only the owned viewport EXTENT from app setup, aligned inside independently observed Native window bounds; it reads no DOM puzzle element positions or solution labels. This chooses a fresh entire-page ROI that includes prompt, brand, all cells, Verify and the existing changed-frame marker, while excluding Chrome toolbar pixels. Native raster/OCR and actual root image reasoning remain the sole puzzle-geometry/semantic inputs. The request receipt includes explicit scope/region/window bounds; private response helper verifies PNG dimensions against that ROI. Whole-window mode remains default and all failed earlier originals stay immutable. Combined source gates and a NEW immutable executable are required before fresh042; old B3/1244 artifact is retained, not relabeled.


## Explicit ROI source gates and actual042

Clean source ddb17288ecc6471952c59e3dfd173674a2ee8182 passed combined format, strict workspace Clippy, docs build, image client and strong types, checkbox and human client regressions, service/MCP parity, handoff documentation and five changed-file link checks. Worker ROI gates passed22 Native,161 MCP and6 image cases plus two existing checkbox regressions. The optimized ci build completed in3m55s. Immutable candidate e052ad7185ad6e6dd62c5b23c83221e14289a8721168e75393bc0c2f5f267901 remains private, uninstalled and nonproduction.

Actual042 used a fresh declared complete-puzzle region inside the independently observed Native window. No selection proposal or Native challenge input occurred: registered native_capture completion failed with browser_native_image_observation_expired. This is a new explicit failure observation, not a passing ROI workflow. The original remains retained and will not be replayed. The parent reported INCOMPLETE with no cleanup failure; independent resource readback follows separately. Worker471 owns the focused source diagnosis and public regression; root retains shared guidance, qualifier and runtime custody.471 remains OPEN,472 blocked,478 input-blocked. Reserved final broad review and remediation remain0/0.

Actual042 independent cleanup and fresh001–042 census qualified1090 recorded process generations absent, unknown0, no positive private processes, Docker resources or owned units. Historical quarantined assignments remain retained; process absence does not retroactively release them.


## Safe expiry diagnostics after042

Worker6b3a07b integrated as52b0d5441. Behavioral public-completion RED lacked bounded clock fields; GREEN covers future and expired observations, exact numeric sanitization and preserved pending/no-replay. Image8, Native22, full diagnostic MCP163, format and strict Clippy passed. One prior full-MCP owner-admission failure remains unexplained and retained; subsequent focused and full diagnostic runs passed without production behavior changes between them.042 remains unexplained: only558ms transport timing is retained, not the failed observation/validation timestamps. New diagnostics preserve the existing future/60second predicate and will expose those exact numeric facts in a fresh scoped attempt. No behavioral expiry repair or471 acceptance is claimed.
