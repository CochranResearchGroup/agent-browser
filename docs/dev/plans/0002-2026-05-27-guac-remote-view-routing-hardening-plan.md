# Guacamole Remote View Routing Hardening Plan

Date: 2026-05-27
State: CLOSED
Lane: P02
Outcome: VALIDATED
Current state: route authority audit is recorded. Service stream records now
carry route metadata, dashboard Guacamole hash repair has been removed,
external open waits for takeover acceptance, and `view_takeover` persists a
service event with viewer metadata. Same-day live RDP/Guacamole validation
passed for the configured shared route.

## Purpose

This plan follows the review of the completed RDP and Guacamole gate in
`docs/dev/plans/0001-2026-05-26-rdp-guac-hardening-test-plan.md`.

Plan 0001 proved that the current deployment can support the first
RDP/Guacamole full-control path. This plan handles the remaining routing and
ownership gaps that prevent that path from scaling beyond one known Guacamole
connection and one local validation setup.

The concrete problems to fix are:

- the dashboard opens external Guacamole streams before the service-owned
  takeover request is known to have succeeded
- `view_takeover` currently returns takeover metadata but does not own or
  mutate viewer lease state
- Guacamole client route repair hardcodes the current connection hash instead
  of deriving the route from service state, provider discovery, or operator
  configuration

This plan covers Guacamole and remote viewing routing only. It does not switch
the default full-control backend away from RDP/Guacamole, and it does not add
CDP streaming or VNC/noVNC. Those remain separate backend-family lanes.

## Roadmap Context

`ROADMAP.md` keeps P01 closed because the validated RDP/Guacamole reliability
gate remains useful evidence for the current deployment. P02 is the follow-up
hardening lane for route authority and multi-browser operator correctness.

The expected end state is that Agent Browser can route multiple external
browser workspaces through Guacamole without any production code depending on
`MQBjAHBvc3RncmVzcWw=` or any other workstation-specific connection id.

## Source Findings

The implementation review found these roadmap alignment issues:

- external open calls `view_takeover` but opens the stream immediately instead
  of awaiting the service response
- `view_takeover` is service-routed, but the handler only echoes request
  metadata and does not record a lease, viewer event, route ownership, or
  reconnect result
- Guacamole root repair is duplicated in dashboard and Rust code with a
  hardcoded client route for the current PostgreSQL-backed connection

Treat the existing hardcoded route as a temporary fixture. It may remain in
test fixtures or local examples when explicitly labeled, but it must not be the
production fallback for service-owned remote viewing.

## Product Invariants

The lane is not done until these are true:

- Service state owns Guacamole route identity for every remote-view workspace.
- Dashboard code never invents or repairs a Guacamole connection route from a
  hardcoded connection hash.
- Every RDP gateway stream can name its source: explicit operator config,
  discovered Guacamole connection, service-retained route, or test fixture.
- External open waits for a service-owned takeover or reconnect decision before
  opening a controllable stream.
- Failed takeover does not open a stale or wrong Guacamole route.
- `view_takeover` records enough durable state for the dashboard to render
  `reconnecting`, `takeover_ready`, `taken_over`, and connected ownership
  transitions without treating a metadata echo as a completed takeover.
- Multiple remote-headed browser records can point to distinct Guacamole
  routes or distinct provider-owned route claims.
- One shared Guacamole route can still be modeled intentionally as a shared
  display, but that state must be explicit and low-contention.
- The current single workstation route can continue working through
  configuration, not hardcoded production defaults.

## Canonical Route Model

Add or formalize a UI-neutral route model that can be reused by CLI, HTTP,
MCP, generated clients, dashboard, and live harnesses.

Required fields or equivalent derived values:

- `provider`: normally `rdp_gateway` for this lane
- `routeId`: stable service route id
- `connectionId`: Guacamole connection id when known
- `connectionName`: operator-facing Guacamole connection name when known
- `connectionSource`: `config`, `discovered`, `retained_state`, `fixture`, or
  `unknown`
- `frameUrl`: dashboard embeddable URL when allowed
- `externalUrl`: direct external or popout URL
- `routeTemplate`: optional root or template URL when the connection must be
  substituted at runtime
- `displayName`: remote desktop display name, such as `:10`
- `displayIsolation`: `shared_display`, `private_virtual_display`, or
  `ambient_display`
- `browserId`: selected service browser identity
- `sessionId`: owning daemon or service session identity
- `profileId`: runtime profile identity
- `viewerLeaseId`: current viewer lease when known
- `viewerRole`: `controller`, `observer`, `pending_controller`, or `none`
- `lastViewerEvent`: `connected`, `disconnected`, `takeover_requested`,
  `taken_over`, `reconnected`, `failed`, or `expired`
