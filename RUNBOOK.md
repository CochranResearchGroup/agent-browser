# Runbook

Current execution and stop-state index. Detailed checkpoints through Turn 312
are preserved in [the September 13 archive](RUNBOOK-history-2026-09-13-through-turn312.md).
[Turn 313 is preserved separately](RUNBOOK-history-2026-09-13-turn313.md).
Keep this file at or below 200 lines under policy 0043.

## Turn 324 | 2026-09-14

[Plan 0181](docs/dev/plans/0181-2026-09-13-lease-authority-kernel-crate-extraction.md)
is OPEN in `PL-PLATFORM` through issue #99 and draft PR #106. Candidate
`8c0a6288` joins current `main` and extracts the canonical kernel and protected stack into
`agent-browser-lease-authority`, deletes the old owner, and leaves one private
CLI Service State adapter. All 108 crate tests, three retained adapter tests,
architecture guards, format, strict Clippy, and comprehensive native-Linux CI
pass.

Full CI run 34846719359 is terminal. macOS ARM compiled the extracted crate and
then failed in inherited CLI-only code; Windows also reached the extracted
crate before fail-fast cancellation. Native E2E passed 42 tests, then retained
one navigation fixture browser and cascaded to 14 failures. Browser repair is
outside P181 authority and does not invalidate its provider-free source proof.
P6 remains a measured 91.95 percent focused-loop improvement without the
stricter acceleration-promotion claim. Joined-tree P7 architecture, crate,
format, strict-Clippy, and patch-hygiene checks pass. The broad-run allowance
remains exhausted. Focused run 34857911397 passes the crate on Linux, macOS ARM,
macOS x86, and Windows after `b6aa71dd` corrected Unix-only test paths.
Ordinary PR CI run 34857911400 is green. After joining P186, local format,
strict Clippy, 108 crate tests, architecture guards, and patch hygiene pass.
Only exact-head revalidation and merge custody remain. No runtime, browser,
profile, provider,
installation, production, release, or Plan 0144 acceptance claim is made.

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

## Turn 320 | 2026-09-13

