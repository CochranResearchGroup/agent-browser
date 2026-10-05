# Remote Control Ready Command Plan

Date: 2026-06-20
State: CLOSED
Lane: P16
Depends On:
- `docs/dev/plans/0003-2026-05-28-guac-rdp-many-to-many-viewing-plan.md`
- `docs/dev/plans/0025-2026-06-01-remote-view-target-attribution-and-idle-display-plan.md`
- `docs/dev/plans/0036-2026-06-18-rdp-ready-to-go-plan.md`
- `docs/dev/plans/0037-2026-06-19-runtime-profile-sharing-plan.md`
- `docs/dev/plans/0038-2026-06-19-remote-headed-cutover-proof-plan.md`

## Purpose

Starting a controllable browser through Guacamole/RDP must be one command or
one API call. It must not require operators to repair route URLs, Guacamole
database state, route-display binding, display permissions, profile reuse, or
X server access after the request starts.

The live LinkedIn authentication attempt on 2026-06-20 exposed a recurring
class of false-ready states:

- The browser was healthy and loaded LinkedIn, but the dashboard initially
  showed a broken document because the hosted dashboard and Guacamole route
  metadata were not sufficient to guarantee a valid embed.
- Guacamole later connected successfully, but showed only an `xterm` because
  the browser was on private Xvfb display `:90` while the selected Guacamole
  route showed the XRDP display `:10`.
- The Guacamole route pool could report ready even when the backing
  PostgreSQL schema was absent.
- Display access existed as a privileged helper capability, but the actual
  route in use was the existing-user XRDP display and needed direct
  helper-mediated access before Chrome could be launched visibly.

This plan makes the combined operator-visible invariant authoritative:

```text
A remote-control browser is ready only when the selected browser window is
loaded, visible, and controllable through the selected external Guacamole/RDP
route.
```

## Goal

Provide a generic remote-control acquisition path that downstream services,
operators, and agents can call without knowing Guacamole route internals.

Target command:

```bash
agent-browser remote-view open \
  --runtime-profile stealthcdp-default \
  --browser-build stealthcdp_chromium \
  --provider rdp_gateway \
  --url https://www.linkedin.com/
```

Target API action:

```json
{
  "action": "remote_view_open",
  "serviceName": "ManualAuth",
  "agentName": "codex",
  "taskName": "manual-auth",
  "runtimeProfile": "stealthcdp-default",
  "browserBuild": "stealthcdp_chromium",
  "url": "https://www.linkedin.com/",
  "params": {
    "provider": "rdp_gateway"
  }
}
```

Done means:

- one command/API call selects or provisions a valid Guacamole/RDP route;
- Guacamole schema, route records, connection permissions, local embed URL,
  public operator URL, and dashboard embed URL are verified before launch;
- the browser is launched or reused on the same X display served by the
  selected Guacamole route;
- the privileged helper grants display access when needed, without repeated
  interactive setup;
- retained browser, tab, view-stream, display-allocation, route, and lease
  records all describe the same route/display/browser target;
- the command returns a dashboard URL and direct external route URL only after
  live verification proves the browser window is visible on the route display;
- doctor and many-to-many gates fail closed when they could still produce an
  unhappy document, a Guacamole internal error page, or a terminal-only
  desktop.

## Current State

This plan is closed. The bounded centralization slice that started on
2026-06-20 now has route-specific live proof, docs, skill guidance, and
downstream handoff.

Closeout facts:

- The initial live failure showed that manual repair could make the external
  Guacamole route render LinkedIn, but that manual path required schema
  initialization, route sync, connection permission repair, display access
  grant, private browser shutdown, and relaunch on the active XRDP display.
- The closed implementation prevents that class from being reported ready by
  binding `remote_view_open`, doctor readiness, route-pool state, retained view
  streams, and dashboard readiness to the same route/display/browser evidence.
- `cli/src/native/remote_view.rs` now owns the shared route-pool display
  matching, route-pool checkout selection, and route readiness-state parsing
  used by service route checkout and remote-view route-pool exhaustion
  incidents.
- The shared readiness parser now fails closed when a top-level `ready`
  object contains a nested non-ready component such as `terminal_only_route`.
- `RemoteViewRoute` and `ViewStream` now preserve `routeDescriptor` so
  local embed, hosted dashboard embed, public operator, health, and legacy
  external URL roles survive checkout instead of being flattened to bare
  `frameUrl` and `externalUrl`.
- Service route checkout now returns a `routeBinding` object that includes the
  selected route id, route-pool entry id, display allocation id, display name,
  launch display name, display isolation, route user, display-access evidence,
  provider, provider mode, connection metadata, URL roles, and readiness.
