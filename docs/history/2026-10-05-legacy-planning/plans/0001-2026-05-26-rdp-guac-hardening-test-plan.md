# RDP And Guacamole Hardening Test Plan

Date: 2026-05-26
State: CLOSED
Lane: P01
Outcome: VALIDATED
Revision: Slice A implemented; Slice B validated with live two-client RDP and
Guacamole evidence. Slice C validated managed browser A/B switching with live
RDP and Guacamole evidence. Slice D is validated with source checks, a live
healthy RDP/Guacamole viewport, and a non-destructive hybrid failure-state
matrix. Slice E validated the same-day reliability gate and release handoff for
the current RDP and Guacamole full-control path.

## Purpose

This note expands the RDP and Guacamole item in
`docs/dev/notes/2026-05-26-remote-view-backends-campaign.md`.

The goal is to make the current `rdp_gateway` path production-reliable before
switching the default full-control backend. The required outcome is not merely
that a browser sometimes appears in Guacamole. The dashboard must survive:

- switching the same remote viewport between two active dashboard viewers
- switching one viewer from managed remote browser A to managed remote browser B
- refresh, stale tab identity, iframe and popout transitions, and provider
  disconnects without falling into a blank white viewport, blank black popout,
  unhappy iframe document, or misleading unavailable state

This plan only covers the first backend family: RDP through Guacamole. CDP
streaming and VNC/noVNC remain separate campaign items.

## Current State

- The repo did not previously have a canonical `docs/dev/plans/` directory, so
  this is the first serialized plan artifact under that surface.
- The active roadmap authority is currently the service roadmap note at
  `docs/dev/notes/2026-04-22-agent-browser-service-roadmap.md`, with this plan
  linked from the remote-view backend productization campaign.
- Slice A is implemented as a dashboard-side ownership, diagnostics, and UX
  state hardening checkpoint. Evidence is recorded in
  `docs/dev/notes/2026-05-26-rdp-guac-slice-a-ownership-audit.md`.
- Slice B is validated for the current RDP and Guacamole deployment. The live
  checkpoint proved simultaneous viewing with Google Chrome and Brave dashboard
  clients, refresh recovery, mobile viewport controls, service-state artifacts,
  screenshots, and an external-open `view_takeover` job.
- Slice B validation evidence is recorded in
  `docs/dev/notes/2026-05-26-rdp-guac-slice-b-live-validation.md`.
- Slice C is validated for the current RDP and Guacamole deployment. The live
  checkpoint proved managed browser A/B route switching, browser B refresh
  recovery, a second viewer on browser A while client 1 stayed routed to B,
  A/B alternation screenshots, and external-open `view_takeover`.
- Slice C validation evidence is recorded in
  `docs/dev/notes/2026-05-26-rdp-guac-slice-c-live-validation.md`.
- Slice D is validated for source readiness behavior and non-destructive live
  readiness rendering. Source evidence is recorded in
  `docs/dev/notes/2026-05-26-rdp-guac-slice-d-readiness-source-checkpoint.md`.
- Slice D live evidence is recorded in
  `docs/dev/notes/2026-05-26-rdp-guac-slice-d-live-validation.md`.
  It records a healthy live RDP/Guacamole baseline and rendered dashboard
  evidence for each failure class, with every item labeled `live`,
  `isolated-live`, or `fixture-backed`.
- Slice E is validated for the current RDP and Guacamole deployment. The
  reliability gate is recorded in
  `docs/dev/notes/2026-05-26-rdp-guac-slice-e-reliability-gate.md`.
- This plan validates RDP/Guac as the supportable first production
  full-control path for this deployment. It does not by itself switch any
  default backend setting.

## Slice State Semantics

Use these states consistently in this plan and in slice handoff notes:

- `PLANNED`: the work is described but implementation has not started.
- `IN_PROGRESS`: files are being changed for the slice.
- `IMPLEMENTED`: source, docs, and targeted non-live validation are complete
  for a slice that has no required live smoke.
- `IMPLEMENTED_PENDING_LIVE_VALIDATION`: source, docs, and targeted non-live
  validation are complete, but a required live or manual smoke has not run.
- `VALIDATED`: required source checks, selector-driven checks, and live or
  manual evidence are recorded in a handoff note.

When a slice includes a live test requirement, do not mark it `VALIDATED`
without dated evidence naming the clients, routes, browser ids, session ids,
provider behavior, screenshots or equivalent artifacts, and residual risk.

