# P300: Compact workspace, Profiles and SiteGuides

Product lane: PL-PLATFORM
Status: ACTIVE
Spec: https://github.com/CochranResearchGroup/agent-browser/issues/523
Accountable human: ecochran76
Integration branch: platform/spec523-workspace
Integration base: bddb5b3fa9437639215a8431664dd6ef2da88575
Review fixed point: proposed integration base above; explicit user confirmation requested before the single final review.

## Authority and scope

The operator directed publishing tickets and implement-spec coordination, with one root ticket and each other unblocked ticket assigned to one worker. Source work is authorized; production installation, provider mutation, arbitrary private account/media access and profile closure are not inferred. Existing task authority is reused when a concrete effect is qualified. Exactly one final Standards/Spec review and one bounded TDD remediation pass follow ticket validation/closure; no repeated broad review loops.

The accepted prototype revision uses one browser inventory rail and compact tools in the existing thin top toolbar. Native Remote View remains the viewer/control/recording/narration owner. Prototype media, identities and repeat results are synthetic and are not live acceptance evidence. The full self-contained public spec is the tracker authority.

## Custody and shared writers

Reuse the clean canonical checkout serially as integration/root525. Preserve separate Auth-retention and inventory-access506 checkouts and their exact branches/candidates. Three necessary worker auxiliaries are admitted serially from the planning checkpoint, one each for524,526,527; no worktree per additional idle agent. Total registered population becomes six. Worker auxiliaries must be integrated and closed with governed custody receipts, or retained explicitly with unresolved work.

Root owns compact workspace/launcher integration, public HTTP/service wiring, central contracts, root package scripts and shared documentation. Worker524 owns dashboard auth implementation and entry gate. Worker526 owns profile/site identity model, dedicated public library handler/client and profile manager. Worker527 owns copy/experiment UX and dedicated tests. Shared module/action/router edits are supplied as narrow integration patches to root unless explicitly transferred. Root integrates serially and validates before unlocking dependents. PR508/issue506 retains inventory access-error behavior; this lane preserves it and serializes overlapping compact workspace integration. Do not alter the preserved foreign checkout.

## Execution ledger

| Ticket | Outcome | Blocked by | State / owner |
| --- | --- | --- | --- |
| #524 | Use existing ingress sign-in for the compact workspace | none | integrated/b35e2d795; cumulative gates pending |
| #525 | Open manual browsers from the existing compact toolbar | none | running/root |
| #526 | Manage site identities and choose matching profiles | none | integrated/129418c21,73b7ed08d,cd9fa2502,94099363b; root gateway final gate pending |
| #527 | Plan and execute independent profile copies through Profile Lab | none | validated/root932d40070 plus root composition; tracker closure pending |
| #528 | Manage SiteGuides and teach through Remote View recordings | #525, #526 | pending |
| #529 | Qualify the integrated Native workspace and narrated teaching journey | #524, #525, #526, #528 | pending |
| #530 | Qualify real-account Chrome and Chromium profile portability | #527 | pending |

## Validation and acceptance

The stable public seams are authenticated browser task lifecycle/native selection, site/identity-aware public profile choice, and Remote View teaching evidence through reviewed and fresh-repeat-qualified guide acceptance. Real-account copying is a linked acceptance track. Tests prove observable behavior; run red before green one vertical slice at a time using existing harnesses. WSL Rust compilation uses cargo-safe, pnpm runs JS/frontend checks, and hosted CI stays disabled. Root records red/green/source/combined checks per ticket before closure.

Source, installed, provider controls, human narration and real-account usability are separate rows.529 cannot be closed with fixtures or synthetic narration; reuse478 only when its actual context is supplied/resumed.530 cannot be closed by copy completion or synthetic pair evidence. Missing selected media/account/donor context blocks only the named acceptance row, with a concrete preparation packet; it does not stop unrelated source work. Preserve unknown original operations and do not replay them.

## Review ledger

