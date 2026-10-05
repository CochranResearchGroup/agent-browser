# RDP Ready To Go Plan

Date: 2026-06-18
State: CLOSED
Lane: P14/P16
Depends On:
- `docs/dev/plans/0025-2026-06-01-remote-view-target-attribution-and-idle-display-plan.md`
- `docs/dev/plans/0026-2026-06-04-resource-monitor-and-garbage-collector-plan.md`
- `docs/dev/plans/0027-2026-06-05-minimal-runtime-profile-reuse-plan.md`
- `docs/dev/plans/0035-2026-06-15-external-byop-browser-adoption-plan.md`

## Purpose

Agent-browser should be ready to run hidden, remotely viewable RDP browser
work without stochastic operator cleanup. The current workstation can pass
basic RDP and Guacamole readiness, but the many-to-many live gate exposed
three readiness gaps:

- stale X lock files can exhaust the private display allocator even when no
  matching X socket or Xvfb process exists;
- Guacamole route readiness can emit public dyndns.org URLs while live iframe
  tests require local embeddable URLs;
- multiple browser launches for the same profile can collide unless the
  service has an explicit profile-sharing model.

This plan makes those behaviors deterministic. A healthy install should know
which RDP routes are usable for local embedding, public operator ingress, and
service launch planning. It should clean or quarantine stale display state
before launches fail. It should also make profile concurrency explicit so
clients share one retained browser safely instead of accidentally starting
competing Chrome processes on the same profile directory.

## Live Evidence

The 2026-06-16 many-to-many RDP gate showed:

- `pnpm test:rdp-gateway-readiness-live` passed for `guacd`, `xrdp`, local TCP,
  Guacamole web, and public ingress.
- `pnpm test:rdp-guac-route-pool-readiness -- --report-only` passed with two
  distinct route candidates.
- The first many-to-many run failed because route URLs used
  `https://agent-browser.ecochran.dyndns.org/guacamole/#/client/...`, while
  the harness requires local embeddable Guacamole frame URLs.
- Rerunning with `AGENT_BROWSER_REMOTE_VIEW_URL=http://127.0.0.1:8092/guacamole/`
  produced local route URLs and passed route-pool readiness.
- The next run failed with `No available X display number found for
  remote_headed launch` because `/tmp/.X90-lock` through `/tmp/.X129-lock`
  existed without matching X sockets.
- After removing stale allocator-range locks, browser A launched on `:90`.
- Browser B then failed because the service tried to launch against browser A's
  already-active runtime profile instead of treating B as a separate browser,
  tab, or window sharing decision.

Conclusion: the RDP backend is not the primary issue. The readiness problem is
service orchestration around display allocation, route URL roles, and profile
concurrency.

## Operating Invariants

```text
Agent-browser owns display allocation, browser/profile leases, route metadata,
and live readiness gates. Operators and downstream clients should not need to
manually repair stale X locks, guess which Guacamole URL to use, or decide
whether a profile may be opened twice.
```

```text
A Chrome profile directory must not be used by two unrelated Chrome process
groups at the same time. Simultaneous client work should share a retained
browser process through tabs or windows, with service-owned leases and
serialized control.
```

## Profile Sharing Position

Several clients sharing a single authenticated profile is desirable, but the
safe unit of sharing is the retained browser lane, not the profile directory.

Supported model:

- one live browser process group owns the profile lease;
- many clients can observe the browser through viewer leases;
- many clients can work in different tabs, or optionally different windows,
  inside that same browser process group;
- control is serialized per browser or per tab through service-owned jobs;
- each tab/window has an owner session, controller lease, trace filter, and
  cleanup policy;
- access-plan recommends `reuse_existing_browser` plus `tab_new`,
  `tab_handle_refresh`, `view_focus`, or future `window_new` instead of
  launching another Chrome process on the same user data directory.

Rejected model:

- two independent Chrome launches using the same profile directory;
- clients bypassing the service and racing CDP directly against one profile;
- profile sharing without tab/window attribution, controller leases, and
  deterministic cleanup.

This means the profile allocator should distinguish:

- `exclusive_process`: only one browser process group may hold the profile;
- `shared_browser_tabs`: multiple client sessions may share the browser via
  separate tabs;
- `shared_browser_windows`: multiple client sessions may share the browser via
  separate browser windows, once window attribution is implemented;
