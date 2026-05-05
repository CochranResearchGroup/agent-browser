# Service Roadmap Alignment Checkpoint

Date: 2026-05-05

## Purpose

This note pauses the recent service-request guard work and realigns the repo
with the agent-browser service roadmap.

The recent work was useful, but it was contract hardening, not a new roadmap
pillar. The service roadmap still points toward an always-available browser
control plane where agent-browser owns browsers, profiles, tabs, queues,
leases, health, policy, and traceable state for both MCP agents and software
API clients.

## Current Roadmap Alignment

The Phase 0 service model is now materially stronger than the original
roadmap notes assumed:

- The control-plane worker and queue path exists and has cancellation,
  timeout, profile-lease wait, and service job state coverage.
- Browser health, recovery, shutdown remedies, incidents, and operator
  escalation are represented in service-owned state.
- Profile lease contention is represented as queue state, job state, events,
  trace summaries, status, HTTP, MCP resources, generated clients, and
  dashboard consumers.
- Profile allocation and profile readiness are backend-owned views rather than
  dashboard-local inference.
- Profile selection prefers authenticated target identity, then target match,
  then service allow-list, with match details exposed to software clients.
- HTTP, MCP, CLI, docs, generated clients, and the installed skill now share
  static guards around service request action drift.
- Service-owned records already exist for site policies, providers, and
  challenges, but those are still mostly data and API surfaces rather than a
  full decision engine.

## Recent Guard Work

The recent commits closed an important contract-drift risk:

- `42692ab` guards native `service_*` actions against browser launch and
  profile-lease-gate drift.
- `15f1301` keeps Rust `SERVICE_REQUEST_ACTIONS` aligned with the service
  request JSON schema enum.
- `3d370c5` proves MCP `service_request` exposes and accepts every service
  request action.
- `35ca0a5` proves HTTP `/api/service/request` accepts every service request
  action and preserves generated request IDs.
- `9cfadfe` documents the schema, MCP, HTTP, Rust, generated-client, and skill
  alignment rule.

This supports the roadmap because agents use MCP and software projects use the
HTTP/API client path. The same browser intent must therefore mean the same
thing across Rust, schema, MCP, HTTP, and generated software-client code.

## What Is Complete Enough To Pause

These lanes should be treated as closed for now unless a regression appears:

- service request action parity across Rust, schema, MCP, HTTP, generated
  clients, docs, and installed skill
- profile lease wait observability
- profile allocation API and dashboard detail consumption
- basic profile readiness and Google manual-seeding guidance
- shutdown remedy classification for degraded browser versus possible OS
  degradation
- incident summary and operator remedy contract parity

Further polishing these lanes without a specific defect would delay the
backend-first roadmap.

## Remaining Product Gaps

The next missing layer is not another parity guard or dashboard panel. The
remaining gap is service-owned policy and readiness authority.

Important gaps:

- Profile readiness can indicate `needs_manual_seeding` and fresh authenticated
  target state, but it does not yet run recurring identity freshness probes.
- Site policies, providers, and challenges exist as service records and
  collection APIs, but there is not yet an access-policy planner that chooses
  headed mode, pacing, interaction mode, auth providers, or challenge handling
  from those records.
- Provider records do not yet drive concrete 2FA, credential, passkey,
  captcha, or manual approval workflows.
- Challenge records are not yet connected to detection and resolution
  lifecycle decisions.
- Remote headed browser viewing and control are roadmap pillars but should
  wait until the service-owned policy and readiness model is more useful.

## Recommended Next Slice

Return to backend-first service authority with a narrow access-policy and
profile-readiness slice:

1. Define the service-owned decision model that combines `SitePolicy`,
   `ServiceProvider`, `Challenge`, profile readiness, and target identity.
2. Add a no-launch planning surface that explains why a request should use a
   given profile, browser mode, interaction mode, auth provider, challenge
   provider, or manual action.
3. Start with Google-style manual seeding and authenticated target freshness,
   because that is already grounded in live browser evidence and current
   profile readiness fields.
4. Validate through no-launch HTTP and MCP reads first, then add one bounded
   live smoke only when the planner affects real browser launch selection.

This keeps anti-anti-bot hardening, 2FA, captcha, passkeys, and remote headed
browser management grounded in authoritative service state instead of pushing
more policy into agents, dashboard components, or per-client recipes.
