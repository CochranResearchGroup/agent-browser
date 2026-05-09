# Generic service profile registration contract

Date: 2026-05-09

## Current decision

The default software-client integration path is service-owned and identity-first:

1. The client names `serviceName`, `agentName`, `taskName`, and a target identity such as `loginId`, `siteId`, or `targetServiceId`.
2. The client asks agent-browser for `GET /api/service/access-plan` or `getServiceAccessPlan()`.
3. If no suitable managed profile exists, the client registers one through `registerServiceLoginProfile()` or HTTP `POST /api/service/profiles/<id>`.
4. Recurring managed profiles should also get a retained `profile_readiness` monitor through `upsertServiceProfileReadinessMonitor()`, MCP `service_monitor_upsert`, or HTTP `POST /api/service/monitors/<id>`.
5. The client submits the planned tab request from `decision.serviceRequest` through `requestServiceTab()`, HTTP `POST /api/service/request`, or MCP `service_request`.

Direct `profile`, `runtimeProfile`, or custom profile paths remain override workflows. The normal path should let agent-browser coordinate profile selection, readiness, leases, browser reuse, and queued control requests.

## What changed

- `55ebfb8` added `createServiceProfileReadinessMonitor()` and `upsertServiceProfileReadinessMonitor()` to the service observability client and taught the managed-profile example how to create the retained freshness monitor.
- `751cea8` updated the generic `examples/service-client/service-request-trace.mjs` workflow so non-Canva software clients can pass `--register-profile-id` plus `--register-readiness-monitor`.
- `7678cde` added no-launch HTTP/MCP coverage for the generic contract in `scripts/smoke-service-config.js`: HTTP profile upsert, MCP `profile_readiness` monitor upsert, HTTP access-plan read, and MCP access-plan resource read all agree on the planned `service_request` recipe.

## Validation

Current validation coverage:

- `pnpm test:service-client-example` validates the generic dry-run workflow.
- `pnpm test:service-client-example-live` validates the generic JavaScript client path against an isolated live daemon and browser session.
- `pnpm test:service-config-live` validates the no-launch HTTP/MCP profile registration, monitor registration, and access-plan contract.
- `pnpm test:service-client` validates generated client exports, types, service request helpers, service observability helpers, and managed-profile flow mocks.

## Guidance for future agents

When a downstream software project proposes creating its own runtime profile directly, first steer it to the generic service-owned path above. A bring-your-own profile is still allowed, but it should be an explicit override, not the default integration pattern.
