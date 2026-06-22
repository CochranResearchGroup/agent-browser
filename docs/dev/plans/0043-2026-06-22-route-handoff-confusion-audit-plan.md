# Route Handoff Confusion Audit Plan

Date: 2026-06-22
State: PLANNED
Lane: P43
Depends On:
- `docs/dev/plans/0039-2026-06-20-remote-control-ready-command-plan.md`
- `docs/dev/plans/0040-2026-06-21-dashboard-binary-harmonization-plan.md`
- `docs/dev/plans/0041-2026-06-22-foreign-cdp-browser-discovery-and-control-plan.md`
- `docs/dev/plans/0042-2026-06-22-runtime-convergence-plan.md`
- `docs/dev/notes/2026-06-22-facebook-remote-view-open-friction.md`

## Purpose

Make a successful remote-view handoff mean that the operator-visible route,
the dashboard left-rail row, the browser target, and the caller's success
criteria all describe the same live browser.

The immediate failure came from the `last30days` Facebook lane. What should
have been a one-line `remote-view open` took a long troubleshooting loop. The
agent eventually returned a Guacamole link and treated the work as successful,
but the dashboard showed terminal-only screens for both a `127.0.0.1` browser
row and a Facebook row. P42 proved binary/runtime convergence. This plan
audits the next layer: route, browser, tab, stream, and visual-proof
convergence.

## Current Evidence

The incident note records these failures:

- A direct remote-headed launch was attempted before the route-bound
  `remote-view open` path. That launched or reused the
  `last30days-facebook` profile without proving that the operator route showed
  the browser.
- The documented one-liner with `remote-view open ... --runtime-profile
  last30days-facebook` after the subcommand did not behave like the caller
  expected. Moving global flags before `remote-view` changed behavior.
- The named-session route-bound command failed with
  `route_pool_unavailable` for
  `display:private_virtual_display:session-last30days-facebook`.
- A later default route-bound command failed while the same profile was locked
  by the direct launch, then succeeded after the direct named session was
  closed.
- The successful response contained route, display, and visible-window proof
  fields, but the dashboard still presented terminal-only views for rows that
  the operator reasonably interpreted as active browser controls.

Live readback on 2026-06-22 added current evidence:

- `agent-browser doctor remote-view --json` reports `success=true`,
  `status=ready`, `runtimeConvergence.status=converged`, and route pool
  readiness. It still recommends the OCR-backed many-to-many gate as the next
  proof.
- `agent-browser service browsers --json` shows `session:default` on profile
  `last30days-facebook`, display `:11`, display allocation
  `remote-view-display:11`, with Facebook tabs and one generic Guacamole view
  stream.
- The same readback shows `session:litscout-ai-smoke-clean` on display `:93`
  with several `127.0.0.1` tabs and its own generic Guacamole view stream.
- Dashboard stream helpers treat `rdp_gateway` streams as embeddable when a
  stream URL exists. They do not prove that the route currently displays the
  target browser window instead of a terminal.

## Root Cause Hypothesis

This is not one bug. It is an orchestration contract gap across several
surfaces:

- **Command posture confusion**: `open` with remote-headed flags and
  `remote-view open` both produce browser-like state, but only the latter is
  supposed to be an operator-visible handoff contract.
- **Flag-placement confusion**: global session/profile flags are easy to put
  after `remote-view open`, where parser behavior and help text do not make
  the consequence obvious enough.
- **Route identity confusion**: route pool entries are keyed by available
  route/display topology, while named sessions can request display allocation
  IDs that no route pool entry can satisfy.
- **Profile ownership confusion**: profile lock errors identify the Chrome PID
  but not the owning agent-browser session, the command that created it, or the
  exact reuse/close remedy.
- **Success-oracle confusion**: CDP URL/title success, Guacamole route
  readiness, display access, and visible-window proof are separate facts. A
  caller can currently treat one of them as task success even when the
  operator-facing row is not proven.
- **Dashboard row confusion**: the left rail binds browser rows to generic
  route streams. It can show an addressable browser row while the stream is
  really a route desktop that may be focused on a terminal or another window.
