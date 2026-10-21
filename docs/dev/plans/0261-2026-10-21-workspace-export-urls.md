# P261: workspace export URL redaction

Status: CLOSED
Product lane: PL-BUGFIX
Work item: CochranResearchGroup/agent-browser#341
Branch: fix/workspace-export-url-redaction
Owner: ecochran76

## Outcome and custody

Bounded child of #68: remove URL userinfo, query and fragment values from
exported workspace diagnostics, chat packets and summaries. Omit opaque URL
payloads while retaining useful HTTP/WebSocket origin/path and task identity.
Runtime URLs and user-requested Copy URL remain untouched. This is not a
guarantee to remove arbitrary secrets from free text or URL path segments.

Reuse clean dashboard primary from canonical 324508a55; preserve foreign
root/resource worktrees and sealed candidates. This writer owns one pure export
redactor and selected-context/chat adapters. P260 exact tab selection stays
unchanged. No production, provider, browser, account, cleanup or CI effects.

## Feedback and qualification

The synthetic diagnostic regression fails with raw page and WebSocket URL
credentials in multiple exported fields. Verify top-level chat and nested
evidence redaction, opaque data payloads, unchanged runtime source and useful
IDs. Run context/chat/console/inspector checks, typecheck, dashboard/docs builds
and documentation contracts; Rust help changes require fmt and strict Clippy.

## Accepted source outcome

Both real diagnostic/chat URL regressions are red before repair and green
afterward. Affected projection/chat/console/inspector tests, typecheck, final
dashboard/docs builds, format, strict Clippy, policy/parity and links pass.
[Acceptance](../notes/0261-2026-10-21-workspace-export-urls.md) records redaction
limits and source immutability. No production effect; broader #68 stays open.
