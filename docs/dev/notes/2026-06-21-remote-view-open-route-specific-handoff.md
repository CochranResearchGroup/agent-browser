# Remote View Open Route-Specific Handoff

Date: 2026-06-21
Source Plan: `docs/dev/plans/0039-2026-06-20-remote-control-ready-command-plan.md`

## Summary

Agent-browser now has a generic route-specific remote-control acquisition path
for Guacamole/RDP browser sessions. Downstream clients should prefer
`remote_view_open` or `agent-browser remote-view open` when they need a
controllable browser visible through the agent-browser dashboard or public
Guacamole route.

This handoff does not modify AuraCall. AuraCall and other downstream agents own
their migrations, profile mapping, live-follow policy, and rollout gates.

## Preferred Path

CLI:

```bash
agent-browser remote-view open https://www.linkedin.com/ \
  --runtime-profile stealthcdp-default
```

HTTP or MCP service request:

```json
{
  "serviceName": "ManualAuth",
  "agentName": "codex",
  "taskName": "manual-auth",
  "runtimeProfile": "stealthcdp-default",
  "browserBuild": "stealthcdp_chromium",
  "action": "remote_view_open",
  "url": "https://www.linkedin.com/",
  "params": {
    "provider": "rdp_gateway"
  }
}
```

When a client already copied a route-pool entry from doctor, route-pool
readiness, or access-plan output, pass `routePoolEntryId`, `routePoolEntry`, or
`routePool` in `params`. Do not parse Guacamole hashes, XRDP display numbers,
or Xauthority state in downstream code.

## Route Descriptor Contract

Preserve route descriptor URL roles:

- `localEmbedUrl`: local dashboard or live harness iframe route.
- `dashboardEmbedUrl`: hosted dashboard iframe route when distinct from local.
- `publicOperatorUrl`: external dyndns.org or equivalent operator route.
- `healthUrl`: provider readiness check URL.
- `externalUrl`: backward-compatible operator URL.

Downstream clients should show `publicOperatorUrl` or `externalUrl` to humans
and use the dashboard/local embed URL role only for the dashboard surface that
matches the current origin.

## Profile Sharing Boundary

Several clients may share one authenticated profile only through one retained
browser process group. Use separate service-owned tabs, windows, viewer leases,
or controller leases on that retained browser. Do not start another independent
Chrome process on the same profile directory unless the caller explicitly sets
`allowDuplicateProfileLane: true` for reviewed throwaway isolation.

Access-plan `decision.profileReuse`, `browserId`, and `sessionName` route hints
remain the source of truth for retained-browser reuse. A copied
`remote_view_open` or tab request must preserve those hints when they are
present.

## Required Gates

Before making Guacamole/RDP `chromium-stealthcdp` the default browser-owner
lane in a downstream repo, require current agent-browser evidence:

```bash
agent-browser install doctor --json
agent-browser doctor remote-view --json
pnpm test:remote-view-open-fixture-live
pnpm test:rdp-guac-many-to-many-live
```

The single-route operator path is ready only when
`doctor remote-view --json` reports `remoteControl.status=ready`. A
simultaneous multi-viewer claim also requires `manyToMany.status=ready`.

## Current Proof

Current Plan 0039 route-specific evidence:

- Installed binary SHA:
  `54248451b6bea3ced7acb6df8dd3e0f7514c866e08584bb025569a2ec6ad28ad`.
- `pnpm test:remote-view-open-fixture-live` passed with artifact directory
  `/tmp/agent-browser-remote-view-open-live-2026-06-21T01-24-32-095Z`.
- The fixture proof used route `guacamole:3`, display `:11`, display
  allocation `remote-view-display:11`, external URL
  `https://agent-browser.ecochran.dyndns.org/guacamole/#/client/MwBjAHBvc3RncmVzcWw=`,
  fixture URL `http://127.0.0.1:37525/`, title
  `REMOTE VIEW OPEN FIXTURE 66806`, and X11 browser window `0x800003`
  matching browser PID `48672`.
- `pnpm test:rdp-guac-many-to-many-live` passed with artifact directory
  `/tmp/agent-browser-rdp-guac-many-to-many-2026-06-21T01-24-32-207Z`.
- `agent-browser doctor remote-view --json` reported `status=ready`,
  `remoteControl.status=ready`, `manyToMany.status=ready`, route
  `guacamole:3`, display `:11`, route-display access ready, route displays
  ready, route pool ready, and viewer prerequisites ready.

## AuraCall Adoption Shape

AuraCall should start with a no-mutation access-plan check that asks for:

```text
browserBuild=stealthcdp_chromium
browserHost=remote_headed
viewStreamProvider=rdp_gateway
controlInputProvider=manual_attached_desktop
displayIsolation=private_virtual_display
```

After access-plan preserves that posture and the required agent-browser gates
pass in the target environment, AuraCall can call `remote_view_open` for manual
authentication or live-follow setup. Identity and account detection should stay
generic: agent-browser supplies route-bound browser acquisition, bounded
probes, diagnostics, UI action recipes, network capture, and tab handles;
AuraCall supplies website-specific recipes or instructions for extracting
provider identity from the target site.
