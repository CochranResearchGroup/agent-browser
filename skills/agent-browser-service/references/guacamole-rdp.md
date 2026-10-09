# Native Remote View presentation

Use the configured native Remote View integration when a person needs to see or
control a browser. Agent Browser owns the profile, browser, sessions and tabs;
Remote View owns desktop allocation, authentication and viewer operation.

## Open and reconnect

Supply the task session and session name explicitly. Select its durable profile
when retained authentication is needed; omitted profile selection is disposable.
Request `remote_view_open` for the selected task. Require
`operatorVisible.state=ready`, then share only the returned `handoffUrl`.
Resolve a relative handoff path against the configured public Agent Browser
origin, which serves its resolver. Keep an absolute URL unchanged.

Never share `providerExternalUrl`, a raw Guacamole URL, `routeBinding`, a local
embed, dashboard embed or health URL. Do not construct the link from the provider
control or native viewer hostname. Reconnect by opening the same durable handoff;
a viewer disconnect does not require another browser or another handoff.

## Capacity and foreground work

New independent profiles prefer ready vacant desktops. Background maintenance
replenishes one spare where limits and current resources allow. At the configured
limit, placement wraps around eligible desktops. Same-profile tasks reuse their
browser and keep separate attributed tabs. Serialize physical foreground input
and human control; background tab-targeted automation can continue independently.

Native Remote View owns idle cooldown, fresh viewer/process checks and physical
retirement. Preserve direct-use and foreign resources. Do not release another
viewer's desktop, run broad GC or kill a browser to make a request succeed.

## Diagnose the exact request

Use its typed error and current native owner evidence. Legacy route-pool counts,
XRDP sessions, root-helper readiness or unrelated ownership history cannot prove
that the selected native request is blocked. Genuine current conflicts for the
selected profile or desktop remain blockers. Report the exact affected browser,
presentation or maintenance operation and its safe next action.

Legacy XRDP route-switch and helper procedures apply only when that legacy
provider has been explicitly selected. They are not native handoff prerequisites.
