# P263: installed existing-profile download acceptance

Status: CLOSED
Product lane: PL-BUGFIX
Work item: CochranResearchGroup/agent-browser#284
Branch: fix/installed-download-284
Owner: ecochran76

## Bounded outcome

Requalify the original #284 public stdio MCP download workflow against the
current installed executable, with an existing synthetic persistent profile and
reviewed production-selected Linux stealthcdp browser. Preserve exact file bytes
and task/browser/tab continuity, including stale/malformed historical custody.
Repair only stale fixture producer context if necessary; no new product behavior.
The first retained-fixture run fails at missing serviceName before download and
cleans up to zero owned residue. Do not call that the original product defect.

## Custody and effects

Reuse clean dashboard primary from canonical a268438cc. Preserve sealed candidates
and foreign root/resource worktrees. This writer owns smoke-mcp-workflow-download
and its acceptance documentation only. P243 source remains authoritative.
All profiles, SQLite, runtime hosts and fault injection stay in disposable private
HOME/XDG/socket namespaces. No production installation/restart, provider, real
account, ingress, auth or CI effect.

## Acceptance

Pin installed and browser SHA256; run ordinary/stale/malformed public fixtures
serially and require exact bytes, stable task identity, concurrent live process
continuity where exercised and fresh zero-owned-process cleanup. Preserve failure
logs. Document the synthetic installed-artifact boundary separately from historical
manual service-recovery/presentation observations and real-account acceptance.

## Accepted outcome

All three current installed-artifact public MCP fixtures pass in about eight
seconds each, preserve executable/process/task identity and leave zero owned
residue. [Acceptance](../notes/0263-2026-10-21-installed-download-acceptance.md)
records hashes, the corrected stale fixture and the original-manual-recovery /
real-account boundaries. No product behavior or production service changed.
