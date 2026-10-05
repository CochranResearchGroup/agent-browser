# Plan 0085: Service-Tab-Handle Profile Routing

State: CLOSED

Created: 2026-07-31

Lane: post-release v0.28.0 defect successor to Plan 0084

Source artifact:

- `/tmp/agent-browser-remote-view-open-live-2026-07-31T23-08-12-663Z`

## Goal

Preserve the service-tab handle's profile identity on follow-on service
requests so access planning cannot select an unrelated default profile before
routing to the handle's browser, session, and target.

## Bounds

- Centralize existing-handle browser, session, target, and profile hints in the
  JavaScript service client.
- Apply the helper only to request builders that already require a valid
  service-tab handle.
- Preserve explicit caller overrides and current request/response schemas.
- One implementation pass and one live-gate rerun.
- Preserve the operator-owned untracked `--full-page` file.
- No release, tag, or publication.

## Acceptance

1. Follow-on request builders preserve `runtimeProfile` and `profileId` from a
   handle unless the caller explicitly overrides them.
2. Focused client tests cover evaluate plus representative attach, diagnostics,
   probe, UI, network, file, detach, refresh, and release requests.
3. Existing service-client and generated-type parity tests pass.
4. `pnpm test:remote-view-open-live` passes with target-bound readback and
   cleanup, or any distinct failure is recorded as a bounded successor.

## Next Action

- Return to last30days Plan 0018 and execute fresh, bounded successor intervals
  for the configured Reddit, X, Facebook, and LinkedIn access methods.

## Result

- `serviceTabHandleRouting` now preserves browser, session, target,
  `runtimeProfile`, and `profileId` across attach, detach, evaluate,
  diagnostics, probe, UI, network, file, refresh, and release builders while
  retaining explicit caller overrides.
- Focused request-client, full service-client, generated-type parity, syntax,
  diff, and route-state regressions pass.
- The one authorized live rerun passed at
  `/tmp/agent-browser-remote-view-open-live-2026-07-31T23-12-57-008Z` with
  route `guacamole:1`, browser-visible handoff, target-bound LinkedIn URL/title
  readback, one matching intent tab, and cleanup.
