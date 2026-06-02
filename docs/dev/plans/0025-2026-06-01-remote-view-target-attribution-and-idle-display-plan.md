# Remote View Target Attribution And Idle Display Plan

Date: 2026-06-01
State: IN PROGRESS
Lane: P12-M
Parent Plan: `docs/dev/plans/0018-2026-06-01-workspace-inspector-tabs-productization-plan.md`
Depends On:
- `docs/dev/plans/0002-2026-05-27-guac-remote-view-routing-hardening-plan.md`
- `docs/dev/plans/0003-2026-05-28-guac-rdp-many-to-many-viewing-plan.md`
- `docs/dev/plans/0013-2026-05-31-selected-workspace-context-plan.md`
- `docs/dev/plans/0016-2026-05-31-effective-stealth-remote-default-launch-plan.md`

## Purpose

Fix the remote-view UX bug where the dashboard presents PID/CDP/stream facts
for a local Guacamole viewer browser as though it were the remote browser
inside the RDP desktop.

The operator symptom was:

```text
PID 93182
RSS unknown
CPU unknown
CDP 38151
Stream 37273
```

but the remote view showed only a Linux terminal. Live inspection found that
PID `93182` is session `dashboard-viewer-plan0016`, using
`/tmp/agent-browser-dashboard-viewer-profile`. It is a browser viewing the
dashboard or Guacamole route, not the target browser that should be visible
inside the RDP display. The route-pool displays `:11` and `:12` only had the
default `xterm` windows, so Guacamole was faithfully rendering an idle remote
desktop.

## Current Evidence

Commands run on 2026-06-01:

```bash
ps -fp 93182
agent-browser service status --json
agent-browser service browsers --json
agent-browser doctor remote-view --json
for d in :10 :11 :12 :13; do DISPLAY=$d xwininfo -root -tree; done
agent-browser --session dashboard-viewer-plan0016 get browser-pid --json
```

Findings:

- `agent-browser service status` reported `436` retained browser records and
  `0` active service browsers.
- Service browser lookup did not find an active browser for PID `93182` or
  stream `37273`.
- PID `93182` is still a live Chromium child of an old
  `agent-browser` process for `dashboard-viewer-plan0016`.
- Route-pool displays are ready, but display `:11` and `:12` contain only
  `xterm` windows:
  - `agent-browser-rdp-b@cooper: ~`
  - `agent-browser-rdp-a@cooper: ~`
- `agent-browser doctor remote-view --json` reported route-pool readiness but
  also install drift:
  - `install_path_command_pnpm_binary_mismatch`
  - `install_path_command_workspace_binary_mismatch`

## Product Contract

Remote view is not complete until the dashboard can distinguish four layers:

- dashboard app browser
- Guacamole or RDP viewer client browser
- remote route desktop
- target browser window inside that route desktop

The workspace viewport must not report a remote browser as ready merely because
a viewer client browser has a PID, CDP port, or local stream. A remote-view
workspace is ready only when service state and display evidence agree that the
selected target browser is present in the routed desktop, or the UI clearly
states that the routed desktop is idle.

## Scope

In scope:

- viewer-client versus target-browser classification
- remote route display content probing
- selected workspace diagnostics for idle route desktops
- `view_focus` and `view_takeover` guardrails
- dashboard labels and action availability
- focused live smoke against Guacamole/RDP route-pool displays
- local runtime publish and hosted validation

Out of scope:

- replacing Guacamole/RDP with a new remote backend
- changing dashboard authentication
- broad retained-browser cleanup beyond records needed for this bug
- adding arbitrary remote desktop window management controls
- modifying route-pool user provisioning unless doctor reports it broken

## Implementation Slices

### Slice M1 | Classify Viewer Clients Separately

Goal: prevent browser records like `dashboard-viewer-plan0016` from being
treated as target workspaces.

Tasks:

- Add a deterministic classifier for viewer-client browsers using signals such
  as:
  - URL or tab title for dashboard/Guacamole routes
  - profile paths like `/tmp/agent-browser-dashboard-viewer-profile`
  - service name, task name, or session names containing dashboard viewer or
    Guacamole client roles
  - Guacamole client URLs in active tabs
- Extend `deriveWorkspaceNodes` so viewer-client sessions are labeled as
  `viewer-client` or excluded from target-browser workspace pools.
- Keep viewer clients inspectable in Service or Activity, but do not offer
  target-browser actions like `Control`, `Focus`, or target Console evidence.
- Add a diagnostic when a viewer client is selected:
  “This is the browser viewing the remote route, not the browser inside it.”

Exit criteria:

- PID `93182` style records no longer appear as active target browsers in the
  left workspace pool.
- Selecting a viewer-client record shows viewer diagnostics, not target browser
  controls.
- Existing non-viewer CDP screencast workspaces still resolve as target
  workspaces.