- RDP route-pool checkout now fails before mutation when the selected entry has
  no concrete `target.displayName` or no concrete Guacamole `#/client/` URL.
- `service_remote_view_route_preflight` is now a no-launch service request
  action accepted by HTTP, MCP, and generated clients. It returns the same
  route binding used by checkout and leaves retained route-pool state
  unchanged.
- The route-specific Guacamole/RDP path is now live-proven on the current
  route pool. `remote-view open` dry-run resolves `guacamole-rdp-a` to
  `guacamole:3`, display `:11`, and display allocation
  `remote-view-display:11` instead of stale retained `guacamole:1` /
  display `:10` state.
- `pnpm test:remote-view-open-fixture-live` passed on the route-specific lane
  with artifact directory
  `/tmp/agent-browser-remote-view-open-live-2026-06-21T01-05-37-262Z`,
  route `guacamole:3`, display `:11`, display allocation
  `remote-view-display:11`, external URL
  `https://agent-browser.ecochran.dyndns.org/guacamole/#/client/MwBjAHBvc3RncmVzcWw=`,
  fixture URL `http://127.0.0.1:36521/`, title
  `REMOTE VIEW OPEN FIXTURE 57134`, and X11 window `0x800003` matching browser
  PID `57825`.
- `pnpm test:rdp-guac-many-to-many-live` passed with artifact directory
  `/tmp/agent-browser-rdp-guac-many-to-many-2026-06-21T01-05-55-809Z`.
- `agent-browser doctor remote-view --json` reports `status=ready`,
  `remoteControl.status=ready`, `manyToMany.status=ready`, route
  `guacamole:3`, display `:11`, route-display access ready, route displays
  ready, route pool ready, and viewer prerequisites ready.
- Audit follow-up on 2026-06-21 fixed two closeout mismatches: the documented
  `remote-view open` target command now accepts `--browser-build
  stealthcdp_chromium` and `--provider rdp_gateway`, and post-launch route
  verification failures now run cleanup before returning the typed error. New
  browser launches close the browser on failure; reused retained browsers keep
  the browser process and close only the opened tab when possible.

Closeout: `remote_view_open` is the documented one-command and one-API-call
path for route-specific Guacamole/RDP browser acquisition. Downstream clients
should consume the handoff in
`docs/dev/notes/2026-06-21-remote-view-open-route-specific-handoff.md`.

## Non-Goals

- Do not hardcode the current external dyndns.org route. Public operator URLs
  must come from route descriptors, config, or environment.
- Do not make this LinkedIn-specific, AuraCall-specific, or account-specific.
- Do not weaken profile safety by allowing independent Chrome processes to
  share one authenticated profile directory.
- Do not require operators to keep route clients open manually before ordinary
  use.
- Do not require downstream services to parse Guacamole hashes, XRDP displays,
  or Xauthority details.

## Operating Invariants

```text
Readiness is a composed property. Browser health, Guacamole route health,
display health, and dashboard embed health are not independently sufficient.
```

```text
The selected route display and browser display must match. If Guacamole shows
display :10, a browser on :90 is not a ready remote-control browser.
```

```text
Doctor and many-to-many validation must be pessimistic. If a normal operator
request can still show an error document or terminal-only desktop, the relevant
gate is not green.
```

```text
Route descriptors must preserve URL audience roles: local embedding, hosted
dashboard embedding, public operator ingress, health checks, and legacy
external URL.
```

## Subagent Work Allocation

Use subagents by slice. Each subagent should return:

```text
Slice:
Goal:
Files changed:
Contract delta:
No-launch validation:
Live validation:
Doctor impact:
Many-to-many impact:
Residual risks:
Next slice readiness:
```

Recommended subagents:

1. Doctor Agent: false-ready audit, Guacamole schema checks, route-display
   checks, display-access checks, and install/runtime drift classification.
2. Route Agent: route selection, existing-user and route-specific display
   discovery, route descriptor URL roles, and connection permission repair.
3. Launch Agent: `remote_view_open` orchestration, profile reuse, browser
   display binding, helper-mediated X access, retained state consistency, and
   fail-closed error taxonomy.
4. Dashboard Agent: hosted/local frame URL selection, unhappy-document
   detection, terminal-only route evidence, and dashboard action copy.
5. Live Gate Agent: one-command live proof, negative regression fixtures, and
   many-to-many gate hardening.
6. Docs Agent: CLI help, README, docs site, skill guidance, runbook, and
   downstream handoff.

Slices A and B can run in parallel. Slice C depends on their shared route and
error taxonomy. Slice D can start after C exposes read models. Slice E depends
on A through D. Slice F closes the plan after validation.

## Error Taxonomy

The new path should return explicit errors instead of generic failure text:

