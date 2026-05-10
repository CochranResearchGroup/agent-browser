# Managed Profile Flow Acquisition Summary

Date: 2026-05-10

The lower-level `examples/service-client/managed-profile-flow.mjs` recipe now mirrors the broker-first helper behavior closely enough for Canva-style integrations:

- It asks for an initial access plan before registration.
- It registers a fallback managed profile only when the initial plan has no selected profile.
- It refreshes the access plan after profile registration, monitor registration, or freshness updates.
- It can run an access-plan-recommended due profile-readiness monitor with `--run-due-readiness-monitor`.
- It refreshes the access plan again after a due monitor run before requesting a tab.
- It prints `profileAcquisitionSummary` so operators can see selected profile ID, registration, due-monitor execution, initial recommendation, and refreshed recommendation in one compact object.

Validation:

- `pnpm test:service-client-managed-profile-flow`

