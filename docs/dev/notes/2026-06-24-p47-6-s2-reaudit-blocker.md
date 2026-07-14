# P47.6 S2 Re-Audit Blocker

Date: 2026-06-24
State: locked

P47.6 ran the clean no-mutation preflight and unlocked exactly one S2 retry.
The retry artifact is:

`/tmp/agent-browser-p47-6-s2-retry-2026-06-24`

The retry proved the P47 viewer-client separation and UX path:

- two external dashboard viewer clients opened;
- both rendered the same route iframe;
- operator B refresh clicked successfully;
- operator A navigation changed the controlled browser URL to
  `https://www.iana.org/domains/reserved`;
- route display `:13` showed Chromium browser windows;
- dashboard and route-display screenshots were captured;
- reset-after returned runtime state to zero active incidents.

The retry still failed because service state reported one active incident before
reset-after:

`remote-view-route:guacamole:3`

Message:

`Remote route 'guacamole:3' is orphaned: orphaned display_allocation display_allocation_unavailable`

Related row:

`remote-view-route-pool:guacamole-rdp-a` remained pending
`remote_view_open_acquisition`.

P46 remains locked. The next remediation should focus on route/display
allocation finalization or reconciliation drift after a functional
operator-visible S2 run, not on dashboard viewer clients consuming route leases.
