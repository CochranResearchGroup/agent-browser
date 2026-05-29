# RDP Guac Viewer Transfer Live Harness

Date: 2026-05-26
State: PASSED
Plan: `docs/dev/plans/0001-2026-05-26-rdp-guac-hardening-test-plan.md`

## Scope

Slice B now has an opt-in live harness:

- `scripts/test-rdp-guac-viewer-transfer-live.js`
- `pnpm test:rdp-guac-viewer-transfer-live`

The harness is intentionally guarded. It refuses to run without a real
Guacamole or RDP gateway URL, a shared remote display, and two different local
browser executables for the dashboard clients.

## Required Inputs

- `AGENT_BROWSER_REMOTE_VIEW_URL`
- `AGENT_BROWSER_REMOTE_HEADED_DISPLAY`
- `AGENT_BROWSER_RDP_TEST_CLIENT_A_EXECUTABLE`
- `AGENT_BROWSER_RDP_TEST_CLIENT_B_EXECUTABLE`

Optional inputs:

- `AGENT_BROWSER_RDP_TEST_BROWSER_A`
- `AGENT_BROWSER_RDP_TEST_PROFILE_A`
- `AGENT_BROWSER_RDP_TEST_PROFILE_B`
- `AGENT_BROWSER_RDP_TEST_PUBLIC_URL`
- `AGENT_BROWSER_RDP_TEST_DISPLAY_ISOLATION`

## Evidence Captured By A Successful Run

The harness creates `/tmp/agent-browser-rdp-guac-hardening-<timestamp>/` and
writes:

- fixture metadata
- launch response
- service status before client 1, after client 2 opens, after client 1
  takeover, after each refresh, and at the end
- dashboard state from both clients
- desktop screenshots for both clients
- a mobile-size screenshot for the Take over and interaction-settings surface
- the external-open `view_takeover` job
- a summary naming the observed outcome

The run passes only when the same workspace opens in both clients, the behavior
classifies as simultaneous viewing or deterministic takeover, the external-open
path queues `view_takeover`, both clients recover after refresh, and the managed
browser remains `ready`.

## Current Validation Status

The live harness passed and produced a successful two-client RDP and Guacamole
provider result. Slice B live evidence is recorded in
`docs/dev/notes/2026-05-26-rdp-guac-slice-b-live-validation.md`.

Confirmed prerequisites from the attempt:

- `pnpm test:rdp-gateway-readiness-live -- --require-html5-client` passed.
- `guacd`, `xrdp`, and `xrdp-sesman` were active.
- The local HTML5 client URL was reachable and redirected.
- An active xrdp-backed `:10` display was present for `agent-browser-rdp`.
- Distinct local dashboard clients were available through Google Chrome and
  Brave.

Resolved harness issues from earlier attempts:

- The first run timed out while discovering or enabling the stream port during
  cold startup. The helper timeout was widened for this live harness path.
- The second run launched the remote-headed browser and wrote the launch,
  fixture, and before-client service-state artifacts, then reached the
  dashboard Superuser login screen instead of the workspace viewport.
- The next auth preflight attempted a direct Node-side
  `/api/dashboard-auth/status` request, which returned the dashboard HTML
  fallback instead of auth JSON.
- A later run showed that the inherited workstation
  `AGENT_BROWSER_DASHBOARD_AUTH_FILE` prevented isolated bootstrap credentials
  from being generated. The harness now pins dashboard auth to its temp
  `AGENT_BROWSER_HOME`.
- Cross-origin Guacamole iframe inspection was being misclassified as a browser
  error. The dashboard now treats cross-origin opacity as expected.

Passing run:

- Command: `pnpm test:rdp-guac-viewer-transfer-live` with
  `AGENT_BROWSER_REMOTE_HEADED_DISPLAY=:10`,
  `AGENT_BROWSER_RDP_TEST_CLIENT_A_EXECUTABLE=/usr/bin/google-chrome`, and
  `AGENT_BROWSER_RDP_TEST_CLIENT_B_EXECUTABLE=/usr/bin/brave-browser`.
- Outcome: `simultaneous_view`.
- Artifact directory:
  `/tmp/agent-browser-rdp-guac-hardening-2026-05-26T14-32-50-612Z`.
