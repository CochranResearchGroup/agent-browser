# Plan 0117 Slice C Source Acceptance

Date: 2026-08-20

Plan: `docs/dev/plans/0117-2026-08-19-runtime-lifecycle-authority-and-convergence-plan.md`

Commit: pending cohesive Slice C commit

## Accepted Outcome

Slice C introduces `native::runtime_reconciliation` as the deep owner of the
join between observed processes and durable runtime authority. A process tree
is effect-capable only when root PID, start token, executable, browser family,
process group, logical browser, canonical profile identity, owner generation,
cleanup obligation, lifecycle state, and package launch identity all match.
Command-line and path parsing remain supplementary evidence.

Package-launched and managed-attached browsers now persist process-group and
package-launch identity with their lifecycle registration. Linux process
observation retains evidence from one-field rewritten Chrome argv. Launch
identity accepts the legitimate shell-wrapper transition from
`/usr/bin/google-chrome` to a same-family sibling executable in the same
installation root while rejecting unrelated executable drift.

Normal owned Chrome close and resource GC use one reviewed process-tree
shutdown protocol. The protocol checks authority immediately before SIGTERM,
checks it again before any SIGKILL escalation, signals the process group, and
requires exact process-tree exit plus profile-lock release. GC removes a stale
`SingletonLock` only after the exact reviewed process group has exited.

Resource classification no longer treats profile-data retention as process
retention. Named and persistent profile data remains protected from deletion,
while an exact lifecycle-owned `closing` process tree can become a reviewed GC
candidate after its grace period. Old temporary processes without exact
lifecycle authority remain protected. CLI resource and GC maintenance commands
load the locked repository at execution rather than carrying a parse-time
Service State snapshot.

The former Xvfb-only live smoke is replaced by an isolated managed-Chrome
smoke. It launches a real Chrome helper tree, simulates owner-daemon loss,
maintains the durable owner sidecar, proves stale review-token rejection after
generation drift, reclaims the full managed process group and stale profile
lock, and preserves an unrelated Chrome process.

## Validation

- Canonical serial Rust suite:
  `scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml -- --test-threads=1`
  passed with 2,191 unit tests, 57 intentionally ignored tests, and all six
  integration tests passing.
- Focused service-resource suite: 15 passed.
- Strict Rust Clippy with `-D warnings`: passed.
- Rust format check: passed.
- Disposable managed resource-GC smoke: passed with one 13-process managed
  Chrome group reclaimed, unrelated Chrome preserved, profile lock removed,
  and owner-generation drift rejected.
- Disposable service CDP streaming smoke: passed through the source-built
  binary and `/usr/bin/google-chrome`, proving the guarded wrapper transition,
  managed lifecycle registration, tab streaming, and close path.
- Docs production build: passed with all 35 pages generated.
- Route-confusion no-launch gate: all eight fixtures passed.
- Remote-view handoff documentation contract: passed.
- Repository and installed shared `agent-browser` skill parity: passed.
- JavaScript syntax check and `git diff --check`: passed.

## Operational Prerequisite Readback

The separately authorized privilege-helper repair completed before Slice C
validation. The installed root-owned helper is byte-for-byte identical to the
source helper and reports the P44 route-desktop v4 contract with non-PAM SHA512
credential updates. The enabled runtime-interlock timer completed one bounded
reconciliation with exit status zero, both route displays ready, and no
password or keyring prompt event.

Install doctor reports no install issues, one current executable generation,
and zero deleted-executable listeners. It also reports nine current-generation
legacy daemon listeners. Those authenticated listeners were preserved; no live
workstation GC or daemon consolidation was attempted in this source slice.
Runtime-host consolidation remains owned by Slices E through I.

## Live Boundary

All Chrome launch, shutdown, and GC validation used disposable temporary HOME,
Agent Browser state, socket directories, profiles, and process groups. No
authenticated browser, retained profile, installed runtime generation, or
workstation daemon was signaled or deleted by Slice C.

## Next Recommendation

Execute Slice D by introducing one retention authority that joins lifecycle,
process, handoff, transaction, rollback, supervisor, and filesystem evidence
before any profile or generation becomes reclaimable.