## Evidence Classes

Use these labels in Slice D and Slice E handoff notes:

- `live`: exercised against the current RDP and Guacamole deployment without
  substituting the dashboard, service, stream, or provider response.
- `isolated-live`: exercised through a real dashboard or service path using an
  isolated `AGENT_BROWSER_HOME`, test auth state, invalid test routes, missing
  test connection ids, or synthetic sentinel browsers that do not alter shared
  provider services.
- `fixture-backed`: exercised through a rendered dashboard or source fixture
  that intentionally supplies a readiness or stream failure payload. This can
  prove dashboard product behavior, but not provider behavior.
- `manual-not-run`: intentionally not exercised. The handoff must explain why
  and whether operator approval, credentials, or a destructive provider action
  would be required.

Slice D can become `VALIDATED` with a mix of `live`, `isolated-live`, and
`fixture-backed` evidence only when the healthy baseline is `live`, every
user-visible failure class has rendered dashboard evidence, and the handoff
names any remaining provider-behavior risk. A destructive shared-provider test
is never required for Slice D unless the operator explicitly asks for it.

## Execution Baseline

Before starting any implementation slice, record these in the slice handoff or
validation note:

- active branch or worktree, current `git status --short`, and whether the
  dirty state belongs to this lane
- validation base ref for `pnpm validation:select -- --base <ref>`, preferably
  the last green CI commit or an explicit branch fork point
- touched surfaces: service contract, generated client, Rust service code,
  dashboard UI, documentation, or live RDP and Guacamole deployment

If the base ref or dirty-state ownership is unclear, resolve that before
calling the slice merge-ready. Each implementation slice must run
`pnpm validation:select -- --base <ref>` and either run or explicitly justify
the selector's recommendations.

For live RDP and Guacamole checkpoints, also record these inputs before the
first client opens:

- `AGENT_BROWSER_REMOTE_VIEW_PROVIDER` and a redacted
  `AGENT_BROWSER_REMOTE_VIEW_URL`
- `AGENT_BROWSER_REMOTE_HEADED_DISPLAY`, including how it was discovered when
  not already configured
- dashboard client executables and whether they are distinct
- `pnpm test:rdp-gateway-readiness-live -- --require-html5-client` result
- relevant `guacd`, `xrdp`, and `xrdp-sesman` service status or log evidence
- whether dashboard auth was initialized in the same isolated
  `AGENT_BROWSER_HOME` used by the harness

For every behavior-changing slice, update the user-facing documentation
surfaces required by `AGENTS.md` in the same slice. That includes
`cli/src/output.rs`, `README.md`, `skills/agent-browser/SKILL.md`,
`docs/src/app/`, and relevant inline source comments when commands, provider
semantics, controls, environment variables, or service behavior change.

## Product Invariants

The implementation is not done until these invariants are true in a live
operator environment:

- Browser process, CDP endpoint, selected target, display, Guacamole route,
  stream ownership, and input ownership are service-owned state.
- The dashboard can request focus, transfer, or reconnect, but cannot invent
  browser truth locally.
- If Guacamole supports only one active viewer for a connection, the UI says so
  and offers a deterministic Take over action.
- If a second viewer takes over, the first viewer sees an explicit disconnected
  or taken-over state, not a white frame, black frame, or Guacamole dead screen.
- Refreshing a workspace URL recovers the selected browser and current live tab
  even when the retained target id is stale.
- Switching from remote browser A to remote browser B focuses and renders B,
  does not leak A's Guacamole state, and preserves both browser records.
- Switching back to browser A refocuses and renders A without launching an
  accidental duplicate browser or stealing B's retained tab identity.
- The browser window remains maximized or resized to the remote viewport shape
  after focus, reconnect, and browser switch operations.
- Mobile and desktop clients expose the same essential control path, including
  Take over, external open, fullscreen, and Guacamole interaction settings.

## Canonical State Model

Slice 1 should audit and then harden the state model around one canonical
remote-control workspace record. A complete RDP-backed workspace needs these
fields or equivalent derived values:

- `browserId`: stable service browser identity.
- `sessionId`: stable logical session identity for the owning agent or user.
- `profileId`: managed runtime profile identity.
- `browserHost`: `remote_headed`.
- `browserBuild`: normally `stealthcdp_chromium` when available and policy
  allows it.
