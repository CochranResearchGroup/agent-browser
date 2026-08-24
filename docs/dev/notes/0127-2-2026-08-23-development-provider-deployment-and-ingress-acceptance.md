# Plan 0127 Slice B Development Provider Deployment And Ingress Acceptance

Date: 2026-08-23

Status: ACCEPTED | CAPACITY PROJECTION ACCEPTED

Authority: DEVELOPMENT PROVIDER EFFECTS | PRODUCTION READ-ONLY

## Accepted Evidence

- the explicit apply returned `provider_ready_ingress_pending` and recorded a
  private receipt;
- three isolated containers run under compose project
  `agent-browser-dev-presentation` on loopback ports 8093, 4823, and 55433;
- six exact development route users and six distinct Guacamole connection IDs
  exist;
- four warm route-user Xorg sessions are ready on distinct current displays;
- the required development doctor passes all provider checks;
- raw Guacamole responds on port 8093, while local and external
  `/guacamole/` requests cross the development dashboard-auth boundary;
- external handoff remains on
  `https://agent-browser-dev.ecochran.dyndns.org` without exposing a raw
  provider URL;
- the production snapshot was unchanged across apply.

The provider transaction also proved retry-safe stopped-container and route
user identity checks, protected secrets, explicit operator permission grants,
profile-aware viewer commands, exact quarantine, and a readiness parser that
selects Xorg rather than the first process owned by a route user.

## Retained Findings

Failed pre-acceptance viewer generations remain quarantined as durable owner
history. They exposed that absent-session close can leave an owner without a
terminal lifecycle receipt and that XRDP route-user sessions outlive viewer
cleanup. Plan 0124 GC acceptance must reconcile these exact identities rather
than deleting authority files or restarting shared XRDP.

The provider inventory now enters Service-owned display, route, pool, and
presentation-capacity authority through source commit `c1ca78e3`. The installed
development generation `0.28.0-1d935dbb107f` reports four warm-idle slots, a
six-slot hard maximum, one human reserve, one recovery reserve, and no binding
warnings. Reconciliation refreshes provider authority without restarting the
ready provider or disturbing its warm viewers.

Four-to-six scale-out, provider-backed desktop evidence, and six-to-four GC
remain Plan 0124 live acceptance work. Current Service resource pressure also
reports unowned Agent Browser processes, so that acceptance must take fresh OS
process and memory readbacks and must not infer cleanup authority from absence
in development Service State.
