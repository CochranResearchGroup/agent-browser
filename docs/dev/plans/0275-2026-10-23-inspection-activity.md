# P275: Inspection reviews in Activity

Status: OPEN
Product lane: PL-BUGFIX
Work item: CochranResearchGroup/agent-browser#68
Branch: fix/inspection-activity-68
Owner: ecochran76

## Outcome and current state

A dashboard operator can record an accepted or rejected inspection observation in the existing Activity feed, attributed to the original workspace/browser/task/tab/profile, inspection run, packet hash and evidence references. Inspection currently lives only in Chat component state; operator turns already append Activity events. Reuse Activity's existing500-event bound, opt-in persistence, reload and Clear behavior. Do not create another server event store or authority system.

Records contain verdict and bounded identity/reference metadata, not prompt, observation prose, private page bodies, raw evidence or ledger paths. The observation remains distinct from the operator's judgment. Recording never executes a suggested action. Persistence stays opt-in; unavailable storage must not be described as durable success. Explicit opposite verdicts append corrections without rewriting evidence; repeated same-verdict intent is idempotent. Late runs and changed selections cannot attach a record to a different task or session.

## Custody and effects

Reuse the clean retained dashboard primary from current origin/main335cbad. Preserve foreign manual-login and enrollment checkouts and pinned candidates. Root owns product source, docs, CLI help, integration and metadata. A necessary temporary test-only auxiliary may own new mounted fixture/runner files only; contract and source writers are disjoint. No package-manager auto-install in the auxiliary: use exact existing dependency tools, and remove only known dependency symlinks before governed closeout.

No production, private-account, real Codex/provider invocation, Native input, shared skill publication or CI effects. Only owned synthetic browser/component acceptance with intercepted external endpoints is admitted. Actual Native Focus and broader68 remain separate.

## Stable seams and acceptance

- Mounted Chat inspection/review to actual Activity and its existing local persistence: catches action wiring, metadata attribution, duplicate/correction behavior and real reload; does not qualify a real model or Native operation.
- Existing public Activity action/store boundary under session transitions and failed storage: catches wrong-session retention, duplicate restore, Clear and truthful persistence feedback; no private storage-layout assertions as primary proof.
- Synthetic inspection API responses are the external boundary. Use real application components/stores and browser storage, no mocked internal collaborators.

Start with one failing mounted accept-and-reload slice; implement only enough to pass, then add rejection/correction, scope/session race, privacy and storage-failure slices. Keep red and green receipts. Run TypeScript and adjacent chat/context/Activity checks, dashboard/docs builds, links and diff hygiene. For any Rust help change run cargo-safe fmt/strict workspace Clippy. Update README, CLI help, repository skill below10000bytes, MDX and inline guidance.

## Completion

Concrete user workflow, relevant gates, independently absent exact fixture processes, canonical PR integration and remote readback. Remove the clean test auxiliary after preserving reviewed commits. Record one root memory disposition; do not close broad68 without its separate Native and remaining accepted outcomes.