- `displayIsolation`: `shared_display` or private display allocation, with the
  low-contention limits explicit.
- `displayName`: X display or remote desktop name, such as `:10`.
- `displaySize`: current desktop size last requested by the viewport.
- `viewStreamProvider`: `rdp_gateway`.
- `controlInputProvider`: `manual_attached_desktop`.
- `guacamoleConnectionId`: service-known Guacamole connection id or route.
- `frameUrl`: dashboard-embeddable Guacamole URL when embedding is allowed.
- `externalUrl`: direct Guacamole URL for popout or external routing.
- `activeViewerLease`: current viewer lease or best-effort owner marker when
  the provider can expose it.
- `lastViewerEvent`: connected, disconnected, taken over, timed out, or
  replaced.
- `currentTargetId`: live CDP target selected for this browser.
- `retainedTargetId`: target id stored in durable state, which may be stale.
- `targetFallback`: index or current page fallback used when retained target
  identity is stale.
- `focusJobId`: last queued service focus request.
- `takeoverJobId`: last queued service takeover or reconnect request.
- `readiness`: local RDP, Guacamole, ingress, auth, and stream health summary.

The service should reject or reconcile duplicate retained sessions that claim
the same live CDP endpoint, display, Guacamole connection, or target unless the
records are explicitly modeled aliases.

## UX State Vocabulary

The viewport should present a small, consistent set of states. These states
should be driven by service data and stream events:

- `preparing_focus`: the service is focusing or maximizing the selected
  browser before connecting.
- `connecting`: the iframe or popout route is loading the stream.
- `connected`: the selected browser is visible and controllable.
- `owned_elsewhere`: another dashboard, popout, or device owns the active
  Guacamole connection.
- `takeover_ready`: this viewer can take over the connection.
- `taken_over`: this viewer was replaced by another viewer.
- `reconnecting`: the viewer has requested ownership and is waiting for a fresh
  stream.
- `stale_target_recovered`: the retained target was stale and the service
  selected a current live tab.
- `provider_unavailable`: RDP, Guacamole, ingress, or auth readiness failed.
- `browser_unavailable`: the browser process or CDP endpoint is unhealthy.

The dashboard should not expose provider internals like "blocked" as the
primary user-facing state. It should say what a human can do next: Take over,
Reconnect, Open externally, Relaunch browser, or Inspect readiness.

## Implementation Slices

### Slice A: Ownership Audit And Diagnostics

State: IMPLEMENTED

Goal: make current failures explainable before changing the viewer behavior.

Tasks:

- Inventory retained service state for browser, session, tab, display,
  Guacamole route, iframe URL, external URL, and readiness.
- Add no-launch diagnostics for duplicate browser/session ownership of the same
  CDP endpoint, display, Guacamole route, or target id.
- Add stale-target detection that distinguishes retained target absence from
  browser process failure.
- Add a dashboard model that maps raw stream and readiness fields into the UX
  state vocabulary above.
- Document whether the currently deployed Guacamole route is single-active
  viewer or simultaneous-viewer capable.

Validation:

- `pnpm test:dashboard-view-streams`
- focused helper test for duplicate endpoint or target detection
- focused helper test for stale target fallback
- service contract, generated client, or Rust focused checks if diagnostics
  change service records or API payload shapes
- `pnpm validation:select -- --base <ref>`
- `git diff --check`

Exit criteria:

- A healthy browser with a stale target cannot appear as a dead stream.
- Duplicate retained browser or session ownership is visible in diagnostics.
- The current Guacamole multi-viewer behavior is recorded with evidence.

Handoff:

- Slice A evidence is recorded in
  `docs/dev/notes/2026-05-26-rdp-guac-slice-a-ownership-audit.md`.

### Slice B: Viewer Lease And Takeover Behavior

State: VALIDATED

Goal: make two active dashboard viewers deterministic.

This slice is intentionally split into a source checkpoint and a live
validation checkpoint. The source checkpoint can prove contract alignment and
dashboard behavior without launching real RDP sessions. The live checkpoint is
what proves the provider behavior and promotes the slice to `VALIDATED`.

Source checkpoint tasks:

- Add a service request or service event contract for viewer takeover and
  reconnect when the backend supports only one active viewer.
- If takeover changes request, event, schema, or generated-client shapes, keep
  service contracts, MCP and HTTP parity, and generated client helpers aligned.
