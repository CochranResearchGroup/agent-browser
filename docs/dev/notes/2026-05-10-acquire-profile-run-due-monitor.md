# Acquire Profile Run-Due Monitor Option

Date: 2026-05-10

## Purpose

This note records the client-side follow-up to `decision.monitorRunDue`.
Software clients that use `acquireServiceLoginProfile()` should not need to
hand-code monitor routes when the access plan says retained profile freshness
should be checked before browser work.

## Change

`acquireServiceLoginProfile()` now accepts `runDueReadinessMonitor`.

When enabled, the helper:

- gets the initial access plan
- registers a fallback profile only if no profile is selected
- optionally registers the retained `profile_readiness` monitor
- runs `runServiceAccessPlanMonitorRunDue()` only when the current access plan
  has `decision.monitorRunDue.recommendedBeforeUse`
- refreshes the access plan after the monitor run
- returns `monitorRunDue` and `monitorRunDueRan`

The default remains false, preserving existing client behavior unless a caller
opts into running due monitors.

## Roadmap Alignment

This keeps profile freshness coordination in agent-browser while giving
software clients a single broker-first helper for the common access-plan,
managed-profile, readiness-monitor, and final-tab-request path.

## Follow-Up

The next useful slice is to make the service-client examples surface
`monitorRunDueRan` and the refreshed recommended action more clearly in their
human-readable output.