- `guacamole_schema_missing`: required Guacamole PostgreSQL tables are absent.
- `guacamole_connection_missing`: selected connection id is not visible.
- `guacamole_connection_permission_missing`: route user lacks READ permission.
- `route_display_missing`: selected route has no active XRDP display.
- `route_display_collapsed`: multiple routes collapse to one display when
  distinct displays are required.
- `x11_auth_denied`: browser-launching user cannot open the route display.
- `display_access_grant_failed`: privileged helper could not grant access.
- `browser_display_mismatch`: browser launched on a display other than the
  selected route display.
- `browser_window_not_visible`: CDP target exists but no browser window is
  present on the route display.
- `dashboard_embed_not_routable`: dashboard-selected frame URL cannot embed.
- `terminal_only_route`: route display is reachable but no selected browser
  window is visible.

## Slice A: Doctor Truth Gates

State: DONE

Goal: make `agent-browser doctor remote-view` unable to report green when a
normal remote-control browser request can still produce the known failure
classes.

Deliverables:

- Add Guacamole schema presence checks for required tables such as
  `guacamole_user`, `guacamole_connection`, `guacamole_connection_parameter`,
  and `guacamole_connection_permission`.
  - Done: `scripts/smoke-rdp-guac-route-pool-readiness.js` now emits an
    explicit `guacamole_schema` readiness component and reports required,
    present, and missing table names under `guacamole.schema`.
- Check connection visibility and READ permission for the configured operator
  header-auth user and any configured route users.
  - Done: route-pool readiness now emits an explicit
    `guacamole_connection_permissions` component and reports READ grant counts
    for selected Guacamole connection ids under `guacamole.permissions`.
    The live route pool currently reports selected connection READ grants, and
    route-client authentication gates copied route-pool entries before checkout.
- Check Guacamole route-client authentication before treating the route pool
  as ready.
  - Done: `scripts/smoke-rdp-guac-route-pool-readiness.js` now loads the
    Guacamole secret file, posts to the token endpoint, emits a
    `guacamole_login` readiness component, and refuses to export
    `AGENT_BROWSER_RDP_ROUTE_POOL_JSON` when the token endpoint rejects the
    configured credentials.
  - Done: copied route-pool entries now carry `readiness.state=failed` when
    `guacamole_login` fails, so downstream clients cannot accidentally check
    out a route whose operator client cannot be acquired.
- Report existing-user route display state separately from route-specific
  display state.
- Report whether the privileged helper can grant display access for the
  actual selected route display, not only route-specific users.
- Add a composed `remoteControlReady` result that is false unless route,
  display, permissions, URL roles, and display access are all ready.
- Make `manyToMany.status` and doctor top-level `status` fail or warn when
  the current topology can only show a terminal unless the test explicitly
  asks for terminal-only diagnostics.
  - Current: top-level doctor and `manyToMany.status` remain
    `needs_route_displays` while only one distinct route display is visible.
    The separate `remoteControl.status=ready` covers the single-route
    one-command operator path.

Acceptance:

- An empty Guacamole database yields `guacamole_schema_missing` and doctor is
  not green.
- A route with no browser-visible display yields `terminal_only_route` or
  `route_display_missing` and doctor is not green.
- Existing-user display access gaps are reported with an actionable helper
  command.

Validation:

```bash
cargo test --manifest-path cli/Cargo.toml remote_view_doctor -- --test-threads=1
pnpm test:rdp-guac-route-pool-readiness
agent-browser doctor remote-view --json
git diff --check
```

Progress evidence after Guacamole schema and permission truth gates:

```text
node --check scripts/smoke-rdp-guac-route-pool-readiness.js
node scripts/smoke-rdp-guac-route-pool-readiness.js --report-only
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo test --manifest-path cli/Cargo.toml remote_view_doctor -- --test-threads=1
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
agent-browser install doctor --json
agent-browser doctor remote-view --json
pnpm test:remote-view-open-live
git diff --check
```

Live doctor evidence on 2026-06-20:

- Installed binary SHA
  `deb434b793c9ac57a56cca2dd80930c178a9de3f3a29bed294b16ecccef2939f`
  is aligned across PATH, workspace, and pnpm package binaries.
- `agent-browser doctor remote-view --json` reports
  `guacamole_schema=ready` with required tables
  `guacamole_user`, `guacamole_entity`, `guacamole_connection`,
  `guacamole_connection_parameter`, and
  `guacamole_connection_permission` present.
- `agent-browser doctor remote-view --json` reports
  `guacamole_connection_permissions=ready` with selected connection READ
  grants `1:2` and `2:2`.