[Plan 0183](docs/dev/plans/0183-2026-09-13-terminal-owner-browser-missing-migration-repair.md)
is OPEN through [issue #104](https://github.com/CochranResearchGroup/agent-browser/issues/104).
The integrated P180 plus P182 candidate at source `caff5e08` and digest
`6b808d53` passed dry-run, but its single preserving apply stopped before
payload mutation in transaction
`upgrade-1f5e27b1-28ee-4277-a1ca-c15b4dddde32`.

The exact blocker is an invalid `browser_missing` tab for
`session:terminal-profile-4efa5eaf85940d2924b62480`. Its browser, session,
tab authority, work lease, runtime lifecycle, and process identity are absent,
but a matching generation 90 owner remains bound to the active registered
Last30Days principal and capability. Post-reboot installed reconciliation
failed before effect through retired legacy-daemon routing. P183 is now a
migration-only repair that synthesizes inert referential placeholders while
preserving owner authority. No second apply is allowed before focused source
proof, integration, candidate preview, and a fresh ready dry-run.

The migration-only regression and its fail-closed matrix pass, as do the full
focused migration module, Rust format, and workspace clippy. Candidate digest
`e30af9fb` accepts the formerly blocking row without mutation and preserves the
Last30Days tab, principal, capability, and generation 90 owner. The preview
reports zero protected removals and also exposes 117 browser plus 115 session
placeholder additions from older retained references. Review that class diff
again from the integrated commit before the single remaining apply gate.

## Turn 319 | 2026-09-13

[Plan 0182](docs/dev/plans/0182-2026-09-13-authentication-resume-state-reconciliation.md)
is OPEN through [issue #96](https://github.com/CochranResearchGroup/agent-browser/issues/96)
on `fix/issue-96-auth-resume-state-reconciliation`. Its source writes are
disjoint from P180, integrated at `44e5dc16`. The operator released Agent #95's
runtime custody to P182 on 2026-09-13 without installing a candidate.

The published P182 source packet at `4f9e1741` reproduces two adjacent stale
candidates and adds one serialized pure-mutator fallback after the first stale
candidate. P181 must reconcile any later adjacent adapter edit. PR #98's
completed fast gates passed; Rust was still running. Host pressure blocked both
local focused builds before compilation.

The installed production identity remains generation
`0.28.0-d0186990d375-3a6142188dd0` with binary digest
`d0186990d3758587c5c3e672330666362e1e3f077f7109a878052b242d677a50`;
the latest candidate transaction is terminally rolled back. The exact run is
still `ready` at transition 0 with zero observations, action receipts, or
pending effect on the same browser, session, and tab. Do not resume on the old
generation. Next: qualify and merge PR #98, install one integrated P180 plus
P182 candidate, re-anchor runtime and run evidence, then issue at most one
same-run recourse.

## Turn 317 | 2026-09-13

[Plan 0180](docs/dev/plans/0180-2026-09-13-pre-drain-browserless-lane-quiescence-repair.md)
is OPEN through [issue #95](https://github.com/CochranResearchGroup/agent-browser/issues/95).
Independent source diagnosis confirms an upgrade-compatibility inversion:
activation persists admission drain before browserless-lane quiescence, while
the selected old runtime from source `0e18b351` does not admit claimed `close`.
Current `main` learned that exception only in `e33d34df`, so the candidate
cannot rely on it to upgrade the executor that enforces the drain.

A scripted legacy-runtime regression first failed on the old ordering with the
exact `runtime_admission_draining` close symptom, then passed after the narrow
activation seam moved quiescence before drain. Failed pre-drain quiescence now
leaves the transaction at `StateMigrationValidated` with no drain and supports
a successful retry. Full shutdown still bypasses preserving quiescence, and an
exact claimed `close` is denied after drain.

All five shared-runtime quiescence tests and all 161 workstation installer
tests pass on the strengthened candidate. The regression now executes exact
status, multi-primary exclusion, browserless close before drain, post-drain
claimed-close denial, and handoff admission; separate tests cover
after-close retry, cooperative-only scope, isolated and full-shutdown bypass,
and selected-socket drift. Rust formatting, workspace clippy with warnings
denied, patch
hygiene, and validation selection also pass. The comprehensive provider-free
Rust runner passed both lanes in 1,215 seconds on the initial repair
checkpoint, including CLI core, CDP transport, CLI integration, and
production-scale Service State performance. PR CI must provide final-head
comprehensive proof before merge.
The first admitted build attempt failed before project compilation when
optional sccache could not spawn under host process pressure; successful runs
retained Cargo admission and cgroups, disabled only that cache, and used one
build job. Publication, integration, and installed acceptance remain open. No
installation, runtime handoff, browser closure, service mutation, or Books
Receipts action occurred.

Issue #96 has moved to P182 for disjoint provider-free source work. Its live
acceptance remains queued behind P180. Do not retry, cancel, replace, or create
a duplicate profile lane from P180. Re-anchor the same run, tab, handle,
installed identity, and writer evidence only after issue #95 hands the shared
runtime back.

## Turn 316 | 2026-09-13

[Plan 0178](docs/dev/plans/0178-2026-09-13-browserless-runtime-lane-quiescence.md)
is source-integrated and BLOCKED on separately authorized installed acceptance.
PR #83 merged browserless lane quiescence as `ae426642`; all fast checks and
comprehensive Rust passed. PR #92 merged exact runtime-admission claims for the
old-runtime status and close commands as `7e59ae35`. Its focused tests, format,
Rust Quality, Dashboard, Service Client, and Version Sync passed; Workstation
Fixtures and comprehensive Rust were still running at the one lazy readback and
were not actively watched.

Both source tips are ancestors of `origin/main`. Their clean worktree and local
and remote refs are retired, so P178 leaves active Git custody. Issue #84 remains
open and BLOCKED behind production maintenance quarantine issue #76. No
installation, restart, handoff, browser closure, or Service State mutation
occurred in closeout.

Provider-free issue #78 is also closed. PR #93 merged the development status
port repair as `4190fdf1`; the fixture, read-only current-runtime JSON and text
readback, documentation checks, and docs build passed. It performed no runtime
mutation. Installed shared-skill parity remains separately owned by issue #79.

## Turn 315 | 2026-09-13

[Plan 0179](docs/dev/plans/0179-2026-09-13-policy-selector-v0-1-26-integration.md)
is CLOSED through [issue #89](https://github.com/CochranResearchGroup/agent-browser/issues/89).
PR #90 merged the v0.1.26 selector rollout as `3f842f59` after policy wiring,
122 selector tests with three source-checkout-only skips, active planning and
goal audits, remote-view documentation checks, and patch hygiene passed.
Canonical `main` is clean and synchronized. Exact ancestry was verified before
the clean topic worktree and local and remote refs were retired. Selected CI
was queued at the one lazy readback and was not actively watched.

No installed runtime, browser, profile, provider, tenant, service, or
supervisor state changed. Turn 316 supersedes the then-current Plan 0178 and
issue #78 next actions with their integrated source outcomes.

## Turn 314 | 2026-09-13

Plan 0177 is CLOSED after [PR #86](https://github.com/CochranResearchGroup/agent-browser/pull/86)
integrated the governance and repository-readiness campaign as
`2d71134a55fc2f919daaf8cb7595efd3c7ebef79`. All selected local checks passed.
The PR's Dashboard, Service Client, Version Sync, Rust Quality, and Workstation
Fixtures jobs passed. Its unselected full Rust lane failed only in a pre-existing
workstation process-exit fixture now separately owned by issue #84 and PR #83;
Plan 0177 did not retry or absorb that source lane.

P177 leaves the active-lane catalog. P169 remains a published clean
`PAUSED_REF` at `32e7ec83`; P178 remains a separate active worktree and PR at
`f6b263f0`. Production maintenance and provider-backed acceptance remain
quarantined under issue #76. The canonical checkout is fast-forwarded only
after this closeout receipt integrates; no runtime mutation is part of closeout.
