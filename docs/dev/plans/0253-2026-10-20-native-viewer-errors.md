# P253: actionable Native viewer failures

Status: CLOSED
Lane: P253
Product lane: PL-BUGFIX
Work item: CochranResearchGroup/agent-browser#329
Branch: fix/native-viewer-errors
Owner: ecochran76

## Outcome

Distinguish failed handoff preparation, known operator-access denial, unreadable embedded page and readable non-viewer page. Explain only observed evidence, preserve the opaque handoff and exact selected task, offer explicit retry and refresh without automatic effects. Remote View retains authentication and display authority.

## Custody and bounds

Reuse the clean dashboard primary from canonical 738311552. Own NativeTaskViewport, its mounted fixture/regressions, and the existing client remote-view helper refusal check plus relevant guidance; foreign stale-session and resource worktrees remain untouched. Retain pinned candidates. One failure-classification repair and its docs; no new authentication, viewer, health polling, runtime restart or browser launch. Source qualification is separate from installed acceptance.

## Qualification

First reproduce the readable503 Native page through the actual mounted component; require distinct failure evidence instead of generic sign-in guidance. Add cross-origin access-page and retry recovery checks; retain existing selection/census/late-response guards, typed access-policy coverage and diagnostic redaction. Run focused mounted tests, inspector action gates, dashboard build and applicable documentation checks. No Rust source changes. Resolve materially new findings locally or report the exact unresolved gate.

## Source acceptance

[Accepted source evidence](../notes/0253-2026-10-20-native-viewer-errors.md) records the original page failure and typed HTTP200 refusal regressions, 13 mounted scenario groups, clients, builds, formatting and strict Clippy. The existing full compact fixture remains explicitly red and is tracked in #330; its source was preserved. Production binary publication and live acceptance remain separate; no installed repair is claimed.
