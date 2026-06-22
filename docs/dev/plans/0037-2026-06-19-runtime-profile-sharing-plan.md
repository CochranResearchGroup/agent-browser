# Runtime Profile Sharing Plan

Date: 2026-06-19
State: OPEN
Lane: P14/P16
Depends On:
- `docs/dev/plans/0027-2026-06-05-minimal-runtime-profile-reuse-plan.md`
- `docs/dev/plans/0036-2026-06-18-rdp-ready-to-go-plan.md`

## Purpose

Agent-browser should let multiple clients use the same authenticated runtime
profile without stochastic profile-lock failures or duplicate Chrome process
pressure. The safe sharing unit is a retained browser process group, not the
profile directory. Clients should acquire separate service-owned tabs now and
separate windows later, with attribution, leases, cleanup, and validation owned
by the service control plane.

## Goal

Make shared authenticated runtime profiles safe and deterministic by routing
concurrent clients through one retained browser process group. A profile
directory remains exclusive to one browser process group, while multiple
clients can share that browser through attributed tabs, viewer leases, queued
control, and explicit cleanup.

## Operating Invariants

```text
A runtime profile directory must not be opened by two independent Chrome
process groups unless the caller explicitly selects reviewed throwaway or
isolated duplicate-process behavior.
```

```text
Profile sharing means retained-browser tab or window sharing. It does not mean
multiple clients may bypass agent-browser and race direct CDP control against
one profile.
```

## Target Sharing Contract

- `profileProcessPolicy`: `exclusive_process` by default.
- `clientSharingPolicy`: `shared_browser_tabs` now, `shared_browser_windows`
  later.
- `defaultAcquisition`: `tab_new` when a compatible retained browser exists,
  `tab_reuse` or `view_focus` when a compatible tab exists, `wait` when the
  profile cannot be shared, and `launch_new_browser` only when no live holder
  exists.
- `maxConcurrentTabs` and `maxConcurrentWindows`: advisory caps when policy
  adds them.
- Service tab handles carry browser, session, profile, tab, caller labels,
  lease, cleanup, validity, and trace-filter metadata.
- Service requests route through returned `browserId` and `sessionName` hints
  instead of launching another browser for the same profile.

## Non-Goals

- Do not support arbitrary duplicate Chrome process groups on authenticated
  profiles.
- Do not make AuraCall-specific selectors, accounts, or profile names part of
  the generic agent-browser contract.
- Do not implement window sharing before tab sharing has attribution, leases,
  cleanup, and live validation.
- Do not require downstream clients to parse raw service state when access-plan
  can return a structured recommendation.

## Subagent Work Allocation

Use subagents by slice when implementation widens beyond a single turn. Each
subagent should return:

```text
Slice:
Goal:
Files changed:
Contract delta:
No-launch validation:
Live validation:
Residual risks:
Next slice readiness:
```

Suggested subagents:

1. Contract Agent: access-plan sharing contract, service tab handle schema, and
   generated client types.
2. Orchestration Agent: service request tab acquisition, route-hint handling,
   lease serialization, and cleanup semantics.
3. Dashboard Agent: profile owner, shared tab list, action reasons, lease
   conflicts, and trace links.
4. Live Gate Agent: two-client synthetic-profile live smoke and duplicate
   process rejection proof.
5. Documentation Agent: README, docs site, CLI help, and skill guidance.

## Slice A: Contract And Attribution

State: DONE

Goal: make the shared-profile tab acquisition contract explicit before adding
more orchestration.

Deliverables:

- Add structured shared-acquisition metadata to access-plan
  `decision.profileReuse` so clients can distinguish policy from incidental
  reuse fields.
- Ensure reusable-browser plans include route hints and a `tab_new`
  acquisition recommendation.
- Extend service tab handle trace filters with caller labels:
  `serviceName`, `agentName`, and `taskName`.
- Regenerate service client types for the public handle shape.
- Add no-launch tests for access-plan metadata and handle attribution.

Acceptance:

- An active compatible browser for a selected profile yields
  `reuse_existing_browser`, `profileProcessPolicy=exclusive_process`,
  `clientSharingPolicy=shared_browser_tabs`, and a structured tab acquisition
  recommendation.
