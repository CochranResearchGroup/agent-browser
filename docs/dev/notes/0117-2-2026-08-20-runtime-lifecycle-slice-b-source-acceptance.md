# Plan 0117 Slice B Source Acceptance

Date: 2026-08-20

Plan: `docs/dev/plans/0117-2026-08-19-runtime-lifecycle-authority-and-convergence-plan.md`

Commit: pending cohesive Slice B commit

## Accepted Outcome

Slice B introduces `native::runtime_lifecycle` as the concrete owner of
managed browser lifecycle transitions. The owner uses the existing locked
Service State repository and P111 owner registry, applies every transition to
a cloned registry, and commits owner generation, lifecycle state, and cleanup
obligation together. Callers describe an intent and do not independently
write lifecycle projection state.

The production paths now route through that owner for managed launch and
attach registration, retained-browser attachment, cooperative transfer,
verified orphan adoption, candidate commit, abort, reverse transfer, legacy
owner revocation, finalize relinquish, recovery relaunch, owned close, and
retained-browser preservation. Effect-capable operations require the exact
current owner binding. A stale daemon may relinquish its process handle only
after a newer ready generation owns both browser effects and the cleanup
obligation.

Managed registration derives canonical profile identity, exact process
identity, CDP endpoint identity, and a sorted target-set identity. Replaying
the same registration refreshes endpoint and target evidence without changing
the owner generation. A conflicting owner requires an explicit lifecycle
transition. A terminal lane with a satisfied cleanup obligation may activate
one replacement at the next generation.

`ChromeProcess` and `BrowserManager` now require lifecycle approval before a
managed close. Dropping a managed process without that approval preserves the
browser and temporary profile instead of silently reclaiming them. Normal
close records `closing` before effects and reaches terminal only after exact
process exit and profile-lock release. Detach records retained preservation
before relinquishing the local process handle.

Service resource output is a read-only projection of lifecycle records. It
includes `runtimeLanes`, managed-lane count, and cleanup-obligation counts for
owned, transferring, satisfied, and unknown states. The projection cannot
write the owner registry.

Seven temporary transfer facades remain explicitly inventoried in
`P117_TEMPORARY_LIFECYCLE_FACADES`. They delegate to the concrete authority
and must be removed before Slice G closes.

## Validation

- Canonical serial Rust suite:
  `scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml -- --test-threads=1`
  passed with 2,186 unit tests, 57 intentionally ignored tests, and all six
  integration tests passing.
- Focused runtime-owner transfer suite: 18 passed.
- Focused runtime-lifecycle suite: 9 passed.
- Focused close and launch suite: 7 passed.
- Lifecycle resource projection test: passed.
- Managed Chrome drop-preservation test: passed.
- Rust format check: passed.
- Strict Rust Clippy with `-D warnings`: passed.
- Docs production build: passed with all 35 pages generated.
- Remote-view handoff documentation contract: passed.
- Repository and installed shared `agent-browser` skill parity: passed.
- `git diff --check`: passed.
- Disposable service CDP streaming smoke: passed using the source-built binary,
  direct `/opt/google/chrome/chrome`, a temporary HOME, isolated Agent Browser
  state, an isolated socket directory, and a unique session. It exercised
  production managed launch, lifecycle registration, service projection, tab
  operation, streaming, and lifecycle-approved close. The temporary runtime
  was removed by the harness.

## Slice C Input Discovered By Validation

The smoke harness's default `/usr/bin/google-chrome` path is a shell wrapper
whose process becomes `/opt/google/chrome/chrome`. The existing strict launch
identity capture compares the wrapper path with the final executable and
rejects the transition. The direct executable passes without weakening
identity checks. Slice C owns package launch identity and process-observation
reconciliation, so it must model this legitimate wrapper-to-browser transition
while continuing to reject unrelated executable drift.

## Live Boundary

No authenticated browser, default profile, installed daemon, dashboard,
systemd unit, runtime generation, or workstation resource was changed during
Slice B. All browser effects used disposable isolated state. Controlled live
workstation convergence remains reserved for separately authorized Slice I.

## Next Recommendation

Execute Slice C by making launch identity and process-tree observation part of
one ownership-backed reconciler, then prove normal close and garbage
collection use the same exact-identity shutdown implementation.
