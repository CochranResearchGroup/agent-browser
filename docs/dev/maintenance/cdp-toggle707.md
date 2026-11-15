# CDP toggle continuation repair

Product lane: PL-BUGFIX
Work item: https://github.com/CochranResearchGroup/agent-browser/issues/707
Branch: fix/cdp-toggle707
Base: 6556af69d
State: SOURCE FIXED; qualification incomplete; time extension pending

The actual external Native workflow completes CDP-off, then leaves the CDP-on
replacement awaiting Native qualification. One replacement process exists;
reconciliation returns `browser_session_launch_publication_conflict` instead
of inspecting that existing launch. This blocks the original navigation-loss
acceptance and must not trigger another process launch.

Reuse the clean canonical primary. Own host refresh, observation-only window
readiness, profile startup arguments and focused regression tests; preserve
the completed mode649 source, all foreign clean checkouts,
retained candidates, original uncertain requests and the exact pending test
replacement. Runtime identifiers and synthetic browser artifacts stay outside Git.

First reproduce the owner refresh failure with the real SQLite launch ledger
and RemoteViewSessionEffects. A peer observation must not force an unqualified
replacement to publish against the original process state. Retain durable launch
custody, local changes and no-relaunch fencing. Then qualify the original
external operation and assert its URL before navigation. The saved-page defect,
Chromium row and full toggle acceptance remain open until observed.

Use one bounded source repair and review cycle. Run focused tests, Rustfmt,
strict workspace Clippy and selected local checks before merge readiness.
No hosted CI. Production effects require the existing sealed candidate and
preserving installation gates; source checks do not qualify the live workflow.

## Reproduction

The external Native workflow completed CDP-off with one launch and correct
debugging posture. CDP-on launched one exact replacement, then retained
`ReplacementPending` with `mode_operation_native_unavailable`. Explicit recovery
returned `browser_session_launch_publication_conflict`. Native observation
finds the replacement's window titled `about:blank - Google Chrome`.

A four-second isolated Chrome restart probe reuses the observed CDP replacement
arguments with a fresh synthetic profile. It loses the original page in4.532s.
Changing only the saved-session restore flag retains the URL and localStorage
marker in4.300s. This isolates the missing startup request; it is not full
external UI acceptance.

Three Rust regressions cover startup intent, pending-launch refresh after a
peer update, and observation-only window readiness after a peer update. Their
first run is queued through Cargo Signal and the repository resource wrapper,
using four jobs, an8GiB claim and hard limit, and the unchanged16GiB host reserve.
The prior default claim was cancelled while waiting, before compilation.
Policy wiring11 cases and changed documentation links pass.

The owner-refresh and startup regressions failed with the expected publication
conflict and missing restore argument. The readiness fixture initially lacked
its catalog; after fixture correction it failed with
`browser_native_target_changed:refresh task`. These are distinct receipts;
the initial fixture failure is not counted as readiness evidence.

The focused `pending_mode_replacement` group now passes5 cases. Owner refresh
begins a fresh observation scope only when no local logical delta exists,
retaining the durable launch record. Both startup paths share the restore
decision. Observation-only readiness keeps process/window identity checks and
leaves publication and effectful input state fences intact. Broader host/runtime,
mode/control, formatting and strict workspace Clippy gates were selected.

## Shared refresh dependency

Concurrent work711 independently reproduces the same cached pending-launch
failure across two profiles. Its worktree is preserved.711 owns integration
of the shared refresh predicate and its two-profile regression;707 owns saved
session startup, observation-only readiness and the mode-specific regression.
The compatible predicate currently in this checkout is frozen validation input.
Integrate711 first, then consume its qualified predicate before707 merges;
do not publish two competing implementations. Its separate runtime-host restart
preserved the pending replacement and changes the installed baseline, so live
recovery requires a fresh process readback.

## Source checkpoint

Focused5, host40, runtime36, Rustfmt and strict workspace Clippy pass. Runtime
qualification retains8 intentionally ignored cases. Pool compilation was
interrupted at the execution ceiling; pool and mode/control gates remain
pending. The qualification job exited130 with76 passing tests and no test
failures. This custody checkpoint is not merge-ready.
The original installed operation reconciled after711's host restart with its
single replacement intact and still returned aboutblank. Manual navigation
confirmed the original storage marker. Its ordinary close reported residue;
one freshly observed, exact-process Native window close then ended that process.
Peer browser processes remain protected; disposable cleanup follows normal GC.

Review against6556af69d separates Standards and Spec. No source standards blocker
was found. Spec requires installed external acceptance and the qualified711
dependency before this work is complete. Effectful Native guards, atomic launch
publication and exact process/window joins are retained.

Draft PR712 preserves commit3c865df09 and this source checkpoint. The recorded
two-hour overall diagnostic ceiling expired; a one-hour extension was requested
and remains unanswered. No production candidate, installation or fixed-build
external acceptance is claimed. Resume the pending local gates and qualified711
dependency before candidate qualification and fresh supported-browser rows.
