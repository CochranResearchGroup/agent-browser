# P53 S4 Single-Profile Window Topology Plan

Date: 2026-06-27
State: COMPLETE
Lane: P53
Depends On:
- `docs/dev/plans/0046-2026-06-24-remote-view-stress-hardening-plan.md`
- `docs/dev/plans/0052-2026-06-27-s3-clearance-pathway-plan.md`
- `docs/dev/notes/2026-06-24-p46-stress-hardening-execution.md`

## Purpose

Unlock P46 S4 without another blind live retry. S4 currently asks one runtime
profile to produce two independent route-bound browser processes. Attempt 2
proved that this shape can report one visible browser, then lose that browser
while the second route-bound open times out. The next slice must make the
single-profile window topology explicit before live execution resumes.

## Current Evidence

- S4 attempt 2 artifact:
  `/tmp/agent-browser-p46-s4-2026-06-27T18-34-00-072Z`.
- Window A `remote-view open` succeeded with:
  - `runtimeProfile`: `p46-s4-profile`;
  - `routePoolEntryId`: `guacamole-rdp-a`;
  - `routeId`: `guacamole:3`;
  - `displayAllocationId`: `remote-view-display:13`;
  - `operatorVisible.state`: `ready`;
  - visible Chrome window proof on display `:13`.
- Window B `remote-view open` used the same runtime profile with
  `guacamole-rdp-b` and timed out.
- The attempt then reported window A process exit, orphaned display allocation
  `remote-view-display:13`, orphaned route `guacamole:3`, and
  `remote_view_finalization_incomplete`.
- `skills/agent-browser/SKILL.md` and docs service-mode guidance already state
  that shared authenticated profiles should share one retained browser lane,
  while duplicate independent profile processes require explicit reviewed
  duplicate-lane intent.
- `cli/src/native/actions.rs` has a duplicate-profile-lane guard and an
  `allowDuplicateProfileLane` service-request field, but the P46 CLI S4 path
  did not express that intent and did not fail early with a typed blocker.

## Non-Negotiable Rules

- Do not run live S4 again until this plan is implemented and no-live
  validation passes.
- Do not claim S4 pass from a duplicate-process workaround.
- Preserve P46's visual proof standard for any retry.
- Treat a same-profile, two-route, two-process request as unsupported unless
  the operator explicitly selects reviewed duplicate-lane behavior and the
  failure mode is typed.
- Do not manually edit service state to pass S4.

## Goal 1: Add A Typed S4 Topology Preflight

Work:

- Before the second S4 window launch, inspect the intended topology:
  one runtime profile, two sessions, and two route-pool entries.
- If the request would create a second independent browser process for the
  same runtime profile without reviewed duplicate-lane intent, stop before the
  second launch with a typed `same_profile_multi_process_unsupported` blocker.
- Record the blocker in the S4 artifact as failure evidence, not as a pass.
- Keep the P46 two-failure lock in place until the blocker is reviewed.

Evidence:

- `node scripts/test-p47-scenario-harness.js`
- `node --check scripts/run-p46-stress-scenario.js`

## Goal 2: Choose The Supported S4 Topology

Work:

- Decide whether S4 should prove multiple browser windows within one retained
  browser lane and one route, or whether the two-route shape should move to a
  later profile-isolation scenario.
- If same-profile multi-window is supported in one browser lane, implement the
  runner through existing route hints, same session ownership, and separate
  top-level windows or tabs without creating a second Chrome profile process.
- If distinct RDP routes are required, require distinct profiles or an
  explicit profile snapshot strategy; do not use the same live user-data-dir in
  two independent Chrome processes.

Evidence:

- Source-backed note in this plan explaining the chosen topology.
- No-live harness coverage proving the runner no longer silently asks for the
  unsupported shape.

## Goal 3: Make Reviewed Duplicate-Lane Intent Observable

Work:

- If the retry still needs duplicate-lane exploration, expose it through the
  service-request path or a documented CLI flag instead of relying on implicit
  launch behavior.
- Ensure a duplicate-lane request that hits a live Chrome profile lock returns
  a clear typed blocker with the profile path, owner PID, and suggested reuse
  path.
- Update all user-facing docs surfaces if a new CLI flag or behavior is added.

Evidence:

- Focused Rust parser or service-request tests for the public contract.
- `pnpm test:service-client` and service API/MCP parity checks if generated or
  schema surfaces change.

