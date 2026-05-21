# Remote View Control Posture Checkpoint

Date: 2026-05-20

## Context

The product direction is to let operators view and control hidden headed
browsers from the main web UX without making the dashboard infer browser host,
viewing, or input decisions locally.

Hidden headed `stealthcdp_chromium` with an RDP-backed view stream is a preferred
posture for sites where true headless is not equivalent to a human-operated
headed browser, but it must remain policy-selected rather than hard-coded.

## Architecture Decision

Access-plan is the correct authority for pre-launch browser posture. It now
reports:

- `decision.launchPosture.viewStreamProvider`
- `decision.launchPosture.viewStreamProviderSource`
- `decision.launchPosture.controlInputProvider`
- `decision.launchPosture.controlInputProviderSource`

The copied queued service request also carries:

- `params.viewStreamProvider`
- `params.controlInputProvider`

This keeps the dashboard, MCP clients, and software clients aligned with the
service-owned decision before any browser is launched.

Follow-up implementation in the next slice persisted `controlInput` on browser
`viewStreams` entries and taught the dashboard to show the view and input
posture in browser rows, browser details, and the embedded stream dialog.

## UPS Default Policy

The shipped UPS site policy now makes the remote-view posture explicit:

```text
browserBuild: stealthcdp_chromium
browserHost: remote_headed
viewStream: rdp_gateway
controlInput: manual_attached_desktop
```

This matches the 2026-05-17 live finding that true headless stealth Chromium did
not load UPS tracking reliably while headed stealth Chromium did.

## Residual Work

- The dashboard can now read a stable service posture, but it still needs more
  UX work to show view and control capability in a dense browser/tab table.
- The launch path records a view stream for `remote_headed`, but gateway
  readiness and URL availability remain deployment concerns.
- `manual_attached_desktop` is the current input posture for RDP gateway
  control. A future provider vocabulary may add a more precise remote desktop
  input value if needed.

## Recommended Next Step

Build the next dashboard slice against this backend contract: show the selected
view stream provider, input provider, embeddability, and focus/control state on
browser and tab rows before adding more visual polish.