- `remoteControl.status=ready` and `remoteControl.ready=true` for
  `guacamole:1` on display `:10`; `manyToMany.status` remains
  `needs_route_displays` with issue code `route_displays_missing_or_collapsed`.

Progress evidence after Guacamole login and route-client acquisition gates:

```text
node --check scripts/open-rdp-guac-route-displays.js
pnpm open:rdp-route-displays -- --dry-run
pnpm open:rdp-route-displays -- --report-only
node --check scripts/smoke-rdp-guac-route-pool-readiness.js
node scripts/smoke-rdp-guac-route-pool-readiness.js --report-only
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo test --manifest-path cli/Cargo.toml remote_view_doctor -- --test-threads=1
```

Live gate evidence on 2026-06-20:

- `pnpm open:rdp-route-displays -- --dry-run` discovered existing-user
  Guacamole routes `guacamole:1` and `guacamole:2`.
- `pnpm open:rdp-route-displays -- --report-only` now fails on
  `guacamole_route_a_login_failed` when the configured Guacamole credentials
  return HTTP 403, instead of continuing to stale display inspection.
- `node scripts/smoke-rdp-guac-route-pool-readiness.js --report-only` now
  reports `guacamole_login=failed`,
  `nextAction=repair_guacamole_admin_credentials`, and copied route-pool
  entries `guacamole-rdp-a` and `guacamole-rdp-b` with
  `readiness.state=failed`.
- Installed binary SHA
  `ace67009edf0a5c5f959343808cae7be674a180764242a90067b075ba4bfe047`
  is aligned across PATH, workspace, and pnpm package binaries.
- `agent-browser install doctor --json` reports `success=true` and no issues.
- `agent-browser doctor remote-view --json` now reports `status=blocked`,
  `nextAction=repair_guacamole_admin_credentials`, issue code
  `guacamole_login_failed`, `remoteControl.ready=false`, and
  `remoteControl.nextAction=repair_guacamole_admin_credentials`.

## Slice B: Route And Display Binding Contract

State: DONE

Goal: make route selection produce a concrete display binding that can be used
directly by launch orchestration.

Deliverables:

- Extend route-pool entries and remote-view route records with a normalized
  `target.displayName`, route user, connection id, route descriptor, and
  display-access state.
  - Done: route-pool entries, remote-view routes, and retained view streams
    now preserve `routeDescriptor`; display matching and checkout selection
    share `cli/src/native/remote_view.rs`.
  - Done: service route checkout now exposes `routeBinding` and writes the
    binding display name/isolation into the display allocation, so launch
    orchestration can use the same selected display rather than deriving a
    private display independently.
  - Done: `routeBinding` includes route user and display-access evidence, and
    `remote_view_open` uses the reported route user for helper-mediated display
    access before launch when the agent user cannot open the selected display.
- Add a no-launch route checkout preflight that selects one concrete route and
  proves the browser launch display before any browser process starts.
  - Done: `build_route_binding` performs the no-launch route/display/URL
    binding check used by checkout.
  - Done: `service_remote_view_route_preflight` exposes the binding through
    the queued service-request surface without mutating retained state.
  - Done: `remote_view_open` now consumes the same route binding, grants
    display access through the installed privileged helper when needed,
    launches or reuses the browser on `routeBinding.launchDisplayName`, and
    proves the visible browser window before checkout.
- Support both existing-user and route-specific XRDP topologies.
- Make route descriptor URL roles authoritative:
  `localEmbedUrl`, `dashboardEmbedUrl`, `publicOperatorUrl`, `healthUrl`, and
  backward-compatible `externalUrl`.
- Ensure public operator URL comes from config or environment, not code.

Acceptance:

- A selected route returns one display name and one Guacamole client route.
- A selected route without display access returns an explicit grant action.
- Route records do not use bare `/guacamole/` as a ready browser stream when
  a concrete connection URL is required.

Progress evidence:

```text
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo test --manifest-path cli/Cargo.toml remote_view -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml service_collection_record_contracts_match_wire_shape -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml service_state_round_trips_nested_entities -- --test-threads=1
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
git diff --check
```

Additional progress evidence after adding route binding:

```text
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo test --manifest-path cli/Cargo.toml remote_view -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml service_model -- --test-threads=1
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
git diff --check
```

Additional progress evidence after adding service preflight:

```text
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo test --manifest-path cli/Cargo.toml remote_view -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml service_request_command -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml service_request_contract_metadata -- --test-threads=1
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
pnpm test:service-client
pnpm test:service-api-mcp-parity
git diff --check
```

Plan audit note: `pnpm run plans:audit -- --keep 39` was attempted from this
checkout, but `package.json` does not define a `plans:audit` script.

Validation:

