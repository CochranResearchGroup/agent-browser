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
drive profile selection for the requested site or login scope. `profileLeasePolicy`
can be `reject` or `wait`; `wait` uses `profileLeaseWaitTimeoutMs` to bound how
long the service request waits for another exclusive profile lease to release.

`service-request-mcp-tool-call.v1.schema.json` describes the MCP `tools/call`
wrapper for invoking `service_request` with the same intent object.

HTTP `GET /api/service/contracts` and MCP `agent-browser://contracts` expose
runtime compatibility metadata for these contract IDs, their shared `v1`
version, route and tool names, and the supported service request action list.
The HTTP contracts metadata also advertises the HTTP-only
`serviceProfileAllocationResponse` contract for
`GET /api/service/profiles/<id>/allocation`.

`packages/client/src/service-request.generated.d.ts` and
`packages/client/src/service-request.generated.js` are generated from these
schemas. Run `pnpm generate:service-client` after changing the schemas and
`pnpm test:service-client-contract` to verify the generated client surface is
current. Run `pnpm test:service-client-types` to type-check the runtime helper
against those declarations.

`packages/client/src/service-observability.generated.d.ts` and
`packages/client/src/service-observability.generated.js` are generated from the
service job, event, incident, incident activity, and trace schemas. The
`@agent-browser/client/service-observability` helper reads those HTTP endpoints
and returns the generated response types, including `getServiceContracts` for
the runtime compatibility metadata endpoint.

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
`GET /api/service/status`. It includes the derived `profileAllocations` view so
software clients can consume profile holder, waiting job, conflict, lease
state, and recommended-action data without reconstructing it from raw state.

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
`sitePolicies`, `providers`, and `challenges` records. Profile records include
derived `targetReadiness` rows for no-launch target-service readiness. Google
targets without authenticated evidence report `needs_manual_seeding` and
recommend detached `runtime login` before attachable automation. They are
guarded by Rust model tests and MCP resource tests so software clients can
consume the same camelCase record fields from HTTP and MCP without inferring
Rust internals.

The matching collection response schemas cover the compact collection envelopes
returned by CLI, HTTP, and MCP resources:

- `service-profiles-response.v1.schema.json`
- `service-browsers-response.v1.schema.json`
- `service-sessions-response.v1.schema.json`
- `service-tabs-response.v1.schema.json`
- `service-site-policies-response.v1.schema.json`
- `service-providers-response.v1.schema.json`
- `service-challenges-response.v1.schema.json`

These schemas guard the collection array field and `count` field. The profiles
response also includes the same derived `profileAllocations` view returned by
service status and the MCP profiles resource. Profile allocation rows include
the same `targetReadiness` rows as the profile record so detail clients do not
have to join back to the full profile collection.

`service-profile-allocation-response.v1.schema.json` describes the response
envelope returned by HTTP `GET /api/service/profiles/<id>/allocation` when a
software client needs one derived profile allocation row without fetching the
full profile collection.

`service-profile-readiness-response.v1.schema.json` describes the response
envelope returned by HTTP `GET /api/service/profiles/<id>/readiness` when a
software client needs one profile's no-launch target-readiness rows without
fetching allocation details or the full profile collection.

`service-profile-lookup-response.v1.schema.json` describes the response
envelope returned by HTTP `GET /api/service/profiles/lookup` when a software
client wants agent-browser to apply the authoritative service profile selector
for a service name plus site or login identity without fetching the full
profile collection.

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
targets currently believed to have usable authenticated state. `targetReadiness`
is the no-launch readiness view derived from those hints and site policy.
Session records include `profileSelectionReason` so clients can distinguish
`authenticated_target`, `target_match`, `service_allow_list`, and
`explicit_profile` profile choices without reconstructing selector behavior
from events.
They also include `profileLeaseDisposition` and
`profileLeaseConflictSessionIds` so clients can see whether the selected
profile started a new browser, reused a retained session browser, or hit
another exclusive session. Service-scoped launches reject or wait on active
exclusive conflicts according to `profileLeasePolicy`.

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
MCP `service_trace`. Its `profileLeaseWaits` object provides a per-job rollup
of profile lease waits, including outcome, timing, conflict sessions, and trace
labels, so clients do not need to reconstruct waits from raw event pairs.

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
