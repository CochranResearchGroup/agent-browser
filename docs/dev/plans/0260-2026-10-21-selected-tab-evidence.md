# P260: exact selected-tab evidence

Status: CLOSED
Product lane: PL-BUGFIX
Work item: CochranResearchGroup/agent-browser#339
Branch: fix/selected-tab-evidence
Owner: ecochran76

## Outcome and custody

Bounded child of #68: diagnostics and contextual chat must describe the exact
selected owned tab rather than its browser default. Normalize logical/target
handles and resolve a secondary target-only selection; missing or foreign
targets produce no substituted page. Preserve browser-wide evidence and the
existing no-explicit-tab default. No new inspector UI or App Intelligence action.

Reuse clean dashboard primary from freshly fetched a1d1c61c8. Preserve foreign
root/resource worktrees and sealed candidates. This writer owns selected-context
projection and its pure fixture only; P237/P248 native selection is unchanged.
No production, provider, account, browser cleanup or CI effects.

## Feedback and acceptance

Two-tab real projection regression fails: tab-second selects tab-live. Qualify
logical/target and prefixed handles, secondary target-only lookup, missing and
foreign targets, diagnostics and chat. Run affected context/chat/projection
checks, dashboard typecheck/build and documentation/planning checks.

## Accepted source outcome

The real two-tab regression is red before the repair and green afterward.
Context/chat/projection/inspector checks, typecheck, final dashboard/docs builds,
format, strict workspace Clippy, policy/parity and links pass.
[Acceptance](../notes/0260-2026-10-21-selected-tab-evidence.md) records exact scope
and runtime limits. #339 is complete; broader #68 remains open.
