# RDP Browser Determinism Audit

Date: 2026-06-22

## Summary

The Facebook dogfood failure was not one bug. It exposed that the current
remote-view stack does not have one deterministic owner for the transaction
"open this URL in an operator-visible RDP browser." The implementation has
useful pieces, but they are still wired as independent recovery surfaces:
profile selection, route-pool selection, display allocation, Guacamole route
readiness, browser launch, tab acquisition, visible-window proof, incident
state, and dashboard stream selection can all drift from each other.

The product shape should be a single route-bound browser acquisition algorithm.
All caller-facing one-liners and service requests should enter that algorithm.
Direct remote-headed launches, daemon CDP streams, stale retained browsers, and
foreign CDP browsers should be represented as separate inventory classes, not
competing interpretations of the same operator handoff.

## Current Evidence

- `docs/dev/notes/2026-06-22-facebook-remote-view-open-friction.md` records that
  the intended one-liner devolved into direct remote-headed launch attempts,
  profile locks, route-pool allocation errors, and only later a successful
  route-bound default-session command.
- `handle_remote_view_open` currently performs a multi-step sequence:
  route binding, launch, tab open, focus, visible-window proof, then route
  checkout. Failures can leave partial process, tab, route, display, or
  incident state behind.
- `service_remote_view_route_binding_from_state` selects a route binding through
  a permissive fallback ladder: inline route entry, explicit IDs, existing
  browser display allocation, checked-out route state, route-pool entry, and
  available route-pool entry. That is helpful for repair, but too permissive
  for the default one-liner because stale browser display state can outrank a
  clean route-pool acquisition.
- `remote_view_open_launch_command` removes `provider` before launching and
  writes `viewStreamProvider` from the route binding. This is correct inside
  the service action, but the CLI language is still confusing because global
  `--provider` also means cloud CDP provider in the main command path.
- `visible_browser_window_proof` accepts any display content whose state is
  `browser_window_visible`. It does not prove the browser window is topmost,
  focused, the selected tab, non-blank, non-terminal-obscured, or visible
  through the actual Guacamole client.
- The live route-handoff audit currently reports one browser
  `session:default`, route `guacamole:5`, display `:14`, and seven retained
  Facebook tabs. That is healthier than the terminal-only failure, but repeated
  open attempts are accumulating tabs instead of acquiring or refreshing a
  single intended handle.
- The full `agent-browser doctor remote-view --json` check exceeded 90 seconds
  during this audit and was terminated. A launch path cannot depend on a slow
  full doctor for its critical path.
- The dashboard code can synthesize daemon-session CDP streams from a selected
  session while also consuming service-owned streams. This is useful for
  inventory, but dangerous when a route-bound Guacamole browser and a daemon
  CDP stream both claim the same session-shaped identity.
- Existing tests already encode parts of the desired behavior:
  `scripts/test-route-handoff-audit.js` classifies route-bound ready,
  terminal-only, proof-missing, direct remote-headed, stale, and foreign CDP
  rows. Dashboard tests also expect stale target recovery and terminal-route
  blocking. Those classifications are not yet the live orchestration contract.

## Root Causes

### 1. There is no single canonical acquisition transaction

The current system has many almost-correct entrypoints:

- direct `open` with remote-headed options;
- `remote-view open`;
- queued `remote_view_open`;
- dashboard view/control actions;
- daemon CDP stream attachment;
- route checkout and route repair actions.

They share data structures, but not a single deterministic state machine. That
lets an agent create a profile lock or route/display mismatch before it reaches
the operator-visible path.

### 2. Provider semantics are overloaded

`--provider rdp_gateway` reads naturally as "use the RDP gateway stream," but
the global CLI provider flag is also the cloud CDP provider lane. The observed
`Unknown provider 'rdp_gateway'` error is a symptom of this naming collision.
The user-facing one-liner should not require knowing whether a flag is parsed
as browser-provider or view-stream-provider at that call site.

### 3. Route binding is repair-friendly, not intent-strict

The route binding resolver accepts existing browser display allocation before
available route-pool selection. For repair tooling this is reasonable. For
"open URL in a route-bound RDP browser," it is unsafe unless the existing
browser, display allocation, route record, route-pool entry, profile identity,
session identity, and visible proof all match the requested intent.