- `readiness`: compact provider and route readiness

The service must reject ambiguous route claims unless they are explicitly
modeled as a shared display route.

## Implementation Slices

### Slice A: Route Authority Audit

State: VALIDATED

Goal: remove hidden assumptions before changing runtime behavior.

Tasks:

- Inventory every production and test reference to the current Guacamole client
  hash.
- Classify each reference as production fallback, local config, fixture,
  validation note, or historical evidence.
- Identify the authoritative source for Guacamole URLs in current runtime:
  `AGENT_BROWSER_REMOTE_VIEW_URL`, service state, access-plan decisions,
  retained browser records, or live readiness scripts.
- Decide the first supported route source order for production code.
- Document which existing validation notes are historical and should not be
  rewritten.

Validation:

- `rg -n "MQBjAHBvc3RncmVzcWw|DEFAULT_GUACAMOLE_CLIENT" .`
- `pnpm validation:select -- --base <ref>`
- `git diff --check`

Exit criteria:

- No production fallback is allowed to rely on the current hardcoded hash.
- Remaining hardcoded hashes are labeled fixture, local example, or historical
  evidence.
- The next slice has a source-order decision for route construction.

Handoff:

- Record the classified references and the chosen route source order in
  `docs/dev/notes/`.
- Recorded in
  `docs/dev/notes/2026-05-27-guac-route-authority-audit.md`.

### Slice B: Service-Owned Guacamole Route Model

State: VALIDATED

Goal: make the service state own Guacamole route identity.

Tasks:

- Add route fields to service browser stream records, or add a route record
  linked from `ViewStream`.
- Preserve backward compatibility for existing `viewStreams[].url` consumers.
- Generate `frameUrl` and `externalUrl` from service-owned route data.
- Replace dashboard-side Guacamole root repair with service-provided URLs or a
  typed route status.
- Replace Rust-side hardcoded route repair with config or service-state route
  resolution.
- Update service contracts, generated clients, docs, and skill guidance for
  the route fields.

Validation:

- focused Rust tests for route serialization and fallback source order
- `pnpm test:service-api-mcp-parity`
- `pnpm test:service-client-contract`
- `pnpm test:service-client-types`
- `pnpm test:dashboard-view-streams`
- `pnpm validation:select -- --base <ref>`
- `git diff --check`

Exit criteria:

- Route identity is visible in service status without inspecting dashboard
  code.
- Dashboard uses service route URLs rather than hardcoded Guacamole repair.
- Generated client helpers expose enough route metadata for software clients
  to avoid raw string guessing.

Current evidence:

- `ViewStream` now exposes optional `frameUrl`, `externalUrl`, `routeId`,
  `connectionId`, `connectionName`, and `routeSource`.
- Rust and dashboard status repair no longer append a hardcoded Guacamole
  client route.
- Generated-client and dashboard contract validation passed.

### Slice C: Deterministic Takeover And External Open

State: VALIDATED

Goal: make iframe and external open use the same service-owned decision.

Tasks:

- Make dashboard external open await the `view_takeover` result before opening
  a controllable route.
- Do not open an external URL when takeover or reconnect fails.
- Have `view_takeover` write a service event or viewer lease update before it
  returns success.
- Return typed takeover results: accepted, rejected, already_owner,
  provider_multi_view, provider_single_view, reconnecting, or failed.
- Surface a dashboard state that distinguishes request accepted from reconnect
  complete.
- Update live harnesses so they fail when external open races ahead of a failed
  takeover request.

Validation:

- `pnpm test:dashboard-view-streams`
- focused dashboard test proving external open awaits takeover success
- focused Rust test for `view_takeover` event or lease mutation
- `pnpm test:rdp-guac-viewer-transfer-live`
- `pnpm test:rdp-guac-browser-switch-live`
- `pnpm validation:select -- --base <ref>`
- `git diff --check`

Exit criteria:

- External open is blocked on failed takeover.
- The service records a viewer ownership transition for every successful
  takeover request.
- The dashboard no longer describes metadata echo as completed takeover.

Current evidence:

- Workspace external open now awaits `view_takeover` acceptance before opening
  `externalUrl`.
- `view_takeover` returns typed acceptance metadata and persists a
  `viewer_takeover_requested` service event containing `viewerLeaseId`,
  `lastViewerEvent`, route/provider details, browser, session, service, agent,
  and task context.

