# Service Client Example Monitor Summary

Date: 2026-05-10

## Purpose

This note records the example-facing follow-up to the
`runDueReadinessMonitor` acquisition option. The generic service-client example
should make profile freshness work visible without requiring operators to
inspect nested access-plan objects.

## Change

`examples/service-client/service-request-trace.mjs` now returns
`profileAcquisitionSummary`.

The summary includes:

- `selectedProfileId`
- `registered`
- `monitorRegistered`
- `monitorRunDueRan`
- `initialRecommendedAction`
- `refreshedRecommendedAction`
- `monitorRunDueChecked`
- `monitorRunDueFailed`

The no-launch example tests now cover the path where the initial access plan
recommends `run_due_profile_readiness_monitor`, the helper runs
`/api/service/monitors/run-due`, and the refreshed access plan recommends
`use_selected_profile`.

## Roadmap Alignment

This keeps the human-readable software-client workflow aligned with the
service roadmap: agent-browser owns the browser/profile/monitor coordination,
and callers can see whether the service advanced readiness before using a
profile.

## Follow-Up

The next useful slice is to propagate the same concise acquisition summary into
the Canva-style `managed-profile-flow.mjs` dry-run and live outputs.