```bash
cargo test --manifest-path cli/Cargo.toml service_model -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml service_remote_view -- --test-threads=1
pnpm test:dashboard-view-streams
git diff --check
```

## Slice C: One-Command Remote View Open

State: DONE

Goal: add the operator-facing command and service API action that own the
entire route, display, browser, and retained-state lifecycle.

Progress:

- Added service request action `remote_view_open`.
- Added CLI parser support for `agent-browser remote-view open`.
- Made no-launch route preflight able to bind an available route-pool entry
  before any browser or display allocation exists.
- The open action now computes a central `routeBinding` first, builds the
  remote-headed launch command from that binding, opens the requested tab, and
  checks out the route with matching display and URL metadata.
- Added dry-run planning so the selected launch, tab, and checkout commands are
  live-testable without spawning Chrome.
- Added `@agent-browser/client` helpers:
  `createServiceRemoteViewOpenRequest()` and
  `requestServiceRemoteViewOpen()`.
- Updated CLI help, README, docs site, and agent skill guidance to prefer the
  high-level route-bound open routine for one-line remote-view browser opens.
- Moved route-display content inspection and terminal-only/browser-window
  classification into `cli/src/native/remote_view.rs`, so dashboard projection
  and service actions share one parser/probe instead of maintaining separate
  route-display heuristics.
- The live `remote_view_open` path now launches, opens the requested tab,
  focuses/maximizes the selected target, inspects the selected route display,
  and refuses to check out the route as ready unless the central proof reports
  `browser_window_visible`.
- Route checkout now carries the post-launch display-content proof into
  retained stream `remoteReadiness`, so dashboard rows consume the same
  terminal-only or browser-window evidence used by the service action.
- Inline route-pool entries copied from the Guacamole readiness script now
  normalize `readiness.state=ready` to an available checkout candidate when
  they omit an explicit `state` field.
- Repeated `remote_view_open` calls can reuse a route-pool entry already
  checked out to the same route allocation, preserving the retained
  `displayAllocationId` instead of deriving a fresh allocation from the display
  name.

Deliverables:

- Add CLI command `remote-view open`.
- Add service request action `remote_view_open`.
- Reuse access-plan profile selection and runtime-profile sharing policy.
- Select or reuse a route, grant display access, launch or reuse a browser on
  the route display, open the requested URL, focus the selected tab, and return
  dashboard and external URLs.
- Verify after launch that:
  - CDP URL/title match the requested page or final navigation;
  - browser process display equals route display;
  - X window tree on the route display contains the selected browser window;
  - retained service state has matching browser, tab, display, route, and
    stream metadata.
- Reject duplicate profile process launches unless access-plan explicitly
  allows reviewed duplicate process behavior.

Acceptance:

- `agent-browser remote-view open --runtime-profile stealthcdp-default --url
  https://www.linkedin.com/` returns a working dashboard URL without manual
  route, desktop, or X server repair.
- If any precondition fails, the command exits before launch or cleans up the
  partial browser and returns a typed error.
- Repeated calls reuse the retained browser where policy says reuse is safe.

Validation:

```bash
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
cargo test --manifest-path cli/Cargo.toml service_request_command -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml service_access_plan -- --test-threads=1
pnpm test:service-api-mcp-parity
pnpm test:service-client
git diff --check
```

Progress evidence after route-bound open dry-run:

```text
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo test --manifest-path cli/Cargo.toml remote_view -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml remote_view_open -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml service_request_command -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml service_request_contract_metadata -- --test-threads=1
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
pnpm test:service-client
pnpm test:service-api-mcp-parity
git diff --check
```

Additional progress evidence after centralizing visible-window proof:

```text
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo test --manifest-path cli/Cargo.toml remote_view -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml remote_view_open -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml dashboard_display_content -- --test-threads=1
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
pnpm test:dashboard-view-streams
```

Additional progress evidence after checked-out route reuse and live proof:

```text
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
cargo test --manifest-path cli/Cargo.toml remote_view -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml remote_view_open -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml service_request_command -- --test-threads=1
pnpm test:dashboard-view-streams
git diff --check
cargo build --manifest-path cli/Cargo.toml
agent-browser install doctor --json
```

Additional progress evidence after first-class live regression script:

```text
pnpm test:service-request-client
pnpm test:service-client-types
pnpm test:service-client-contract
pnpm test:remote-view-open-live
git diff --check
```

Additional progress evidence after doctor single-route readiness split:

```text
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo test --manifest-path cli/Cargo.toml remote_view_doctor -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml remote_view -- --test-threads=1
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
pnpm test:service-request-client
pnpm test:service-client-types
pnpm test:remote-view-open-live
agent-browser install doctor --json
agent-browser doctor remote-view --json
git diff --check
```

