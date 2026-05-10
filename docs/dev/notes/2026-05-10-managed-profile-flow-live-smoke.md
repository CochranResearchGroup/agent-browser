# Managed Profile Flow Live Smoke

Date: 2026-05-10

Added `pnpm test:service-client-managed-profile-flow-live` for the Canva-style managed profile broker recipe.

The smoke uses an isolated `AGENT_BROWSER_HOME`, seeds one authenticated managed profile with fresh target readiness, seeds one active but never-checked `profile_readiness` monitor, disables the background monitor scheduler, and runs:

```bash
examples/service-client/managed-profile-flow.mjs --run-due-readiness-monitor
```

The expected proof is:

- The initial access plan selects the existing profile and recommends `run_due_profile_readiness_monitor`.
- The example does not register a fallback profile.
- `runServiceAccessPlanMonitorRunDue()` checks exactly one monitor and fails zero.
- The refreshed access plan recommends `use_selected_profile`.
- The queued tab request succeeds.
- Persisted service state records `profile_readiness_fresh` and keeps the target in `authenticatedServiceIds`.
