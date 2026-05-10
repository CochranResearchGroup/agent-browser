# Access Plan Monitor Run-Due Recipe

Date: 2026-05-10

## Purpose

This note records the follow-up to the access-plan readiness probe-due signal.
The goal is to make the recommended due-monitor action directly executable
from the planner response instead of requiring agents or software clients to
know the monitor route, MCP tool, or CLI command.

## Change

Access-plan decisions now include `decision.monitorRunDue`.

The recipe includes:

- `available`
- `recommendedBeforeUse`
- `monitorIds`
- `neverCheckedMonitorIds`
- `targetServiceIds`
- HTTP `POST /api/service/monitors/run-due`
- MCP `service_monitors_run_due`
- CLI `agent-browser service monitors run-due`
- service-client helper `runServiceAccessPlanMonitorRunDue()`
- fallback helper `runDueServiceMonitors()`

The recipe remains no-launch planning metadata. Executing it runs due active
monitors through the existing serialized service worker path.

## Roadmap Alignment

This keeps profile freshness orchestration inside agent-browser. Callers ask
for an access plan, inspect the service-owned recommendation, and execute the
advertised recipe when retained profile freshness should be checked before a
tab or browser action uses that identity.

## Follow-Up

The next useful slice is to make `acquireServiceLoginProfile()` optionally run
`runServiceAccessPlanMonitorRunDue()` and refresh the access plan when
`decision.monitorRunDue.recommendedBeforeUse` is true.
