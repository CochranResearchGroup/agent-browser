# Retire global upgrade client lockout

State: OPEN
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

## Validation

Replace old blocking assertions with valid and malformed-marker workflows for
explicit open, implicit open, lazy task tab acquisition and physical recovery.
Prove native dispatch reaches its actual target checks instead of global drain
rejection. Run focused consumer/action and installer contracts, fmt and strict
workspace clippy. Update help, README, skill and installation docs. Verify
production disposable open/close independently of source qualification.
