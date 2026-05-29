# Guacamole RDP Many-To-Many Viewing Plan

Date: 2026-05-28
State: COMPLETE
Lane: P03
Outcome: COMPLETE
Current state: Slice A provider topology audit, Slice B service allocation
contracts, and Slice C private display allocation are complete. Slice D has
service-side static route-pool checkout, target-mismatch, contention, and
exhaustion checks. Slice E has explicit viewer/controller lease mutation and
denial contracts. Slice F has no-launch dashboard support for opening a tiled
remote-workspace view that renders two embeddable service-owned routes side by
side and warns when the chosen routes are shared. Slice G reconcile and
route-pool repair is complete. Slice H now has a passing guarded end-to-end
live gate. The route-specific Guacamole connections 4 and 5 map to independent
XRDP displays `:12` and `:11`; `agent-browser doctor remote-view` reports route
pool ready, route displays ready, route display access ready, and simultaneous
viewing ready. The OCR-backed many-to-many gate passed on 2026-05-29 with
artifacts at
`/tmp/agent-browser-rdp-guac-many-to-many-2026-05-29T01-34-49-701Z`.
The gate proves Browser A and Browser B are simultaneously visible through
separate Guacamole/RDP routes, exercises refresh, closes Browser A, verifies
Browser B remains ready, and proves route-pool release on close.
The one-time `agent-browser` privileged group/helper is now installed on the
live host and `agent-browser doctor remote-view` reports
`privileged helper: ready=true`, `userInGroup=true`, and
`requires interactive sudo: false`. The normal installer now owns this setup
through `agent-browser install --with-deps --with-remote-view-privileges`, so
future release binaries can configure the desktop maintenance path with one
intentional authorization instead of a repo-local manual script.
`pnpm setup:rdp-guac-route-pool` is now guarded by the route-display inspector:
it refuses to create route-specific users unless current evidence shows the
existing `agent-browser-rdp` topology has collapsed to one display, or an
operator passes a reviewed `--force` override.
After the route-specific sessions are open, the browser-launching user also
needs local X access to those XRDP-owned displays. `pnpm
grant:rdp-route-display-access -- --dry-run` reports the exact `xhost` grants,
and `pnpm grant:rdp-route-display-access -- --apply` applies them through the
installed helper when available.

Route maintenance now has a narrow `agent-browser` privileged group path
instead of repeated broad sudo. `pnpm install:privileges -- --apply` installs a
root-owned helper outside the writable checkout, adds the operator user to the
group, and installs sudoers limited to that helper for group members. Do not
allow passwordless sudo for a script under the mutable repository checkout.

## Purpose

Agent Browser must support more than focusing one browser on one shared RDP
desktop. The intended durable behavior is:

- each remote-headed browser can get its own isolated virtual desktop
- each isolated desktop can be exposed through its own Guacamole/RDP route
- multiple browser workspaces can be viewed at the same time
- multiple external viewers can observe the same workspace when the provider
  supports it
- write/control access is governed by an explicit controller lease
- the dashboard can switch, tile, refresh, and recover views without stale
  target state or hardcoded Guacamole client routes

The failure mode seen during manual validation on 2026-05-27 is the motivating
example: two `remote_headed` browsers were healthy, but both were launched into
the same configured shared display and the same Guacamole route. Agent Browser
could focus Browser A or Browser B, but it did not prove simultaneous viewing
of two separate browser desktops.

## Roadmap Context

P01 proved baseline RDP/Guacamole reliability. P02 removed hardcoded route
repair and made shared-route ownership service-visible. P03 is the distinct
route and private-display expansion that P02 intentionally deferred.

This lane keeps RDP/Guacamole as the full-control backend. It does not replace
the route with CDP streaming or noVNC. Those remain separate backend-family
lanes and can later reuse the same allocation and viewer-lease concepts.

## Definitions

- **Browser workspace**: one service-owned browser record plus its owner
  daemon session, active tab identity, profile, browser build, display
  allocation, and stream route.
- **Display allocation**: the concrete remote desktop target that hosts one
  or more browser windows. For this lane, the preferred allocation is one
  private virtual display per browser workspace.
- **Guacamole route allocation**: the provider route that exposes one display
  allocation to external viewers. It includes the Guacamole connection id,
  frame URL, external URL, route id, route source, and readiness state.
- **Viewer lease**: service-owned state for a human or software viewer that is
  observing or controlling a workspace.
- **Controller lease**: the single write/control lease for a workspace when the
  provider or policy allows only one controller.
- **Many-to-many viewing**: many browser workspaces can be open at the same
  time, and many viewers can observe one or more of those workspaces without
  forcing browser focus switching on a shared desktop.

## Product Invariants

This lane is not complete until these invariants hold:

- A browser workspace with `displayIsolation=private_virtual_display` has a
  display allocation that is not reused by unrelated live browsers.
- A private display allocation has a corresponding Guacamole route allocation,
  or the service marks the workspace as not externally viewable with a typed
  readiness reason.
- Two private browser workspaces never point at the same Guacamole route unless
  the route is explicitly modeled as a shared route.
- Shared display remains available only as an explicit low-contention override.
- Dashboard Control and View never infer a route from a hardcoded Guacamole
  client hash.
- Dashboard Control and View open the selected workspace's route, not whichever
  browser is currently focused on a shared desktop.
- Switching between Browser A and Browser B does not require moving either
  browser window on a shared display when both have private routes.
- A viewer can observe Browser A and Browser B simultaneously in separate
  dashboard tabs or tiles.
- Multiple viewers can observe the same browser workspace when the provider is
  in `simultaneous_view` mode.
- If the provider is single-viewer or single-controller, the service reports
  that state and offers takeover or reconnect actions instead of showing a
  blank iframe.
- A controller lease is explicit, durable enough for dashboard refresh, and
  visible in service trace output.
- Reconcile can identify and repair stale display, route, target, and viewer
  claims after daemon restart, browser crash, provider restart, or manual
  browser reuse.

## Non-Goals

- Do not add a second production fallback that synthesizes
  `MQBjAHBvc3RncmVzcWw=` or any other workstation-specific route.
- Do not treat CDP screenshots as a substitute for RDP full-control validation.
- Do not require all providers to support simultaneous control. Observation and
  control are separate capabilities.
- Do not make the dashboard own browser/process/display state. The dashboard is
  a client of service state.
- Do not require one Guacamole deployment topology. Static route pools,
  generated Guacamole connections, and provider-discovered routes are all valid
  if they produce the same service contract.
- Do not add host users, Guacamole records, config files, or environment
  variables as the first response to an unclear remote-view failure. Doctor
  discovery must identify existing reusable state and recommend the smallest
  reversible action first.

## Doctor-First Setup Contract

P03 now treats setup discovery as a required product surface, not a collection
of helper scripts. The remote-view doctor must answer the operator question:

```text
What is installed, what is configured, what is healthy, what is stale, and what
is the one next command that changes the least state?
```

The implementation target is one doctor entry point that can later be exposed
as `agent-browser doctor`, `agent-browser doctor remote-view`, or a service
doctor endpoint. It should compose existing checks rather than copy their
logic into yet another script.

Doctor sections:

- **Install**: active `agent-browser` binary path, package/install doctor
  status, checkout binary status, browser executable, stealthcdp manifest, and
  launch-config readiness.
- **Runtime**: default runtime profile, profile lock owner, live browser PID,
  DevTools reachability, service worker status, queue depth, retained
  incidents, and reconcile freshness.
