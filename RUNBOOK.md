# Runbook

Current execution and stop-state index. Detailed checkpoints through Turn 312
are preserved in [the September 13 archive](RUNBOOK-history-2026-09-13-through-turn312.md).
[Turn 313 is preserved separately](RUNBOOK-history-2026-09-13-turn313.md).
[Turns 314 through 320 are archived](RUNBOOK-history-2026-09-14-turn314-through-turn320.md).
[The P169 challenge history through its Turn 319 is archived separately](RUNBOOK-history-2026-09-14-p169-through-turn319.md).
Keep this file at or below 200 lines under policy 0043.

## Turn 329 | 2026-09-14

[Plan 0187](docs/dev/plans/0187-2026-09-14-challenge-countermeasure-control-plane-blueprint.md)
entered W0 custody normalization. Published feature checkpoint
`2ae7a68332b7a505c74bbd1e987d8a82fb52409d` remains preserved while
`challenge/p169-control-plane` reconciles current `origin/main` through a
merge without rewriting the old branch. Current main supplies the extracted
lease-authority crate that W2 must consume.

The merge must preserve the first-request Guacamole header fix, current-main
runtime-profile and shared-host isolation, and both sides' runbook evidence.
The challenge packets are renumbered to Plan 0188 and Plan 0189. Parent
[issue #127](https://github.com/CochranResearchGroup/agent-browser/issues/127)
is open and in progress; issue #66 remains the blocked fixture leaf. W0 still
requires merge validation and published branch custody. No hCaptcha retry,
browser input, provider mutation, production mutation, or release is authorized.

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

## Turn 324 | 2026-09-14

[Plan 0181](docs/dev/plans/0181-2026-09-13-lease-authority-kernel-crate-extraction.md) is CLOSED through PR #106, which merged exact validated head `91aa3204` into
`main` as `b5a78faf`. The new crate owns the canonical kernel and protected
stack; the old owner is deleted and the CLI retains one private adapter.

Full CI run 34846719359 is terminal. macOS ARM compiled the extracted crate and
then failed in inherited CLI-only code; Windows also reached the extracted
crate before fail-fast cancellation. Native E2E passed 42 tests, then retained
one navigation fixture browser and cascaded to 14 failures. Browser repair is
outside P181 authority and does not invalidate its provider-free source proof.
P6 measured a 91.95 percent focused-loop improvement without the stricter
promotion claim. Focused run 34857911397 passes Linux, both macOS targets, and
Windows after `b6aa71dd` fixed Unix-only fixture paths; ordinary CI run
34857911400 is green. Final pre-join head `8a57dce5` also passes focused run
34861501476 and ordinary run 34861501499. The second P186 repair merged to
`main` afterward and was joined cleanly. Exact focused run 34864731916 passes
Linux, both macOS targets, and Windows. Ordinary run 34864731908 passes every
fast gate, including comprehensive Rust and no-launch smokes. No runtime,
browser, profile, provider, installation, production, release, or Plan 0144
acceptance claim is made. No P181 execution remains.

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
requires when no viewer survives. [Plan 0186](docs/dev/plans/0186-2026-09-14-route-viewer-admission-drain-recovery.md)
and [issue #112](https://github.com/CochranResearchGroup/agent-browser/issues/112)
own a canonical-viewer-only admission repair. Ordinary and tenant profiles
must remain blocked. PR #113 merged the first repair as `3c7d29da`; exact
candidate `34318d21` admitted route A launch, but `set headers` lacked the
global runtime profile at claim attachment and its generated launch failed
before effect. The temporary host was terminated with no display left. Next:
integrate exact session-profile shaping, complete the revision-bound forward
resume, then install one exact integrated generation. Do not retry any tenant
browser workflow.