- **Client handoff confusion**: downstream clients such as `last30days` do not
  have a single machine-checkable field that says, "the operator-visible
  browser for this row is showing this target."

## Vocabulary

- **Route-bound browser**: a browser launched or reused through the
  `remote-view open` contract, with a selected route, display allocation,
  route pool entry, and operator-view proof.
- **Direct remote-headed browser**: a browser launched through the normal
  `open` path with remote-headed display settings. It may be visible
  somewhere, but it is not automatically a route handoff.
- **Operator-visible proof**: evidence that the route stream currently shows a
  browser window for the intended browser target, not merely that CDP answered
  or Guacamole is reachable.
- **Row-bound proof**: operator-visible proof associated with the exact
  dashboard row, browser ID, tab ID, route ID, and display allocation that the
  UI exposes.
- **Terminal-only route**: a route stream whose active visual content is a
  shell or desktop background rather than the intended browser window.

## Operating Invariants

- Infrastructure readiness is not task success.
- CDP target success is not operator handoff success.
- A Guacamole URL is a route to a desktop, not proof that the desktop is
  focused on the intended browser.
- `remote-view open` should be the one-line route-bound handoff path for
  callers that need Guacamole/RDP.
- Direct remote-headed launches must be labeled as direct launches, not
  silently promoted to route-bound handoffs.
- Profile lock errors must name the owning browser/session when the owner is
  detectable.
- Dashboard live rows must not claim controllable browser readiness unless the
  row carries current route-bound proof or clearly marks proof as missing.
- Terminal-only or desktop-only visual states belong in actionable route
  diagnostics, not as successful browser controls.

## Audit Slices

### Slice A: Incident Replay And State Snapshot

Slice progress: done on 2026-06-22.

Goal: produce a no-mutation audit artifact that joins browser, tab, profile,
route, stream, and visual-proof state.

Deliverables:

- Add a read-only route-handoff audit command or script that collects:
  - active service browsers;
  - tabs and service tab handles;
  - route pool entries;
  - display allocations;
  - remote-view routes;
  - viewer leases;
  - active daemon/runtime identity;
  - view stream URLs and route descriptors;
  - latest route/display visible-window proof when available.
- Emit a compact table keyed by `browserId`, `profileId`, `displayName`,
  `displayAllocationId`, `routeId`, `routePoolEntryId`, `tabId`, URL/title,
  stream provider, and proof state.
- Include classification for `route_bound_ready`,
  `route_bound_proof_missing`, `route_bound_terminal_only`,
  `direct_remote_headed`, `foreign_cdp`, and `stale_or_retained`.

Acceptance:

- The audit explains why `session:default` Facebook and
  `session:litscout-ai-smoke-clean` `127.0.0.1` rows can both show generic
  Guacamole streams without proving the row's browser is visible.
- The audit can be attached to future incident notes without exposing cookies,
  auth state, screenshots, or private page contents.

Completed on 2026-06-22:

- Added `scripts/audit-route-handoff.js` and package command
  `pnpm audit:route-handoff`.
- The audit reads `agent-browser service status --json` and, unless
  `--skip-doctor` is supplied, `agent-browser doctor remote-view --json`.
- The output schema is `agent-browser.route-handoff-audit.v1`.
- JSON and text output join browser rows, tabs, display allocations,
  remote-view routes, route-pool entries, viewer leases, runtime convergence,
  stream URLs, route descriptors, and retained visible-window proof.
- Rows are keyed by browser, profile, display, display allocation, route,
  route-pool entry, tab, URL/title, stream provider, proof state, and visual
  state.
- Classification values are `route_bound_ready`,
  `route_bound_proof_missing`, `route_bound_terminal_only`,
  `direct_remote_headed`, `foreign_cdp`, and `stale_or_retained`.
- Added no-launch fixture coverage in `scripts/test-route-handoff-audit.js`
  and package command `pnpm test:route-handoff-audit`.
