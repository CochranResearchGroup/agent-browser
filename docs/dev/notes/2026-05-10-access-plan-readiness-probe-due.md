# Access Plan Readiness Probe-Due Checkpoint

Date: 2026-05-10

## Purpose

This note records the next backend-first service-roadmap slice after service
read MCP parity. The goal is to let access-plan push profile freshness work
forward before a browser-control request depends on stale retained auth state.

## Change

Access-plan now reports matching active `profile_readiness` monitors that are
due or never checked for the requested target identity.

The no-launch `monitorFindings` block includes:

- `profileReadinessProbeDue`
- `profileReadinessDueMonitorIds`
- `profileReadinessNeverCheckedMonitorIds`
- `dueTargetServiceIds`

When a matching profile-readiness monitor is due, the service decision sets
`decision.monitorProbeDue` and recommends
`run_due_profile_readiness_monitor`.

This does not launch Chrome. It tells agents and software clients to run the
serialized monitor path before trusting retained profile freshness.

## Roadmap Alignment

This keeps profile freshness authority inside agent-browser instead of pushing
the decision into MCP agents, software clients, or dashboard components. It is
a small access-policy and profile-readiness step that supports the larger
roadmap for provider execution, challenge handling, CDP-free operation, and
remote headed browser management.

## Follow-Up

The next useful slice is to make access-plan optionally include a copyable
monitor-run recipe, similar to `decision.serviceRequest` and
`decision.postSeedingProbe`, so clients can act on `monitorProbeDue` without
knowing route or tool details.
