# P52 S3 Clearance Pathway Plan

Date: 2026-06-27
State: COMPLETE
Lane: P52
Depends On:
- `docs/dev/plans/0046-2026-06-24-remote-view-stress-hardening-plan.md`
- `docs/dev/plans/0050-2026-06-26-s3-binary-authority-visible-window-plan.md`
- `docs/dev/plans/0051-2026-06-27-daemon-authority-guard-plan.md`

## Purpose

Clear P46 S3 by removing the daemon-authority ambiguity that blocked P50, then
rerun the S3 gates from the explicit rebuilt repo binary. This plan exists so
the S3 unlock path is durable, auditable, and executable from repo state rather
than chat continuity.

## Non-Negotiable Rules

- Do not run full S3 until `s3-open` passes.
- Do not run either S3 gate while the default daemon socket has multiple live
  listeners, a deleted-executable listener, or no listener matching the
  intended explicit binary.
- Treat named non-default sockets as telemetry unless they conflict with the
  default lane under test.
- Reset runtime state before and after each S3 gate.
- Any failed S3 gate stops execution and records audit evidence before another
  retry.
- Two consecutive failures in the same S3 gate lock this plan for maintainer
  planning.

## Goal 1: Make Daemon Authority Default-Socket Precise

`/goal execute P52 goal 1: make install doctor distinguish default-socket authority from named socket telemetry`

Work:

- Keep full daemon listener inventory in `install doctor`.
- Add default-socket match and deleted-executable counts.
- Scope readiness-blocking daemon socket issues to `default.sock`.
- Preserve named socket listeners in the inventory so other active lanes remain
  visible without blocking the default S3 lane.
- Add focused Rust tests proving named sockets alone do not create default-lane
  daemon authority issues.

Evidence:

- `cargo test --manifest-path cli/Cargo.toml install_doctor_flags`
- `agent-browser install doctor --json` exposes default-socket counts.

## Goal 2: Converge The Default Runtime Lane

`/goal execute P52 goal 2: safely converge default daemon socket ownership to one intended binary`

Work:

- Confirm runtime state before process repair.
- Use service close commands for addressable stale sessions where possible.
- Stop only stale `default.sock` daemon listeners after confirming they are not
  carrying active S3 browser state.
- Leave named non-default socket listeners alone unless they are proven to
  conflict with the default lane.
- Relaunch the default lane from `./cli/target/debug/agent-browser`.

Evidence:

- `./cli/target/debug/agent-browser --json service status`
- `ss -xlpn` default-socket listener inventory.
- `./cli/target/debug/agent-browser --json install doctor`

## Goal 3: Rerun The Narrow S3 Open Gate

`/goal execute P52 goal 3: rerun s3-open with explicit repo binary and daemon match enforcement`

Command:

```bash
node scripts/run-p46-stress-scenario.js \
  --scenario s3-open \
  --reset-before \
  --reset-after \
  --agent-browser-command ./cli/target/debug/agent-browser \
  --require-explicit-agent-browser-command \
  --require-agent-browser-daemon-command-match
```

Pass criteria:

- Command metadata names the explicit repo binary.
- Daemon metadata proves the default listener matches the explicit binary.
- `remote-view open` reports `operatorVisible.state=ready`.
- Visible-window proof reports `browser_window_visible`.
- Reset-after returns service state to zero active S3 sessions, browsers, tabs,
  and active incidents.

## Goal 4: Rerun Full S3

`/goal execute P52 goal 4: run full S3 after s3-open passes`

Command:

```bash
node scripts/run-p46-stress-scenario.js \
  --scenario s3 \
  --reset-before \
  --reset-after \
  --agent-browser-command ./cli/target/debug/agent-browser \
  --require-explicit-agent-browser-command \
  --require-agent-browser-daemon-command-match
```

Pass criteria:

- The default profile serves multiple operators in separate tabs.
- Remote-view UX has visual confirmation for each operator.
- Focus, navigate, open tab, switch tab, and close or release controls are
  functional.