Live proof on 2026-06-20:

- Rebuilt `cli/target/debug/agent-browser`, restarted the default daemon, and
  replaced the user-scoped PATH, workspace, and pnpm package binaries with the
  rebuilt binary. `agent-browser install doctor --json` returned
  `success=true` with no issues, matching SHA values for all three binaries,
  remote-view privileges ready, and no-launch service readiness true.
- First live call:
  `agent-browser remote-view open https://www.linkedin.com/
  --runtime-profile stealthcdp-default --display :10
  --display-isolation shared_display --route-pool-entry-json <guacamole-rdp-a>`
  returned `status=opened`, `routeId=guacamole:1`,
  `displayAllocationId=remote-view-display:guacamole-1`,
  `routePoolEntry.state=checked_out`, and
  `visibleWindowProof.displayContent.state=browser_window_visible`.
- Repeated live call with the same route returned `status=opened`,
  `launch.reused=true`, the same route id and display allocation id, a new tab
  for `https://www.linkedin.com/`, and
  `visibleWindowProof.displayContent.state=browser_window_visible`.
- CDP readback through `agent-browser get url` and `agent-browser get title`
  returned `https://www.linkedin.com/` and `LinkedIn: Log In or Sign Up`.
- Retained service state agrees across browser, display allocation, route, and
  stream: browser `session:default` is ready on display `:10`; display
  allocation `remote-view-display:guacamole-1` is ready, shared, owned by
  `session:default`, and references `guacamole:1`; remote-view route
  `guacamole:1` is ready and has route/display/browser metadata for the same
  target; stream `remote-headed-view` has provider `rdp_gateway`, route
  `guacamole:1`, display allocation `remote-view-display:guacamole-1`, and
  remote readiness display content `browser_window_visible`.
- `xwininfo -display :10 -root -tree` showed Chromium windows and the service
  proof classified the display as `browser_window_visible` despite the route
  also containing an xterm.
- Added `pnpm test:remote-view-open-live`, backed by
  `scripts/smoke-remote-view-open-live.js`, as the repeatable proof for the
  successful route-bound open path. The gate uses the installed binary and
  real operator runtime, discovers the Guacamole/RDP route-pool entry, performs
  first and repeated CLI `remote-view open` calls, performs the same open
  through `requestServiceRemoteViewOpen()`, asserts stable route and display
  allocation reuse, checks CDP URL/title readback, verifies retained browser,
  route, display allocation, and stream agreement, fails if `xwininfo` does
  not show Chrome or Chromium on the selected route display, and uses `xprop`
  to prove a Chrome or Chromium X11 window on that display has `_NET_WM_PID`
  matching the retained browser PID.
- The live gate passed with artifact directory
  `/tmp/agent-browser-remote-view-open-live-2026-06-20T23-51-40-261Z`,
  command `/home/ecochran76/.local/bin/agent-browser`, route
  `guacamole:1`, display `:10`, display allocation
  `remote-view-display:guacamole-1`, external URL
  `https://agent-browser.ecochran.dyndns.org/guacamole/#/client/MQBjAHBvc3RncmVzcWw=`,
  URL `https://www.linkedin.com/`, and title
  `LinkedIn: Log In or Sign Up`. The X11 window proof matched Chromium window
  `0x800003` on display `:10` to retained browser PID `57263`.
- `createServiceRemoteViewOpenRequest()` and
  `requestServiceRemoteViewOpen()` now preserve `routePoolEntry` and
  `routePool` in request params, so HTTP callers can pass the same selected
  route material as the CLI path instead of losing the route descriptor before
  execution.
- `agent-browser doctor remote-view --json` now reports a separate
  `remoteControl` section for the single-route operator path. On the installed
  binary with SHA
  `3ebc4b6b2d35b4b7a6ed5c330c43db069c94b801358732791cb9c94d9ef48944`,
  `remoteControl.status=ready`, `remoteControl.ready=true`,
  `routeId=guacamole:1`, `displayName=:10`,
  `routeDisplayAccessReady=true`, and the display access probe returned
  `name of display:    :10`. The top-level doctor status and
  `manyToMany.status` intentionally remain `needs_route_displays` because two
  distinct route displays are not currently visible.

Audit follow-up on 2026-06-21:

- `agent-browser remote-view open --runtime-profile stealthcdp-default
  --browser-build stealthcdp_chromium --provider rdp_gateway --url
  https://www.linkedin.com/` is now accepted by the CLI parser instead of
  failing on undocumented flags.
- If tab open, focus, visible-window proof, or route checkout fails after launch, the command
  runs cleanup before returning the typed error. New browser launches close the
  browser; reused retained browsers keep the shared browser process and close
  only the opened tab when possible.

