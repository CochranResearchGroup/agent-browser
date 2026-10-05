# Plan 0084: Handle-Bound Live-Gate Readback

State: CLOSED

Created: 2026-07-31

Lane: post-release v0.28.0 defect successor to Plan 0083

Source artifact:

- `/tmp/agent-browser-remote-view-open-live-2026-07-31T23-05-42-963Z`

## Goal

Make the remote-view-open live gate verify URL and title through the exact
`serviceTabHandle` returned by the final HTTP open, rather than rediscovering a
generic active tab in the daemon session.

## Bounds

- Change only handle-bound readback and its focused normalization regression.
- Preserve all route, display, operator-visible, target-ID, and cleanup checks.
- One implementation pass and one live-gate rerun.
- Preserve the operator-owned untracked `--full-page` file.
- No release, tag, or public publication.

## Acceptance

1. The harness evaluates `location.href` and `document.title` against the
   returned service-tab handle and requires the evaluated target ID to match.
2. Focused tests cover current and compatibility response envelopes.
3. `pnpm test:remote-view-open-live` passes and proves cleanup.
4. Any further failure becomes a separately classified successor rather than
   weakening target-bound evidence.

## Next Action

- Plan 0085 must preserve the handle's profile identity on follow-on service
  requests. Plan 0084's target-bound readback and normalization regressions
  pass; its live rerun reached the distinct client-routing defect at artifact
  `/tmp/agent-browser-remote-view-open-live-2026-07-31T23-08-12-663Z`.

## Result

- The live gate now evaluates URL/title through the final HTTP open's valid
  service-tab handle and requires the evaluated target ID to match.
- Current nested, compatibility, and normalized evaluate-response fixtures
  pass.
- The sole live rerun reached the intended target-bound evaluate request, then
  the client dropped `profileId=remote-view-open-live-19885-profile`. Service
  planning selected unrelated profile `default` and rejected the request on
  that profile's lease conflict.
- Cleanup restored the test profile and route to available.
