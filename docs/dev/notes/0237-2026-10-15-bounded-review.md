# P237 bounded integration review

Product lane: PL-PLATFORM
Disposition: active-input
Owning plan or work item: P237, #261
Related lanes: P238 workflow compatibility, P240 resource bounds

## Frozen packet

One broad review, executed serially in the primary context after all seven tickets reached locally validated. Baseline `3e5a38679d28ac8a20439e29004ea1315475ef8e`; frozen head `bc5bdc38c764a51202cd42d34b7e56e7366d2d69`. Commands: `git diff 3e5a38679d28ac8a20439e29004ea1315475ef8e...bc5bdc38c764a51202cd42d34b7e56e7366d2d69` and the matching two-dot commit list. Approved issue261 synthesized specification and plan0237 version2 govern. No prototype source is adopted. The original baseline includes integrated P238/P240 source; review covers their compatibility at the P237 joins.

Standards sources: AGENTS.md, CONTEXT.md, policies0010,0031,0033,0036,0042,0054 and the code-review skill's labelled smell baseline. No CONTRIBUTING.md or CODING_STANDARDS.md is present. Tool-enforced formatting, types, parity, schema and lint checks are evaluated through their retained receipts. High-risk source review covers dashboard routing and all native workspace controls, public/authenticated locator and private callback boundaries, operator identity/focus/close authorization, owner publication/recovery, native first-use and retirement contracts, SQLite custody/publication, verified process descendants, and workflow MCP transport/tab dispatch. Fixture, contract, client and guidance coverage uses the cumulative changed-surface tests and reconciled native receipts. This is source review; it does not qualify installed publication or public ingress.

## Standards

- S1, nonblocking_backlog, judgement call, owner P237/#105: terminal retirement custody remains in the SQLite ledger document and ledger() decodes/validates all records during admission. This preserves required evidence but can grow admission work with history. Follow-up should bound active decode cost while preserving attributable terminal evidence; do not delete or truncate unknown effects. Source: browser_idle_retirement_store.rs ledger(), reject_retirement_admission() and terminal publication. The wider resource convergence program remains separate.
- S2, blocking low, owner P237: the active ticket283 table row still says its next step is default retirement/native qualification after the section records those gates passed. Policy0010 requires truthful current limitations. Reconcile that validation cell during closeout.

Architecture review accepts the existing browser owner, SQLite transaction and native provider fence boundaries; no second viewer/auth ceremony or effect authority is granted by a model record. The new source checks and private fixture receipts remain distinct from installed effects. The accidental historical live-test selection is separately retained as an execution incident; it is not successful acceptance.

## Spec

- P1, blocking high, owner P237/P238 compatibility: workflow.rs tab(close) focuses the requested owned tab, then sends browser_session_tab_close with only sessionId. The owner closes its current tab. A concurrent selection between these commands can close another tab within the task. This violates exact target intent and peer preservation; bind an optional expected tab identity at the owner close boundary, send it from MCP, and reject stale identity before close effects. Preserve legacy callers that omit the expectation. Source: workflow.rs tab(), browser_session_host.rs browser_session_tab_close branch.
- P2, blocking medium: issue261 implementation decision says “Use small accessible SVG actions”; native-handoff-drawer.tsx and native-handoff-delete.tsx render row actions as text only. Keep accessible names/keyboard behavior while making the row controls SVG.
- P3, blocking medium: approved solution requires a content-sized animated drawer, with the implementation decision anchoring it alongside the rail. globals.css centers it using the shared modal geometry and sets animation:none for all motion preferences. Position the content-sized drawer next to the rail on desktop, retain narrow-screen usability, and animate only when motion is allowed.
- P4, blocking medium: user story12 requires Properties with supplied “task, job, request” context, and the issue's Properties section includes creation time. native-tab-actions.tsx omits jobId, requestId and creation despite BrowserTaskMetadata already carrying them. Display them with Unknown for absent values and retain coherent metadata through recreation.

Other reviewed requirements reconcile to the seven-ticket native/source proof map: authenticated exact native selection and stale exclusion; independent lifetimes, revocation and last-tab behavior; original-link recovery without lost-memory claims; first-use qualification and pressure protections; sole default with optional diagnostics; unchanged HTTP/client/auth/profile boundaries. Explicit native-only ignores are not passes.

## Adjudication and remediation boundary

Accepted Standards findings: one low blocking documentation correction and one nonblocking performance follow-up. Accepted Spec findings: four blockers, highest high for the close race. Start exactly one bounded remediation pass for S2 and P1 through P4. Establish failing focused checks, repair only these findings, then verify their regressions and affected critical workflow. Do not run a second broad review. This review does not authorize push, merge, publication, CI, installed/provider-home mutation or tracker closure.