- Make iframe and popout routes use the same lease and reconnect semantics.
- Convert Guacamole disconnect, refused iframe, and missing connection cases
  into dashboard states instead of raw blank frames.
- Preserve browser process and session state when a viewer disconnects,
  refreshes, opens externally, or is replaced by another viewer.
- Add a small viewport control for Guacamole interaction settings on mobile and
  desktop.
- Update user-facing docs for takeover states, controls, provider semantics,
  and any new scripts or environment variables in this slice.

Source checkpoint validation:

- `pnpm test:dashboard-view-streams`
- focused dashboard source test for Take over and external-open routing
- `pnpm test:service-api-mcp-parity` if request, resource, or action contracts
  change
- `pnpm test:service-client-contract` and `pnpm test:service-client-types` if
  generated client payloads change
- focused Rust tests for service request, service event, or service model
  changes
- `pnpm validation:select -- --base <ref>`
- `git diff --check`

Source checkpoint exit criteria:

- Take over queues a service-owned takeover or reconnect request rather than
  only refreshing local iframe state.
- Iframe and popout entry points use the same selected workspace, takeover, and
  reconnect semantics.
- The dashboard exposes explicit `takeover_ready`, `reconnecting`, and
  `taken_over` states for provider handoff paths.
- Browser process, retained browser identity, and session records are not
  closed, relaunched, or rewritten by viewer handoff actions.
- The slice state may become `IMPLEMENTED_PENDING_LIVE_VALIDATION`, but not
  `VALIDATED`.

Live test requirement:

Prerequisites:

- Confirm RDP and Guacamole readiness with
  `pnpm test:rdp-gateway-readiness-live -- --require-html5-client`.
- Use a real RDP-backed stream URL, a known remote display, and two distinct
  dashboard client executables.
- In isolated harness homes, initialize dashboard auth through the dashboard
  page and authenticate from the browser client context before asserting
  workspace viewport state. Do not classify the Superuser login page as an RDP
  viewport failure.
- If auth, ingress, or provider readiness fails before both clients reach the
  workspace route, record the attempt as a live harness issue rather than Slice
  B provider validation.

1. Start managed remote browser A through the service on the RDP path.
2. Open workspace A in client 1 with one browser executable, such as Google
   Chrome.
3. Open the same workspace A in client 2 with a different browser executable,
   such as Brave.
4. Prove one of these outcomes:
   - both clients can view the same desktop simultaneously, or
   - client 2 can take over and render A while client 1 moves to an explicit
     taken-over or takeover-ready state
5. Client 1 takes over again and renders A.
6. Client 2 moves to the explicit taken-over or takeover-ready state.
7. Refresh both clients one at a time and prove the selected browser remains
   alive and recoverable.

Live checkpoint validation:

- `pnpm test:service-dashboard-remote-control-ui-live`
- `pnpm test:rdp-guac-viewer-transfer-live`
- desktop screenshot evidence from both clients
- mobile screenshot evidence for the Take over and interaction settings
  controls
- service-state samples before takeover, after client 2 takeover, after client
  1 takeover, and after refresh recovery
- `pnpm validation:select -- --base <ref>`
- `git diff --check`

Live checkpoint exit criteria:

- Two active viewers no longer create silent white, black, unhappy-document, or
  ambiguous disconnected states.
- The service session and browser process survive both takeover directions.
- The handoff behavior is visually clear in iframe and popout modes.
- The recorded evidence states whether the current Guacamole deployment supports
  simultaneous viewers or single-active-viewer takeover.

Handoff:

- Source checkpoint evidence is recorded in
  `docs/dev/notes/2026-05-26-rdp-guac-slice-b-source-checkpoint.md`.
- The guarded viewer-transfer live harness is recorded in
  `docs/dev/notes/2026-05-26-rdp-guac-viewer-transfer-live-harness.md`.
- Live validation evidence is recorded in
  `docs/dev/notes/2026-05-26-rdp-guac-slice-b-live-validation.md`.
- The current deployment was classified as `simultaneous_view`, while the
  external-open path still queued `view_takeover`.

### Slice C: Managed Remote Browser Switching

State: VALIDATED

Goal: make one viewport switch cleanly between at least two managed remote
browsers.

Slice B is validated. Slice C now uses two managed `remote_headed` daemon
sessions with distinct runtime profiles in the same isolated service home,
serves the dashboard from browser A's stream server, and verifies that
dashboard workspace route switches queue `view_focus` against the selected
browser session.

Tasks:

