# Worktree reconciliation, 2026-11-09

Product lane: PL-BUGFIX
Work item: 613
Owner: ecochran76
State: COMPLETED
Next review: 2026-11-10

The operator requested worktree reconciliation before the retained-handoff repair.
The registered population fell from eleven to five through six durable closeout
operations. No new checkout was created for this reconciliation. Source branches
were preserved. Five pinned candidates were archived, byte verified and resolved
by fresh-process lookup. Supporting local binary and build-state receipts were
also preserved privately. No candidate was discarded.

## Closed checkouts

| Branch | Source custody | Candidate disposition |
| --- | --- | --- |
| ops/interactive-human-handoff-deploy | d38e038016a472953f67f0eed3596f8cc0bc56d5 in origin/main | One archived |
| docs/publication-reconciliation-acceptance | 238110466207e0dee94463caca2863cea33f56a6 in origin/main | Two archived |
| docs/runtime-reset-deployment | 75ee7405d97e84fc399ae39db97f73e028ab6697 in origin/main | One archived |
| auth/enrolled-helper-overleaf97 | ed1ebee86e3f21f99918e888706d997ccc060a2c in origin/main | No pins |
| fix/overleaf97-dependencies579580 | fbc65f6fd68bfc5019caa215490ceaf32139d7e8 in origin/main | No pins |
| fix/cleanup102-publication-trace | d1497095057dc45197efc8277dc7814359231f71 on matching local and origin branch | One archived; unmerged source retained |

Worker-status source is integrated, but three live process working directories
still refer to its retained checkout.
The cleanup102 checkpoint is deliberately preserved rather than called merged.
Its closed diagnostic outcome does not require keeping an idle checkout.

## Retained checkouts

| Branch | Continuing purpose |
| --- | --- |
| fix/607-operator-mode-relaunch | Canonical active source lane, PR608 and observed build claims; preserve all candidate pins |
| auth/native-saved-login596 | Active source lane and PR609; preserve its sealed candidate |
| fix/terminal-control-reconcile611 | Active source lane, PR612 and live candidate executables; preserve both candidate pins |
| fix/worker-status-routing | Integrated source retained solely for three live process working directories |
| fix/authelia-workspace-authority | Integrated source in the former579 checkout retained for a live candidate executable and all three pins |

All retained pins have explicit retain disposition in the private reconciliation
inventory. Owner ecochran76 reviews these exceptions on 2026-11-10. A later
runtime-owner retirement or migration must precede closing live dependencies;
this reconciliation did not stop, reparent or replace any process.

## Evidence and bounds

Fresh status, remote ancestry, PR state, candidate-build claims, process cwd and
executable references, file descriptors, mappings and nested mounts qualified
each removal. Available Cargo profile locks were held during removal of build targets.
Every removal used inspect, begin, consequence preview and apply through the
[governed closeout contract](../policies/0052-session-and-worktree-admission.md).
Fresh registration readback proves the six exact paths are no longer worktrees.

Measured build-output allocation fell from 82.66 GiB to 54.17 GiB on the same
set of discovered directories. The five new candidate archives occupy 0.243 GiB;
the repository candidate archive store totals 2.155 GiB separately. Shared and active targets were preserved; no broad cache deletion
ran. Concurrent build activity means filesystem free-space deltas are not an
exclusive measure of this operation. The remaining build storage exceeds the
20 GiB review threshold and is an explicit active-lane/runtime-custody exception
through the next review, not approval for another build.

Private detailed inventory, process references, candidate requests, support
archives, terminal receipts and archive locators remain outside tracked source.
Use `pnpm run worktree:closeout lookup-archive --candidate-id <id>` to resolve
an archived candidate before reuse. No install, deployment, profile reset,
website action, handoff recovery or new handoff link occurred. Issue613 and the
separate login-control warning diagnosis remain open.