- A service tab handle derived from a service-owned tab includes browser,
  profile, session, service, agent, and task trace fields.
- Existing handle users remain compatible because added fields are optional.

Validation:

```bash
cargo test --manifest-path cli/Cargo.toml service_access_plan -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml refresh_service_tab_handles -- --test-threads=1
pnpm test:service-client-contract
pnpm test:service-client-types
git diff --check
```

Completed on 2026-06-19:

- `decision.profileReuse.sharedAcquisition` now reports shared-browser tab
  policy, acquisition mode, retained browser/session route hints, required
  route-hint fields, service queue serialization, cleanup expectation, and
  duplicate-process policy.
- `ServiceTabHandleTraceFilter` now carries `serviceName`, `agentName`, and
  `taskName` so multiple clients sharing one retained browser remain
  attributable from the handle alone.
- `docs/dev/contracts/service-tab-record.v1.schema.json`, generated client
  declarations, README, docs site, CLI help, and the `agent-browser` skill were
  updated for the new public contract.
- The installed shared `agent-browser` skill copy was synced.

Validation passed:

```bash
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo test --manifest-path cli/Cargo.toml service_access_plan -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml refresh_service_tab_handles -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml service_model -- --test-threads=1
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
pnpm test:service-client-contract
pnpm test:service-client-types
pnpm --dir docs build
diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md
```

## Slice B: Shared Tab Acquisition

State: IN PROGRESS

Goal: execute the access-plan recommendation through service request without
creating duplicate profile process pressure.

Deliverables:

- Route `tab_new` through `browserId` and `sessionName` when access-plan
  returns a compatible retained browser.
- Return response evidence naming whether the request reused a browser, opened
  a new tab, waited, or rejected.
- Keep direct duplicate process launch rejected by default for authenticated
  profiles.
- Add no-launch and isolated live tests for retained-browser tab acquisition.

Completed on 2026-06-19:

- `tab_new` responses now include `sharedAcquisition` evidence with policy,
  acquisition mode, whether the routed browser was reused, whether a tab was
  opened, duplicate-process policy, routed browser/session ids, requested route
  hints, and profile id when known.
- `createServiceTabRequestFromAccessPlan` now copies
  `decision.profileReuse.sharedAcquisition.browserId` and `sessionName` into
  the service request when the access plan recommends `mode=tab_new`.
- Generated service request declarations now expose
  `ServiceSharedTabAcquisition` on `ServiceTabNewData` and the access-plan
  shared-acquisition shape.
- The isolated service-request live smoke now asserts retained-browser reuse:
  access-plan must recommend shared tab acquisition, the client helper must copy
  route hints, and the planned `tab_new` response must report
  `browserReused=true`.
- `tab_new` now persists service-owned tab records and session tab references
  for returned `serviceTabHandle` values, so later release and trace operations
  operate against retained service state instead of handle-only response data.
- The service-request live smoke now uses the preferred hidden
  `remote_headed` plus `stealthcdp_chromium` profile posture for the selected
  authenticated profile and includes stronger assertions for second-session
  duplicate launch rejection, physical release of tab A, and continued
  evaluation on tab B.
- Chrome launch now retries once for the narrow WSL pre-DevTools failure
  signature `UtilAcceptVsock` plus `accept4 failed 110`, so service-owned
  launch jobs can distinguish a recovered transient from a repeated lane
  failure.

Validation passed:

```bash
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo test --manifest-path cli/Cargo.toml tab_new_shared_acquisition -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml service_profile_lease_gate_allows_duplicate_lane_route_hints -- --test-threads=1
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
node scripts/test-service-request-client.js
pnpm test:service-client-contract
pnpm test:service-client-types
git diff --check
```

Live validation blocked:

```bash
node scripts/smoke-service-request.js
```

The original retained-browser assertions passed once on 2026-06-19 with:
`pnpm test:service-request-live` reporting `Service request HTTP/MCP live smoke
passed`. After the smoke was strengthened to prove duplicate-lane rejection,
physical tab release, and tab-B survival, the first live Chrome launch again
exited before DevTools with WSL `UtilAcceptVsock` `accept4 failed 110` even
after the selected profile requested `remote_headed` and
`stealthcdp_chromium`. A follow-up run with the one-retry launch hardening
proved both attempts failed with the same WSL signature. Keep Slice B open
until the first-launch lane is hardened outside this service-request path or
the smoke can use a browser executable/display route that does not hit this WSL
failure.