- Ensure workspace selection queues a service-owned `view_focus` request for
  the selected browser before reconnecting the stream.
- Keep browser A and browser B records distinct by browser id, session id,
  profile id, display, Guacamole route, and current target id.
- When shared display is still used, make focus semantics explicit and prevent
  stale Guacamole connection data from making A appear while B is selected.
- If private display allocation is implemented, ensure the stream points to the
  selected workspace display and does not reuse the previous workspace URL.
- Maximize or resize the selected browser after every focus and viewport resize
  event.
- Update user-facing docs if browser switching changes route behavior,
  provider semantics, controls, scripts, or environment variables.

Live test requirement:

1. Launch managed remote browser A with a harmless sentinel page whose title
   and body clearly identify A.
2. Launch managed remote browser B with a different harmless sentinel page
   whose title and body clearly identify B.
3. Client 1 opens workspace A and verifies title, URL, screenshot, and viewport
   contents for A.
4. Client 1 switches to workspace B through the dashboard route and verifies
   title, URL, screenshot, and viewport contents for B.
5. Client 1 refreshes the workspace B URL and verifies B remains selected.
6. Client 2 opens workspace A while client 1 is on B and verifies the expected
   viewer behavior for A.
7. Alternate A to B to A to B three times, including one iframe to popout
   transition.

Suggested test sessions:

- `rdp-hardening-a`
- `rdp-hardening-b`

Suggested sentinel pages:

- `data:text/html,<title>RDP Hardening A</title><h1>RDP Hardening A</h1>`
- `data:text/html,<title>RDP Hardening B</title><h1>RDP Hardening B</h1>`

Validation:

- `pnpm test:dashboard-workspace-nodes`
- `pnpm test:dashboard-workspace-navigator`
- `pnpm test:dashboard-view-streams`
- `pnpm test:rdp-guac-browser-switch-live`
- retained-state before and after samples showing distinct browser records
- screenshots for A, B, refresh on B, and return to A
- `pnpm validation:select -- --base <ref>`
- `git diff --check`

Exit criteria:

- The viewport never shows browser A while the route and selected workspace are
  browser B, or browser B while selected workspace is A.
- Switching workspaces does not launch accidental duplicate remote browsers.
- Stale retained target identity is repaired without losing the selected
  workspace.

Handoff:

- Slice C harness details are recorded in
  `docs/dev/notes/2026-05-26-rdp-guac-slice-c-browser-switch-harness.md`.
- Slice C live validation evidence is recorded in
  `docs/dev/notes/2026-05-26-rdp-guac-slice-c-live-validation.md`.
- Residual risk: one earlier browser B `view_focus` job remained `running` in
  the final retained state even though later browser B focus jobs succeeded and
  the final B screenshot was captured. Carry retained job lifecycle cleanup
  into Slice D.

### Slice D: Readiness And Failure Productization

State: VALIDATED

Goal: surface actionable failures before a human sees a blank stream.

Revision note:

- Slice C proved managed browser A/B switching, refresh recovery, simultaneous
  viewing on the current deployment, and external-open `view_takeover`. It also
  left one earlier browser B `view_focus` job in `running` state after later
  focus jobs succeeded and the final browser B screenshot was captured.
- Slice D must therefore distinguish provider readiness, dashboard auth,
  browser health, viewer ownership, and retained focus or takeover job
  ambiguity before Slice E repeats the full live gate.
- Slice D does not switch the default backend and does not make RDP/Guac the
  default production full-control path. That remains Slice E's exit.

Source checkpoint tasks:

- Add a compact readiness result for `xrdp`, `xrdp-sesman`, `guacd`,
  Guacamole web app, Guacamole connection existence and permission, backend TCP
  reachability, dashboard auth, iframe embedding, local ingress, public ingress,
  selected browser health, selected stream URL, and current focus or takeover
  job lifecycle.
- Keep the readiness result UI-neutral enough for CLI, HTTP, MCP, and dashboard
  reuse. It should expose a compact component name, status, evidence string,
  next-action token, and operator-facing recovery copy or equivalent fields.
- Surface readiness in launcher eligibility rows before a user opens a broken
  workspace route.
- Surface readiness in viewport connection states after route load, including
  the distinction between browser failure, provider failure, auth failure,
  iframe or public ingress failure, another viewer owning the connection, and a
  stale or ambiguous focus or takeover job.