Closure proof:

- The route-specific one-command path is live-gated, doctor-visible, and
  documented. Slice A and Slice E have current positive evidence for route
  pool, route display access, fixture open, and many-to-many live operation.
  Slice F added the final docs and downstream handoff.

## Slice D: Dashboard Fail-Closed UX

State: DONE

Goal: make the dashboard distinguish broken Guacamole, terminal-only route,
viewer takeover, and browser-display mismatch without hiding them behind a
blank frame or generic error icon.

Deliverables:

- Show route/display/browser mismatch evidence in the workspace viewport.
  - Done: the dashboard workspace viewport now prioritizes
    `stream.remoteReadiness` over generic `stream.readiness`, so the
    route-display proof produced by `remote_view_open` cannot be masked by an
    older stream-ready record.
- Add detection for terminal-only route when the service has no matching
  browser window evidence on the route display.
  - Done: terminal-only and browser-window classification now comes from
    the shared native remote-view module, and `remote_view_open` stores that
    proof under retained stream `remoteReadiness` after launch.
  - Done: dashboard readiness treats `terminal_only_route`,
    `browser_window_not_visible`, `browser_display_mismatch`,
    `route_display_missing`, and `dashboard_embed_not_routable` as blocking
    remote-view states. The view-stream contract smoke asserts
    `terminal_only_route` renders `status=blocked` with operator recovery copy
    instead of embedding the Guacamole iframe as ready.
- Keep hosted dashboard iframe selection origin-aware and route-descriptor
  based.
- Add visible recovery actions that call `remote_view_open` or route repair
  rather than asking operators to manually patch state.

Acceptance:

- A terminal-only route cannot be presented as a healthy browser workspace.
- A Guacamole schema or permission failure appears as a service readiness
  issue with the specific error code.
- Recovery buttons are disabled unless the service contract advertises the
  supporting action.

Validation:

```bash
pnpm test:dashboard-view-streams
pnpm test:dashboard-inspector-actions
pnpm build:dashboard
git diff --check
```

Progress evidence after remote-readiness fail-closed priority:

```text
pnpm test:dashboard-view-streams
pnpm test:dashboard-inspector-actions
pnpm build:dashboard
git diff --check
```

## Slice E: Live Regression Gates

State: DONE

Goal: prove the one-command path and make the known regressions impossible to
call green.

Deliverables:

- Added `pnpm test:remote-view-open-live` for a manual-auth style public site
  using the existing `stealthcdp-default` managed profile, with no private auth
  data captured.
- Added `pnpm test:remote-view-open-fixture-live`, which runs the same
  route/display/browser/X11 proof against an isolated local HTTP fixture page
  and asserts exact fixture URL/title readback.
- Added `pnpm open:rdp-route-displays`, a smaller live helper that reads the
  doctor route pool, opens both Guacamole route clients in durable viewer
  sessions, authenticates them, and then runs the route-display inspector.
  This turns "open both Guacamole routes" from a manual instruction into one
  JSON-producing command.
- Extend many-to-many live gate so each route must prove:
  - Guacamole client route connects;
  - selected browser display equals route display;
  - visible browser window exists on that display;
  - OCR or DOM evidence distinguishes browser content from terminal content;
  - hosted dashboard URL selects a non-loopback, concrete route URL;
  - local harness selects local embed URL.
- Add negative fixtures for:
  - missing Guacamole schema;
  - missing connection permission;
  - route display without browser window;
  - browser on hidden private display while route shows XRDP display.

Acceptance:

- Many-to-many and doctor are not green under the known unhappy-document or
  terminal-only conditions.
- The one-command live smoke returns a dashboard URL that renders browser
  content in at least one external operator client.
- A second run does not require manual setup or state correction.

Validation:

```bash
pnpm test:remote-view-open-live
pnpm test:rdp-guac-route-pool-readiness
pnpm test:rdp-guac-many-to-many-live
agent-browser doctor remote-view --json
git diff --check
```

Progress evidence after isolated fixture mode:

```text
node --check scripts/smoke-remote-view-open-live.js
pnpm test:remote-view-open-fixture-live
pnpm test:remote-view-open-live
git diff --check
```

Live fixture evidence on 2026-06-20:

- `pnpm test:remote-view-open-fixture-live` passed with artifact directory
  `/tmp/agent-browser-remote-view-open-live-2026-06-21T00-05-48-562Z`,
  fixture marker `REMOTE VIEW OPEN FIXTURE 64908`, route `guacamole:1`,
  display `:10`, display allocation `remote-view-display:guacamole-1`, and
  X11 window `0x800003` matching browser PID `57263`.
