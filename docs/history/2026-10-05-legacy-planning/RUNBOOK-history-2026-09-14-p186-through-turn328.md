# Runbook History | Plan 0186 Through Turn 328

This archive preserves the superseded Plan 0186 execution checkpoints removed
from the active `RUNBOOK.md` when the plan closed on 2026-09-14.

## Turn 328 | 2026-09-14

PR #120 merged atomic route profile-record synchronization as `f65bf907`;
exact candidate `e7e98600` passes the pinned source-free fixture. Route A then
reached display readiness, but route B failed before effect only when A had
started the shared host first. B succeeded alone and was closed cleanly. A
focused regression reproduces the failure: a shared host inherited A's
process-wide profile defaults and applied them while resolving B. PR #121
merged the first command-source guard as `5d07b94f`; exact candidate `852d9f58`
passes the pinned fixture but reproduced the B denial through a second direct
environment read in auto-launch option construction. Its focused regression
fails red on the merge and passes at `fb754890`; both cross-lane tests, format,
and clippy pass under the 8 GB reserve. The task-owned candidate browser and
hosts are closed. Next: integrate this final entry-point guard, build the exact
merge once, run the pinned fixture, then require simultaneous three-route
readiness before resuming transaction revision 21. Do not retry tenant
workflows.

## Turn 325 | 2026-09-14

PR #114 merged the exact route-viewer secondary-command claim shaping as
`bba8b7a3`; merged candidate digest `5610a704` passes the strengthened fixture,
the 28-test admission sweep, format, clippy, and selected workstation checks.
Runtime acceptance proved the admission repair through route A launch, header,
and navigation, then route B failed before effect as
`existing_session_profile_identity_unproven`. B and C retain ready generation-1
owners for obsolete generation-relative profile identities without matching
browser, session, tab, process, principal, or lease rows. Their exact lifecycle
rows are terminal with cleanup satisfied and process-exit plus
profile-lock-release evidence. Issue #112 is reopened. Root cause is the raw
registry session matcher returning terminal history as live before ambiguity
and profile selection, followed by the guarded relaunch rejecting the stable
runtime-profile path because it differs from the historical profile digest.
Next: preserve sole terminal relaunch, exclude exact terminal history when a
current replacement exists, allow only exact canonical route-path migration,
merge one repair, then resume transaction revision 17 exactly once.
Last30Days owner generation 90 remains unchanged; do not retry tenant workflows.

## Turn 323 | 2026-09-14

P184 merged through PR #109 as `3b7e8411`; its exact integrated binary digest
is `a2899457`. Fresh preview had zero protected removals, changes, or removals.
The changed-source apply preserved the external browser, committed Service
State with no changes, and selected generation
`0.28.0-a28994570dd3-9d43d7f4e826`. Transaction
`upgrade-8c858bc3-4507-48a0-8eea-c85cd3326fbf` is forward-only at revision 17
with admission drained and exact resume as its only completion action.

P185's managed-profile repair merged through PR #111 as `ffc6e510`, and exact
integrated candidate `bce36a4c` built successfully. Reboot then removed all
route viewers. A transaction-bound attempt to recreate route A failed before
effect as `runtime_admission_draining`: the drain permits claimed Service
reconcile but not the launch, headers, navigation, and cleanup that reconcile
requires when no viewer survives. Plan 0186 and issue #112 own a
canonical-viewer-only admission repair. Ordinary and tenant profiles must
remain blocked. PR #113 merged the first repair as `3c7d29da`; exact candidate
`34318d21` admitted route A launch, but `set headers` lacked the global runtime
profile at claim attachment and its generated launch failed before effect. The
temporary host was terminated with no display left. Next: integrate exact
session-profile shaping, complete the revision-bound forward resume, then
install one exact integrated generation. Do not retry any tenant browser
workflow.