- `duplicate_process_allowed`: only for reviewed throwaway or explicitly
  isolated cases, never as the default for authenticated profiles.

## URL Routing Position

The dyndns.org Guacamole URL is still the right public operator ingress. The
local URL appeared because the current live harness embeds Guacamole from the
local workstation and rejects public Guacamole URLs unless a reviewed public
ingress diagnostic override is set.

The fix is not to choose local or public globally. The fix is to model route
URLs by audience:

- `localEmbedUrl`: local dashboard and live harness iframe embedding, usually
  `http://127.0.0.1:8092/guacamole/#/client/...`;
- `publicOperatorUrl`: externally reachable operator link, usually
  `https://agent-browser.ecochran.dyndns.org/guacamole/#/client/...`;
- `healthUrl`: route used by TCP or HTTP readiness checks;
- `dashboardEmbedUrl`: route selected by the dashboard based on whether the
  dashboard is running locally or through public ingress;
- `externalUrl`: backward-compatible public URL for humans and handoff notes.

Access-plan and service status should return a structured route descriptor
instead of one ambiguous URL. Tests can assert that the local embedding route
is used for local iframe harnesses while public ingress remains available for
remote operators.

## Parent Goal Definition

Goal: make hidden RDP-backed browser work deterministic and ready to use from
agent-browser without manual cleanup or route guessing.

Done means:

- stale private-display lock files do not cause false allocator exhaustion;
- readiness and doctor surfaces identify stale display state before launch;
- Guacamole routes expose local embedding and public operator URLs separately;
- route-pool readiness proves both local and public routes where configured;
- many-to-many live smoke launches two hidden RDP browser workspaces without
  manual environment fixes;
- profile concurrency policy is explicit in service state, access-plan, and
  launch behavior;
- authenticated profiles default to one retained browser process group with
  shared tab/window acquisition rather than duplicate process launch;
- no-launch tests cover stale locks, URL role selection, and profile-sharing
  decisions;
- live tests prove local embedding, public ingress, and multi-client tab
  sharing on an isolated synthetic profile;
- docs, CLI help, generated clients, dashboard guidance, and the
  `agent-browser` skill explain the new ready-to-go model.

## Non-Goals

- Do not weaken profile-lock safety by allowing arbitrary duplicate Chrome
  processes on authenticated profiles.
- Do not remove public dyndns.org operator ingress.
- Do not require public ingress for local iframe tests.
- Do not make Guacamole credentials, cookies, or private route secrets part of
  test artifacts.
- Do not mutate downstream client repos as part of this plan.
- Do not make AuraCall-specific rules, selectors, or profile paths part of
  agent-browser.

## Subagent Work Allocation

Use one subagent per slice. Each subagent should return:

```text
Slice:
Goal:
Files changed:
Contract delta:
No-launch validation:
Live validation:
Readiness impact:
Residual risks:
Next slice readiness:
```

Recommended subagents:

1. Display Hygiene Agent: stale X lock detection, allocator hardening, doctor
   integration, cleanup command.
2. Route Semantics Agent: local/public Guacamole route descriptor, readiness
   output, docs, client types.
3. Profile Concurrency Agent: profile-sharing policy, access-plan decisions,
   tab/window acquisition semantics.
4. Live Gate Agent: many-to-many harness, local and public route proof,
   dashboard iframe smoke.
5. Documentation Agent: README, CLI help, docs site, skill updates, migration
   guidance for downstream clients.

Slices A and B can run in parallel after they agree on readiness JSON shape.
Slice C can run in parallel for no-launch policy work, but its live proof waits
for Slice B route descriptors. Slice D waits for A-C. Slice E follows each
public contract change and closes the plan.

## Slice A: Private Display Hygiene

State: DONE for allocator hardening and RDP gateway readiness reporting. The
single consolidated ready-to-go doctor surface remains in Slice E.

Goal: make private virtual display allocation resilient to stale X state.

Deliverables:

- Add a display-state scanner for the allocator range, currently `:90` through
  `:129`.
- Classify each display as:
  - `free`;
  - `active_socket`;
  - `active_x_process`;
  - `stale_lock_no_socket`;
  - `stale_lock_reused_pid`;
  - `unknown`.
- Prefer a robust allocator path that either uses `Xvfb -displayfd` when
  available or validates lock, socket, and process state before rejecting a
  display number.
- Add a service-owned cleanup command for stale allocator-range locks only.
- Integrate the scanner into `agent-browser install doctor` and RDP readiness
  output.