### Slice M2 | Add Route Display Content Probe

Goal: make an idle RDP desktop visible as an explicit state.

Tasks:

- Add a bounded no-secret route-display inspection helper that can report:
  - display name
  - visible top-level window classes and titles
  - whether a Chrome/Chromium target browser window exists
  - whether only the default route-pool terminal exists
- Prefer structured APIs where available. For Linux/X11 route displays, use
  bounded `xwininfo`/window-manager inspection with timeouts and sanitize
  titles before retaining evidence.
- Surface the result in service state or a dashboard proxy as compact
  display-content evidence.
- Avoid polling too frequently; cache evidence with a short timestamp so the
  dashboard does not hammer X displays.

Exit criteria:

- Displays `:11` and `:12` report `idle_terminal_only` when only xterm is
  present.
- A routed Chrome/Chromium target window reports `browser_window_present`.
- Probes time out safely and report `display_probe_unavailable` rather than
  blocking dashboard rendering.

### Slice M3 | Gate Remote View Readiness On Target Evidence

Goal: the viewport readiness state must match the actual routed desktop.

Tasks:

- Extend `workspace-viewport-state` or selected workspace context so
  `rdp_gateway`/Guacamole streams include route-display content readiness.
- If a remote route is reachable but only terminal windows are present, show:
  - status: `idle display`
  - evidence: `route display contains xterm only`
  - next action: `focus or relaunch the target browser into this display`
- If the selected record is a viewer client, show:
  - status: `viewer client`
  - evidence: `browser is viewing Guacamole/dashboard route`
  - next action: `select the target browser workspace or launch one`
- Keep Guacamole network/auth/readiness failures separate from idle display
  evidence.

Exit criteria:

- A route desktop with only xterm does not render as a ready target browser.
- The viewport copy distinguishes provider readiness from target-window
  presence.
- Action buttons are contextual and actionable.

### Slice M4 | Make Focus And Takeover Act On The Target

Goal: `Focus`, `View`, and `Control` must use the target browser identity, not
the viewer client.

Tasks:

- Audit `daemonSessionNameForBrowser`, `view_focus`, and `view_takeover`
  parameter construction for selected viewer-client and service-browser
  records.
- Ensure `view_focus` is sent only to the daemon session that owns the target
  browser window.
- If the selected workspace lacks a target daemon session, disable focus and
  explain the missing target identity.
- Add a recovery action that can relaunch or request a new remote-headed target
  browser into an available route display when the route display is idle.
- Preserve existing viewer lease and controller takeover behavior for real
  target browsers.

Exit criteria:

- `Focus` on a target browser maximizes the target browser in the route
  display.
- `Focus` on a viewer client is disabled with explicit copy.
- `Control` cannot silently bind to a stale viewer browser PID.

### Slice M5 | Retained State And Install Drift Hygiene

Goal: stale retained records should not make the workspace navigator lie.

Tasks:

- Add a retained-browser stale-viewer diagnostic for live processes that are not
  in active service state.
- Review whether `service reconcile` should demote viewer-client records or
  stale dashboard viewer sessions more aggressively.
- Keep destructive cleanup behind existing explicit prune/repair commands.
- Run and document `agent-browser install doctor --json` after publish. If the
  installed binary, workspace binary, and `PATH` command intentionally differ
  during local debug publish, record that as local-runtime debug state rather
  than remote-view route failure.

Exit criteria:

- The workspace navigator does not present retained viewer-client records as
  live remote targets.
- Install drift is visible as an environment warning, not confused with an RDP
  content failure.

### Slice M6 | Live Retest Harness

Goal: prevent regressions with a smoke that fails on the current bug.

Tasks:

- Extend or add a smoke that:
  - opens the hosted dashboard
  - selects a remote-view workspace
  - inspects the route display content
  - fails if the selected workspace claims target-ready while only xterm is
    visible
  - launches or focuses a test target browser into a route display
  - proves the browser window becomes visible through Guacamole/RDP
- Reuse the route-pool readiness command as the preflight:
  `pnpm test:rdp-guac-route-pool-readiness`
- Add JSON evidence fields for:
  - selected workspace id
  - stream provider and route id
  - display name
  - visible window classes/titles summary
  - readiness classification
  - screenshot path

Exit criteria:

- The smoke fails against the current terminal-only route.
- After M1-M5, it passes with one of:
  - clear `idle display` diagnostic for terminal-only route
  - visible target browser window after focus/relaunch
- The smoke closes disposable target browser sessions and releases viewer
  leases.

## Validation Matrix

Source checks:

