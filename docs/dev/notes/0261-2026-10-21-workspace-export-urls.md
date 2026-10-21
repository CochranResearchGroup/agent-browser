# P261 workspace export URL acceptance

Product lane: PL-BUGFIX
Disposition: accepted-evidence
Owning plan or work item: docs/dev/plans/0261-2026-10-21-workspace-export-urls.md / CochranResearchGroup/agent-browser#341
Related lanes: P260, #68

## Reproduction and repair

The existing copied selected-workspace diagnostic bundle retained URL userinfo,
query and fragment values in page, stream and evidence fields. Chat's existing
sensitive-key redaction also missed its top-level page URL. Synthetic sentinels
reproduced both defects before implementation. An initial chat test referenced
a helper from the other fixture; correcting that harness mistake yielded the
real raw-URL-versus-sanitized-URL failure. No private data was used.

A dependency-free export projection now recursively sanitizes HTTP/WebSocket
URLs in diagnostics, chat packets and summaries. It strips userinfo, queries
and fragments, retaining origins and paths. Relative URL fields receive the
same treatment; filesystem paths remain literal. Opaque data/blob/javascript
URLs cause the whole export string to be omitted, including whitespace/markup
payloads. This avoids preserving a tail after a partial URL match. Existing
chat sensitive-key filtering remains intact.

The projection creates new objects/arrays and never edits live URLs. Browser
selection, native navigation, explicit Copy URL, task identity, resource paths
and operational authority remain unchanged. This is a bounded URL-component
repair, not a guarantee to remove arbitrary secrets from prose or URL paths.

## Qualification

Diagnostic and chat regressions fail on prior source and pass after the repair.
Cases cover credentials and arbitrary query/fragment values, nested evidence,
WebSocket and relative stream URLs, whole opaque markup payload omission,
filesystem path preservation and source immutability. Existing context/chat
packet, console, contextual chat, inspector actions, view projection and
workspace inspector gates pass. Typecheck, final dashboard build, docs build,
Cargo-safe format and strict workspace Clippy pass, along with policy wiring,
service/API/MCP parity, documentation links and P261 planning audit.

Standards review checks pure export ownership, field scoping and unchanged
source values. Spec review checks both top-level and nested exports and keeps
URL sanitization limits explicit. No unresolved findings.

## Delivery boundary

Source only. No browser launch, installed publication, service restart, native
provider operation, private-account access, shared skill publication, process
cleanup or CI dispatch. #341 is the bounded repair; #68 remains open. Foreign
root/resource worktrees and sealed candidates remain preserved.
