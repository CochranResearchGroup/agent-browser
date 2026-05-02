# API Contracts

This directory holds machine-readable schemas for service API records that
software clients and MCP agents are expected to consume directly.

## Service Job Record v1

`service-job-record.v1.schema.json` describes service job records returned by:

- HTTP `GET /api/service/jobs`
- HTTP `GET /api/service/jobs/<id>`
- MCP `agent-browser://jobs`
- service trace job arrays

The schema is guarded by Rust tests for the model, HTTP, and MCP surfaces. Keep
new contractual job fields in this schema before relying on them from external
clients.

`service-jobs-response.v1.schema.json` describes the response envelope returned
by:

- CLI `agent-browser service jobs`
- HTTP `GET /api/service/jobs`
- HTTP `GET /api/service/jobs/<id>`

The schema covers list and detail responses, including the returned job array,
count, matched and total counters, and the detail-only `job` field.

## Service Request v1

`service-request.v1.schema.json` describes the service request intent object
accepted by:

- HTTP `POST /api/service/request`
- MCP `service_request` `arguments`

The schema requires only `action` for compatibility, but callers should include
`serviceName`, `agentName`, and `taskName` when known so retained jobs remain
traceable. Target hints such as `siteId`, `loginId`, and `targetServiceId`
drive profile selection for the requested site or login scope.

`service-request-mcp-tool-call.v1.schema.json` describes the MCP `tools/call`
wrapper for invoking `service_request` with the same intent object.

`packages/client/src/service-request.generated.d.ts` and
`packages/client/src/service-request.generated.js` are generated from these
schemas. Run `pnpm generate:service-client` after changing the schemas and
`pnpm test:service-client-contract` to verify the generated client surface is
current. Run `pnpm test:service-client-types` to type-check the runtime helper
against those declarations.

## Service Incident Record v1

`service-incident-record.v1.schema.json` describes grouped service incident
records returned by:

- HTTP `GET /api/service/incidents`
- HTTP `GET /api/service/incidents/<id>`
- MCP `agent-browser://incidents`
- MCP `service_incidents`
- service trace incident arrays

The schema is guarded by Rust tests for the model, HTTP, MCP, and service trace
surfaces. Keep new contractual incident fields in this schema before relying on
them from external clients.

`service-incidents-response.v1.schema.json` describes the response envelope
returned by:

- CLI `agent-browser service incidents`
- HTTP `GET /api/service/incidents`
- HTTP `GET /api/service/incidents/<id>`
- MCP `service_incidents`

The schema covers list and detail responses, including the returned incident
array, count, matched and total counters, list filters, and detail-only related
events and jobs.

When changing incident summary grouping or filters, run both no-launch guards:
`pnpm test:service-incident-summary-http` and
`pnpm test:service-incident-summary-mcp`. Together they verify that HTTP
`summary=true` and MCP `service_incidents` with `summary: true` preserve the
same grouped remedy contract across state, severity, escalation,
handling-state, browser, profile, session, service, agent, task, and since
filters.

## Service Event Record v1

`service-event-record.v1.schema.json` describes retained service event records
returned by:

- HTTP `GET /api/service/events`
- MCP `agent-browser://events`
- service trace event arrays

The schema is guarded by Rust tests for the model, HTTP, MCP, and service trace
surfaces. Keep new contractual event fields in this schema before relying on
them from external clients.

`service-events-response.v1.schema.json` describes the response envelope
returned by:

- CLI `agent-browser service events`
- HTTP `GET /api/service/events`

The schema covers the returned event array plus count, matched, and total
counters for filtered event list consumers.

## Service Collection Records v1

`service-status-response.v1.schema.json` describes the full service status
response returned by `agent-browser service status` and HTTP
`GET /api/service/status`.

The service collection record schemas describe compact records returned by HTTP
collection APIs and the matching MCP resources:

- `service-profile-record.v1.schema.json`
- `service-browser-record.v1.schema.json`
- `service-session-record.v1.schema.json`
- `service-tab-record.v1.schema.json`
- `service-site-policy-record.v1.schema.json`
- `service-provider-record.v1.schema.json`
- `service-challenge-record.v1.schema.json`

