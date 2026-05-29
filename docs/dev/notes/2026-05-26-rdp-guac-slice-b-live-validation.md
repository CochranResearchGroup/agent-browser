# RDP Guac Slice B Live Validation

Date: 2026-05-26
State: VALIDATED
Plan: `docs/dev/plans/0001-2026-05-26-rdp-guac-hardening-test-plan.md`

## Scope

This note records the Slice B live checkpoint for two active dashboard viewers
on the RDP and Guacamole path. It validates the service-owned `view_takeover`
source checkpoint against the live local RDP, xrdp, guacd, Guacamole, and
dashboard-client environment.

## Environment

- Local time: 2026-05-26 09:40:50 CDT.
- `AGENT_BROWSER_REMOTE_VIEW_PROVIDER`: `rdp_gateway`.
- `AGENT_BROWSER_REMOTE_VIEW_URL`: redacted public Guacamole client route.
- `AGENT_BROWSER_REMOTE_HEADED_DISPLAY`: `:10`.
- Dashboard client A: `/usr/bin/google-chrome`.
- Dashboard client B: `/usr/bin/brave-browser`.
- `pnpm test:rdp-gateway-readiness-live -- --require-html5-client`: passed.
- `guacd`, `xrdp`, and `xrdp-sesman`: active.
- xrdp display evidence: `Xorg :10` owned by `agent-browser-rdp`.

## Source Fixes Required By The Live Run

- The per-session stream server now serves `/api/dashboard-auth/status`,
  `/api/dashboard-auth/login`, `/api/dashboard-auth/logout`, and
  `/api/dashboard-auth/verify`, because the embedded dashboard bundle uses the
  same auth gate as the standalone dashboard.
- Live smoke harnesses pin `AGENT_BROWSER_DASHBOARD_AUTH_FILE` to their
  isolated temp `AGENT_BROWSER_HOME` so they do not depend on or mutate the
  workstation dashboard auth store.
- The workspace viewport no longer treats cross-origin Guacamole iframe
  inspection limits as `browser-error`. Cross-origin frame opacity is expected
  for the public Guacamole route.
- The service-dashboard remote-control live smoke now authenticates before UI
  assertions and follows the current Service, Sessions, and Tabs navigation
  markup.

## Live Evidence

Passed:

- `pnpm test:rdp-guac-viewer-transfer-live`
- `pnpm test:service-dashboard-remote-control-ui-live`

Viewer-transfer artifact directory:

- `/tmp/agent-browser-rdp-guac-hardening-2026-05-26T14-32-50-612Z`

Viewer-transfer summary:

- Browser id: `session:rdp-guac-transfer-57602`.
- Service tab id: `target:FCE6CCE9C2BE0449EDFA724212DEDA7F`.
- Display isolation: `shared_display`.
- Outcome: `simultaneous_view`.
- External-open takeover job:
  `http-service-request-view_takeover-1a93421a-4c5e-499a-9aea-9efa3dab217e`.
- Screenshots captured: client 1 connected, client 1 after client 2 open,
  client 1 after takeover, client 1 mobile viewport, client 1 after refresh,
  client 2 connected, client 2 after client 1 takeover, and client 2 after
  refresh.
- Service-state samples captured before client 1, after client 2 opens, after
  client 1 takeover, after client 1 refresh, and after client 2 refresh.

Remote-control UI smoke summary:

- Browser id: `session:dashboard-remote-control-82641`.
- Tab id: `target:06BBCBCD5BB582789E9CEDF7466DB1A5`.
- Browser inspector dialog URL:
  redacted public Guacamole client route.
- Tab dialog URL: redacted public Guacamole client route.
- Browser focus job:
  `http-service-request-view_focus-24544934-6eab-49c3-8a20-6b77207b390c`.
- Tab focus job:
  `http-service-request-view_focus-c0b4e88e-effe-4644-b151-17033a60e884`.

## Result

Slice B is validated for the current RDP and Guacamole deployment. The observed
provider behavior is simultaneous viewing for two active dashboard clients.
External open still queues the service-owned `view_takeover` action, so the
single-active-viewer path remains exercised by the dashboard contract even
though this deployment did not require takeover to keep both clients visible.

## Residual Risk

- The live evidence covers viewer transfer and refresh recovery for one managed
  remote browser on the shared `:10` display. Slice C still needs a separate
  managed browser A/B switching live proof.
- Cross-origin Guacamole disconnect pages cannot be inspected from the dashboard
  iframe. Same-origin or proxy-served Guacamole routes can still expose explicit
  frame failure text to the detector.