Broad review count: 0. Bounded post-review TDD remediation count: 0. Review diff will be git diff <confirmed-fixed-point>...HEAD. Spec and Standards remain independent; root adjudicates findings before the one remediation pass.

## Implementation checkpoint

Worker auxiliaries524,526,527 retain exact branches under `/tmp/agent-browser-p300-<ticket>`.524 source04fe756e447f4aac37587a5b9b3d735bc4cad301 passed its focused real HTTP owner verification, mounted auth gate, auth status and dashboard TypeScript checks.526 source228079c25fd5918b082e91a89b12b1a5e3679bee plus receipt87e8ab0709f3cbd9d103e3ce32ded748d2790ead passed five public model tests, the service-model suite, mounted manager/client and generated schema parity; registered transport qualification remains running.527 sourcecd10692b33e153e5274b5e84c610749af3bfc91c and docsea6f1879dd233cf3ca0329446fd45031508e9918 passed mounted Profile Lab, all18 original-operation reload paths, documentation links and Remote View documentation checks. Root reran Profile Lab and original-operation checks after integration.

Root525 red/green evidence: public service-model disposable Native posture regression and actual native host regression both passed after reproducing the persistent-only refusal. Disposable exact executable pinning passed its model red/green, including refusal of changed build on the same original task after serialization. Mounted launcher checks pass defaults, account search, explicit conflict, exact returned task, lost-reply preservation and readback without replay. Existing compact desktop/mobile bounded rail scrolling passes with the new compact controls. These are source/fixture checks, not installed Native or real-account acceptance.

Root Cargo uses isolated target `/tmp/agent-browser-p300-root-target`; workers use their own targets. A separate active lane owns the canonical CLI target. No target cache or foreign process was removed. Root model/native focused tests used a reviewed8GiB claim and8GiB cap with8 jobs; full cumulative gates retain ordinary admission. No hosted CI is dispatched.

Shared documentation custody transferred serially527 then524; root retains launcher and cumulative guidance. Profiles transport module/action/router custody remains526 until its followup checkpoint. The selected real donor/account and actual human-narrated recording are requested;529/530 remain unqualified. No ticket is tracker-closed at this checkpoint.

## Integrated source gate checkpoint

Current worker integrations are auth524 b35e2d795/55ad334b1, profile526 cd9fa2502/94099363b, and copies527932d40070 plus prior source/docs commits.527 now retains optional frozen original source mode/build observation in the same owner binding. Qualified Linux loaded-digest evidence is distinguished from on-disk evidence; old/missing observations stay unknown. Public continuation and pure operation lookup preserve that original observation. Focused duplication3/3, mounted Profile Lab and18 original-operation/history paths pass; root reran the mounted Lab after integration. No actual account portability is claimed.

Root cumulative cargo-safe workspace Clippy with warnings denied passed on the integrated working source. Cargo-safe formatting check passed. The complete service-model suite passed, including all integration binaries and compile-fail doctests. Source checkpoint contains unfinished525 acceptance, so this does not close525. Root's mounted launcher, known-no-effect versus uncertain recovery, explicit existing-task return, complete compact workspace, toolbar/manager/Profile Lab composition, profile manager/client and public service/MCP client checks pass. Docs build and links pass.524 integrated auth-status/mounted gate and dashboard production build pass; its repeated frozen build used unchanged hashes across the five relevant UI files, with only existing static-export rewrite warnings.

The actual public dashboard launcher HTTP test passed anonymous/spoof refusal, authenticated catalog, preflight refusal, private/unknown profile protection, original readback and no browser/daemon launch. A final composition rerun adds the authenticated dashboard profile-library forwarder after526's fresh native inventory overlay. Root's disposable Native launch and original-request tests passed, including serialization/reopen and recovery of an existing allocation whose final reply was missing. That recovery acknowledges exact allocation only; navigation remains unconfirmed and is never replayed.

Review count remains0 and bounded final TDD count0. Tests used synthetic fixtures and isolated homes; no production/provider/account/media effects occurred. Draft integration PR: https://github.com/CochranResearchGroup/agent-browser/pull/532.