- **Network**: dashboard ingress, Guacamole web ingress, Guacamole iframe
  reachability, public URL health, Docker network, guacd TCP, host XRDP TCP,
  and host firewall symptoms when observable without mutation.
- **RDP host setup**: existing Linux users that are known RDP users, whether
  `agent-browser-rdp` exists, whether route-specific users exist, XRDP
  `sesman.ini` policy, `MaxSessions`, `X11DisplayOffset`, active Xorg/Xvnc
  displays, and whether the existing-user color-depth strategy is compatible.
- **Guacamole provider setup**: Compose directory, container names, database
  availability, RDP connection records, selected managed route-pool entries,
  redacted target identities, connection color depth, permissions, and whether
  stale or duplicate records should be repaired or left alone.
- **Secrets and config inventory**: user-scoped secret files and env/config
  sources by path and key presence only. The doctor must never print passwords,
  tokens, cookies, browser auth artifacts, signed links, or raw private
  route URLs beyond the already configured public route roots.
- **State ownership**: which facts come from tracked repo docs, user-scoped
  config, service state, Guacamole DB, Docker runtime, host OS, or live browser
  runtime.
- **Recommended action**: exactly one primary next command plus safe
  alternatives. Examples include `agent-browser install doctor`,
  `pnpm sync:rdp-guac-existing-user-route-pool`,
  `pnpm test:rdp-guac-route-pool-readiness -- --report-only`,
  `pnpm inspect:rdp-route-displays`, `agent-browser service reconcile`, or the
  final many-to-many live gate.

Doctor invariants:

- Prefer reuse of `agent-browser-rdp` when it exists and the XRDP policy can
  support distinct sessions by color depth.
- Do not recommend creating `agent-browser-rdp-a` or
  `agent-browser-rdp-b` unless the doctor proves the existing-user path cannot
  produce distinct displays.
- Do not recommend deleting Guacamole records automatically. Surface stale,
  duplicate, or unmanaged records with exact ids and a dry-run repair command.
- Route selection must prefer managed P03 route records over the legacy shared
  route, but the legacy route remains labeled as shared fallback.
- Every mutating setup command must be paired with a read-only doctor or
  dry-run command that explains why it is needed.
- The doctor result should be available in JSON and text modes so docs,
  dashboard, MCP, and shell users can consume the same evidence.

## Target Architecture

### Allocation Model

Add a service-owned allocation model with three linked records:

1. `DisplayAllocation`
2. `RemoteViewRoute`
3. `ViewerLease`

`DisplayAllocation` fields:

- `id`
- `displayName`, for example `:21`
- `displayIsolation`: `private_virtual_display`, `shared_display`, or
  `ambient_display`
- `ownerBrowserId`
- `ownerSessionId`
- `profileId`
- `browserBuild`
- `host`: `remote_headed`
- `state`: `allocating`, `ready`, `degraded`, `released`, `orphaned`, or
  `failed`
- `pidHints`: X server, window manager, XRDP, and browser process ids when
  available
- `createdAt`, `updatedAt`, and `lastHealthCheckAt`
- `readiness`: typed component readiness for X server, window manager, XRDP,
  browser paint, and route reachability

`RemoteViewRoute` fields:

- `id`
- `provider`: normally `rdp_gateway`
- `displayAllocationId`
- `browserId`
- `sessionId`
- `routeSource`: `config`, `pool`, `discovered`, `generated`, `retained_state`,
  `fixture`, or `unknown`
- `connectionId`
- `connectionName`
- `routeTemplate`
- `frameUrl`
- `externalUrl`
- `readOnly`
- `controlInput`: normally `manual_attached_desktop`
- `providerMode`: `simultaneous_view`, `single_viewer`, `single_controller`,
  `unknown`, or `unavailable`
- `state`: `allocating`, `ready`, `reconnecting`, `degraded`, `released`,
  `orphaned`, or `failed`
- `lastProviderEvent`
- `readiness`: Guacamole web, guacd, RDP backend, auth, iframe, ingress, and
  viewer ownership checks

`ViewerLease` fields:

- `id`
- `routeId`
- `browserId`
- `viewerId`
- `viewerName`
- `viewerRole`: `observer`, `controller`, `pending_controller`, or `none`
- `openMode`: `embedded`, `external`, `fullscreen`, or `tile`
- `state`: `requested`, `connected`, `observing`, `controlling`,
  `takeover_requested`, `reconnecting`, `disconnected`, `expired`, or `failed`
- `lastViewerEvent`
- `expiresAt`
- `createdAt`
- `updatedAt`
- `lastHeartbeatAt`
- `serviceEventId`

### Route Provisioning Options

Support these route sources in order:

1. explicit service request fields for the workspace
2. persisted service provider config
3. a configured Guacamole route pool
4. provider discovery from Guacamole connection metadata
5. generated provider route, if an enabled provisioner can create one
6. retained state reuse after health verification

The first implementation can use a static pool if dynamic Guacamole connection
creation is not ready. The contract must still model each route as a distinct
allocation rather than a copied URL string.

### Display Provisioning Options

Support these display sources:

- private Xvfb or Xorg display per browser
- private XRDP session per display when required by Guacamole
- existing shared display when explicitly requested
- ambient display only for operator-local debugging

The preferred route for production remote control is:

```text
browser workspace -> private display -> XRDP target -> Guacamole connection -> dashboard route
```

If XRDP cannot expose multiple private displays cleanly on this host, add a
documented provider decision point before implementation continues:

- use one XRDP login session per browser display
- use one containerized RDP target per browser display
- switch the many-display provider to noVNC while preserving RDP/Guacamole as
  the shared full-control fallback

## Implementation Slices

### Slice A: Current-State And Provider Topology Audit

State: VALIDATED

Goal: establish what the current workstation can actually provision before
coding dynamic allocation.

Tasks:

- Inventory current Guacamole connection definitions and identify whether the
  deployment can expose multiple RDP targets or only one.
- Inventory current XRDP and guacd configuration.
- Record whether a Guacamole connection can target a distinct display, port,
  user session, container, or host.
- Verify whether `private_virtual_display` launches are visible through any
  existing provider route.
- Classify current route config as shared route, route pool, generated route,
  or unknown.
- Record the exact provider constraint that caused the 2026-05-27 manual
  test to collapse into focus switching.

Validation:

- `pnpm test:rdp-gateway-readiness-live -- --require-html5-client`
- no-launch inspection of persisted service provider config
- manual Guacamole admin or provider config readback, recorded in
  `docs/dev/notes/`
- `git diff --check`

Exit criteria:

- The repo has a note that names the supported first provider topology.
- The note states whether static route pool, generated Guacamole connections,
  or containerized targets are the first implementation path.
- Existing shared-route behavior is preserved and labeled as shared route.
- Recorded in
  `docs/dev/notes/2026-05-28-guac-rdp-p03-provider-topology-audit.md`.

### Slice B: Service Allocation Contracts

State: COMPLETE

Goal: make display, route, and viewer leases first-class service contracts.

Tasks:

- Add schemas for display allocations, remote view routes, route pools, and
  viewer leases under `docs/dev/contracts/`.
- Add compact collection responses for each record type.
- Extend service browser records so `viewStreams` reference route ids and
  display allocation ids.
- Extend service jobs so launch, focus, takeover, route allocation, and release
  actions retain requested and resolved allocation ids.
