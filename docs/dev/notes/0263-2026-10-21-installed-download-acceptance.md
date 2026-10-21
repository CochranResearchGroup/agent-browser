# P263 installed existing-profile download acceptance

Product lane: PL-BUGFIX
Disposition: accepted-evidence
Owning plan or work item: docs/dev/plans/0263-2026-10-21-installed-download-acceptance.md / CochranResearchGroup/agent-browser#284
Related lanes: P243, P257, #286

## Installed qualification

The original existing-profile public stdio MCP download reproduction passes on
the current installed executable SHA256
`9c8edd62a16f17e79235d0f1bddde3b81be3ac796afca1ac74606e8e266d4d65`,
generation `0.28.0-9c8edd62a16f-3ef0c5e95c2e`. This is the installed artifact
previously qualified and deployed under P257, not a new build or publication.
The fixture pins the production-selected reviewed Linux stealthcdp executable,
SHA256 `70bb98cb3aea3dd5dabbd9ebd0420fab93949d7566c119dcd4ab1738947cb078`.
Both hashes are unchanged after acceptance.

The fixture creates an existing persistent test profile in a disposable HOME,
XDG tree, socket namespace and SQLite database. Through public MCP it opens the
synthetic download page, snapshots it, retrieves exactly `issue284-synthetic`,
and continues with the same task/browser/tab. It now asserts that the browser's
actual executable matches the requested pin and that PID/start identity survives
download and continuation. No owner repair or provider access is required.

## Results and preserved failures

| Mode | Result | Elapsed |
| --- | --- | --- |
| Ordinary existing-profile retrieval | Pass, exact bytes and stable executable/process/task | 8.10 s |
| Stale published/unpublished historical custody | Pass, two concurrent continuations and exact download | 8.04 s |
| Malformed historical custody | Pass, two concurrent continuations and exact download | 8.13 s |

Every mode separately reports a fresh zero-owned-process census before removing
its disposable home. Fault injection is limited to that fixture database. The
production dashboard and runtime-host PIDs remain unchanged at 42686 and 46532.
No production service was restarted.

The unchanged retained fixture first failed before download because its
`browser_open` omitted the now-required serviceName, agentName and taskName.
That run also cleaned up to zero owned residue. This is a stale test setup, not
a recurrence of the original owner-binding defect. The fixture now supplies
explicit synthetic attribution. Its first corrected ordinary run passed; the
three final runs above include the subsequent executable/process assertions.

JavaScript syntax, policy wiring, documentation links, patch hygiene and the
P263 planning audit pass. Standards/spec review finds no unresolved fixture
issue: current producer context, actual public tools, exact file/process evidence
and isolated cleanup are covered. No Rust or product behavior changes, so no
Cargo build or source Rust suite rerun is claimed or needed for this slice.

## Disposition and boundaries

This closes #284's original task-owned existing-profile download defect and its
stale installed-generation acceptance boundary. [P243](0243-2026-10-14-ordinary-browser-availability.md)
retains the original failing executable, source repair and broader source gates.
The current normal workflow succeeds without the historical manual service
recovery detour. The old manual recovery executable/presentation observations
are not independently reproduced or claimed repaired here.

This is synthetic installed-artifact acceptance using isolated runtime state,
not a private bank/QBO login, statement download, native-provider rendering or
production-profile run. It does not claim those broader outcomes. No new
installation, production/browser reset, provider effect, ingress/auth change or
CI dispatch occurred. Foreign worktrees and sealed candidates remain preserved.
