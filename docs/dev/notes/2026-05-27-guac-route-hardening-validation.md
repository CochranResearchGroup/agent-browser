# Guacamole Route Hardening Validation

Date: 2026-05-27
Plan: `docs/dev/plans/0002-2026-05-27-guac-remote-view-routing-hardening-plan.md`
Lane: P02

## Summary

P02 route authority and takeover hardening passed local source validation and
same-day live RDP/Guacamole validation. The current workstation still uses one
shared Guacamole route, so multi-browser validation is shared-route evidence,
not distinct-route evidence.

## Live Evidence

- RDP gateway readiness: passed with `guacd`, `xrdp`, `xrdp-sesman`, backend
  TCP, and configured HTML5 route ready.
- Viewer transfer: passed with outcome `simultaneous_view`.
  - Artifacts: `/tmp/agent-browser-rdp-guac-hardening-2026-05-27T19-40-36-319Z`
  - Browser: `session:rdp-guac-transfer-38056`
  - Display isolation: `shared_display`
  - External takeover job: `http-service-request-view_takeover-1b7f6cfa-3f87-4497-96f2-2bcd74004c85`
- Browser switch: passed with outcome `simultaneous_view`.
  - Artifacts: `/tmp/agent-browser-rdp-guac-browser-switch-2026-05-27T19-41-29-855Z`
  - Browser A: `session:rdp-guac-switch-a-43161`
  - Browser B: `session:rdp-guac-switch-b-43161`
  - Display isolation: `shared_display`
  - External takeover job: `http-service-request-view_takeover-b7d19b62-a543-4edf-88c5-af67b386b94e`

Both live service-state samples expose service-owned stream metadata with
`frameUrl`, `externalUrl`, `routeSource: service_request`, derived
`routeId`, and derived `connectionId`.

## Remaining Limitations

- The live workstation has one configured Guacamole route. This proves explicit
  shared-route behavior and browser switching, not two distinct Guacamole
  connections.
- Public ingress and Guacamole auth remain provider concerns. Readiness passed
  for the configured route, but this lane does not bypass auth or manage
  provider secrets.
- Private-display allocation remains a later backend-family lane. This
  validation intentionally used `shared_display` because the configured RDP
  gateway views that display.