### 4. Visible proof is too weak

The display probe can say `browser_window_visible` when a terminal is still the
operator-visible surface, or when the browser window exists but is not the
window the Guacamole user actually sees. The proof must bind CDP target,
display window, focus/top-level stacking, and Guacamole route evidence.

### 5. The route desktop starts non-browser UI

XRDP/Openbox route desktops can expose XTerm. That is acceptable for a generic
desktop, but not for an agent-browser control route. Route users should start a
minimal browser-only session or the launch path must remove or background
bootstrap UI before claiming readiness.

### 6. The dashboard live surface mixes inventory classes

The left rail currently has concepts that should not compete:

- service-owned controllable browsers;
- detected non-owned CDP browsers;
- daemon/session streams;
- viewer clients;
- stale retained records;
- incidents and "needs attention" rows.

Only live controllable surfaces belong in the primary control rail. Foreign
CDP browsers can be addressable, but must be explicitly grouped as non-owned.
Stale retained rows and incident-only rows belong in logs or diagnostics.

### 7. Foreign browser discovery is incomplete

The desired "non-owned but addressable" group requires systematic discovery:
scan local browser processes, inspect listening DevTools ports, read `/json` and
`/json/version`, classify ownership by profile path, executable, parent
process, service state, and explicit adoption records. Auracall and
im-receipts windows are examples of browsers that should appear in this group
when CDP is reachable and policy permits non-mutating inspection.

### 8. Incident and stale-tab recovery are not authoritative enough

The service resolve path recorded resolution metadata while leaving a stale
incident active in runtime state. Dashboard stale target recovery exists, but
the URL can still carry a stale `tab=target:*` that misleads the user into a
dead or wrong surface. The service should canonicalize or reject stale target
params before rendering the control viewport.

### 9. Full doctor is too slow for the happy path

`doctor remote-view` is valuable, but too broad for the critical path. The
one-liner needs a fast cached route-preflight contract with typed freshness
timestamps. The full doctor should be invoked for repair and diagnosis, not
every launch decision.

## Deterministic Algorithm

The product should implement one path for "open URL in operator-visible RDP
browser":

1. Normalize intent.
   - Inputs: URL, service name, agent name, task name, desired profile identity,
     browser build, session hint, and whether a fresh tab or compatible tab
     reuse is required.
   - Output: a typed `RemoteViewOpenIntent`.
   - Reject ambiguous provider fields. `rdp_gateway` belongs to
     `viewStreamProvider`, never cloud `provider`.

2. Resolve profile and browser reuse.
   - If a compatible service-owned browser already holds the requested profile,
     reuse it only if browser host, browser build, display allocation, route,
     stream provider, and control input match.
   - If not compatible, fail with a specific reuse conflict or launch a new
     route-bound browser according to the profile policy.
   - Never start a direct remote-headed browser as fallback for an
     operator-visible request.

3. Select a route-pool entry.
   - Prefer an explicitly requested route only when it is current and matches
     the requested owner.
   - Otherwise select an available route-pool entry whose Guacamole connection,
     route user, display name, credentials, and public/local URLs are fresh.
   - Treat retained browser display allocation as reusable only after proving
     the route-pool entry, remote-view route, display allocation, browser ID,
     session ID, and profile ID all agree.

4. Acquire display and route atomically.
   - Reserve route-pool entry and display allocation in a pending state.
   - Grant display access if needed.
   - Clean or verify the route desktop before launch. XTerm-only or
     terminal-topmost desktops are not acceptable pending states.

5. Launch or attach browser on the exact display.
   - Launch patched Chromium with the selected runtime profile and exact
     display.
   - Record PID, CDP endpoint, browser build, profile ID, display allocation,
     and route ID in the same service transaction.
   - If attach/reuse is selected, verify the existing process is service-owned
     and route-bound before use.

6. Acquire the target tab.
   - Reuse a compatible existing tab when the request asks for reuse.
   - Otherwise open one tab and mark prior duplicate tabs as stale or close
     them according to policy.
   - Return a current target ID. Reject stale target IDs in dashboard URLs or
     replace them with a current compatible tab and mark the recovery.