## Slice C: Lease And Cleanup Semantics

State: IN PROGRESS

Goal: make tab sharing safe over time.

Deliverables:

- Add or harden tab leases, controller leases, heartbeat, release, and stale
  lease expiry.
- Ensure one client's tab cleanup does not close another client's tab or the
  shared browser unless policy says so.
- Report abandoned shared tabs and lease conflicts as service incidents.

Completed on 2026-06-19:

- Added service request action `tab_handle_release` for conservative
  service-state release of a retained tab handle.
- Release accepts a current or stale `serviceTabHandle`, marks only the
  matching retained tab record closed, refreshes the handle to stale
  `tab_closed` evidence, records a tab lifecycle event, and explicitly reports
  `browserProcessPreserved=true`, `sessionRoutePreserved=true`, and
  `closeBrowserOnRelease=false`.
- Release preserves the retained browser process, active browser session ids,
  session lease, and session tab references so other clients can keep using the
  shared profile lane and cleanup can prune closed tab records later.
- Added `createServiceTabHandleReleaseRequest()`,
  `requestServiceTabHandleRelease()`, and `releaseServiceTabHandle()` client
  helpers plus generated `ServiceTabHandleReleaseData` typing.
- Updated the service request schema, Rust service action list, generated
  client declarations, README, docs site, CLI help, and the `agent-browser`
  skill.
- `refresh_derived_views()` now derives `profile_lease_conflict` service
  incidents when an active session records conflicting profile-holder sessions.
- `refresh_derived_views()` now derives `shared_tab_abandoned` service
  incidents when an active shared session still references a missing, closed,
  or crashed tab.
- Shared-profile coordination incidents classify as service-triage warnings
  with recommended actions that preserve the retained browser lane before
  pruning stale tab state.
- Added explicit no-launch stale session lease expiry through
  `ServiceState::expire_stale_session_leases(observed_at)`: due active
  sessions move to `LeaseState::Expired`, are removed from browser
  `activeSessionIds`, and stale tab handles report `lease_expired` without
  closing the retained browser.
- `service_reconcile` now runs stale lease expiry during reconciliation and
  returns `expiredSessionLeases` plus `expiredSessionLeaseCount` so clients and
  operators can see which shared-profile leases were retired.
- Updated the reconcile response schema, README, docs site, CLI help, and the
  `agent-browser` skill for the new reconcile evidence. The installed shared
  skill copy was synced.
- `tab_handle_release` now best-effort closes the exact physical CDP target
  when the routed live browser owns it, preserves the browser process and
  session route, and falls back to service-state-only release with structured
  `physicalTabClose` evidence when no live browser is attached, the target is
  missing, the last tab must be preserved, or the request disables physical
  close.
- Added `BrowserManager::tab_close_target_id()` so release cleanup can target
  the handle's target id instead of relying on active-tab or index state.
- Updated `ServiceTabHandleReleaseData`, the service request contract
  description, README, docs site, CLI help, and the `agent-browser` skill for
  physical target-close evidence. The installed shared skill copy was synced.
- Service-owned `tab_new` now persists retained tab records and session tab
  references for returned handles, matching the existing external BYOP adopted
  tab persistence path. This closes the live gap where physical target close
  succeeded but `tab_handle_release` reported `tabMissing=true`.

Validation passed:

```bash
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo test --manifest-path cli/Cargo.toml expire_stale_session_leases_preserves_browser_and_marks_handles_stale -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml reconcile_expires_stale_session_leases_with_response_evidence -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml test_service_reconcile_response_matches_contract -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml shared_profile_coordination_derives_incidents_for_conflicts_and_abandoned_tabs -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml service_model -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml service_health -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml service_contracts -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml tab_handle_release -- --test-threads=1
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
node scripts/test-service-request-client.js
pnpm test:service-client-exports
pnpm test:service-client-contract
pnpm test:service-client
pnpm test:service-observability-client
pnpm test:service-client-types
pnpm test:service-api-mcp-parity
pnpm --dir docs build
diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md
git diff --check
```

