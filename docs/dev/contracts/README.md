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

## Service Event Record v1

`service-event-record.v1.schema.json` describes retained service event records
returned by:

- HTTP `GET /api/service/events`
- MCP `agent-browser://events`
- service trace event arrays

The schema is guarded by Rust tests for the model, HTTP, MCP, and service trace
surfaces. Keep new contractual event fields in this schema before relying on
them from external clients.

## Service Collection Records v1

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