- Live read-only audit on 2026-06-22 reported Facebook rows as
  `route_bound_ready` on `guacamole:3` with retained
  `browser_window_visible` proof, and LitScout `127.0.0.1` rows as
  `direct_remote_headed` with no ready route binding.

### Slice B: One-Line CLI Contract And Help

Slice progress: done on 2026-06-22.

Goal: make the intended Facebook-style handoff command obvious and hard to
misuse.

Deliverables:

- Give `remote-view open --help` command-specific examples for the supported
  one-liner and flag placement.
- Make global session/profile flag placement either accepted consistently for
  `remote-view open` or rejected with an explicit actionable message.
- Add a preflight warning when a caller mixes direct remote-headed flags with a
  route-bound handoff goal.
- Update README, docs site, CLI help, and skill guidance for the corrected
  command shape.

Acceptance:

- A caller can discover the correct Facebook handoff command from help output
  without reading plan notes.
- Tests cover the documented flag order and the most likely wrong flag order.

Completed on 2026-06-22:

- Added command-specific `agent-browser remote-view open --help` output through
  the `remote-view` help branch.
- Help output now includes the Facebook-style one-liner, route-pool dry-run
  example, and global flag-placement guidance.
- `parse_remote_view_open` now preserves global `--session-name` into the
  `remote_view_open` command. Command-position `--session-name` still
  overrides the global value when parsed directly.
- Added parser tests proving `--runtime-profile`, `--session-name`,
  `--session`, `--browser-build`, and `--provider` work when supplied after
  `remote-view open`.
- Updated `README.md`, `docs/src/app/commands/page.mdx`, and
  `skills/agent-browser/SKILL.md` with the one-liner and the distinction
  between `--session` daemon routing and `--session-name` browser state.

### Slice C: Route Allocation Diagnostics

Goal: make route-pool failures explain the available choices instead of
requiring route topology knowledge.

Slice progress: done on 2026-06-22. `route_pool_unavailable` and adjacent
stale route-pool selection failures now preserve their stable error codes and
append compact `diagnostic={...}` JSON with requested route, pool, display,
provider, owner session/browser/profile, matching entries, available entries,
ready display allocations, existing remote-view routes, and recommended
commands. The existing `remote-view open --dry-run` route plan remains the
pre-launch route selection surface.

Deliverables:

- When `route_pool_unavailable` occurs, include the requested display
  allocation, requested session/profile, available route pool entries,
  available display allocation IDs, and recommended command.
- Add a dry-run or preflight mode for `remote-view open` that reports which
  route pool entry would be selected before launching Chrome.
- Prefer route pool entry selection by explicit route entry or available
  route-bound display when the caller's named session does not already have a
  compatible allocation.

Acceptance:

- The named-session `last30days-facebook` failure recommends either the default
  route-bound command or an explicit available route pool entry.
- The error no longer leaves the caller to infer that
  `display:private_virtual_display:session-last30days-facebook` and
  `remote-view-display:11` are incompatible identities.

### Slice D: Profile Lock Ownership And Reuse

Goal: turn profile locks into a repairable ownership handoff.

Deliverables:

- On Chrome profile lock failure, inspect service state and live process
  metadata to map the lock PID to a browser ID, session ID, profile ID, and
  command posture when possible.
- Report exact safe remedies:
  - reuse the route-bound browser if it is compatible;
  - close the owning agent-browser session;
  - wait for a profile lease if a service request owns it;
  - manual review if the owner is foreign or unknown.
- Preserve the current safety posture: do not kill or close foreign processes
  automatically.

Acceptance:

- The Facebook profile-lock failure points at the direct named session that
  owns the profile and suggests the exact close or reuse command.
- Unit coverage protects owned, service-owned, direct remote-headed, and
  unknown-owner lock messages.

### Slice E: Operator-Visible Success Contract

Goal: make `remote-view open` success depend on the same proof the operator
needs.

Deliverables:

- Define `operatorVisible` in the `remote-view open` response with:
  - `state`;
  - browser ID;
  - tab ID;
  - route ID;
  - display allocation ID;
  - display name;
  - proof timestamp;
  - failure reason;
  - recommended repair command.