- Map known failure modes to recovery copy for connection missing, connection
  refused, black popout, white desktop, auth expired, active viewer takeover,
  browser not maximized, missing stream URL, and stale focus or takeover job.
- Treat stale retained focus or takeover jobs as evidence, not as proof that a
  healthy rendered browser is blocked. Active pending jobs may surface as
  `preparing_focus` or `reconnecting`, but a later successful focus with a
  visible stream must not be hidden behind an older retained job.
- Make the external-open link share the same selected workspace, takeover
  state, and recovery copy as the embedded route.
- Update user-facing docs for readiness states, recovery actions, public and
  local route expectations, scripts, and environment variables in this slice.

Source checkpoint validation:

- `pnpm test:dashboard-view-streams`
- `pnpm test:dashboard-launcher-eligibility`
- dashboard source tests for readiness mapping and disabled-state copy
- `node --check scripts/smoke-rdp-gateway-readiness.js` if the readiness script
  changes
- `pnpm test:service-api-mcp-parity` if service request, service resource, or
  contract metadata changes
- `pnpm test:service-client-contract` and `pnpm test:service-client-types` if
  generated client payloads change
- focused Rust tests for service readiness, service health, service request, or
  service model changes
- `pnpm --dir docs build` if docs site files change
- `pnpm validation:select -- --base <ref>`
- `git diff --check`

Source checkpoint exit criteria:

- A missing stream, dead browser, dashboard auth challenge, iframe embedding
  block, public ingress failure, Guacamole connection failure, another viewer
  owning the connection, and retained focus or takeover job ambiguity each map
  to a visible state and next action.
- Launcher disabled-state copy and viewport recovery copy agree on the failure
  class and next action.
- Embedded and external routes use the same selected workspace and takeover
  state.
- The slice state may become `IMPLEMENTED_PENDING_LIVE_VALIDATION`, but not
  `VALIDATED`.

Live validation requirement:

Prerequisites:

- Confirm the healthy baseline with
  `pnpm test:rdp-gateway-readiness-live -- --require-html5-client`.
- Use an isolated `AGENT_BROWSER_HOME`, a real RDP-backed stream URL, a known
  remote display, and dashboard auth initialized in the same isolated home.
- Capture local and public route values, with public URLs redacted where needed.
- Do not stop or reconfigure shared `xrdp`, `xrdp-sesman`, `guacd`, Guacamole,
  dashboard auth, or public ingress services merely to create a failure. Use
  isolated homes, invalid test routes, expired test auth, missing test
  connection ids, or source-level fixtures unless the operator explicitly
  approves a destructive provider test.
- Prefer isolated-live failures when they can be induced without changing
  shared services. Use fixture-backed rendered failures for provider outages,
  iframe policy blocks, auth challenges, or public ingress failures that would
  otherwise require destructive shared-state changes.
- Do not classify the Guacamole Superuser login page as a provider failure.
  Treat it as dashboard or Guacamole auth state and record the evidence class
  used to trigger it.

Live checkpoint tasks:

1. Record a `live` healthy readiness result and connected viewport screenshot.
2. Prove dashboard or Guacamole auth failure maps to auth recovery copy rather
   than a provider failure. Prefer `isolated-live`; allow `fixture-backed` when
   auth mutation would affect a shared operator path.
3. Prove missing or invalid Guacamole connection data maps to connection
   recovery copy rather than a browser failure. Prefer `isolated-live` invalid
   test connection ids.
4. Prove refused or unreachable RDP/Guacamole route data maps to provider or
   ingress recovery copy rather than a blank iframe or black popout. Use
   `fixture-backed` evidence unless the operator approves a real provider
   outage test.
5. Prove a viewer-takeover or remote-disconnect event maps to `takeover_ready`,
   `taken_over`, or `reconnecting` with the correct action. Use `live` evidence
   from the current simultaneous-view deployment where possible, and
   `fixture-backed` evidence for single-active-viewer disconnect states that
   the current deployment does not naturally emit.
6. Prove browser process or CDP failure maps to `browser_unavailable` and a
   relaunch or inspect-browser action. Prefer `isolated-live` synthetic
   browsers; use `fixture-backed` evidence if terminating a live browser would
   disturb an operator workspace.
7. Prove stale retained focus or takeover jobs from earlier route switches do
   not mask a later visible, healthy stream. This requires a rendered healthy
   stream or a rendered healthy stream fixture plus retained-job evidence from
   Slice C or the new harness.