- Make failed remote-headed launch diagnostics report the blocked display
  range and stale-lock count, not only `No available X display number`.
- Ensure cleanup never removes sockets for active X servers and never touches
  displays outside the agent-browser allocator range.

Acceptance:

- No-launch tests cover stale lock without socket, active socket, reused PID,
  and active X server cases.
- A live smoke creates synthetic stale allocator-range lock files in a temp
  namespace or controlled fixture and proves cleanup frees the allocator.
- The remote-headed launch path either self-heals safe stale locks or returns a
  one-command remediation hint.

Suggested validation:

```bash
cargo test --manifest-path cli/Cargo.toml display_allocation -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml install_doctor -- --test-threads=1
pnpm test:rdp-gateway-readiness-live
```

## Slice B: Guacamole Route Descriptor

State: DONE for route-pool readiness output and live harness consumption.

Goal: remove ambiguity between local embeddable URLs and public operator URLs.

Deliverables:

- Introduce a route descriptor in service state, readiness output, and client
  types with at least:
  - `localEmbedUrl`;
  - `publicOperatorUrl`;
  - `dashboardEmbedUrl`;
  - `healthUrl`;
  - `externalUrl`;
  - `embeddingPolicy`;
  - `providerMode`.
- Update route-pool readiness to emit both local and public URLs when both are
  configured.
- Keep dyndns.org URLs as the public operator path.
- Use local URLs for local iframe harnesses by default.
- Permit public ingress iframe diagnostics only with an explicit reviewed flag.
- Make dashboard route selection deterministic based on dashboard origin and
  route descriptor availability.
- Preserve backward compatibility for older consumers that read `frameUrl` or
  `externalUrl`.

Acceptance:

- No-launch tests prove route descriptor selection for local dashboard, public
  dashboard, public diagnostic override, and missing local route cases.
- Route-pool readiness reports local and public availability separately.
- A live smoke proves `127.0.0.1:8092` local embedding and dyndns.org public
  ingress both respond, without treating one as a replacement for the other.

Suggested validation:

```bash
pnpm test:rdp-guac-route-pool-readiness -- --report-only
pnpm test:rdp-gateway-readiness-live
pnpm test:service-client
pnpm test:service-api-mcp-parity
```

## Slice C: Profile Sharing And Tab Acquisition Policy

State: DONE for explicit access-plan sharing metadata and duplicate-process
profile lease proof. Window sharing remains future work.

Goal: make simultaneous client sharing explicit and safe.

Deliverables:

- Add profile concurrency metadata to service profiles and access-plan output:
  - `profileProcessPolicy`;
  - `clientSharingPolicy`;
  - `defaultAcquisition`;
  - `maxConcurrentTabs`;
  - optional `maxConcurrentWindows`.
- Teach access-plan to recommend shared acquisition when a live browser already
  holds the selected profile:
  - `reuse_existing_browser` plus `tab_new` for a new client tab;
  - `reuse_existing_browser` plus `view_focus` for an existing compatible tab;
  - future `window_new` when window attribution is available.
- Keep direct second process launch rejected by default for active profile
  leases.
- Add a reviewed escape hatch for explicitly isolated duplicate process lanes
  only when the profile policy allows it.
- Ensure service jobs serialize mutating actions per browser or per tab and
  expose controller lease conflicts clearly.
- Ensure tab handles include profile origin, browser id, session name, tab id,
  controller lease state, and cleanup policy.

Acceptance:

- No-launch tests prove an active profile holder yields a shared-tab
  recommendation, not a duplicate launch.
- No-launch tests prove duplicate process launch remains rejected for
  authenticated profiles.
- A live smoke launches one browser with an isolated synthetic profile, opens
  two service-owned tabs for two client sessions, and proves both tabs remain
  attributable and controllable.
- The smoke proves cleanup can close one client tab without closing the shared
  profile browser unless policy says otherwise.

Suggested validation:

```bash
cargo test --manifest-path cli/Cargo.toml service_access_plan -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml profile_lease -- --test-threads=1
pnpm test:service-request
pnpm test:service-client
```

## Slice D: Many-To-Many RDP Live Gate

State: DONE for the current local workstation gate with workspace-backed
runtime.

Goal: make the many-to-many RDP gate pass from a clean workstation without
manual URL or X-lock intervention.

Deliverables:

- Update `pnpm test:rdp-guac-many-to-many-live` to consume the route descriptor
  instead of raw `frameUrl` assumptions.
- Add preflight assertions that fail with targeted remediation for display,
  route, profile, and Guacamole readiness.
- Prove two hidden RDP browser workspaces can launch using distinct route
  candidates.
- Prove two clients can connect through local embedding.
- Prove the public operator URLs remain available for the same routes.
- Capture artifacts that distinguish backend health, route selection, display
  allocation, profile sharing decision, viewer lease, controller lease, and
  cleanup.

Acceptance:

- The live gate passes after a fresh route-pool readiness run.
- The live gate does not require manual deletion of `/tmp/.X*-lock` files.
- The live gate does not require hand-exporting a local URL if the route
  descriptor is configured.
- Failed runs leave no stale allocator-range locks and no active throwaway
  Chrome processes.

Suggested validation:

```bash
pnpm test:rdp-guac-route-pool-readiness -- --report-only
pnpm test:rdp-guac-many-to-many-live
pnpm test:rdp-guac-private-display-live
```

## Slice E: Ready-To-Go Doctor And Documentation

State: DONE

Goal: make readiness visible before a client depends on RDP browser work.

Deliverables:

- Add a single ready-to-go doctor summary for RDP browser operation that
  includes:
  - `guacd`;
  - `xrdp`;
  - Guacamole containers;
  - local embed route;
  - public operator route;
  - route-pool candidate count;
  - private display allocator state;
  - stale display locks;
  - profile lease pressure;
  - duplicate profile pressure;
  - dashboard iframe readiness.
- Add CLI help, README, docs site, and skill guidance for:
  - why local embed and public operator URLs both exist;
  - how clients should request hidden RDP browsers;
  - how clients should share profiles safely;
  - what to do when readiness is degraded.
- Add generated client helper summaries for RDP readiness and route
  descriptors if public API shape changes.

Implemented:

- `agent-browser doctor remote-view` now runs the RDP gateway readiness helper
  and includes `rdpGateway` data in the consolidated doctor report.
- The doctor summary now reports private display allocator readiness and
  local/public Guacamole route readiness alongside route-pool, route-display,
  display-access, privilege, viewer-prerequisite, config, and drift evidence.
- The route-pool entry schema and generated observability client type now name
  `routeDescriptor`.
- CLI help, README, docs site guidance, and the `agent-browser` skill describe
  local embed URLs, public operator URLs, and the retained-browser profile
  sharing model.

Acceptance:

- A user can run one documented command and know whether the workstation is
  ready for hidden RDP browser work.
- The command reports actionable remediation, not raw implementation errors.
- Docs explicitly say dyndns.org remains the public remote-management surface.
- Docs explicitly say simultaneous profile sharing happens through retained
  browser tabs or windows, not duplicate Chrome processes.

Suggested validation:

```bash
pnpm validation:select -- --base HEAD
pnpm --dir docs build
diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md
git diff --check
```

## Overall Validation Matrix

No-launch:

```bash
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
cargo test --manifest-path cli/Cargo.toml service_access_plan -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml service_config -- --test-threads=1
pnpm test:service-client
pnpm test:service-api-mcp-parity
pnpm validation:select -- --base HEAD
```

Live:

```bash
pnpm test:rdp-gateway-readiness-live
pnpm test:rdp-guac-route-pool-readiness -- --report-only
pnpm test:rdp-guac-many-to-many-live
pnpm test:rdp-guac-private-display-live
```

Docs and handoff:

```bash
pnpm --dir docs build
git diff --check
```

## Done Definition

- A fresh or long-running workstation can report RDP browser readiness without
  manual inspection of `/tmp` or Guacamole containers.
- Stale display locks are safely classified and cleaned or bypassed.
- Local and public Guacamole URLs are both first-class route outputs.
- Public dyndns.org operator ingress remains supported and tested.
- Local embedding remains supported and tested.
- Authenticated profiles are shared through retained browser tabs or windows,
  not duplicate Chrome process groups.
- The many-to-many RDP live gate passes without ad hoc environment edits.
- The dashboard and generated clients expose enough structured readiness data
  for downstream clients to decide whether to launch, reuse, attach, or wait.

## Progress Update

Updated on 2026-06-18.

Implemented:

- Added Linux X display state classification in the Chrome remote-headed
  launcher for the private display allocator range `:90` through `:129`.