- Extend generated `@agent-browser/client` helpers for route lookup,
  allocation lookup, viewer lease request, controller takeover, and release.
- Keep `viewStreams[].url` backward compatible while making `frameUrl`,
  `externalUrl`, `routeId`, and `displayAllocationId` authoritative.

Validation:

- schema validation tests for every new contract
- `pnpm test:service-client-contract`
- `pnpm test:service-client-types`
- focused Rust serialization tests
- `pnpm validation:select -- --base <ref>`
- `git diff --check`

Exit criteria:

- A software client can discover whether Browser A and Browser B have distinct
  routes without parsing URLs.
- Dashboard can render route readiness without provider-specific string
  guessing.
- Existing P02 shared-route tests still pass.

Current evidence:

- Added no-launch schemas for display allocation, remote view route, route pool
  entry, and viewer lease records plus compact collection response envelopes.
- Extended `service-browser-record.v1.schema.json` so browser records and
  `viewStreams[]` can carry `displayAllocationId`, `providerMode`,
  `viewerLeaseIds`, and `controllerLeaseId`.
- Added persisted service-state collections, HTTP `GET` collection routes, MCP
  resources, and contract metadata for display allocations, remote view routes,
  route pool entries, and viewer leases.
- Extended service jobs so retained queued, running, terminal, cancelled, and
  failed-to-enqueue records can preserve requested and resolved display
  allocation, remote-view route, route-pool entry, viewer lease, and controller
  lease ids.
- Extended generated `@agent-browser/client/service-observability` types and
  runtime helpers with `getServiceDisplayAllocations()`,
  `getServiceRemoteViewRoutes()`, `getServiceRoutePool()`,
  `getServiceViewerLeases()`, `findServiceDisplayAllocation()`,
  `findServiceRemoteViewRoute()`, and `findServiceViewerLease()`.
- Added `service_remote_view_route_checkout`,
  `service_remote_view_route_release`, `service_viewer_lease_request`,
  `service_viewer_lease_release`, and
  `service_controller_lease_takeover` to the HTTP and MCP service request
  action contract.
- Added service-state mutations for route checkout, route release, viewer
  lease request, viewer lease release, and controller lease takeover. The
  mutation path updates retained display allocations, remote-view routes,
  route pool entries, viewer leases, and browser `viewStreams[]` metadata
  without launching a browser.
- Extended generated `@agent-browser/client/service-request` types and runtime
  helpers with `requestServiceRemoteViewRouteCheckout()`,
  `requestServiceRemoteViewRouteRelease()`, `requestServiceViewerLease()`,
  `releaseServiceViewerLease()`, and `takeoverServiceControllerLease()`.
- Validated contract metadata, MCP read resources, HTTP collection routing,
  record/response schema shape, generated client contract sync, JSON syntax,
  and whitespace checks.
- Added a focused no-launch Rust test for route checkout, viewer lease request,
  controller takeover, viewer release, route release, route-pool release, and
  browser view-stream update behavior.
- Added dashboard route rendering for browser rows, workspace nodes, selected
  browser details, view-stream cards, and workspace viewport headers. The
  rendered route summary includes route id, display allocation id, provider
  mode, viewer count, controller lease state, and readiness without parsing
  Guacamole URLs.
- Additional passing checks: `cargo fmt --manifest-path cli/Cargo.toml -- --check`,
  `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`,
  `cargo test --manifest-path cli/Cargo.toml service_model -- --test-threads=1`,
  `cargo test --manifest-path cli/Cargo.toml control_plane -- --test-threads=1`,
  `cargo test --manifest-path cli/Cargo.toml service_contracts -- --test-threads=1`,
  `cargo test --manifest-path cli/Cargo.toml service_health -- --test-threads=1`,
  `pnpm test:service-api-mcp-parity`, `pnpm test:service-client-contract`,
  `pnpm test:service-client-types`, `pnpm test:service-observability-client`,
  `pnpm test:service-client`, `pnpm test:dashboard-view-streams`,
  `pnpm test:dashboard-workspace-nodes`,
  `pnpm test:dashboard-workspace-navigator`,
  `pnpm test:dashboard-inspector-actions`, `pnpm test:dashboard-browser-table`,
  `pnpm build:dashboard`, `pnpm --dir docs build`, `jq empty` on
  `docs/dev/contracts/*.json`, `pnpm validation:select -- --base HEAD`, and
  `git diff --check`.
- Slice B exit criteria are met for no-launch service allocation contracts and
  dashboard route-readiness rendering. Live shared-route and private-route
  proof remains in later slices.

### Slice C: Private Display Allocator

State: COMPLETE

Goal: allocate one private virtual desktop per remote-headed browser when
requested or selected by access plan.

Tasks:

- Add a service allocator for private display names and lifecycle state.
- Ensure remote-headed launches without an explicit shared display default to
  `private_virtual_display`.
- Start the required X server, window manager, and browser process for each
  allocation.
- Persist the allocation before browser launch completes.
- Record display allocation ids on browser records and jobs.
- Add release behavior that closes browser-owned display resources without
  affecting unrelated browsers.
- Add orphan detection for displays whose browser process exited.
- Keep explicit `shared_display` requests working for the existing route.

Validation:

- unit tests for allocation idempotency, duplicate display prevention, release,
  and orphan detection
- Rust launch tests that assert distinct display names for two private
  remote-headed sessions