```bash
pnpm test:dashboard-workspace-nodes
pnpm test:dashboard-workspace-navigator
pnpm test:dashboard-workspace-inspector-tab
pnpm test:dashboard-view-streams
pnpm test:dashboard-inspector-actions
pnpm test:dashboard-launcher-eligibility
pnpm build:dashboard
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
cargo test --manifest-path cli/Cargo.toml view_focus view_takeover remote_view -- --nocapture
git diff --check
node scripts/dev/select-validation.js --base HEAD --json
```

Remote-view checks:

```bash
agent-browser doctor remote-view --json
agent-browser install doctor --json
pnpm test:rdp-guac-route-pool-readiness
pnpm test:service-remote-view-control-live
pnpm test:rdp-guac-many-to-many-live
```

Hosted retest:

```bash
node scripts/smoke-local-dashboard-runtime.js \
  --dashboard-url https://agent-browser.ecochran.dyndns.org/ \
  --workspace-session remote-view-target-attribution-qa \
  --browser-profile /tmp/agent-browser-remote-view-target-attribution-qa \
  --json
```

Add the dedicated M6 smoke and make it the authoritative hosted retest before
closing this plan.

## Progress Notes

2026-06-02:

- Implemented `WorkspaceNode.role` with `target-browser` and `viewer-client`
  values. Viewer clients remain inspectable but target actions are disabled
  with explicit diagnostic copy.
- Added viewer-client classification for dashboard viewer sessions, Guacamole
  client pages, and recursive Agent Browser control pages based on ids,
  ownership, and active tab evidence. RDP stream URLs are not classification
  evidence because valid remote targets also expose Guacamole stream URLs.
- Added `idle-route-display` diagnostics from stream readiness or
  `displayContent` evidence. Terminal-only route displays move the workspace
  row to attention while preserving recovery controls.
- Extended `scripts/inspect-rdp-route-displays.js --windows` to report route
  display windows and classify displays as `terminal_only`,
  `browser_window_visible`, `non_browser_windows`, `empty_display`, or
  `probe_failed`.
- Published the local dashboard runtime after rebuilding the dashboard and CLI.
  The normal browser smoke was blocked by a locked `stealthcdp-default`
  profile, so the runtime publish was rerun with `--skip-browser`; the
  dashboard service restarted as PID `32748`.

Validation completed:

```bash
pnpm test:dashboard-workspace-nodes
pnpm test:dashboard-selected-workspace-context
pnpm build:dashboard
pnpm publish:local-dashboard -- --skip-browser
pnpm inspect:rdp-route-displays -- --windows
AGENT_BROWSER_REMOTE_HEADED_DISPLAY=:12 agent-browser --session plan0025-route-target-qa --profile /tmp/agent-browser-plan0025-route-target-qa --browser-host remote_headed --view-stream-provider rdp_gateway --control-input-provider manual_attached_desktop --display-isolation shared_display open https://example.com --json
agent-browser --session plan0025-route-target-qa close --json
node scripts/smoke-local-dashboard-runtime.js --dashboard-url https://agent-browser.ecochran.dyndns.org/ --agent-browser-bin /home/ecochran76/.local/bin/agent-browser --skip-browser --json
systemctl --user show agent-browser-dashboard.service --property=ActiveState --property=MainPID --property=ActiveEnterTimestamp
```

Live display evidence after the first diagnostic slice:

- Route A: display `:12`, state `terminal_only`, visible windows are Openbox
  and `agent-browser-rdp-a@cooper: ~` xterm.
- Route B: display `:11`, state `terminal_only`, visible windows are Openbox
  and `agent-browser-rdp-b@cooper: ~` xterm.
- Disposable target proof: launching `plan0025-route-target-qa` into display
  `:12` changed Route A to `browser_window_visible` with
  `Example Domain - Chromium`, then closing that session returned Route A to
  `terminal_only`.
- Hosted smoke passed for `https://agent-browser.ecochran.dyndns.org/` with
  `11870` HTML bytes, title `agent-browser`, and `12` static chunks. Browser
  launch was skipped to avoid the known locked `stealthcdp-default` profile.

## Closeout Requirements

- The plan may close only after hosted UX no longer presents a Guacamole viewer
  browser as a target browser.
- A terminal-only route display must produce an explicit idle-display
  diagnostic with an actionable next step.
- A target remote-headed browser must be visible through the route display
  after focus or relaunch.
- Disposable QA sessions must be closed and route leases released.
- The local dashboard runtime must be published and the hosted URL retested.

## Risks

- X11 display inspection can hang if not bounded. Every probe must have a
  timeout and fall back to `display_probe_unavailable`.
- Window titles may contain sensitive user data. Store compact class/title
  summaries and avoid raw full-title logs unless the operator explicitly asks
  for deep debugging.
- Shared route displays can legitimately contain another operator's window.
  Diagnostics should describe contention rather than stealing focus.
- Install drift can obscure validation. Keep source, installed runtime, and
  hosted proof separate in closeout.
