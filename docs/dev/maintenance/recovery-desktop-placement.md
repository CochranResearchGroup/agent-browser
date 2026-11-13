# Replacement desktop placement

Product lane: PL-BUGFIX
Issue: #672
State: OPEN
Branch: fix/recovery-desktop-placement
Owner: ecochran76

## Outcome and boundary

After positive original-profile absence, ordinary replacement launches prefer vacant eligible Native desktops and use the persisted wrap cursor at capacity. Live browsers stay in place. Explicit placement, task/profile/tab/handoff identity, immutable launch history and competing-launch/retirement fences survive. No production installation or browser cleanup is authorized by this source packet.

## Execution

Reuse the clean canonical primary checkout from freshly fetched origin/main; no additional worktree. Existing lifecycle lanes #599 and #649 are integrated dependencies, not parallel writers. Bound to two implementation/test cycles and 45 minutes before reassessment. Initial RED: real session-host recovery chooses occupied old desktop despite a verified vacant assignment, repeated twice. Bypassing selection alone causes SQLite custody conflict.

Implement replacement placement and custody qualification together. Validate the actual session-host replay, explicit placement/live reuse, restart/wrap, uncertainty and custody guards, handoff continuity, Rustfmt, strict workspace Clippy and changed-surface documentation checks locally. Hosted CI remains disabled.

## Source qualification

Ordinary pooled recovery now uses observed vacancy filtering and the persisted independent-launch cursor after positive whole-profile absence. SQLite admission rechecks original process absence inside its existing immediate writer transaction before admitting a different desktop; prior publication, profile occupancy, peer generation and retirement/release checks remain. The manager accepts only the exact adapter-admitted replacement assignment. Explicit placement remains separately selected, fixed-assignment adapters keep their supported behavior, and live-profile reuse performs no placement.

Ordinary handoff mappings join original and replacement assignments to their separate immutable published launch records. Kernel-backed CDP and manual recovery retain the same task/profile/tab/link and human-control epoch after changing desktops. No old desktop release is invented.

### Validation

Baseline: `88be02cc9d4d3edd06a301b2e8c9e3c8405d0720` from freshly fetched origin/main. Focused Linux/WSL checks used disposable HOME/XDG directories, synthetic providers and owned process fixtures. Cargo used the governed wrapper, eight jobs and bounded cgroups; the cache was disabled after the diagnosis's stalled compiler-version probe.

- The original real session-host reproduction failed twice before repair. The repaired vacancy and wrap/restart cases pass and retain logical IDs and durable cursor state.
- The exact final CLI test binary passed 45 selected cases: replacement recovery (2), SQLite launch custody (14), ordinary continuation and handoff recovery (17), pool demand (9), existing wrap/restart (1), spare replenishment (1), uncertain spare isolation (1).
- `AGENT_BROWSER_CARGO_CACHE=off scripts/ci/cargo-safe.sh test -p agent-browser-service-model --manifest-path Cargo.toml -- --test-threads=1` passed the complete service-model package, integration binaries and doc tests. This is package coverage, not the complete workspace Rust suite.
- `AGENT_BROWSER_CARGO_CACHE=off scripts/ci/cargo-safe.sh clippy --workspace --manifest-path Cargo.toml -- -D warnings` passed.
- `AGENT_BROWSER_CARGO_CACHE=off scripts/ci/cargo-safe.sh fmt --all --manifest-path Cargo.toml -- --check` passed.
- `pnpm test:policy-wiring`, `pnpm test:documentation-links -- --base origin/main`, `pnpm test:remote-view-handoff-docs`, and `pnpm --dir docs build` passed. The docs build retained its existing multiple-lockfile warning.
- Provider-free workstation provisioning, VM harness, Guacamole assets, PostgreSQL durability and route-user synchronization contracts passed. The separately invoked installer fixture is tracked below; installer executable behavior is outside the repair.

Initial repair attempts remain explicit: the first replay exposed an additional manager publication identity guard, which was narrowed to the exact pending launch assignment. Strict Clippy rejected one unnecessary unwrap, corrected without suppressions. Kernel-backed handoff fixtures initially ignored the occupied-peer simulation, then the manual-worker probe reused a hardcoded old provider target; both fixture defects were corrected before the final 45-case pass. No failed attempt is counted as passing evidence.

The final test executable SHA256 is `416c40d501857a925564b269b3b879f0a5b57fb49da30286c331e9f38d494044`. Exact selected filters are `replacement_recovery_`, `browser_launch_custody_store::tests`, `pool_tests::ordinary_`, `pool_tests::pool_demand_`, `pool_tests::native_overflow_`, `pool_tests::background_pool_`, and `pool_tests::unknown_spare_`, each with serial test threads and a disposable HOME/XDG tree. Selected tests execute the actual CLI session host, SQLite store and provider adapters without launching a production browser.

No hosted CI was dispatched. Local Linux evidence does not establish other target platforms, installed behavior or browser visibility. Shared installed guidance was not overwritten during source qualification. There is no production installation, window relocation, browser close, profile deletion or provider mutation in this source change. The original five-window historical allocation sequence remains unproven; the matching recovery mechanism is reproduced and repaired.

### Supplemental installer fixture disposition

`pnpm test:workstation-install-fixture` passed fixture cleanup fencing, then its unchanged source-free installer scenario exceeded the bounded supplemental run (approximately twelve minutes). The exact child installer was terminated after verifying its parent and executable; the fixture's own finally cleanup was allowed to run. This interrupted supplemental check is not passing installer validation and is not a product-failure finding. Installer behavior is unchanged, and installation is outside this source-only outcome. Host provisioning and the other selected provider-free workstation contracts passed separately.

The interrupted installer returned exit1 because its planned transaction had not reached the expected terminal phase when terminated. Its own cleanup receipt confirmed two owned fixture hosts retired with zero residue; a fresh process read also found no owned validation residue. No installer pass or full installer qualification is claimed.
