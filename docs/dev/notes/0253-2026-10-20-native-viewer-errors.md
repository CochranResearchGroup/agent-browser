# P253 Native viewer failure source qualification

Product lane: PL-BUGFIX
Disposition: accepted-evidence
Owning plan or work item: [P253](../plans/0253-2026-10-20-native-viewer-errors.md), CochranResearchGroup/agent-browser#329
Related lanes: P251, P248, UX301

## Result

The dashboard distinguishes four observable failures: handoff preparation, explicit operator-access policy refusal, a readable non-viewer page and an unreadable embedded access page. It retains the opaque handoff when preparation succeeded. Unreadable content does not prove expired login; readable error content is not reflected into the dashboard. Refresh browsers reads inventory. Retry explicitly prepares the selected task again; no failure automatically repeats focus or repairs services. Remote View retains authentication and connection ownership.

The existing client remote-view helper now preserves a typed operation refusal before validating viewer readiness. A normal HTTP200 failure previously became a missing-readiness error, losing policy recourse. No public request shape or authentication implementation changes.

## Evidence

The actual mounted NativeTaskViewport first fails a synthetic same-origin HTTP503 case because no distinct viewer-page state exists. After the repair, it distinguishes that page from a cross-origin access redirect, retains the opaque link and recovers through one explicit retry. Refresh reads inventory without a new focus operation. Typed policy denials at HTTP400 and HTTP200 retain bounded access guidance without raw producer diagnostics or an invented handoff. Existing single-focus, maximize, census expiry, confirmed absence and latest-selection checks remain.

The added HTTP200 denial case initially fails against the unmodified client helper; the refusal check makes it pass. The final mounted suite has 13 reported scenario groups and no page errors. The full service-client bundle, service API/MCP parity, dashboard build/type checks, docs build, inspector-action contracts, Remote View handoff docs, Rust formatting and strict workspace Clippy pass. Rust changes are help text only; no Rust behavioral-test claim is made. No CI was enabled or dispatched.

The existing full compact-workspace fixture remains red against prior integrated behavior: it expects absent Properties provenance as Unknown rows and an independent pre-focus request. Exploratory fixture alignment reached its stale view_focus assertion; those fixture edits were reverted. Issue330 owns complete fixture reconciliation. This is not described as a passing full compact suite. The focused mounted suite tests the production component and actual client through real HTTP boundaries.

## Publication boundary

This is source qualification. The installed production dashboard remains the P251 candidate; this repair has not been published as a binary or accepted in a live desktop workflow. The development skill directory was synchronized and its status verified; the shared production skill is deliberately retained with the installed generation until the next qualified publication. The source guidance and public help describe the new behavior. No existing browser/profile, Native desktop, authentication account or provider was changed. All test browsers are disposable fixture processes closed by their harnesses.

Private red/green logs and development guidance receipts remain outside Git in user-scoped state. The dashboard primary is retained for continuing work, along with prior pinned candidates; foreign stale-session and resource checkouts remain untouched.
