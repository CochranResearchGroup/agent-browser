# Guacamole RDP P03 Provider Topology Audit

Date: 2026-05-28
Plan: `docs/dev/plans/0003-2026-05-28-guac-rdp-many-to-many-viewing-plan.md`
Lane: P03
Slice: A

## Summary

The current workstation topology supports the P01/P02 shared-route behavior,
but it does not yet support P03 many-to-many viewing. There is one Guacamole
connection, one configured Guacamole client route, and one host XRDP listener.
That topology can show whichever browser is focused on the shared XRDP
desktop, but it cannot show Browser A and Browser B simultaneously as separate
remote desktops.

The first P03 implementation path should be a static Guacamole route pool backed
by distinct RDP targets. Dynamic Guacamole connection generation can be added
later, but the current deployment has no route-pool or provider-discovery
surface to build on today.

## Evidence

### Readiness

`pnpm test:rdp-gateway-readiness-live -- --require-html5-client` passed.

Readiness result:

- `guacd`: ready
- `xrdp`: ready
- `xrdp_sesman`: ready
- `backend_tcp`: ready
- `guacamole_web_app`: ready
- `public_ingress`: ready
- `dashboard_auth`: unknown in the readiness smoke, validated by browser
  harnesses
- `iframe_embedding`: unknown in the readiness smoke, validated by dashboard
  harnesses

The readiness smoke confirms the shared route is healthy. It does not prove
distinct private-display routing.

### Runtime Config

Observed non-secret environment state:

- `AGENT_BROWSER_REMOTE_VIEW_PROVIDER=rdp_gateway`
- `AGENT_BROWSER_REMOTE_VIEW_URL` is set to the public Guacamole client route
- `AGENT_BROWSER_REMOTE_VIEW_FRAME_URL` is unset
- `AGENT_BROWSER_REMOTE_VIEW_EXTERNAL_URL` is unset
- `AGENT_BROWSER_REMOTE_VIEW_ROUTE_ID` is unset
- `AGENT_BROWSER_GUACAMOLE_CONNECTION_ID` is unset
- `AGENT_BROWSER_GUACAMOLE_CONNECTION_NAME` is unset
- `AGENT_BROWSER_REMOTE_HEADED_DISPLAY` is unset by default

The current service state has no persisted provider records:

```json
{
  "providers": {},
  "sessionsCount": 266,
  "browsersCount": 289
}
```

The user-scoped Guacamole Compose stack is:

- `agent-browser-guacamole`: `guacamole/guacamole:1.5.5`, bound to
  `127.0.0.1:8092`
- `agent-browser-guacd`: `guacamole/guacd:1.5.5`
- `agent-browser-guacamole-postgres`: `postgres:16-alpine`

The host services are:

- `guacd.service`: active, system service
- `xrdp.service`: active, system service
- `xrdp-sesman.service`: active, system service

The Docker stack uses its own `agent-browser-guacd` container. The system
`guacd.service` is also running and reachable, but the Guacamole web container
is configured with `GUACD_HOSTNAME=guacd`, so the web app talks to the Compose
service by default.

### Guacamole Database

The Guacamole database currently contains exactly one connection:

```text
1 | Local XRDP (agent-browser host) | rdp
```

The connection parameters are:

```text
hostname=host.docker.internal
port=3389
username=agent-browser-rdp
security=any
ignore-cert=true
resize-method=display-update
enable-audio-input=false
enable-drive=false
enable-theming=false
enable-wallpaper=false
password=<redacted>
```

Connection counts:

```text
connections=1
groups=0
sharing_profiles=0
active_history=0
```

The current Guacamole route therefore represents one route to the host XRDP
service, not a pool of distinct routes.

### XRDP Configuration

Host XRDP listens on `*:3389`. `xrdp-sesman` listens on `127.0.0.1:3350`.
Relevant `sesman.ini` settings:

- `X11DisplayOffset=10`
- `MaxSessions=50`
- `Policy=Default`
- `KillDisconnected=false`

This means XRDP can create multiple sessions in principle, but the current
Guacamole provider configuration has only one connection entry and no route
identity that binds one browser workspace to one XRDP session or display.

### Current Browser Records

Recent RDP browser records show both topologies:

- private display example: `session:odollo-carrier-ups`
  - `displayIsolation=private_virtual_display`
  - `displayName=:10`
  - stream URL was the Guacamole root, not a distinct client route
- shared display examples: `session:rdp-review-a-20260527-160454` and
  `session:rdp-review-b-20260527-160454`
  - both used `displayIsolation=shared_display`
  - both used `displayName=:0`
  - both used the same Guacamole client route

The private-display browser launch path exists, but private display is not yet
paired with a distinct externally viewable Guacamole route.

## Provider Constraint

The 2026-05-27 manual two-browser test collapsed into focus switching because
both browser records advertised the same Guacamole route. The route targets one
XRDP endpoint on the host. Agent Browser can focus Browser A or Browser B on
that endpoint, but the provider cannot simultaneously show two different
browser desktops until there are distinct route allocations.

The constraint is provider topology, not browser launch capability:

- browser launch can create private displays
- service records can carry route metadata
- the current provider has no route pool
- the current provider has no dynamic connection provisioner
- the current service state has no provider records describing multiple
  Guacamole routes

## First Supported P03 Topology

Use a static Guacamole route pool first.

Recommended first shape:

```text
browser workspace A -> private display A -> isolated RDP target A -> Guacamole route A
browser workspace B -> private display B -> isolated RDP target B -> Guacamole route B
```

The static pool should be declared in user-scoped service provider config, not
hardcoded in production code. Each pool entry should include:

- `routeId`
- `connectionId`
- `connectionName`
- `frameUrl`
- `externalUrl`
- target host and port
- expected display or target identity
- `providerMode`
- readiness state

The route pool can initially point at separate XRDP users, separate XRDP
sessions, or containerized RDP targets. The key P03 requirement is that the
route identity is distinct and service-owned.

## Decision

Slice A is complete for the current workstation:

- current topology: one shared Guacamole route to host XRDP
- first implementation path: static Guacamole route pool
- generated Guacamole connections: later enhancement
- containerized RDP targets: acceptable backend if XRDP per-display routing is
  not reliable enough
- existing shared route: preserve as explicit fallback

## Next Slice

Start Slice B by adding service contracts for:

- display allocations
- remote view route allocations
- route pool entries
- viewer leases

The first contract slice should be no-launch and backward compatible. It should
let clients determine whether two browsers have distinct route ids without
parsing Guacamole URLs.
