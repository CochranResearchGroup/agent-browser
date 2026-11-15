# Unqualified launch refresh

Product lane: PL-BUGFIX
State: OPEN
Work item: CochranResearchGroup/agent-browser#711
Branch: fix/request-host-isolation711
Baseline: 6556af69d1695445dd646b12a6f64fe0becdd96d

## Current State

Runtime recovery restored DNS Buddy Grok acceptance, but permanent host refresh repair is not integrated. An unrelated peer update causes refresh to publish a pending observed launch despite no logical state delta. The existing operation owner already preserves its durable journal while clearing temporary pending state for a new operation.

## Packet

One coordinator owns the host refresh predicate, a real-owner/SQLite two-profile regression, and source-bound validation. Preserve the dirty CDP-TOGGLE-707 checkout. Its compatible pending-refresh change overlaps the same predicate; use the same narrow behavior and integrate serially without importing its other mode/startup changes.

Acceptance: Bob publishes a usable task after Alice has an observed but unqualified replacement and a peer bookkeeping update; Alice's session/browser and original unpublished launch record remain unchanged, with no Alice replay. Existing publication of genuine local deltas remains protected.

Non-goals: per-profile host architecture, journal deletion, automatic qualification, private input, live provider calls, shared-state edits or another broad review campaign.

Bounds: one red-capable regression; one minimal fix; one focused review/remediation cycle; 90 minute overall packet ceiling. Reassess after two no-outcome checkpoints or 30 minutes. Use capped local Cargo, no hosted CI. No live installation before source qualification and a reviewable candidate.

## Validation record

Initial Cargo invocation selected all package test targets. The unrelated `remote_view_native_control` integration target fails to compile at the frozen baseline because its test crate omits `native_login_perception` and `native_login_ocr`. This is not a red regression verdict. Preserve the failure receipt and run the intended CLI unit target explicitly with `--bin agent-browser`; no unrelated integration source changes belong in this packet.
