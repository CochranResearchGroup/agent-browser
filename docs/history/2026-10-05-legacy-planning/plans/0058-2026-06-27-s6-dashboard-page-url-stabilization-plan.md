# P58 S6 Dashboard Page URL Stabilization Plan

Date: 2026-06-27
State: SUPERSEDED BY P59
Lane: P58
Depends On:
- `docs/dev/plans/0046-2026-06-24-remote-view-stress-hardening-plan.md`
- `docs/dev/plans/0057-2026-06-27-s6-viewer-client-cdp-swap-diagnostics-plan.md`
- `docs/dev/notes/2026-06-24-p46-stress-hardening-execution.md`

## Purpose

Repair the S6 dashboard swap after P57 proved that the reconnect path was
running before Chromium's DevTools page list reflected the requested swapped
dashboard URL.

## Current Evidence

- P57-authorized S6 retry artifact:
  `/tmp/agent-browser-p46-s6-2026-06-27T20-23-53-164Z`.
- `operator-a-swapped-to-profile-b-navigate.json` records the requested profile
  B URL.
- `operator-a-swapped-to-profile-b-reconnect-discovery.json` exists, but its
  chosen page URL still points at profile A.
- `operator-a-swapped-to-profile-b-reconnect.json` was written with
  `domainEnableSkipped: true`, proving the P57 command-sequence repair executed.
- The next dashboard-state read timed out on `Runtime.evaluate`.
- Reset-after closed both retained S6 sessions and final readback showed zero
  sessions, zero browsers, zero tabs, zero active incidents, and both route
  pool entries available.

## Non-Negotiable Rules

- Do not run S6 again until the page-URL stabilization repair has no-live
  coverage.
- Preserve P57 reconnect discovery artifacts.
- Preserve P56 reset-buffer and retained-browser cleanup behavior.
- Do not weaken S6 swapped selected-browser, selected-tab, refresh, and
  screenshot proof.

## Goal 1: Wait For Swapped Dashboard Page URL

Work:

- Add a viewer-client helper that polls the viewer-client DevTools `/json` page
  list until the selected page URL equals the requested swapped dashboard URL.
- Write explicit page-URL artifacts for both swapped operators.
- Only reconnect CDP after the page URL has stabilized.

Evidence:

```bash
node --check scripts/lib/p47-viewer-client.js
node --check scripts/run-p46-stress-scenario.js
node scripts/test-p47-viewer-client-separation.js
node scripts/test-p47-scenario-harness.js
```

## Goal 2: Authorize One S6 Retry

After Goal 1 passes and live preflight is green, run exactly one S6 retry from
the explicit rebuilt-binary lane.

## Execution Log

### 2026-06-27

Implemented Goal 1:

- `scripts/lib/p47-viewer-client.js` now exports
  `waitForDashboardViewerClientPageUrl`, which polls DevTools `/json` until
  the chosen page URL matches the requested dashboard URL and writes the last
  page-list evidence on success or timeout.
- `scripts/run-p46-stress-scenario.js` now writes
  `operator-a-swapped-to-profile-b-page-url.json` and
  `operator-b-swapped-to-profile-a-page-url.json` before reconnecting CDP.
- No-live tests cover the new page-URL wait and require it before reconnect.

Validation:

```bash
node --check scripts/lib/p47-viewer-client.js
node --check scripts/run-p46-stress-scenario.js
node scripts/test-p47-viewer-client-separation.js
node scripts/test-p47-scenario-harness.js
```

All checks passed. P58 authorizes one S6 retry after live preflight.

P58-authorized S6 retry:

```bash
node scripts/run-p46-stress-scenario.js \
  --scenario s6 \
  --reset-before \
  --reset-after \
  --agent-browser-command ./cli/target/debug/agent-browser \
  --require-explicit-agent-browser-command \
  --require-agent-browser-daemon-command-match
```

Artifact:

```text
/tmp/agent-browser-p46-s6-2026-06-27T20-28-52-103Z
```

Result: failed.

Evidence:

- `operator-a-swapped-to-profile-b-navigate.json` requested profile B.
- `operator-a-swapped-to-profile-b-page-url.json` showed the chosen DevTools
  page URL remained on profile A.
- The failure was a page-URL stabilization timeout before reconnect.
- Reset-after closed both retained sessions and final runtime readback showed
  zero sessions, zero browsers, zero tabs, zero active incidents, and both
  route-pool entries available.

P58 did not unlock S6. Follow-up repair moved to
`docs/dev/plans/0059-2026-06-27-s6-dashboard-history-swap-plan.md`.
