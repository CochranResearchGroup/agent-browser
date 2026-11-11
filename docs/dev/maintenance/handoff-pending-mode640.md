# Live handoff during pending mode change

Product lane: PL-BUGFIX
Work item: 640
Owner: ecochran76
Branch: fix/handoff-pending-mode
Base: 61fdd9432582606d304f193f8f3ba4b2efefb42d
State: ACTIVE

A valid durable handoff must present the exact original live browser even when a
mode conversion remains pending. Reconnection must not resume or cancel conversion,
erase the original operation, close the browser, or launch a replacement.
Retain ordinary recovery for absent originals and preserve its operation fences.

Reuse the clean canonical checkout without adding a worktree. The inventory has
nine registrations, including five missing checkouts; preserve every foreign
registration and dirty compact596 source. This task claims only its existing
checkout, browser_session_host native handoff resolution and regression test,
and bounded guidance. Adjacent585 retirement and631/638 return remain unchanged.

Validation: original authenticated503 and owner resolve refusal captured privately.
Add real host-path live/absent regression before repair; run focused Rust, format,
strict workspace Clippy and relevant handoff tests. Merge through a linked PR.
Prior operator authorization covers preserving deployment and installed same-link
acceptance; never replay the original mode operation or enter website input.

## Implementation checkpoint

The actual owner resolver regression uses SQLite with a retained Closing mode
operation. Before repair it refuses the live original with
browser_profile_operation_in_progress in3.79seconds. After repair both live and
absent branches pass in9.08seconds: live yields the exact original browser/tab,
absent retains the mode fence, no replacement launches, and the complete operation
journal compares unchanged. The initial JSON-only fixture failure is retained as
invalid setup rather than product RED evidence.

Native handoff resolution now checks original liveness before entering recovery.
It does not weaken ordinary command, lifecycle, link, or provider identity checks.
The production defect was a recovery precondition applied to an already-live
presentation, rather than a broken link or failed authentication.

Focused regression, Rust format, policy wiring11cases, documentation links,
handoff guidance, actual dashboard policy-refresh fixture and docs build pass.
Strict workspace Clippy and related host regressions remain pending. This is an
intermediate source checkpoint, not merge or installed acceptance evidence.
