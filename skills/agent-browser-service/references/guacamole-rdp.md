# Use Guacamole and RDP presentation

Use this guide when a person must see or control a service-owned browser.

## Decide whether presentation is required

Browser automation and operator presentation are separate capabilities. Do not
reserve a Guacamole route for ordinary CDP work unless the access plan or site
policy selects `rdp_gateway`, `remote_headed`, or manual attached control.

If presentation is required, run the no-launch route preflight or request
`remote_view_open`. Let Agent Browser select the route. Do not choose a route
from process lists or stale dashboard tiles.

## Handle an occupied route pool

When all route entries are checked out:

1. Preserve every browser and profile.
2. Inspect the typed preflight or request result.
3. Reuse the requested browser's current route when available.
4. Wait when another active controller makes every route non-parkable.
5. Request an explicit route switch only when the operator needs the new
   presentation now.

Route switch may park another live browser's presentation while preserving its
browser process and profile. Agent Browser rejects parking when an active
controller lease is protected unless the caller has explicit takeover
authority. Parking can still interrupt an observer, so do not request it as an
automatic workaround for ordinary automation.

Add route capacity only after measured demand shows that more than two
simultaneous non-preemptible operator desktops are required. A one-time
checked-out snapshot is not capacity evidence.

## Share and reconnect a handoff

Share only `handoffUrl`, shaped as `/remote-view/<handoff-id>`, after
`operatorVisible.state=ready`.

Never share:

- `providerExternalUrl`
- a raw Guacamole URL
- a `routeBinding` URL
- `localEmbedUrl`
- `dashboardEmbedUrl`
- `healthUrl`

Reconnect by opening the same durable handoff. Do not launch another browser
because a route, display, or viewer lease changed.

## Distinguish view operations

- `service_remote_view_route_preflight` reads presentation eligibility without
  launching.
- `remote_view_open` launches or reuses a browser and establishes a durable
  presentation.
- `service_remote_view_browser_reattach` restores presentation for a retained
  browser without launching Chrome.
- `service_remote_view_route_switch` moves a specific retained browser to an
  available or parkable route.
- Viewer and controller lease actions manage observation and control. They do
  not own browser lifecycle.
