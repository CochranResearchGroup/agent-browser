# Plan 0083: Route-Owned Live-Gate Attribution

State: CLOSED

Created: 2026-07-31

Lane: post-release v0.28.0 defect

Source artifacts:

- `/tmp/agent-browser-remote-view-open-live-2026-07-31T22-54-48-779Z`
- `/tmp/agent-browser-remote-view-open-live-2026-07-31T22-56-04-094Z`
- last30days Plan 0018 checkpoint C30

## Goal

Make the remote-view-open live gate attribute retained stream proof to the
browser named by the authoritative route record. A stale browser that retains
the same historical route ID must not cause a newly opened, correctly bound
browser to fail validation.

## Bounds

- Change only live-gate evidence selection and its focused regression.
- Do not weaken route, browser, display, visible-window, or readiness checks.
- Preserve the operator-owned untracked `--full-page` file.
- One implementation pass and one live-gate rerun before reclassification.
- No formal release, tag, or public publication.

## Acceptance

1. A fixture with stale and current browser streams sharing one route ID
   selects the stream owned by the route's browser.
2. An unattributed or differently owned top-level stream cannot substitute for
   the authoritative browser's stream.
3. The focused no-launch regression passes.
4. `pnpm test:remote-view-open-live` passes against the installed runtime and
   cleanup releases the test browser, route, display allocation, and profile.
5. If the live gate exposes another product defect, stop and record the exact
   successor boundary rather than weakening the assertion.

## Next Action

- Plan 0084 must replace generic active-tab rediscovery with target-bound
  service-tab-handle evaluation. Plan 0083's route-owned selection and focused
  regression pass; its sole live rerun reached the next distinct defect at
  artifact `/tmp/agent-browser-remote-view-open-live-2026-07-31T23-05-42-963Z`.

## Result

- `selectRouteOwnedStream` now prefers the stream nested under the route's
  authoritative browser and accepts a top-level fallback only when it names
  that same browser.
- The duplicate-route, attributed-fallback, and foreign/unattributed rejection
  regressions pass.
- The live rerun passed all three route-bound opens, each with
  `operatorVisible=ready` and a valid LinkedIn service-tab handle. It then
  failed because generic session `get url` rediscovered `about:blank` instead
  of addressing that handle.
- Cleanup restored the test profile lease and route availability.