## Goal 4: Retry S4 Only After No-Live Gates Pass

Preflight:

```bash
./cli/target/debug/agent-browser --json service status
./cli/target/debug/agent-browser --json service incidents --summary
./cli/target/debug/agent-browser --json install doctor
./cli/target/debug/agent-browser --json doctor remote-view
node scripts/smoke-rdp-guac-route-pool-readiness.js --report-only
node scripts/inspect-rdp-route-displays.js --display-content
```

Retry command, only after Goals 1 through 3 are complete:

```bash
node scripts/run-p46-stress-scenario.js \
  --scenario s4 \
  --reset-before \
  --reset-after \
  --agent-browser-command ./cli/target/debug/agent-browser \
  --require-explicit-agent-browser-command \
  --require-agent-browser-daemon-command-match
```

Pass criteria:

- Two visible, controllable browser windows match the chosen supported
  topology.
- Profile lease and duplicate-lane behavior is explicit.
- Closing one window does not release or corrupt the other.
- Reset-after leaves zero retained S4 sessions, browsers, tabs, routes, and
  active incidents.

## Execution Log

### 2026-06-27

Initial diagnosis completed from current source, repo docs, and the S4 attempt
2 artifact. P46 remains locked. No live S4 retry was run.

Implemented Goal 1 no-live guard:

- `scripts/run-p46-stress-scenario.js` now evaluates the S4 topology after
  window A opens and before window B launches.
- The current one-profile, two-session, two-route-pool-entry shape stops with
  typed blocker `same_profile_multi_process_unsupported` unless reviewed
  duplicate-lane intent is explicit.
- The runner writes `s4-topology-preflight.json` into the scenario artifact and
  records the blocker as failure evidence, not a pass.
- `scripts/lib/p46-scenario-harness.js` classifies this failure as
  `profile_topology`.
- `scripts/test-p47-scenario-harness.js` covers the invariant, classifier, and
  runner source wiring.

Validation:

```bash
node --check scripts/run-p46-stress-scenario.js
node scripts/test-p47-scenario-harness.js
```

Both checks passed. No live S4 retry was run.

Chosen Goal 2 topology:

- S4 will prove one retained remote-headed browser process, one runtime
  profile, one route lease, and two top-level browser windows.
- The abandoned two-route shape moves to a later profile-isolation or profile
  snapshot scenario. It is not a valid same-profile proof because it asks two
  Chrome processes to use the same live profile directory.
- Added `agent-browser window new [url] --same-profile` so a retained browser
  can create another top-level target in the current profile instead of a new
  browser context.
- The S4 runner now uses one route-bound `remote-view open` for window A and
  `window new --same-profile` for window B in the same daemon session.
- The S4 runner now uses a unique `p46-s4-window-<timestamp>` daemon session
  per run so stale named sessions cannot silently execute an older binary.
- S4 metadata now expects one route lease total, one shared browser id, one
  shared route id, one shared display, and distinct target ids.

Additional validation to run before live retry:

```bash
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo test --manifest-path cli/Cargo.toml test_window_new_same_profile_with_url
node --check scripts/run-p46-stress-scenario.js
node scripts/test-p47-scenario-harness.js
```

Completed live retry:

```bash
node scripts/run-p46-stress-scenario.js --scenario s4 --reset-before --reset-after --agent-browser-command ./cli/target/debug/agent-browser --require-explicit-agent-browser-command --require-agent-browser-daemon-command-match
```

Artifact:

```text
/tmp/agent-browser-p46-s4-2026-06-27T19-12-55-449Z
```

Result: passed.

Evidence:

- reset-before closed no sessions and active incidents stayed at zero;
- the explicit command authority check saw one matching default daemon
  listener for `./cli/target/debug/agent-browser`;
- window A used route `guacamole:3`, display `:13`, and finalized with
  `routePoolEntryId` `guacamole-rdp-a`;
- window B used `window new --same-profile`, returned target id
  `A90D50B9E35702B7AF31B2B5B04AC360`, and shared browser
  `session:p46-s4-window-2026-06-27T19-12-53-709Z`, route `guacamole:3`,
  display `:13`, and runtime profile `p46-s4-profile`;
- both operator dashboard refresh controls clicked successfully;
- closing window A left the shared browser row for window B `ready`;
- reset-after closed the S4 session and left zero active incidents.

P53 is complete. P46 can move past S4 to S5 with the one-process,
one-route, same-profile window topology as the supported S4 contract.