Remaining:

- Rerun the strengthened live shared-profile smoke after the first-launch lane
  is healthy enough to reach its physical-close and surviving-tab assertions.

## Slice D: Dashboard And Observability

State: DONE

Goal: make shared runtime profiles understandable to operators.

Deliverables:

- Show retained browser as the profile owner.
- Show shared clients, tabs, lease holders, and controller conflicts under the
  browser/profile.
- Add trace filters by profile, browser, tab, service, agent, and task.
- Explain why duplicate launch actions are disabled.

Completed on 2026-06-19:

- Profile allocation rows now label the retained owner browser for the
  profile lane instead of only listing browser ids.
- Profile allocation rows and detail inspection now show shared client summary
  by service, agent, and task, plus holder, waiting, conflict, and tab counts.
- Profile allocation detail inspection now includes duplicate-launch guidance
  that tells operators to reuse the retained owner browser when one exists.
- The existing trace explorer remains the profile, browser, tab, service,
  agent, task, and time-window filter surface for shared-profile work.
- README, docs site, and the `agent-browser` skill were updated. The installed
  shared skill copy was synced.

Validation passed:

```bash
pnpm test:dashboard-profile-allocation
pnpm test:dashboard-inspector-actions
pnpm test:dashboard-trace
pnpm run build:dashboard
pnpm --dir docs build
diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md
```

## Slice E: Live Validation And Documentation

State: IN PROGRESS

Goal: prove the end-to-end behavior and teach downstream clients to use it.

Deliverables:

- Add a live synthetic-profile smoke:
  - launch one retained remote-headed browser;
  - client A opens tab A;
  - client B opens tab B with the same runtime profile;
  - prove both tabs are attributable and controllable;
  - prove closing or releasing A does not close B or the browser;
  - prove duplicate process launch remains rejected.
- Update README, docs site, CLI help when needed, and the `agent-browser`
  skill.
- Add generated client helper summaries if public API shape expands further.

Completed on 2026-06-19:

- Added stronger live shared-profile assertions to
  `scripts/smoke-service-request.js`: two service-owned tabs route through the
  same retained browser/session, a second daemon session is rejected when it
  tries to launch the same runtime profile without route hints, releasing tab A
  must physically close only tab A, and tab B must remain evaluable afterward.
- The selected synthetic authenticated profile now requests the preferred
  hidden `remote_headed` plus `stealthcdp_chromium` posture for this live gate.
- RDP/Guacamole route-pool readiness passed with two distinct selected route
  candidates, local embeddable Guacamole URLs, public operator URLs under
  `https://agent-browser.ecochran.dyndns.org/guacamole/`, and reachable RDP
  backends.
- The full RDP/Guacamole many-to-many live gate passed and wrote artifacts to
  `/tmp/agent-browser-rdp-guac-many-to-many-2026-06-19T23-41-11-162Z`.

Validation passed:

```bash
pnpm test:rdp-guac-route-pool-readiness
pnpm test:rdp-guac-many-to-many-live
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo test --manifest-path cli/Cargo.toml transient_wsl_predevtools -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml tab_handle_release -- --test-threads=1
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
node scripts/test-service-request-client.js
git diff --check
```

Live validation still blocked:

```bash
pnpm test:service-request-live
```

The strengthened service-request live smoke is present but currently fails
before reaching the new assertions because first Chrome launch exits before
DevTools with WSL `UtilAcceptVsock` `accept4 failed 110`. With the bounded
launch retry in place, both attempts fail with the same signature before
DevTools. This is a live lane health blocker, not an assertion failure in the
new shared-tab checks.

## Done Definition

- Multiple clients can safely share one authenticated runtime profile without
  duplicate Chrome profile locks.
- Access-plan tells clients whether to reuse, open a tab, wait, reject, or
  launch.
- Direct duplicate process launches remain blocked by default.
- Shared tabs have ownership, traceability, leases, and cleanup.
- A live test proves two clients sharing one profile through one retained
  browser.
- Docs clearly say profile sharing means retained browser tab or window
  sharing, not multiple Chrome processes.