### Slice D: Multi-Browser Guacamole Routing Harness

State: VALIDATED

Goal: prove that multiple external browser workspaces route through distinct or
explicitly shared Guacamole route state.

Tasks:

- Extend the live browser-switch harness to record route ids, connection ids,
  frame URLs, external URLs, display isolation, and viewer lease ids for both
  browsers.
- Add a fixture mode with two distinct Guacamole route records.
- Add a live mode that can use two configured Guacamole routes when available.
- Prove that browser A cannot silently open browser B's Guacamole route, and B
  cannot silently open A's route.
- Prove that an intentionally shared route is labeled as shared display and
  low-contention.
- Keep artifacts under `/tmp/agent-browser-guac-routing-<timestamp>/` and
  summarize them in a dated handoff note.

Validation:

- `pnpm test:dashboard-workspace-nodes`
- `pnpm test:dashboard-workspace-navigator`
- `pnpm test:dashboard-view-streams`
- `pnpm test:rdp-gateway-readiness-live -- --require-html5-client`
- new or extended Guacamole route live harness
- desktop and mobile screenshots for two browser workspaces
- retained service-state samples before and after external open
- `git diff --check`

Exit criteria:

- Multiple remote-headed browser records preserve distinct route identity.
- Shared-route mode is explicit and visible.
- Live or fixture-backed evidence proves route mixups are detected before a
  human sees the wrong desktop.

Current evidence:

- Live RDP/Guacamole harness artifacts now include route URL, frame URL,
  external URL, route id, connection id, and connection name.
- Dashboard duplicate-route diagnostics now prefer `routeId`, `connectionId`,
  `frameUrl`, and `externalUrl` before falling back to legacy `url`.
- Viewer-transfer and browser-switch live gates passed on 2026-05-27 with a
  shared Guacamole route and service-visible route identity. Distinct-route
  evidence remains a future provider-configuration expansion, not a blocker for
  this shared-route hardening lane.

### Slice E: Release Gate And Documentation Closeout

State: VALIDATED

Goal: make the hardened routing path merge-ready and operator-readable.

Tasks:

- Run or cite same-day Slice C and Slice D validation.
- Re-run the current RDP gateway readiness smoke.
- Update `README.md`, `cli/src/output.rs`, `skills/agent-browser/SKILL.md`,
  docs site pages, service contracts, and inline comments where behavior
  changed.
- Sync the installed agent-browser skill copy.
- Record remaining limitations for Guacamole auth, public ingress, destructive
  provider failure tests, and private display allocation.
- Keep P01 closed and close P02 only after route authority and takeover
  semantics are validated.

Required final validation:

- `pnpm test:dashboard-view-streams`
- `pnpm test:dashboard-workspace-nodes`
- `pnpm test:dashboard-workspace-navigator`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:service-client-contract`
- `pnpm test:service-client-types`
- `pnpm test:rdp-gateway-readiness-live -- --require-html5-client`
- `pnpm test:rdp-guac-viewer-transfer-live`
- `pnpm test:rdp-guac-browser-switch-live`
- new or extended Guacamole route hardening harness
- Rust format, clippy, and focused Rust tests if Rust changed
- `pnpm validation:select -- --base <ref>`
- `git diff --check`

Exit criteria:

- No production code path hardcodes the current Guacamole client hash.
- Service state owns route identity and viewer ownership transitions.
- Dashboard external open waits for service takeover success.
- The final handoff names routes, route sources, browsers, sessions, profiles,
  displays, screenshots, and remaining provider limitations.

Current evidence:

- README, CLI help, docs site, service contracts, generated observability
  client, and repo/installed `agent-browser` skill mention the new route fields
  and takeover event.
- Final local and live validation is recorded in
  `docs/dev/notes/2026-05-27-guac-route-hardening-validation.md`.

## Risks And Boundaries

- The current workstation may only have one Guacamole connection. Fixture-backed
  multi-route tests are acceptable until a second live route exists, but the
  release gate must clearly label that evidence class.
- The existing `AGENT_BROWSER_REMOTE_VIEW_URL` behavior must keep working for
  a single configured route.
- Guacamole authentication and Authelia ingress are provider concerns. This
  lane should surface readiness and recovery copy, not embed secrets or bypass
  auth.
- This plan should not silently convert Guacamole into the dashboard root app.
  Guacamole remains a provider behind the dashboard-owned workspace viewport.
- CDP streaming and VNC/noVNC remain separate lanes. Do not solve those backend
  families inside this plan except to preserve the shared `viewStreams`
  contract.
