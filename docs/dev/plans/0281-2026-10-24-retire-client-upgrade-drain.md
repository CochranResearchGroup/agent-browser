# Retire global upgrade client lockout

State: CLOSED
Product lane: PL-BUGFIX
Owner: ecochran76
Work item: CochranResearchGroup/agent-browser#394
Branch: fix/retire-client-upgrade-drain
Target: main

## Outcome

Ordinary browser opening, navigation, continuation and scoped cleanup must not
consult the persistent workstation admission marker. Installer metadata may
remain for transaction coordination and recovery; it grants no client lockout.
Per-browser owner transfer, profile policy and operation fences remain enforced.

## Custody and boundaries

Fresh origin/main 611f9beef is the baseline. Four checkouts were inspected:
primary authentication work is dirty, three peers are clean. This fifth primary
owns only drain retirement and matching tests/docs. Preserve all peer branches,
candidates and runtime identities. Catalog historical absent-worktree claims
are stale advice; no peer cleanup. User authorized removing the obsolete block.
The exact production marker was archived outside Git under workstation and
coordination locks after selected-binary and supervisor checks. Its failed
upgrade transaction is preserved and remains unaccepted.

## Concurrent integration

Main advanced through PR392/395 dashboard static delivery and PR396 accepted
upgrade recovery. P280 belongs to static delivery; this lane is renumbered P281.
Merge preserves both guidance/catalog additions, the dashboard dependency and
both recovery validators. Client gate removal changes no installer recovery
fence. Requalify merged source before readiness; retain earlier consumer proof
only for its unchanged module and exact fixture scope.

## Validation

Replace old blocking assertions with valid and malformed-marker workflows for
explicit open, implicit open, lazy task tab acquisition and physical recovery.
Prove native dispatch reaches its actual target checks instead of global drain
rejection. Run focused consumer/action and installer contracts, fmt and strict
workspace clippy. Update help, README, skill and installation docs. Verify
production disposable open/close independently of source qualification.

## Closeout

[Acceptance](../notes/0281-2026-10-24-retire-client-upgrade-drain.md) records
203 selected Rust passes, one explicit Native viewer fixture exclusion,
strict Clippy/format, documentation checks, two real Chrome owner-exit cycles,
full workstation fixture and independent zero-residue scans. PR397 owns source
integration. The production marker mitigation passed ordinary MCP open/close;
the original upgrade remains unaccepted and this source binary is unpublished.
No sealed candidate is pinned here. Remove this clean primary through the
worktree closeout helper after integration; preserve all foreign checkouts.
