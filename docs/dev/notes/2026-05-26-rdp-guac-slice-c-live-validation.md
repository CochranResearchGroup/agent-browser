# RDP Guac Slice C Live Validation

Date: 2026-05-26
State: VALIDATED
Plan: `docs/dev/plans/0001-2026-05-26-rdp-guac-hardening-test-plan.md`

## Scope

This note records the Slice C live checkpoint for managed remote browser A/B
switching on the RDP and Guacamole path.

## Environment

- Local time: 2026-05-26 10:10 CDT.
- `AGENT_BROWSER_REMOTE_VIEW_PROVIDER`: `rdp_gateway`.
- `AGENT_BROWSER_REMOTE_VIEW_URL`: redacted public Guacamole client route.
- `AGENT_BROWSER_REMOTE_HEADED_DISPLAY`: `:10`.
- Dashboard client A: `/usr/bin/google-chrome`.
- Dashboard client B: `/usr/bin/brave-browser`.
- `pnpm test:rdp-gateway-readiness-live -- --require-html5-client`: passed.
- `guacd`, `xrdp`, and `xrdp-sesman`: active by readiness and prior Slice B
  service evidence.
- xrdp display evidence: `Xorg :10` owned by `agent-browser-rdp`.

## Source Fixes Required By The Live Run

- Browser A and browser B must launch with distinct runtime profiles. Without
  that, the second Chrome launch can exit cleanly before exposing DevTools
  because it hands work to the existing profile owner.
- Focus and takeover jobs must be read from the selected browser daemon's
  stream server, not only from browser A's dashboard stream server. The
  dashboard can serve one route while `view_focus` is owned by browser B.
- The harness treats `view_focus` as an observed service-owned focus request
  for rapid A/B alternation, while `view_takeover` must still reach
  `succeeded`.

## Live Evidence

Passed:

- `pnpm test:rdp-guac-browser-switch-live`

Command environment:

- `AGENT_BROWSER_REMOTE_HEADED_DISPLAY=:10`
- `AGENT_BROWSER_RDP_TEST_CLIENT_A_EXECUTABLE=/usr/bin/google-chrome`
- `AGENT_BROWSER_RDP_TEST_CLIENT_B_EXECUTABLE=/usr/bin/brave-browser`

Artifact directory:

- `/tmp/agent-browser-rdp-guac-browser-switch-2026-05-26T15-10-16-045Z`

Summary:

- Browser A id: `session:rdp-guac-switch-a-78134`.
- Browser A tab id: `target:01B6792C528C647E8C7415CEEC4532D5`.
- Browser B id: `session:rdp-guac-switch-b-78134`.
- Browser B tab id: `target:36A09370D0BFF0C57954710086F64404`.
- Display isolation: `shared_display`.
- Cross-browser viewer outcome: `simultaneous_view`.
- External-open takeover job:
  `http-service-request-view_takeover-1fbd2fc0-1387-4be2-a03f-b0a8dd4e0d0b`.

Screenshots captured:

- client 1 browser A connected
- client 1 switched to browser B
- client 1 browser B after refresh
- client 1 after client 2 opens browser A
- client 2 browser A connected
- client 1 alternation 1 to A
- client 1 alternation 2 to B
- client 1 alternation 3 to A
- client 1 alternation 4 to B
- client 1 after external open

Service-state artifacts captured:

- after both browser launches
- after client 1 switches to B
- after client 1 refreshes B
- after client 2 opens A
- final retained state

## Result

Slice C is validated for the current RDP and Guacamole deployment. The live run
proved two distinct managed `remote_headed` browser sessions with distinct
runtime profiles, browser A to browser B route switching, browser B refresh
recovery, a second client opening browser A while client 1 stayed routed to
browser B, four A/B alternation screenshots, and external-open `view_takeover`.

## Residual Risk

- One earlier browser B `view_focus` job
  `http-service-request-view_focus-aa9512f5-6084-4700-ad0f-c00858e57bdb`
  remained `running` in the final service-state artifact, even though later
  browser B focus jobs succeeded and the final browser B screenshot was
  captured. Treat this as a retained job lifecycle cleanup risk for Slice D.
- This validation used `shared_display`, so focus and native-window state
  remain shared resources. Private display allocation still belongs to a later
  backend hardening slice.
