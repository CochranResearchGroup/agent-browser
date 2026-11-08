# Retained handoff retirement recovery

Product lane: PL-BUGFIX
Work item: 579
Branch: fix/retained-handoff-retirement-579
Base: bfdf162b896aec462e8341a57bfcec89dc8fefad
State: OPEN

## Current State

Public viewer reproduction is unavailable after a durably settled original browser close. The exact Native retirement remains prepared and nonterminal because physical close publication was interrupted; logical task/browser/handoff rows still exist. #570 qualified only already-ended logical scopes. No shared runtime state has been edited.

## Outcome and bounds

Recover the same paused task and handoff after the original dead browser's settled close, without replaying close or website input. One isolated repair lane, one deterministic host/SQLite regression and one bounded fix. Preserve canonical dirty CONTEXT.md, existing peers and source worktrees. Current root plus four existing source checkouts were inventoried; all auxiliary roots are clean, root CONTEXT.md is foreign dirty state. Root source equals refreshed origin/main. The existing ops/docs/runtime source lanes are retained by their owners; no checkout is repurposed or removed. This sixth checkout owns only #579 and source candidate custody, with cleanup after integration or durable handoff.

Acceptance requires exact original Native fence readback, settled effects, unchanged physical identity, scoped atomic publication, same logical authority and untouched unrelated scopes. Unknown effects, live/reused original processes, provider uncertainty and identity changes stay fenced. No manual custody edits, reset, profile deletion, auth weakening, lock removal, generic return or foreign closure. Coordinator owns any later install and actual viewer acceptance.

## Validation

Use the existing public host and SQLite fixture at the recovery boundary. Demonstrate RED on retained logical rows, then GREEN without another physical close; include negative controls and current #570 regressions. Local required CLI/model tests, format and strict Clippy apply. Hosted CI is skipped under operator policy. Record commands, resource budget, exact source result and missing live proof. Do not consume Overleaf's final broad review/remediation counters.

Causal finding: one settled close with exact frozen browser/tab equality is still unacknowledged because `record_publication(current)` requires an idle-closed marker. The #570 request filter cannot reconcile retained rows lacking that marker. Proposed repair must atomically publish only the settled physical-close projection after fresh original/profile absence and exact Native readback, then reload the host baseline before ordinary launch. Keep original transaction and logical task authority intact.


## Adjacent qualification disposition

The initial qualified CLI artifact passed the new public/SQLite regression and 72 retirement tests. Handoff and recovery selections retained three first failures. Two stale fixture expectations conflict with byte-identical baseline dispatch: handoff resolution refreshes durable state before lookup, while Native presentation refreshes and reuses the already-recreated live owned target independently of compatibility view issuance. The coordinator accepted test-only corrections: require the exact ready browser/tab/target with unchanged launches, view requests and full saved aggregate; require a deleted refreshed handoff to report missing with all original peer/effect/state assertions retained. This is source-causal exclusion against the exact base, not a baseline-binary execution claim. A third unchanged 20ms-lock fixture failed its initial snapshot during compilation, then passed alone on the identical artifact; retain both receipts and rerun in the integrated gate. The integration worker owns corrected-case compilation and execution once model compilation exits. No product behavior changed for these fixture corrections.
