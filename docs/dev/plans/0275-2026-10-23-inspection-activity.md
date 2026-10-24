# P275: Inspection reviews in Activity

Status: COMPLETED (source qualified; canonical integration recorded by PR)
Product lane: PL-BUGFIX
Work item: CochranResearchGroup/agent-browser#68
Branch: fix/inspection-activity-68
Owner: ecochran76

## Outcome and current state

A dashboard operator can record an accepted or rejected inspection observation in the existing Activity feed, attributed to the original workspace/browser/task/tab/profile, inspection run, packet hash and evidence references. Inspection currently lives only in Chat component state; operator turns already append Activity events. Reuse Activity's existing 500-event bound, opt-in persistence, reload and Clear behavior. Do not create another server event store or authority system.

Records contain verdict and bounded identity/reference metadata, not prompt, observation prose, private page bodies, raw evidence or ledger paths. The observation remains distinct from the operator's judgment. Recording never executes a suggested action. Persistence stays opt-in; unavailable storage must not be described as durable success. Explicit opposite verdicts append corrections without rewriting evidence; repeated same-verdict intent is idempotent. Late runs and changed selections cannot attach a record to a different task or session.

## Custody and effects

Reuse the clean retained dashboard primary from current origin/main 335cbad. Preserve foreign manual-login and enrollment checkouts and pinned candidates. Root owns product source, docs, CLI help, integration and metadata. A necessary temporary test-only auxiliary may own new mounted fixture/runner files only; contract and source writers are disjoint. No package-manager auto-install in the auxiliary: use exact existing dependency tools, and remove only known dependency symlinks before governed closeout.

No production, private-account, real Codex/provider invocation, Native input, shared skill publication or CI effects. Only owned synthetic browser/component acceptance with intercepted external endpoints is admitted. Actual Native Focus and broader #68 remain separate.

## Stable seams and acceptance

- Mounted Chat inspection/review to actual Activity and its existing local persistence: catches action wiring, metadata attribution, duplicate/correction behavior and real reload; does not qualify a real model or Native operation.
- Existing public Activity action/store boundary under session transitions and failed storage: catches wrong-session retention, duplicate restore, Clear and truthful persistence feedback; no private storage-layout assertions as primary proof.
- Synthetic inspection API responses are the external boundary. Use real application components/stores and browser storage, no mocked internal collaborators.

Start with one failing mounted accept-and-reload slice; implement only enough to pass, then add rejection/correction, scope/session race, privacy and storage-failure slices. Keep red and green receipts. Run TypeScript and adjacent chat/context/Activity checks, dashboard/docs builds, links and diff hygiene. For any Rust help change run cargo-safe fmt/strict workspace Clippy. Update README, CLI help, repository skill below 10000 bytes, MDX and inline guidance.

## Completion

Concrete user workflow, relevant gates, independently absent exact fixture processes, canonical PR integration and remote readback. Remove the clean test auxiliary after preserving reviewed commits. Record one root memory disposition; do not close broad #68 without its separate Native and remaining accepted outcomes.

## Source qualification

Implemented through the existing Activity owner. A bounded local review collection survives daemon stream replacement and shares the existing session key, 500-event cap, opt-in Persist, reload and Clear. Metadata is frozen before inspection awaits. A late A result cannot be recorded under B; returning to exact A permits a historical review retaining distinct observed and reviewed times. Verdict corrections append, and current-verdict repeats deduplicate by original source and reviewer.

The mounted baseline failed on the missing accepted-review control. Eight final mounted workflows pass: explicit acceptance/reload, corrections/repeated intent, persistence off, quota failure with truthful feedback, late response/session isolation/peer-safe Clear, actual WebSocket stream replacement, invalid restored timestamp, and wrong restored selection keys. The latter timestamp regression reproduced an actual Invalid time value page error before the guard repair. Final run intercepted nine synthetic inspections with zero unexpected endpoints or page errors. Seven retained kernel process generations are independently absent, with zero marker references or denied reads. No real model or Native calls were made.

TypeScript, selected-workspace packet, contextual Chat, operator-agent and inspector-action checks pass. Dashboard and docs production builds, strict workspace Clippy, Cargo-safe format, handoff guidance, changed documentation links and patch hygiene pass. Repository skill entry is 9998 bytes. CLI changes are help text only, so workstation mutation fixtures and unrelated Rust execution compartments are outside this slice. The validation selector's production publication/shared skill suggestions are excluded by the explicit effect boundary. CI remains paused.

Test-only auxiliary commits are preserved as ancestors. Its two documented dependency symlinks were removed without altering their targets; clean checkout removal uses governed closeout. No installed runtime or shared skill was changed. Broad #68 stays open for separately admitted actual Native Focus acceptance.
