# Plan 0096: Durable Remote-View Handoff

Date: 2026-08-07
Completed: 2026-08-08
State: CLOSED
Lane: P96
Depends On:
- `docs/dev/plans/0066-2026-06-28-rdp-browser-reattachment-plan.md`
- `docs/dev/plans/0069-2026-07-06-shared-profile-routing-and-handoff-deepening-plan.md`
- `docs/dev/plans/0095-2026-08-07-remote-control-duplicate-pressure-readiness-repair-plan.md`

## Goal

Return an authenticated remote-view handoff URL whose identity outlives the
Guacamole connection, route checkout, viewer lease, and CDP target selected by
the original open. Opening that URL must make a best effort to recover the
same logical browser tab through the originally requested viewing and control
posture.

## Public Contract

- A successful `remote_view_open` returns a durable HTTPS `externalUrl` under
  `/remote-view/<handoff-id>`.
- The raw provider route remains available separately as `frameUrl`,
  `routeBinding.externalUrl`, and the route descriptor public-operator URL.
- The durable URL carries only an opaque handoff id. Browser target URLs,
  profile paths, target ids, Guacamole connection ids, and route ids do not
  appear in it.
- Dashboard forward auth preserves the durable path. After login, the
  dashboard resolves the handoff before opening the workspace viewport.
- Resolution prefers the exact retained browser and tab. When those runtime
  ids are stale after unexpected loss, it reuses the original session/profile
  and desired URL through the existing route-bound open operation.
- Resolution preserves the original view-stream provider, control-input
  provider, browser host, browser build, and display-isolation posture while
  reacquiring current route, display, Guacamole connection, and viewer state.
- A retained tab marked explicitly closed or released is terminal. The URL
  reports that state and requires an explicit Reopen action instead of silently
  recreating the tab.

## Durable And Ephemeral Identity

Durable handoff state includes:

- handoff id and creation/update timestamps;
- original normalized remote-view intent;
- logical browser, session, profile, and tab identity;
- desired tab URL and last observed title;
- original viewing, input, host, build, and display-isolation posture;
- latest resolution state and latest resolved route evidence.

The resolver never treats these as durable:

- CDP target id;
- Guacamole connection or client URL;
- route id, route-pool checkout, or display allocation;
- viewer or controller lease.

## Safety

- Dashboard authentication remains mandatory. Possession of the opaque id is
  not authorization.
- Reject missing, malformed, unknown, or terminal handoffs with structured
  states. Do not fall back to an unrelated browser.
- Strip stale route and display selectors before reacquisition.
- Preserve same-profile ownership and duplicate-process guards already
  enforced by `remote_view_open`.
- Do not infer an explicit close from Guacamole disconnect, viewer takeover,
  stale target id, browser crash, or route reconciliation alone.
- Keep raw target URLs and profile paths out of the public URL.

## Vertical Slices

1. Add a failing Rust contract test proving that an opened handoff persists an
   opaque id and returns a durable URL while preserving the raw provider URL.
2. Inject the service job id into dispatched commands, persist typed handoff
   records, and project the durable URL from the public ingress origin.
3. Add a failing resolver test for an ephemeral route and implement
   `service_remote_view_handoff_resolve` by replaying the normalized intent
   without stale route/display selectors.
4. Add explicit-close and unknown-handoff tests, then implement the terminal
   fail-closed states.
5. Add dashboard auth and resolver tests, then resolve `/remote-view/<id>` into
   the existing workspace viewport selection with the intended provider.
6. Update service contracts, generated clients, CLI help, README, skill,
   docs-site guidance, inline comments, ROADMAP, and RUNBOOK.
7. Run focused Rust and dashboard tests, generated-client parity, formatting,
   strict Clippy, selected changed-surface validation, canonical Rust tests,
   and an isolated live smoke that never visits a private target site.

## Results

- `remote_view_open` now emits an opaque authenticated
  `/remote-view/<handoff-id>` URL while keeping the current Guacamole URL in
  `providerExternalUrl` and route-binding evidence.
- Dashboard login uses a real navigation for preserved Guacamole paths. The
  authenticated durable route resolves through the original daemon lane,
  preserves the intended provider, and exposes Retry for transient provider
  reacquisition without weakening the explicit Reopen gate for a deliberately
  closed tab.
- Durable handoff records are projected into normal service state and retained
  in `~/.agent-browser/service/remote-view-handoffs.json`. The sidecar is an
  upgrade boundary: retained legacy daemons that do not understand the new
  field can rewrite `state.json` without erasing durable handoffs.
- The final live canary emitted handoff `r370864` for browser
  `session:p0096-durable-link-live-4`, target
  `81C5236939B7978AF1AF84408F3B39E8`, route `guacamole:2`, and provider
  `rdp_gateway`. A retained legacy daemon removed the primary-state field, the
  sidecar preserved it, a separate `service reconcile` retained it, and the
  authenticated resolver reacquired route B with `operatorVisible.state=ready`
  and the same target id.
- Logged-out validation reached dashboard authentication with the complete
  durable path and returned to that path after login. Missing handoffs render
  the authenticated unavailable state instead of leaking provider details.
- The canonical Rust gate passed with 1,265 tests discovered in the main
  partition, 1,208 run, 57 ignored, and all serial partitions green. Focused
  service-store tests passed 7 of 7, including the legacy-writer regression.
  Formatting, strict Clippy, dashboard contract tests, production builds,
  generated-client parity, docs build, workstation gates, and live route
  smokes also passed.
- The installed release-mode executable is
  `5e8953346d0bc085cd6edbb786acb71b9f28ea9ca81015c5633658415497a5ae`.
  The served dashboard bundle is
  `2afb5491d1f4e1487a5b55416fa34189fea7635cc83a690afce66ea22b60dacd`.
  Final install doctor and remote-view doctor both returned success with zero
  issues; workstation payload, dashboard runtime, runtime convergence,
  viewer prerequisites, and many-to-many readiness are green.

## Done Definition

- An anonymous request to an emitted durable URL reaches dashboard login with
  the complete `/remote-view/<handoff-id>` next path and returns to that path
  after successful authentication.
- Replacing or releasing the original Guacamole route does not invalidate the
  durable URL. Resolution acquires a current route and returns a ready
  operator-visible handoff for the same logical tab.
- Stale target ids recover through retained tab/session/profile evidence.
- Explicitly closed or released tabs do not reopen automatically.
- The emitted durable URL does not contain a Guacamole fragment or target URL.
- Source, contracts, generated clients, user documentation, installed runtime,
  and validation evidence agree.