- `pnpm test:remote-view-open-live` passed afterward with artifact directory
  `/tmp/agent-browser-remote-view-open-live-2026-06-21T00-05-58-559Z`,
  URL `https://www.linkedin.com/`, title `LinkedIn: Log In or Sign Up`, route
  `guacamole:1`, display `:10`, and the same X11 browser-window PID proof.

Route-specific fixture and many-to-many evidence on 2026-06-20:

- Repaired retained route-pool state from the current readiness report after
  backing up
  `~/.agent-browser/service/state.json.pre-route-pool-refresh-2026-06-21T00-56-42-211Z`.
  The retained `guacamole-rdp-a` and `guacamole-rdp-b` entries now match
  current Guacamole connections `3` and `4`.
- `remote_view_open` route binding now prefers supplied/current route-pool
  identity over stale retained route id and display allocation state, treats a
  requested route-pool entry id as authoritative for allocation lookup, and
  accepts top-level route-pool `readiness.state=ready` even when informational
  nested components are not ready.
- `scripts/smoke-remote-view-open-live.js` now derives display name and
  display isolation from the selected route-pool entry for CLI, HTTP, retained
  state, and X11 proof checks.
- `pnpm test:remote-view-open-fixture-live` passed with artifact directory
  `/tmp/agent-browser-remote-view-open-live-2026-06-21T01-05-37-262Z`, route
  `guacamole:3`, display `:11`, and display allocation
  `remote-view-display:11`.
- `pnpm test:rdp-guac-many-to-many-live` passed with artifact directory
  `/tmp/agent-browser-rdp-guac-many-to-many-2026-06-21T01-05-55-809Z`.
- Validation passed:
  `cargo fmt --manifest-path cli/Cargo.toml -- --check`;
  `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`;
  `cargo test --manifest-path cli/Cargo.toml remote_view_open_dry_run_prefers_inline_route_pool_identity_over_stale_state -- --test-threads=1`;
  `cargo test --manifest-path cli/Cargo.toml remote_view_doctor -- --test-threads=1`;
  `node --check scripts/smoke-rdp-guac-route-pool-readiness.js`;
  `node --check scripts/open-rdp-guac-route-displays.js`;
  `node --check scripts/test-rdp-guac-many-to-many-live.js`;
  `node --check scripts/smoke-remote-view-open-live.js`;
  `agent-browser install doctor --json`;
  `agent-browser doctor remote-view --json`;
  `git diff --check`.

## Slice F: Documentation And Handoff

State: DONE

Goal: make the new behavior the documented default for agents and downstream
clients.

Deliverables:

- Update `README.md`, CLI help, docs site, and the installed
  `agent-browser` skill.
- Document the one-command path and service API action.
- Document the fail-closed error taxonomy and recovery commands.
- Update `RUNBOOK.md`, `ROADMAP.md`, and close this plan only after live gates
  pass.
- Write a downstream handoff note for AuraCall and other clients describing
  `remote_view_open`, profile sharing, route descriptors, and required gates.

Acceptance:

- Operators no longer need a multi-step runbook to get a controllable browser.
- Downstream clients have one generic API call and do not need Guacamole,
  XRDP, Xauthority, or display-selection knowledge.
- The plan is closed only after source, contract, dashboard, and live gates
  prove the invariant.

Validation:

```bash
pnpm --dir docs build
diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md
pnpm validation:select -- --base HEAD
git diff --check
```

Closure evidence:

```text
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo test --manifest-path cli/Cargo.toml remote_view_open -- --test-threads=1
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
pnpm --dir docs build
pnpm test:service-client
cargo test --manifest-path cli/Cargo.toml remote_view_doctor -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml service_contracts -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml service_model -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml service_access_plan -- --test-threads=1
pnpm test:service-api-mcp-parity
pnpm test:dashboard-view-streams
pnpm test:dashboard-inspector-actions
pnpm build:dashboard
agent-browser install doctor --json
agent-browser doctor remote-view --json
node --check scripts/smoke-remote-view-open-live.js
node --check scripts/test-rdp-guac-many-to-many-live.js
pnpm test:remote-view-open-fixture-live
pnpm test:rdp-guac-many-to-many-live
diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md
```

Final validation is recorded in `RUNBOOK.md`. The installed binary SHA was
`54248451b6bea3ced7acb6df8dd3e0f7514c866e08584bb025569a2ec6ad28ad`.
`pnpm test:remote-view-open-fixture-live` passed with artifact directory
`/tmp/agent-browser-remote-view-open-live-2026-06-21T01-24-32-095Z`, and
`pnpm test:rdp-guac-many-to-many-live` passed with artifact directory
`/tmp/agent-browser-rdp-guac-many-to-many-2026-06-21T01-24-32-207Z`.