- Runtime reset after the scenario leaves no active S3 sessions, browsers,
  tabs, or active incidents.

## Goal 5: Record Unlock Or Lock State

`/goal execute P52 goal 5: update durable plan state from current evidence`

Work:

- If `s3-open` and full S3 pass, mark this plan complete and update P46/P50 so
  S3 is clear and P46 can advance to S4.
- If a gate fails, record the artifact path, failure mode, reset proof, and
  whether the retry count locks P52.
- Keep validation and live-runtime evidence in this plan.

Closeout:

- P52 is complete only when install doctor is precise for the default socket,
  the default lane is converged, `s3-open` passes, full S3 passes, and P46/P50
  record the resulting S3 clearance.

## Execution Log

### 2026-06-27

Initial live evidence:

- P46 state before this plan: `LOCKED AT S3 AFTER P50 DAEMON-AUTHORITY DISCOVERY`.
- P50 state before this plan: `LOCKED AFTER DAEMON-AUTHORITY DISCOVERY`.
- Live install doctor result before this plan: `success: false`.
- Live daemon inventory before this plan:
  - `listenerCount: 10`;
  - `defaultSocketListenerCount: 8`;
  - `currentExecutableMatchCount: 0`;
  - `deletedExecutableCount: 7`.
- Blocking issue codes before this plan:
  - `daemon_socket_multiple_listeners`;
  - `daemon_socket_current_executable_mismatch`;
  - `daemon_socket_deleted_executable`;
  - `dashboard_runtime_stale_or_unreadable`;
  - `active_runtime_stale_executable`.

Graphiti discovery:

- `~/.local/bin/graphiti-runtime doctor` reported healthy.
- Focused discovery for the S3 daemon-authority lock returned only advisory
  baseline context. Repo plans and live doctor output remain the authority.

Implemented default-socket daemon authority precision:

- `install doctor` now reports:
  - `defaultSocketCurrentExecutableMatchCount`;
  - `defaultSocketDeletedExecutableCount`.
- Daemon socket readiness issues now scope to `default.sock`, while named
  non-default sockets remain visible as inventory telemetry.
- The P46 stress runner now enforces one matching `default.sock` listener for
  `--require-agent-browser-daemon-command-match` instead of treating every
  named socket in the socket directory as a default-lane conflict.