These schemas cover `profiles`, `browsers`, `sessions`, `tabs`,
`sitePolicies`, `providers`, and `challenges` records. They are guarded by Rust
model tests and MCP resource tests so software clients can consume the same
camelCase record fields from HTTP and MCP without inferring Rust internals.

The matching collection response schemas cover the compact collection envelopes
returned by CLI, HTTP, and MCP resources:

- `service-profiles-response.v1.schema.json`
- `service-browsers-response.v1.schema.json`
- `service-sessions-response.v1.schema.json`
- `service-tabs-response.v1.schema.json`
- `service-site-policies-response.v1.schema.json`
- `service-providers-response.v1.schema.json`
- `service-challenges-response.v1.schema.json`

These schemas guard the collection array field and `count` field.

The service config mutation schemas describe write response envelopes returned
by HTTP service APIs and matching MCP tools:

- `service-profile-upsert-response.v1.schema.json`
- `service-profile-delete-response.v1.schema.json`
- `service-session-upsert-response.v1.schema.json`
- `service-session-delete-response.v1.schema.json`
- `service-site-policy-upsert-response.v1.schema.json`
- `service-site-policy-delete-response.v1.schema.json`
- `service-provider-upsert-response.v1.schema.json`
- `service-provider-delete-response.v1.schema.json`

These schemas cover the authoritative path ID, mutation flag, and returned or
removed record payload for persisted profile, session, site policy, and
provider writes.

Profile mutation inputs are policy checked before persistence. The
`caller_supplied` allocation requires `userDataDir`, and `per_service` profiles
may list at most one `sharedServiceIds` entry. Session mutation inputs infer
`owner` from `agentName`, then `serviceName`, when omitted, require `profileId`
to reference a persisted profile, and enforce profile `sharedServiceIds`
allow-lists.

Profile records separate caller ownership from target login scope.
`sharedServiceIds` names caller services allowed to use the profile,
`targetServiceIds` names target sites or identity providers whose credentials
or login state should live in the profile, and `authenticatedServiceIds` names
targets currently believed to have usable authenticated state.

The operator remedy mutation schemas describe write response envelopes returned
by HTTP service APIs and matching MCP tools:

- `service-job-cancel-response.v1.schema.json`
- `service-browser-retry-response.v1.schema.json`
- `service-incident-acknowledge-response.v1.schema.json`
- `service-incident-resolve-response.v1.schema.json`

These schemas cover queued job cancellation, manual browser recovery retry
overrides, and durable incident acknowledgement or resolution metadata.

`service-reconcile-response.v1.schema.json` describes the response envelope
returned by `agent-browser service reconcile` and HTTP
`POST /api/service/reconcile`. It covers the reconciliation flag, browser
counts, changed browser count, and returned service state snapshot.

## Service Trace Aggregate Records v1

`service-trace-response.v1.schema.json` describes the full `service_trace`
response returned by `agent-browser service trace`, HTTP `GET
/api/service/trace`, and MCP `service_trace`.

`service-trace-summary-record.v1.schema.json` describes the `summary` object
returned by `agent-browser service trace`, HTTP `GET /api/service/trace`, and
MCP `service_trace`.

`service-trace-activity-record.v1.schema.json` describes normalized `activity`
items returned by:

- CLI `agent-browser service activity <incident-id>`
- CLI `agent-browser service trace`
- HTTP `GET /api/service/incidents/<id>/activity`
- HTTP `GET /api/service/trace`
- MCP `agent-browser://incidents/{incident_id}/activity`
- MCP `service_trace`

The schemas are guarded by Rust model/action tests and live HTTP/MCP trace
smokes so dashboards, API clients, and agents can use the aggregate trace
payload without rejoining raw event, job, and incident records themselves.

`service-incident-activity-response.v1.schema.json` describes the standalone
incident activity response returned by `agent-browser service activity
<incident-id>`, HTTP `GET /api/service/incidents/<id>/activity`, and MCP
`agent-browser://incidents/{incident_id}/activity`.
