# Guacamole Route Authority Audit

Date: 2026-05-27
Plan: `docs/dev/plans/0002-2026-05-27-guac-remote-view-routing-hardening-plan.md`
Lane: P02

## Summary

The current Guacamole client hash was used in two production repair paths and
several test or historical references. Production code must not synthesize a
Guacamole client route from that hash. The route authority order for production
code is now:

1. explicit stream route URLs on the service request: `frameUrl`,
   `viewStreamFrameUrl`, `externalUrl`, or `viewStreamExternalUrl`
2. explicit route metadata on the service request: `routeId`,
   `viewStreamRouteId`, `connectionId`, `guacamoleConnectionId`,
   `connectionName`, or `guacamoleConnectionName`
3. `AGENT_BROWSER_REMOTE_VIEW_URL` only when it already contains a Guacamole
   `#/client/` route
4. retained service browser stream state
5. provider discovery, once implemented
6. unknown route status without inventing a connection id

The service may keep `viewStreams[].url` for backward compatibility, but
dashboard routing should prefer service-owned `frameUrl` and `externalUrl`.

## Classified References

Production fallback references removed or replaced:

- `cli/src/native/actions.rs`: replaced Rust-side root repair with
  service-owned `frameUrl` and `externalUrl` metadata population. It no longer
  appends a default Guacamole client route.
- `cli/src/native/stream/dashboard.rs`: replaced dashboard service-status
  response repair with optional `frameUrl` and `externalUrl` population from an
  already-routed configured URL. It no longer appends a default Guacamole
  client route.
- `packages/dashboard/src/components/workspace-remote-viewport.tsx`: removed
  client-side Guacamole root hash repair. The workspace viewport now consumes
  service-owned route URLs.

Fixture references that may keep the hash when explicitly labeled:

- focused Rust tests that verify preservation of a configured Guacamole client
  URL
- dashboard workspace fixture data under `scripts/`

Historical evidence references that should not be rewritten:

- prior dated notes under `docs/dev/notes/`
- Plan 0002 references that describe the removed hardcoded hash as a problem

## Current Source Order Decision

The first supported production source order is explicit service request route
URLs, explicit route metadata, routed `AGENT_BROWSER_REMOTE_VIEW_URL`, retained
state, future provider discovery, then unknown. A bare Guacamole root URL is
not enough to construct a client route.