Live checkpoint validation:

- `pnpm test:rdp-gateway-readiness-live -- --require-html5-client`
- local route smoke
- public route smoke when ingress is available
- rendered `agent-browser` inspection of healthy and intentionally failed
  readiness states, with each assertion labeled by evidence class
- screenshots or equivalent artifacts for healthy, auth failure, provider
  failure, browser failure, and viewer-ownership states
- retained service-state samples showing readiness, selected browser, selected
  stream URL, and focus or takeover job state
- `pnpm validation:select -- --base <ref>`
- `git diff --check`

Live checkpoint exit criteria:

- Every known RDP/Guac failure mode maps to a user-visible state and a next
  action.
- The dashboard tells the difference between a browser failure, provider
  failure, auth failure, another viewer owning the connection, and retained job
  ambiguity.
- No failed readiness path presents as an unexplained white viewport, black
  popout, unhappy iframe document, or generic unavailable state.

Handoff:

- Live validation evidence is recorded in
  `docs/dev/notes/2026-05-26-rdp-guac-slice-d-live-validation.md`.
- The live handoff includes an evidence matrix that states which checks are
  `live`, `isolated-live`, `fixture-backed`, or `manual-not-run`; which checks
  prove public ingress; and which manual operator steps remain outside Agent
  Browser.
- Source checkpoint evidence is recorded in
  `docs/dev/notes/2026-05-26-rdp-guac-slice-d-readiness-source-checkpoint.md`.
  The slice is validated, with destructive provider outages intentionally
  covered by fixture-backed evidence rather than shared-service mutation.

### Slice E: Reliability Gate And Release Handoff

State: VALIDATED

Goal: make RDP/Guac the supportable first production full-control path.

Tasks:

- Run the two live critical smokes from Slice B and Slice C in the same
  validation session.
- Re-run or cite the Slice D live readiness and failure-state checkpoint from
  the same day. If Slice D is not validated, Slice E is blocked rather than
  merely deferred.
- Capture screenshots and service-state snippets under a dated validation note.
- Verify desktop, mobile-width, iframe, popout, refresh, and fullscreen paths.
- Verify that behavior-changing slices already updated user-facing docs and the
  installed agent-browser skill where behavior, commands, provider semantics,
  or operator controls changed.
- Record remaining provider limitations and the fallback plan to CDP streaming
  or VNC/noVNC.

Required final validation:

- `pnpm test:dashboard-view-streams`
- `pnpm test:dashboard-launcher-eligibility`
- `pnpm test:dashboard-workspace-nodes`
- `pnpm test:dashboard-workspace-navigator`
- `pnpm test:service-dashboard-remote-control-ui-live`
- `pnpm test:rdp-gateway-readiness-live -- --require-html5-client`
- `pnpm test:rdp-guac-viewer-transfer-live`
- `pnpm test:rdp-guac-browser-switch-live`
- `pnpm validation:select -- --base <ref>`
- Rust format, clippy, and focused Rust tests if service or CLI Rust changed
- rendered `agent-browser` visual inspection across desktop and mobile-width
  dashboard routes
- `git diff --check`

Exit criteria:

- The final handoff names the tested clients, routes, browsers, sessions,
  profiles, screenshots, and public ingress path.
- The same validation proves viewer transfer and managed browser switching.
- Slice D source and live readiness handoffs are recorded, or the final handoff
  explicitly states that readiness failure-state evidence is still blocking
  release.
- Remaining limitations are explicit enough to drive the CDP and VNC/noVNC
  campaign items without redesigning the dashboard viewport.

Result:

- Slice E reliability evidence is recorded in
  `docs/dev/notes/2026-05-26-rdp-guac-slice-e-reliability-gate.md`.
- The live gate passed `pnpm test:service-dashboard-remote-control-ui-live`,
  `pnpm test:rdp-gateway-readiness-live -- --require-html5-client`,
  `pnpm test:rdp-guac-viewer-transfer-live`, and
  `pnpm test:rdp-guac-browser-switch-live` in one validation session.
- The final handoff cites same-day Slice D readiness evidence and records the
  remaining provider limitations, shared-display constraint, and fallback path
  to CDP streaming or VNC/noVNC.

## Automation Design

Add live tests as opt-in scripts because they require a running service, RDP,
Guacamole, dashboard auth, and at least one remote display.

Implemented scripts:

- `scripts/test-rdp-guac-viewer-transfer-live.js`
- `scripts/test-rdp-guac-browser-switch-live.js`
- `scripts/test-rdp-guac-readiness-failures-live.js`

Implemented package scripts:

- `pnpm test:rdp-guac-viewer-transfer-live`
- `pnpm test:rdp-guac-browser-switch-live`
- `pnpm test:rdp-guac-readiness-failures-live`

Both live scripts are guarded manual harnesses that refuse to run unless all
required environment variables are present.

Existing readiness baseline:

- `pnpm test:rdp-gateway-readiness-live -- --require-html5-client`

Slice D automation:

- a guarded hybrid readiness failure-state harness:
  `scripts/test-rdp-guac-readiness-failures-live.js`
- a package script: `pnpm test:rdp-guac-readiness-failures-live`
- an evidence matrix emitted by the harness or handoff note, covering healthy,
  auth failure, Guacamole connection failure, provider or ingress failure,
  viewer ownership, browser unavailable, and retained-job ambiguity

Test harness requirements:

- Run the healthy readiness baseline before failure assertions.
- Use two independent browser clients, preferably two executables, when viewer
  ownership is part of the run.
- Use separate agent-browser sessions and profiles for the test clients.
- Store artifacts under
  `/tmp/agent-browser-rdp-guac-hardening-<timestamp>/`.
- Store Slice C browser-switch artifacts under
  `/tmp/agent-browser-rdp-guac-browser-switch-<timestamp>/`.
- Capture dashboard screenshots, viewport screenshots, URL state, service
  status snippets, selected workspace ids, selected browser ids, target ids,
  and Guacamole route ids.
- Label each failure assertion as `live`, `isolated-live`, `fixture-backed`, or
  `manual-not-run`.
- Clean up isolated test sessions, profiles, and retained test records when the
  run uses synthetic sentinel browsers.
- Leave real operator workspaces untouched unless the operator explicitly asks
  to use them as validation fixtures.

The automation should accept environment overrides for existing fixtures:

- `AGENT_BROWSER_RDP_TEST_BROWSER_A`
- `AGENT_BROWSER_RDP_TEST_BROWSER_B`
- `AGENT_BROWSER_RDP_TEST_PROFILE_A`
- `AGENT_BROWSER_RDP_TEST_PROFILE_B`
- `AGENT_BROWSER_RDP_TEST_PUBLIC_URL`
- `AGENT_BROWSER_RDP_TEST_CLIENT_A_EXECUTABLE`
- `AGENT_BROWSER_RDP_TEST_CLIENT_B_EXECUTABLE`

## Manual Validation Checklist

Use this checklist for any interim slice where full automation is not ready:

1. Confirm `agent-browser service status` shows the expected managed remote
   browsers and no duplicate session claims.
2. Open the dashboard workspace URL in client 1 and capture the connected
   viewport.
3. Open the same URL in client 2 and capture either simultaneous view or clean
   takeover behavior.
4. Take control back from client 1 and capture client 2's taken-over state.
5. Switch from browser A to browser B in client 1 and capture B's title, URL,
   and viewport.
6. Refresh the B URL and verify B remains selected.
7. Open browser A from client 2 and verify browser A's route does not show B.
8. Test iframe, popout, fullscreen, and mobile-width controls.
9. Confirm healthy readiness, auth failure, provider failure, browser failure,
   viewer ownership, and stale focus or takeover job states each show distinct
   recovery copy.
10. Record screenshots, service-state snippets, and any provider logs needed to
   explain failures.

## Risks And Boundaries

- The current Guacamole deployment may be single-active-viewer by design. That
  is acceptable only if takeover is deterministic, visible, and reversible.
- Shared display mode can make many managed browsers contend for focus. The
  first hardening pass may keep shared display as an explicit low-contention
  mode, but the many-browser slice should drive toward private display or
  equally explicit allocation.
- CDP target ids are not durable enough to be the only selected-tab identity.
  The dashboard needs service-backed fallback to current live targets.
- Public ingress, dashboard auth, and Guacamole auth can each fail
  independently. Readiness must distinguish them.
- Readiness failure validation should avoid stopping shared provider services
  unless the operator explicitly approves that live failure mode test.
- Synthetic sentinel pages prove routing and viewer behavior. A later smoke can
  re-run the same route against a real operator workspace such as the UPS
  session, but private or task-specific pages should not be the default test
  fixture.