- Treat terminal-only, desktop-only, no-window, wrong-display, and stale-proof
  states as not successful for route-bound handoff unless the caller explicitly
  opts into infrastructure-only readiness.
- Use existing visible-window and OCR-backed gates where possible instead of
  inventing another visual detector.

Acceptance:

- `remote-view open` cannot report a successful operator handoff when the route
  stream shows only a terminal.
- The response still distinguishes recoverable visual-focus failures from
  browser launch failures.

### Slice F: Dashboard Row Binding And UX

Goal: make the left rail and workspace detail show route proof and ownership
truthfully.

Deliverables:

- Attach route-bound proof metadata to each service-owned browser row.
- Display route/source labels that distinguish:
  - service-owned route-bound browser;
  - direct remote-headed browser;
  - foreign CDP browser;
  - route stream with missing proof;
  - route stream showing terminal-only content.
- Disable or warn on browser-control affordances when the row has no current
  operator-visible proof.
- Keep inactive, retained, and no-action diagnostic records out of the live
  left rail.

Acceptance:

- A Facebook row cannot silently show the generic route desktop as a successful
  browser stream without current row-bound proof.
- A `127.0.0.1` tab from LitScout remains visibly separate from the Facebook
  route and does not imply Facebook success.

### Slice G: Downstream Client Contract

Goal: make clients such as `last30days` wait for the right success signal.

Deliverables:

- Update service-client helpers and examples to require `operatorVisible` for
  route-bound handoff workflows.
- Add a client-side summary helper that logs route, tab, profile, and visual
  proof status in one line.
- Update `last30days` guidance to call only the route-bound one-liner for
  Facebook manual login/search and to reject CDP-only success for Guacamole
  handoff.

Acceptance:

- `last30days` cannot declare Facebook handoff success from CDP URL/title alone
  when the operator view is terminal-only.
- The happy path remains a one-line command plus a short proof summary.

### Slice H: Live Gates

Goal: preserve the fixed behavior with repeatable no-launch and live tests.

Deliverables:

- Add no-launch fixtures for:
  - wrong flag placement;
  - named-session route pool mismatch;
  - profile lock with known owner;
  - direct remote-headed launch before route-bound handoff;
  - dashboard row with stream but missing visual proof.
- Add an OCR-backed live gate that opens a neutral test URL through
  `remote-view open`, verifies the route display shows a browser, and verifies
  the dashboard row reports matching proof.
- Add a live Facebook smoke only as an opt-in downstream dogfood step, not as
  core CI.

Acceptance:

- `pnpm validation:select -- --base <ref>` recommends the focused gates when
  route, dashboard stream, service client, or remote-view command files change.
- The live route gate fails if the route shows a terminal-only screen.

## Initial Audit Questions

- Which CLI paths can currently launch a remote-headed browser that looks like
  a route-bound handoff but is not one?
- Which service state fields are authoritative for browser-to-route binding?
- Where does the dashboard choose a stream for a browser row, and does it know
  whether the stream is row-bound or merely route-bound?
- Can visible-window proof be stored and refreshed without persisting sensitive
  image content?
- Should `remote-view open` default to retaining or reusing a compatible tab
  before it tries to launch a new browser with the same profile?
- What exact success field should downstream agents require before reporting a
  Guacamole/RDP browser link as usable?

## Closeout Criteria

P43 can close when:

- route handoff audit output explains current browser, tab, profile, route,
  stream, and proof state in one read-only command;
- the CLI one-liner is documented, parser-safe, and covered by tests;
- route-pool and profile-lock failures include actionable ownership remedies;
- `remote-view open` exposes row-bound `operatorVisible` proof;
- the dashboard left rail labels missing proof and terminal-only routes instead
  of presenting them as successful browser controls;
- downstream clients have a stable proof field to require before claiming
  handoff success;
- an OCR-backed live route gate fails on terminal-only output and passes on a
  browser-visible route.