Validation:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml install_doctor_flags`
- `node scripts/test-p47-scenario-harness.js`

Default-lane repair:

- Pre-repair live doctor showed eight `default.sock` listeners, seven deleted
  executable listeners, and zero listeners matching the rebuilt repo binary.
- `./cli/target/debug/agent-browser --json close --session default` closed the
  addressable default session.
- Remaining stale `default.sock` listener PIDs were stopped after
  `service status` reported zero browsers, zero tabs, zero sessions, and zero
  active incidents.
- Named non-default socket listeners were left alone.
- A throwaway `about:blank` launch from the rebuilt binary established one
  matching `default.sock` listener for the S3 gate.

First `s3-open` retry:

- Command:
  `node scripts/run-p46-stress-scenario.js --scenario s3-open --reset-before --reset-after --agent-browser-command ./cli/target/debug/agent-browser --require-explicit-agent-browser-command --require-agent-browser-daemon-command-match`
- Artifact: `/tmp/agent-browser-p46-s3-open-2026-06-27T16-28-16-197Z`
- Result: failed.
- The daemon gate passed:
  - `defaultSocketListenerCount: 1`;
  - `defaultSocketMatchingListenerCount: 1`;
  - `singleMatchingListener: true`.
- Failure classification: `route_bound_finalization`.
- Failure symptoms:
  - `remote-view open` timed out;
  - no route ID or display name was returned;
  - route displays still showed only `Openbox`;
  - active incidents were `session:default` faulted and
    `remote-view-route-pool-exhausted:display:private_virtual_display:session-default`.

Investigation:

- Route-pool readiness had two ready entries for `:13` and `:14`.
- A no-live dry run with the same `AGENT_BROWSER_RDP_ROUTE_POOL_JSON` selected
  `guacamole-rdp-a`, `remote-view-display:13`, and route `guacamole:3`.
- The dry-run route binding still reported
  `displayIsolation: private_virtual_display` because a stale retained
  `remote-view-display:13` allocation had private-display metadata.
- That stale allocation overrode the fresh route-pool target, so live launch
  created private display `:90` while the route stream metadata pointed at
  Guacamole route A. The browser was never visible on route display `:13`.

Remediation:

- `build_route_binding` now lets a route-pool entry with `target.displayName`
  default to `shared_display` unless the entry explicitly provides
  `target.displayIsolation`.
- Stale retained display allocation isolation no longer overrides a fresh
  route-pool display target.
- Regression validation:
  `cargo test --manifest-path cli/Cargo.toml route_pool_target_display_overrides_stale_private_allocation_isolation`

Second `s3-open` retry:

- Command:
  `node scripts/run-p46-stress-scenario.js --scenario s3-open --reset-before --reset-after --agent-browser-command ./cli/target/debug/agent-browser --require-explicit-agent-browser-command --require-agent-browser-daemon-command-match`
- Artifact: `/tmp/agent-browser-p46-s3-open-2026-06-27T16-36-31-141Z`
- Result: passed.
- Evidence:
  - `defaultSocketListenerCount: 1`;
  - `defaultSocketMatchingListenerCount: 1`;
  - `operatorVisible` reached ready through finalization evidence;
  - display `:13` reached `browser_window_visible`;
  - route `guacamole:3`, display allocation `remote-view-display:13`, and
    route-pool entry `guacamole-rdp-a` finalized with no blockers;
  - reset-after ended with zero active incidents.

Full S3:

- Command:
  `node scripts/run-p46-stress-scenario.js --scenario s3 --reset-before --reset-after --agent-browser-command ./cli/target/debug/agent-browser --require-explicit-agent-browser-command --require-agent-browser-daemon-command-match`
- Artifact: `/tmp/agent-browser-p46-s3-2026-06-27T16-37-24-659Z`
- Result: passed.
- Evidence:
  - default profile browser `session:default` was ready with profile `default`;
  - route `guacamole:3` finalized on display `:13`;
  - display proof reached `browser_window_visible`;
  - operator A and operator B both had dashboard viewport evidence;
  - both dashboard refresh controls were clicked;
  - tab IDs were distinct;
  - tab A remained on
    `https://www.iana.org/domains/reserved?p46=s3-tab-a` after tab B
    navigation;
  - tab B moved from `https://example.com/?p46=s3-tab-b` to
    `https://example.org/?p46=s3-tab-b`;
  - reset-after ended with zero active incidents.
- Runner warning: simultaneous independent tab control is still serialized
  through the shared browser session. This is not an S3 failure, but it remains
  a product hardening note for later scenarios.

Final validation:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml install_doctor_flags`
- `cargo test --manifest-path cli/Cargo.toml route_pool_target_display_overrides_stale_private_allocation_isolation`
- `node scripts/test-p47-scenario-harness.js`
- `cargo build --manifest-path cli/Cargo.toml`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- Final service status artifact:
  `/tmp/agent-browser-p52-final-service-status.json`
  - zero browsers;
  - zero sessions;
  - zero tabs;
  - zero active incidents.
- Final install doctor artifact:
  `/tmp/agent-browser-p52-final-install-doctor.json`
  - default daemon authority is clean:
    - `defaultSocketListenerCount: 1`;
    - `defaultSocketCurrentExecutableMatchCount: 1`;
    - `defaultSocketDeletedExecutableCount: 0`;
  - install doctor still reports non-S3 drift:
    - `current_executable_path_command_mismatch`;
    - `dashboard_runtime_stale_or_unreadable`;
    - `active_runtime_stale_executable`.

Closeout:

- P52 is complete.
- P46 S3 is clear.
- P46 may advance to S4 from the explicit rebuilt-binary lane.