7. Prove operator visibility.
   - CDP proof: selected target ID has expected URL/title readiness.
   - X11 proof: browser window for that process is mapped, non-minimized,
     focused or topmost, and not obscured by terminal windows.
   - Route proof: remote-view route record, route-pool entry, display
     allocation, and browser row all agree.
   - Guacamole proof: route URL is routable and, where practical, the client
     frame/popup rendered a non-terminal browser image.
   - Only then set `operatorVisible.state=ready`.

8. Persist canonical state.
   - Write browser, tab, route, route-pool entry, display allocation, stream,
     lease, and operator-visible proof together.
   - On failure, roll back pending reservations and close partial browser/tab
     artifacts unless explicitly preserving diagnostics.

9. Render dashboard from canonical state.
   - Primary rail groups:
     - Service-owned controllable browsers.
     - Non-owned addressable CDP browsers.
   - Logs/diagnostics, not primary rail:
     - stale retained browsers;
     - inactive browsers;
     - resolved incidents;
     - viewer clients;
     - "needs attention" rows without operator action.
   - Daemon CDP streams may be shown as detected/non-owned inventory unless
     they are backed by a service-owned browser record.

10. Expose repair as explicit actions.
    - Route refresh, viewer reconnect, controller takeover, route-pool repair,
      browser close, and foreign-browser adoption should remain explicit
      service actions with dry-run/readiness output.
    - The one-liner should not require manual JSON edits, Guacamole DB edits,
      XTerm killing, or route-pool hand selection.

## Audit Items For The Next Implementation Slice

1. CLI parser harmonization.
   - Make the documented `remote-view open ... --provider rdp_gateway` either
     work as a view-stream-provider alias within that subcommand or replace it
     everywhere with `--view-stream-provider rdp_gateway`.
   - Add regression coverage for flag position before and after
     `remote-view open`.

2. Route-bound acquisition strict mode.
   - Add an intent-strict resolver for `remote_view_open`.
   - Existing display allocation reuse must require owner/profile/route/display
     agreement and fresh proof.
   - Repair-friendly fallback behavior should move behind explicit repair or
     reuse modes.

3. Browser-only route desktop.
   - Change route-user session bootstrap so XTerm is not started for
     agent-browser RDP routes, or ensure it is never mapped/topmost before
     readiness.
   - Add a live or fixture test that terminal-topmost display proof fails.

4. Stronger visible proof.
   - Extend display proof beyond "some browser window exists."
   - Include topmost/focus, selected target correlation, and terminal-obscured
     detection.

5. Fast route preflight.
   - Add a no-launch cached preflight used by the one-liner.
   - Keep full `doctor remote-view` for deep diagnostics and repair, not the
     critical path.

6. Dashboard rail cleanup.
   - Remove inactive/stale/"needs attention without action" rows from the live
     control rail.
   - Add a distinct "Non-owned addressable browsers" group for detected CDP
     browsers.
   - Keep viewer clients and stale retained records in diagnostics/log views.

7. Foreign CDP discovery.
   - Implement a read-only scanner for local browser processes and DevTools
     endpoints.
   - Classify ownership and capabilities before offering inspect/mutate
     actions.
   - Allow adoption only through an explicit service action.

8. Stale target and duplicate-tab handling.
   - Strip or recover stale `tab=target:*` URL params before rendering control.
   - Add tab acquisition policy to avoid repeated Facebook opens accumulating
     duplicate tabs.

9. Incident resolver correctness.
   - Fix incident resolution so resolved/recovered incidents do not continue to
     appear as active left-rail work.
   - Add a regression around `service resolve` and dashboard grouping.

10. State durability.
    - Replace manual service-state edits with service actions for route-pool
      cleanup and incident correction.
    - Harden Guacamole Postgres setup and checkpoint behavior against WSL hard
      stops, then test cold restart recovery.

## Recommended Next Plan

Create a bounded plan focused on deterministic route-bound acquisition:

- parser alias or docs correction for `remote-view open` provider semantics;
- strict route-binding resolver for operator-visible opens;
- browser-only route desktop bootstrap;
- stronger visible proof fixture tests;
- dashboard grouping cleanup for stale, incident, and foreign CDP rows;
- one live smoke that cold-starts Facebook and proves the dashboard URL without
  a stale `tab` param renders the RDP browser, not a terminal.