- The allocator now treats socket-free stale locks as recoverable and removes
  only those safe stale lock files before selecting the display.
- The remote-headed display exhaustion error now reports active, stale, and
  unknown display counts instead of only saying no display was available.
- `pnpm test:rdp-gateway-readiness-live` now reports a
  `private_display_allocator` readiness component with free, stale, active,
  and first-available display evidence.
- `pnpm test:rdp-guac-route-pool-readiness -- --report-only` now emits a
  structured `routeDescriptor` per route with `localEmbedUrl`,
  `publicOperatorUrl`, `dashboardEmbedUrl`, `healthUrl`, `externalUrl`,
  `embeddingPolicy`, and `providerMode`.
- Route-pool readiness keeps dyndns.org as `externalUrl` and
  `publicOperatorUrl`, while using the local `127.0.0.1:8092` Guacamole URL
  for `frameUrl` and local embedding by default.
- Route-pool readiness now infers active XRDP display names from
  `scripts/inspect-rdp-route-displays.js` when explicit
  `AGENT_BROWSER_RDP_ROUTE_A_DISPLAY_NAME` and
  `AGENT_BROWSER_RDP_ROUTE_B_DISPLAY_NAME` variables are not set.
- The many-to-many live harness now consumes route descriptors, passes explicit
  top-level `sessionName`, `runtimeProfile`, and `profile` values, and
  persists route descriptors in seeded route-pool entries.
- `RoutePoolEntry` now preserves `routeDescriptor` in service state.
- Access-plan profile reuse now reports `profileProcessPolicy:
  exclusive_process`, `clientSharingPolicy: shared_browser_tabs`,
  `defaultAcquisition`, `maxConcurrentTabs`, and `maxConcurrentWindows`.

Validation evidence:

```bash
cargo test --manifest-path cli/Cargo.toml x_display -- --test-threads=1
cargo fmt --manifest-path cli/Cargo.toml -- --check
pnpm test:rdp-gateway-readiness-live
pnpm test:rdp-guac-route-pool-readiness -- --report-only
node --check scripts/smoke-rdp-gateway-readiness.js
node --check scripts/smoke-rdp-guac-route-pool-readiness.js
node --check scripts/test-rdp-guac-many-to-many-live.js
cargo test --manifest-path cli/Cargo.toml service_access_plan_reuses_external_byop_attached_browser_without_host_request -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml service_access_plan_recommends_waiting_for_profile_lease -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml test_service_profile_lease_gate_blocks_duplicate_live_profile_lane -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml test_service_profile_lease_gate_allows_duplicate_lane_route_hints -- --test-threads=1
AGENT_BROWSER_RDP_TEST_USE_INSTALLED=0 pnpm test:rdp-guac-many-to-many-live
```

Live result:

- The workspace-backed many-to-many live smoke passed with artifacts at
  `/tmp/agent-browser-rdp-guac-many-to-many-2026-06-18T21-40-45-039Z`.
- The passing route-pool JSON included local embed URLs, public dyndns.org
  operator URLs, and inferred display bindings `:12` and `:11`.

Additional closeout update:

- `agent-browser doctor remote-view --json` now exposes the canonical
  ready-to-go summary. A `cargo run` debug-binary invocation reported expected
  install drift against the installed PATH binary, but the RDP-specific data
  was ready: private display allocator ready, route pool ready, route displays
  ready, route display access ready, viewer prerequisites ready, and
  simultaneous viewing ready.
- `pnpm test:rdp-gateway-readiness-live` passed with private display allocator
  evidence `:90-:129 free=38 stale_locks=2 active=0 first_available=:92`.
- `pnpm test:rdp-guac-route-pool-readiness -- --report-only` passed with
  `localEmbedReady=true`, `publicOperatorReady=true`, dyndns.org public
  operator URLs, local `127.0.0.1:8092` embed URLs, and inferred route display
  bindings `:12` and `:11`.
- Route descriptor schema and generated observability client typing were
  updated after the public route-pool shape became explicit.

Residual:

- The installed user-scoped runtime at `/home/ecochran76/.local/bin/agent-browser`
  was not replaced in this plan execution; the live many-to-many proof used
  `AGENT_BROWSER_RDP_TEST_USE_INSTALLED=0` to run the workspace source.
- Dashboard iframe readiness remains a doctor component with `unknown` status
  until the dashboard browser harness runs in the target operator environment.
