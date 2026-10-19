# P249: recover ordinary tasks from a stale session host

Product lane: PL-BUGFIX. Owner: ecochran76. Issue: #320.
Branch: fix/stale-session-host. Status: COMPLETED (source repair); integration tracked by PR #321.

A peer persistence writer can advance the durable session state while the shared
host retains an older snapshot. Preserve the stale-write fence while allowing a
new ordinary command to adopt current state at a clean pre-effect boundary.

Seam: BrowserSessionHost command dispatch with real disposable SQLite persistence
and controlled browser effects. Proves task open/replay/close preserves peer tasks
without another browser launch; does not prove installed browser acceptance.
Retain lower-level CAS refusal. Add a regression that unpublished local work
cannot be discarded when a peer advances state.

Primary writer owns browser_session_host.rs and this plan/catalog entry. P248 is
integrated; preserve its focus/reaper changes. #316 pending Native acquisition,
#286 launch custody and foreign dashboard/resource work are separate and must not
be reset or adopted. Reuse the clean former integrated P247 checkout; three
registered worktrees observed, all clean before this branch. No new checkout.

Run red/green through scripts/ci/cargo-safe.sh, then relevant host, consumer,
pool and persistence regressions. Hosted CI skipped at operator direction.
Source implementation does not authorize production installation or global
runtime repair. Preserve live credential and peer state boundaries.

Qualification: exact conflict red, then 9 host tests, 12 persistence tests and
107 consumer/pool tests pass; 1 real native-viewer fixture remains explicitly
ignored. Policy wiring (11 tests plus assertions), formatting, whitespace and
changed-file secrets checks pass. The first broader batch's obsolete assertion
and omitted harness stack setting are retained in the evidence note. PR #321
publishes this source repair; no installed runtime adoption is claimed.