- live smoke that launches two private remote-headed browsers and verifies
  distinct display allocation records
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`

Exit criteria:

- Two private remote-headed browsers do not share a display name.
- Closing Browser A does not release Browser B's display.
- Service status clearly distinguishes private and shared display records.

Current evidence:

- Remote-headed browser persistence now upserts a `DisplayAllocation` keyed by
  display isolation and session identity. Private allocations use per-session
  ids, while shared and ambient allocations retain explicit display scope.
- Browser records retain `displayAllocationId`, and missing
  `viewStreams[].displayAllocationId` fields are filled from the resolved
  allocation.
- Launch metadata now fills the persisted display name from the launched
  remote-headed browser when the caller did not provide one.
- Browser process-exit persistence marks the owned display allocation
  `orphaned` and records a typed readiness reason
  `browser_process_exited`.
- Remote-headed launches now default to `private_virtual_display`. Explicit
  `shared_display` and `ambient_display` requests continue to bypass private
  Xvfb allocation policy.
- Closing a service-owned browser marks only that browser's owned display
  allocation `released`; unrelated browser display allocations remain ready.
- Added `pnpm test:rdp-guac-private-display-live`, which launches two live
  private remote-headed browsers, verifies distinct display allocation ids and
  distinct live display names, closes Browser A, and verifies Browser B remains
  ready. Passing artifact directory:
  `/tmp/agent-browser-rdp-guac-private-display-2026-05-28T03-38-49-676Z/`.
- Passing checks:
  `cargo fmt --manifest-path cli/Cargo.toml -- --check`,
  `cargo test --manifest-path cli/Cargo.toml remote_headed_browser_record_upserts_private_display_allocation -- --test-threads=1`,
  `cargo test --manifest-path cli/Cargo.toml process_exited_browser_health_marks_display_allocation_orphaned -- --test-threads=1`,
  `cargo test --manifest-path cli/Cargo.toml private_display_allocation_ids_are_scoped_per_session -- --test-threads=1`,
  `cargo test --manifest-path cli/Cargo.toml close_releases_only_owned_display_allocation -- --test-threads=1`,
  `cargo test --manifest-path cli/Cargo.toml apply_launch_host_hints -- --test-threads=1`,
  `cargo test --manifest-path cli/Cargo.toml service_health -- --test-threads=1`,
  `cargo test --manifest-path cli/Cargo.toml control_plane -- --test-threads=1`,
  `cargo test --manifest-path cli/Cargo.toml test_private_remote_display_policy_controls_xvfb_fallback -- --test-threads=1`,
  `cargo test --manifest-path cli/Cargo.toml test_remote_headed_virtual_displays_use_distinct_live_display_names -- --test-threads=1`,
  `pnpm test:rdp-guac-private-display-live`, and
  `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`.

Slice C exit criteria are met. Route binding remains in Slice D.

### Slice D: Guacamole Route Pool And Provisioner

State: IN_PROGRESS

Goal: bind each private display allocation to a distinct Guacamole route.

Tasks:

- Add provider config for a Guacamole route pool.
- Support static pool entries with `routeId`, `connectionId`, `connectionName`,
  `frameUrl`, `externalUrl`, and target display metadata.
- Add health checks that prove a pool entry reaches the expected display.
- Add route checkout and release semantics.
- Refuse to assign the same route to two private live browser workspaces unless
  the route is explicitly declared shared.
- Add optional provider discovery hooks for Guacamole connection metadata.
- Add optional generated-route hooks for future dynamic connection creation.
- Persist route allocation before the dashboard opens a workspace viewport.

Validation:

- route-pool parser and validation tests
- route checkout contention tests
- readiness tests for route target mismatch
- live smoke with at least two configured route entries
- live smoke that proves Browser A and Browser B have different `connectionId`
  or `routeId` values

Exit criteria:

- Two private browser workspaces can hold two distinct Guacamole routes.
- A stale route lease is not reused until health verification passes.
- The service reports a typed failure when no route is available.

Current evidence:

- `service_remote_view_route_checkout` now resolves a compatible static route
  pool entry when the caller does not provide `routePoolEntryId`, using the
  browser's retained private `displayAllocationId` and the pool entry target
  metadata.
- Route pool targets can bind by `displayAllocationId`, `browserId`,
  `sessionId`, or `displayName`. A requested pool entry that targets a
  different private display fails with `route_pool_target_mismatch`.
- The checkout path refuses to assign a ready route that is already bound to a
  different private display allocation and returns `route_pool_contention`.
- If a private display requires a pool-backed route and no compatible
  available entry exists, checkout returns `route_pool_unavailable` instead of
  falling back to a copied Guacamole URL.
- Successful checkout marks the selected pool entry `checked_out`, records the
  current route allocation id, and updates the browser `viewStreams[]` route
  metadata before dashboard clients open the workspace route.
- Checkout now rejects an explicitly failed or stale pool-entry readiness
  payload before marking a route externally viewable. The typed error is
  `route_pool_not_ready` and preserves missing-readiness static entries for
  existing configured pools until live provider probes are available.
- Added focused no-launch tests for automatic selection of two distinct
  matching pool entries, target mismatch plus contention rejection, explicit
  readiness rejection, and route pool exhaustion.
- Passing checks:
  `cargo fmt --manifest-path cli/Cargo.toml -- --check`,
  `cargo test --manifest-path cli/Cargo.toml remote_view_route_checkout -- --test-threads=1`,
  `cargo test --manifest-path cli/Cargo.toml service_health -- --test-threads=1`,
  `cargo test --manifest-path cli/Cargo.toml service_model -- --test-threads=1`,
  `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`, and
  `git diff --check`.

Remaining Slice D work:

- Add or point the workstation at at least two live Guacamole route-pool
  entries backed by distinct RDP targets.
- Add live provider probes that populate ready or failed route-pool readiness
  for the expected target display before checkout.
- Run the live two-route smoke proving Browser A and Browser B carry different
  `connectionId` or `routeId` values.

### Slice E: Viewer And Controller Lease Semantics

State: IN_PROGRESS

Goal: make many viewers and one controller per workspace explicit.

Tasks:

- Add `viewer_request`, `viewer_heartbeat`, `viewer_release`, and
  `controller_takeover` service actions, or extend `view_takeover` into those
  typed operations.
- Keep `view_takeover` backward compatible as a controller request.
- Track viewer identity from dashboard superuser, codex observer, external
  URL, or caller-supplied metadata.
- Record whether the provider supports simultaneous observers.
- Allow multiple observer leases when provider readiness says
  `simultaneous_view`.
- Enforce one controller lease unless provider policy says otherwise.
- Make failed controller takeover leave existing observers intact.
- Emit service events for viewer connected, viewer disconnected, controller
  requested, controller granted, controller denied, and route released.

Validation:

- service action tests for observer fan-out, controller contention, expiration,
  and release
- HTTP and MCP parity tests
- dashboard test for takeover and reconnect state
- live two-client same-route observer test
- live two-client controller takeover test

Exit criteria:

- A dashboard refresh preserves viewer/controller state.
- A second viewer can observe without stealing control when provider policy
  allows it.
- A controller takeover is explicit and auditable.

Current evidence:

- `service_viewer_lease_request`, `service_viewer_lease_heartbeat`,
  `service_viewer_lease_release`, and
  `service_controller_lease_takeover` are accepted service request actions and
  have generated `@agent-browser/client/service-request` helper coverage.
- Viewer lease request persists observer leases onto the retained route,
  viewer lease collection, and browser `viewStreams[]` metadata without
  launching a browser.
- Viewer heartbeat updates `lastHeartbeatAt`, `updatedAt`, optional
  `expiresAt`, and returns typed `viewer_heartbeat` mutation metadata.
- Routes with `providerMode: "single_viewer"` return a typed `viewer_denied`
  result for additional active viewers and retain a `controller_denied`
  audit event with the denial reason.
- Controller requests preserve existing observers, reject non-takeover
  controller conflicts with typed `controller_denied` metadata, and allow the
  explicit takeover action to replace the route controller lease.
- Route release disconnects retained viewer leases, clears controller state,
  returns the released viewer lease ids, and records a `route_released` event.
- Added contractual service event kinds for `viewer_connected`,
  `viewer_disconnected`, `controller_requested`, `controller_granted`,
  `controller_denied`, and `route_released`.
- Passing checks:
  `cargo test --manifest-path cli/Cargo.toml viewer_lease -- --test-threads=1`,
  `cargo test --manifest-path cli/Cargo.toml service_event_record_contract_matches_wire_shape -- --test-threads=1`,
  `pnpm test:service-client-contract`, and
  `pnpm test:service-request-client`.

Remaining Slice E work:

- Add dashboard takeover and reconnect state tests against the new explicit
  lease actions.
- Run the live two-client same-route observer test.
- Run the live two-client controller takeover test.

### Slice F: Dashboard Many-Workspace Viewing

State: IN_PROGRESS

Goal: make simultaneous viewing usable in the operator dashboard.

Tasks:

- Add a workspace route selector that prefers private route matches over shared
  route fallbacks.
- Add a tile or split view that can show Browser A and Browser B at the same
  time when both have ready routes.
- Keep single-workspace control view for detailed work.
- Show display id, route id, provider mode, viewer count, and controller owner
  in the viewport header or readiness strip.
- Prevent stale selected tabs from forcing a route back to shared display.
- Add an explicit warning when multiple browser rows share one route.
- Add recovery actions for route reconnect, viewer release, controller
  takeover, and route refresh.
- Keep external open behind service acceptance.

Validation:

- `pnpm test:dashboard-workspace-nodes`
- `pnpm test:dashboard-workspace-navigator`
- `pnpm test:dashboard-view-streams`
- dashboard tests for two private routes displayed simultaneously
- dashboard tests for one shared route warning
- rendered inspection with two active RDP-capable workspace rows open at once

Exit criteria:

- Browser A and Browser B can be visible at the same time in dashboard.
- The dashboard explains shared-route fallback instead of silently switching
  focus.
- Refreshing either viewport does not break the other viewport.

Current evidence:

- Dashboard workspace derivation now scores a browser's retained view streams
  and selects the best private/provider-owned route instead of blindly using
  the first stream. Pool, generated, or discovered routes, non-shared display
  allocation ids, RDP gateway streams, controllable streams, ready streams, and
  simultaneous-view provider mode all outrank shared fallback metadata.
- The workspace remote viewport uses the same scoring posture when restoring a
  selected browser from URL state, so a stale shared fallback stream does not
  override a private route after refresh.
- Existing duplicate Guacamole route diagnostics remain attached to affected
  workspace rows and explain when multiple browser records share one route.
- The dashboard route parser now accepts `view=workspace:tile` without a
  selected workspace. The workspace navigator includes an explicit tiled
  workspace action that opens this service-wide remote viewing surface.
- Tile mode selects the top two embeddable service-owned routes from retained
  browser stream state, using the same private/provider route scoring as
  single-workspace view mode. Each tile keeps its own iframe URL and refresh
  nonce, so refreshing Browser A does not remount Browser B.
- Tile cards show the selected browser title, route summary, per-tile refresh,
  external-open action, and a visible shared-route warning when two retained
  browser streams still point at the same Guacamole route.
- Single-workspace remote viewport now exposes explicit recovery controls for
  route refresh, observer reconnect, controller takeover, and retained viewer
  lease release. These controls call the service-owned actions
  `service_remote_view_route_checkout`, `service_viewer_lease_request`,
  `service_controller_lease_takeover`, and `service_viewer_lease_release`
  before refreshing service status or remounting the affected iframe.
- Added dashboard node fixture coverage proving a browser with both a shared
  fallback stream and a private pool stream opens the private pool route and
  reports the correct route id, display allocation id, route source, provider
  mode, viewer leases, and route summary.
- Passing checks:
  `pnpm test:dashboard-workspace-nodes`,
  `pnpm test:dashboard-workspace-navigator`,
  `pnpm test:dashboard-view-streams`,
  `pnpm test:service-client-types`,
  `pnpm build:dashboard`, and
  `git diff --check`.

Remaining Slice F work:

- Add rendered browser inspection for `view=workspace:tile` with two active
  RDP-capable workspace rows open at once.
- Run live inspection with two distinct Guacamole route-pool entries and
  confirm the tile surface keeps both browsers visible without focus switching.

### Slice G: Reconcile, Repair, And Cleanup

State: COMPLETE

Goal: keep allocation state accurate after crashes, restarts, and manual
intervention.

Tasks:

- Reconcile browser processes against display allocations.
- Reconcile Guacamole route leases against live browser workspaces.
- Mark orphaned display allocations and routes separately.
- Add dry-run and apply repair actions for stale routes.
- Add release behavior for expired viewer leases.
- Add incident grouping for route pool exhausted, route unreachable, display
  missing, provider auth failed, and iframe blocked.
- Ensure repair never steals a route from a healthy live browser.

Validation:

- reconcile unit tests for stale display, stale route, stale viewer, and stale
  controller cases
- service incident tests for grouped route failures
- live restart smoke for dashboard and daemon restart
- live browser crash smoke with route cleanup

Exit criteria:

- `service reconcile` reports actionable allocation drift.
- Repair can clean stale state without closing healthy unrelated browsers.
- Operators can distinguish browser crash from provider route failure.

Current evidence:

- `service_reconcile` now reconciles remote-view allocation state after browser
  and tab health reconciliation. It marks browser-owned display allocations
  `orphaned` when the owner browser is missing or not ready, marks dependent
  remote-view routes `orphaned` with typed readiness evidence, disconnects
  retained viewer leases whose routes are unavailable, expires stale viewer
  leases, clears stale controller leases, and keeps healthy routes intact.
- The reconciliation event now includes a `remoteView` summary with counts for
  orphaned display allocations, orphaned routes, released viewer leases,
  expired viewer leases, and cleared controller leases.
- Repository reconcile merge now persists reconciled display allocations,
  remote-view routes, route-pool entries, and viewer leases, so remote-view
  repair survives the same persisted reconcile path as browser and tab repair.
- Service incident derivation now creates grouped remote-view incidents from
  retained allocation state. It distinguishes route-pool exhaustion, route
  unreachable, missing display allocation, provider-auth failure, and iframe
  blocked readiness evidence, with service-triage recommendations that point
  operators at route readiness, display allocation state, provider auth, and
  route-pool availability.
- Added `service_route_pool_repair` as a reviewed service-request repair
  action for stale checked-out route-pool entries. It defaults to dry-run,
  reports candidate entry ids and stale reasons, preserves active ready routes,
  and only resets stale entries to `available` when `apply` is true.
- The service request contract, generated client types, and hand-authored
  client helper now include route-pool repair so dashboard or operator scripts
  can preview and apply stale checkout cleanup without hand-crafting request
  JSON.
- Added focused service-health tests for orphaning remote-view state after an
  unavailable browser and expiring viewer leases without releasing a healthy
  route.
- Added service-model incident tests for route readiness failures and route
  pool exhaustion.
- Added focused action tests for route-pool repair dry-run reporting and apply
  behavior that resets stale checkouts while leaving an active route checkout
  untouched.
- Added `pnpm test:rdp-guac-route-cleanup-live` for the live restart and
  browser-crash cleanup gate. The smoke launches an isolated `remote_headed`
  browser, seeds and checks out a route-pool entry, restarts the stream daemon,
  verifies a healthy route is not repairable, kills the browser process, runs
  `service reconcile`, then dry-runs and applies `service_route_pool_repair`.
- The live route-cleanup gate passed on 2026-05-28 with artifacts at
  `/tmp/agent-browser-rdp-guac-route-cleanup-2026-05-28T04-52-11-882Z`.
  Evidence includes `after-restart-reconcile-response.json`, which shows the
  checked-out route remained ready after stream restart; the browser-crash
  reconcile artifact, which shows the browser as `process_exited` and route as
  `orphaned`; the stale repair dry-run artifact, which reports one stale
  checkout candidate; the stale repair apply artifact, which reports one
  repaired checkout; and
  `after-route-pool-repair-service-status.json`, which shows the pool entry
  returned to `available` with `currentRouteAllocationId: null`.
- Passing checks:
  `pnpm generate:service-client`,
  `node scripts/generate-service-request-client.js --check`,
  `node scripts/generate-service-observability-client.js --check`,
  `node --check scripts/test-rdp-guac-route-cleanup-live.js`,
  `pnpm test:service-request-client`,
  `pnpm test:rdp-guac-route-cleanup-live`,
  `pnpm test:service-client-types`,
  `cargo fmt --manifest-path cli/Cargo.toml -- --check`,
  `cargo test --manifest-path cli/Cargo.toml service_route_pool_repair -- --test-threads=1`,
  `cargo test --manifest-path cli/Cargo.toml repair_route_pool_service_state -- --test-threads=1`,
  `cargo test --manifest-path cli/Cargo.toml service_contracts -- --test-threads=1`,
  `cargo test --manifest-path cli/Cargo.toml reconcile_orphans_remote_view_state_for_unavailable_browser -- --test-threads=1`,
  `cargo test --manifest-path cli/Cargo.toml reconcile_expires_remote_viewer_leases_without_releasing_healthy_route -- --test-threads=1`,
  `cargo test --manifest-path cli/Cargo.toml refresh_derived_views_groups_remote_view_route_failures -- --test-threads=1`,
  `cargo test --manifest-path cli/Cargo.toml refresh_derived_views_groups_route_pool_exhaustion_and_unreachable_entries -- --test-threads=1`,
  `cargo test --manifest-path cli/Cargo.toml service_reconcile -- --test-threads=1`,
  `cargo test --manifest-path cli/Cargo.toml service_health -- --test-threads=1`,
  `cargo test --manifest-path cli/Cargo.toml service_model -- --test-threads=1`,
  `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`, and
  `git diff --check`.

Remaining Slice G work:

- None.

### Slice H: End-To-End Many-To-Many Live Gate

State: IN_PROGRESS

Goal: prove the feature with real external viewing, not only service records.

Required live matrix:

- Browser A on private display A with route A
- Browser B on private display B with route B
- Viewer 1 opens Browser A and Browser B simultaneously
- Viewer 2 opens Browser A and Browser B simultaneously
- Viewer 1 controls Browser A while Viewer 2 observes Browser A
- Viewer 2 controls Browser B while Viewer 1 observes Browser B
- Browser A refresh does not affect Browser B
- Browser B refresh does not affect Browser A
- Closing Browser A releases only display A and route A
- Browser B remains visible after Browser A is closed

Validation command target:

```bash
pnpm test:rdp-guac-many-to-many-live -- --require-distinct-routes --require-two-viewers
```

The live gate must save:

- service status before launch
- service status after launch
- display allocation records
- route allocation records
- viewer lease records
- screenshots for both viewers and both browser workspaces
- direct provider route evidence for both route ids
- focused failure notes for any provider auth, iframe, ingress, or RDP issue

Exit criteria:

- The live gate proves two distinct routes and two distinct displays.
- The live gate proves simultaneous viewing without relying on focus switching.
- The live gate leaves enough artifacts to debug provider failures without
  rerunning immediately.

Current evidence:

- Added `pnpm test:rdp-guac-many-to-many-live` as the guarded Slice H live
  gate. The harness requires two distinct route-pool entries from
  `AGENT_BROWSER_RDP_ROUTE_POOL_JSON` or paired
  `AGENT_BROWSER_RDP_ROUTE_A_*` and `AGENT_BROWSER_RDP_ROUTE_B_*`
  environment variables, then launches two private `remote_headed` browsers,
  seeds and checks out distinct routes, creates two observer leases per route,
  grants Viewer 1 control for Browser A and Viewer 2 control for Browser B,
  opens `view=workspace:tile` in two dashboard clients, refreshes Browser A's
  tile, closes Browser A, and verifies Browser B remains ready.
- Tightened `pnpm test:rdp-guac-many-to-many-live` so passing requires visual
  target-binding proof. The harness crops the actual iframe rectangles from
  dashboard screenshots, runs OCR with `tesseract` against those remote-view
  pixels, and requires Browser A and Browser B marker text inside the matching
  route crops. This prevents a pair of Guacamole connection ids or iframe URLs
  from passing when they only show unrelated XRDP desktops.
- Extended `pnpm test:rdp-guac-many-to-many-live` to support route-targeted
  XRDP displays. Route entries may include `target.displayName` in
  `AGENT_BROWSER_RDP_ROUTE_POOL_JSON`, or
  `AGENT_BROWSER_RDP_ROUTE_A_DISPLAY_NAME` and
  `AGENT_BROWSER_RDP_ROUTE_B_DISPLAY_NAME` in paired environment variables.
  When display targets are present, the harness launches each browser directly
  onto the route's expected XRDP display with
  `displayIsolation="shared_display"`, asserts the two display names are
  distinct, and still requires visual OCR proof from the corresponding
  Guacamole route. Without display targets, the harness continues to use the
  service private-display allocator.
- After the OCR target-binding change, the current live invocation still fails
  at the earlier route-pool input gate, before any browser launch, because the
  workstation has no two-entry route-pool configuration. Failure artifact:
  `/tmp/agent-browser-rdp-guac-many-to-many-2026-05-28T05-19-57-600Z/failure.json`.
- Added documentation for the many-to-many gate in `README.md`,
  `docs/src/app/service-mode/page.mdx`, and `skills/agent-browser/SKILL.md`.
- Passing checks:
  `node --check scripts/test-rdp-guac-many-to-many-live.js`,
  `pnpm test:dashboard-view-streams`, and
  `git diff --check`.
- The first live invocation failed early as designed because the workstation
  does not currently expose two distinct route-pool entries. Failure artifact:
  `/tmp/agent-browser-rdp-guac-many-to-many-2026-05-28T04-59-03-972Z/failure.json`.
- A follow-up current-state check on 2026-05-28 confirmed the live blocker is
  provider configuration, not private browser display allocation. The
  user-scoped `.env` only defines `AGENT_BROWSER_REMOTE_VIEW_PROVIDER` and the
  shared `AGENT_BROWSER_REMOTE_VIEW_URL`. `~/.agent-browser/service/state.json`
  has zero persisted `remoteViewRoutes`, zero `routePool` entries, zero
  `displayAllocations`, and zero `viewerLeases` after cleanup. The running
  Guacamole database has one RDP connection:
  `Local XRDP (agent-browser host)` targeting `host.docker.internal:3389`.
- Added `pnpm test:rdp-guac-route-pool-readiness` as the preflight for the
  remaining provider blocker. It inspects the local Guacamole Compose
  containers and RDP connection metadata without printing passwords, probes
  the Guacamole web route, checks guacd-to-RDP TCP reachability for the
  selected route candidates, requires at least two distinct route candidates
  by default, and emits a copyable `AGENT_BROWSER_RDP_ROUTE_POOL_JSON` value
  with redacted target identity and readiness metadata when the provider is
  ready. The current run with `--report-only` reports Guacamole Postgres,
  Guacamole web ingress, guacd, and the existing `host.docker.internal:3389`
  RDP backend ready, but blocks on one RDP Guacamole connection and one
  distinct target identity.
- Added `pnpm setup:rdp-guac-route-pool` as the interactive provisioning
  command for the first static two-route provider shape. It creates or updates
  two local XRDP users, creates or updates two Guacamole RDP connections,
  grants Guacamole read permission, stores generated XRDP passwords in the
  user-scoped Guacamole secret file, restarts XRDP, and tells the operator to
  rerun the route-pool readiness smoke. The script passed `bash -n`; this
  session could not run it live because non-interactive `sudo` is unavailable.
- Added `pnpm sync:rdp-guac-existing-user-route-pool` as the no-sudo route
  record sync for workstations that already have the reusable
  `agent-browser-rdp` account. It reads the existing XRDP username and
  password from the user-scoped Guacamole secret file, creates or updates two
  Guacamole RDP connection records, and differentiates them with RDP color
  depths 24 and 32. This matches the current XRDP `Policy=Default`, which keys
  sessions by user and bit depth.
- Ran `pnpm sync:rdp-guac-existing-user-route-pool` successfully on
  2026-05-28. It created/updated Guacamole connections 2 and 3:
  `Agent Browser RDP Existing User Route A` with color depth 24 and
  `Agent Browser RDP Existing User Route B` with color depth 32.
- After selecting managed route-pool connections before legacy fallback
  connections, `pnpm test:rdp-guac-route-pool-readiness -- --report-only`
  reports `ready`, selecting connection ids 2 and 3, with Guacamole web,
  guacd, RDP TCP, and distinct selected target identities ready.
- Added `pnpm setup:rdp-guac-route-pool -- --dry-run` so the static host-XRDP
  bootstrap can be reviewed without changing host users, secrets, Guacamole
  records, or services.
- Added `pnpm inspect:rdp-route-displays` as a non-sudo post-bootstrap helper.
  After both RDP route sessions are open, it inspects active X server
  processes for the route users and prints
  `AGENT_BROWSER_RDP_ROUTE_A_DISPLAY_NAME` and
  `AGENT_BROWSER_RDP_ROUTE_B_DISPLAY_NAME` when they are distinct. This gives
  the live gate the route-target display binding required by the first
  host-XRDP topology. The display inspector and route-pool readiness smoke
  also accept `--shell` for copyable `export ...` lines, so the final live gate
  can be run without hand-editing JSON.
- A source and host topology check on 2026-05-28 confirmed that the current
  private-display browser allocator creates service-owned Xvfb display
  allocations, while host XRDP creates separate login Xorg sessions. Therefore
  a second host-XRDP Guacamole connection is necessary but not sufficient for
  P03 completion. The many-to-many live gate must still prove Browser A and
  Browser B are actually visible through the distinct routes at the same time.
  The first testable host-XRDP topology is route-targeted display launch:
  create two XRDP sessions, record their display names on the route entries,
  and let the live gate launch Browser A and Browser B onto those displays.

Remaining Slice H work:

- Add the unified doctor/discovery surface described above before any further
  route provisioning. It must record that this workstation already has
  `agent-browser-rdp`, the existing-user route-pool sync, managed Guacamole
  connections 2 and 3, XRDP `Policy=Default`, and the current absence of two
  active route displays.
- Teach the doctor to distinguish install drift, service/runtime health,
  network/ingress health, Guacamole route-pool readiness, host RDP user state,
  active display state, and final many-to-many gate readiness.
- Make doctor output the canonical setup status handoff for P03, replacing
  scattered inference from `getent`, `docker exec`, secret-file inspection,
  and ad hoc process lists.
- Keep the route-pool readiness smoke green after the second route is added,
  including Guacamole web ingress and guacd-to-RDP TCP probes for both
  selected routes.
- Open both route RDP sessions and run `pnpm inspect:rdp-route-displays`; export
  the displayed route-target `DISPLAY` values with the route pool before the
  many-to-many live gate.
- Add or run a target-binding validation that proves each route reaches the
  browser display it claims, not only a separate XRDP login desktop.
- Re-run `pnpm test:rdp-guac-many-to-many-live` and record the passing
  service-state, viewer-lease, tile screenshot, target-binding crop and OCR,
  route evidence, refresh, and close/release artifacts.

## Completion Plan From Current Blocker

This is the remaining path to finish P03 hardening without regressing the
validated shared-route fallback.

### Step 0: Land Unified Doctor And Setup Discovery

Goal: make the current install/setup state discoverable before changing more
host or provider state.

Implementation requirements:

- Add a doctor command or service-readable doctor endpoint that composes:
  `agent-browser install doctor`, runtime status, service status, RDP gateway
  readiness, route-pool readiness, route-display inspection, Guacamole DB
  record inventory, Docker container/network checks, and user-scoped secret
  key presence checks.
- Record setup status as data with stable fields, not just prose:
  `install.status`, `runtime.status`, `service.status`, `network.status`,
  `rdpHost.status`, `guacamole.status`, `routePool.status`,
  `routeDisplays.status`, `manyToMany.status`, and `recommendedAction`.
- Include a `stateSources` section that names the authority for each fact:
  repo docs, user config, service state, Docker runtime, Guacamole DB,
  host OS, or live browser.
- Include an `inventory` section with redacted records:
  active binary, browser executable, service state path, Guacamole compose
  directory, secret file key names, RDP users present, XRDP policy, Guacamole
  connection ids/names/target identity hashes, selected route-pool entries,
  active displays, and live viewer/browser sessions.
- Include a `drift` section for unmanaged or stale records:
  legacy shared Guacamole route, duplicate managed routes, missing permissions,
  route records whose target no longer matches the selected topology, stale
  service route-pool checkouts, and profile/browser runtime locks.
- Include a `nextAction` section with one primary command and a short reason.
  Mutating actions must be recommended only after the doctor states the
  existing reusable path is unavailable or insufficient.
- Support JSON output and concise human output.
- Document the doctor as the first command to run when P03 setup appears
  inconsistent.

Exit evidence:

- A no-launch doctor command succeeds on the current workstation.
- Doctor output reports `agent-browser-rdp` present and does not recommend
  creating `agent-browser-rdp-a` or `agent-browser-rdp-b` while the
  existing-user route path is viable.
- Doctor output reports managed Guacamole connections 2 and 3 as selected
  route-pool candidates and the legacy connection 1 as shared fallback.
- Doctor output reports route-pool readiness ready, route-display readiness
  blocked, and recommends opening/inspecting the two route sessions before
  the many-to-many gate.
- JSON output redacts secrets and contains no passwords, tokens, cookies,
  signed links, or raw private auth artifacts.
- Docs and skill instructions point operators to the doctor before setup or
  sync commands.

### Step 1: Provision Or Reuse Two Distinct RDP Targets

Goal: create two independently addressable remote desktops that Guacamole can
route to at the same time.

Preferred first implementation:

- Reuse the existing `agent-browser-rdp` account.
- Use managed Guacamole route-pool records with distinct color depths when
  XRDP `Policy=Default` supports separate sessions by user and bit depth.
- Treat route-specific Linux users as a fallback only after doctor evidence
  shows the existing-user path cannot produce distinct displays.
- Each target must have a stable host and port or an equivalent stable
  Guacamole target identity.
- Each target must be able to host a remote-headed browser without relying on
  focus changes on the shared host display.

Acceptable provider shapes:

- one existing XRDP user with two Guacamole records that differ in a session
  key recognized by current XRDP policy, such as bit depth under
  `Policy=Default`, if live display inspection proves two displays
- two containerized XRDP targets, one per browser workspace class
- two host XRDP users or sessions if they can be pinned reliably to distinct
  desktop sessions
- two generated Guacamole connections that target distinct backend sessions

Rejected shapes for P03 completion:

- two Guacamole URLs that point to the same connection id
- two route-pool entries that differ only by display label but land on the
  same shared desktop
- any production code path that reconstructs a workstation-specific Guacamole
  client hash

Exit evidence:

- Doctor output explains which provider shape is selected and why.
- If the existing-user path is selected, `pnpm sync:rdp-guac-existing-user-route-pool`
  runs or dry-runs without sudo and the doctor records the resulting managed
  route entries.
- If route-specific users are required, `pnpm setup:rdp-guac-route-pool
  --dry-run` shows the intended host, users, connection names, and secret file
  without changing state, and the doctor explains why sudo is required.
- Guacamole admin or database readback shows at least two RDP connections or
  route identities.
- The two entries have distinct connection ids or route ids.
- Direct route checks can distinguish target A from target B.
- `pnpm test:rdp-guac-route-pool-readiness` reports `ready` without
  `--report-only`.
- A target-binding check proves route A displays Browser A and route B
  displays Browser B, not only distinct XRDP login sessions.

### Step 2: Declare The Static Route Pool In User-Scoped Config

Goal: make the route pool explicit service input rather than dashboard or
production-code string synthesis.

The live gate already accepts either:

- `AGENT_BROWSER_RDP_ROUTE_POOL_JSON`
- paired `AGENT_BROWSER_RDP_ROUTE_A_*` and `AGENT_BROWSER_RDP_ROUTE_B_*`
  environment variables

Each entry must include:

- `routeId`
- `connectionId`
- `connectionName`
- `frameUrl`
- `externalUrl`
- `providerMode`
- target host, port, display, browser id, session id, or other target
  metadata sufficient for checkout matching
- readiness evidence for Guacamole web, guacd, RDP backend, auth, ingress, and
  iframe embedding when available

Configuration should live in user-scoped state such as `~/.agent-browser/.env`
or another ignored local provider config file. Tracked docs may list variable
names and shapes, but must not contain live passwords, Guacamole auth tokens,
or private signed links.

Exit evidence:

- `pnpm test:rdp-guac-many-to-many-live` no longer fails at route-pool input
  parsing.
- `pnpm test:rdp-guac-route-pool-readiness` emits a non-empty
  `AGENT_BROWSER_RDP_ROUTE_POOL_JSON` value for the first two distinct route
  candidates.
- Service status after harness seeding shows two available route-pool entries
  before checkout.

### Step 3: Add Provider Readiness Probes For The Pool

Goal: fail before dashboard viewing when a configured route cannot reach the
expected target.

Probe requirements:

- Guacamole web route returns reachable HTML client state.
- guacd is reachable from the Guacamole web container.
- RDP backend for each pool entry accepts a connection.
- The route identity maps to the expected target, not the shared fallback.
- Iframe embedding through the dashboard ingress is allowed.
- Provider auth failure, route unreachable, iframe blocked, and target
  mismatch are typed readiness reasons.

Exit evidence:

- Ready pool entries can be checked out for private displays.
- Failed or stale entries return `route_pool_not_ready` and are not assigned
  to live browsers.
- Route-pool exhaustion remains a typed service incident, not a blank iframe.

### Step 4: Run The Full Many-To-Many Live Gate

Goal: prove real simultaneous viewing.

Run:

```bash
pnpm test:rdp-guac-many-to-many-live
```

The gate must prove:

- Browser A and Browser B launch with different private display allocation ids.
- Browser A and Browser B check out different route ids or connection ids.
- Browser A marker text is visible inside route A's cropped remote-view iframe.
- Browser B marker text is visible inside route B's cropped remote-view iframe.
- Viewer 1 and Viewer 2 can both observe both workspaces.
- Viewer 1 controls Browser A while Viewer 2 observes it.
- Viewer 2 controls Browser B while Viewer 1 observes it.
- `view=workspace:tile` shows both browser workspaces at the same time.
- Refreshing Browser A's tile does not remount or blank Browser B.
- Closing Browser A releases only Browser A display and route state.
- Browser B remains visible and ready after Browser A is closed.

Exit evidence:

- passing artifact directory under
  `/tmp/agent-browser-rdp-guac-many-to-many-<timestamp>/`
- before and after service status
- display allocation records
- route allocation records
- route-pool records
- viewer lease records
- dashboard tile screenshots for both viewer clients
- cropped route iframe screenshots and OCR text proving target binding
- direct provider route evidence for both route ids

### Step 5: Close P03 Only After Shared And Private Paths Are Both Labeled

Goal: keep the hardened route behavior explicit for future operators.

P03 can close when:

- the shared Guacamole route remains usable as a labeled fallback
- private route-pool routes are preferred for simultaneous viewing
- route identity comes from service state and provider config, not URL parsing
- stale route repair and reconcile remain validated after the two-route smoke
- docs tell operators how to distinguish shared focus-switching from true
  many-to-many viewing

## Contract And Documentation Updates

When this lane touches user-facing behavior, update all required documentation
surfaces:

- `cli/src/output.rs`
- `README.md`
- `skills/agent-browser/SKILL.md`
- `docs/src/app/`
- inline doc comments in relevant source files
- generated client contracts and examples

Docs must describe:

- the doctor-first workflow for install/setup/network/health discovery
- where setup state is recorded and which source owns each fact
- private display as the preferred route
- shared display as an explicit fallback
- route pool configuration
- viewer and controller lease behavior
- dashboard many-workspace viewing
- provider limitations and readiness failures

## Rollout Strategy

1. Land contracts and no-launch state shape first.
2. Land private display allocation with no Guacamole route changes.
3. Land route pool checkout against static configured routes.
4. Land dashboard route selection and shared-route warnings.
5. Land viewer/controller leases.
6. Land reconcile and repair.
7. Add unified doctor/setup discovery before further provider mutation.
8. Add the live many-to-many gate.
9. Only then change default launcher behavior to prefer private route
   allocation when a route pool is ready.

Default behavior should remain conservative:

- if route pool is unavailable, keep the current shared RDP route explicit
- if private display is available but no route is available, expose service
  readiness rather than showing a blank iframe
- if a route is shared, label it shared and rely on `view_focus`
- if routes are distinct, prefer simultaneous viewing

## Acceptance Criteria

P03 is complete when all of the following are true:

- A unified doctor/setup discovery surface reports install, runtime, service,
  network, Guacamole, RDP user, route-pool, route-display, and many-to-many
  gate status from authoritative sources.
- The doctor records the existing `agent-browser-rdp` setup and recommends
  reuse before route-specific user creation.
- The doctor identifies managed route-pool entries, legacy shared fallback,
  stale/unmanaged records, and the next minimal state-changing action.
- Service state can list display allocations, route allocations, and viewer
  leases.
- Two remote-headed browsers can be launched with private display allocations.
- Two private browser displays can be exposed through two distinct Guacamole
  routes.
- Dashboard can show both browsers simultaneously.
- Two independent viewers can observe both browsers.
- Controller ownership is explicit and auditable.
- Shared-route fallback is preserved and labeled.
- Hardcoded workstation Guacamole hashes remain absent from production route
  synthesis.
- Reconcile and repair cover stale display, stale route, stale viewer, stale
  controller, browser crash, daemon restart, and provider restart.
- A live gate proves the full many-to-many matrix and records artifacts under
  `/tmp/agent-browser-rdp-guac-many-to-many-<timestamp>/`.

## Open Questions

- Should the first distinct-route implementation use a static Guacamole route
  pool or dynamically generated Guacamole connections?
- Can the current XRDP deployment expose multiple private displays directly,
  or does it need one target user/session/container per route?
- Should viewer leases use dashboard auth identities only, or allow external
  viewer identities from signed route links?
- Should controller leases expire automatically on missing heartbeat, tab
  close, route disconnect, or all three?
- What is the minimum provider-admin surface needed to validate route pool
  membership without storing provider secrets in the repo?
