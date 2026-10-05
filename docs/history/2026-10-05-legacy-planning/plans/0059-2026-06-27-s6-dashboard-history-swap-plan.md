# P59 S6 Dashboard History Swap Plan

Date: 2026-06-27
State: COMPLETE
Lane: P59
Depends On:
- `docs/dev/plans/0046-2026-06-24-remote-view-stress-hardening-plan.md`
- `docs/dev/plans/0058-2026-06-27-s6-dashboard-page-url-stabilization-plan.md`
- `docs/dev/notes/2026-06-24-p46-stress-hardening-execution.md`

## Purpose

Repair the S6 dashboard swap after P58 proved that `location.assign()` did not
change the external viewer-client page URL for the same-origin dashboard
workspace swap.

## Current Evidence

- P58-authorized S6 retry artifact:
  `/tmp/agent-browser-p46-s6-2026-06-27T20-28-52-103Z`.
- `operator-a-swapped-to-profile-b-navigate.json` requested profile B.
- `operator-a-swapped-to-profile-b-page-url.json` showed the DevTools page URL
  remained on profile A after the wait.
- Reset-after closed both retained S6 sessions and final readback showed zero
  sessions, zero browsers, zero tabs, zero active incidents, and both
  route-pool entries available.

## Non-Negotiable Rules

- Do not run S6 again until the history-swap helper has no-live coverage.
- Preserve P58 page-URL artifacts.
- Preserve P57 reconnect discovery artifacts and P56 reset-buffer cleanup.
- Do not weaken S6 swapped selected-browser, selected-tab, refresh, and
  screenshot proof.

## Goal 1: Use Same-Origin History Swap

Work:

- Change the dashboard swap helper to use `history.pushState` for same-origin
  dashboard workspace URL changes.
- Dispatch a `popstate` event after the history update so the dashboard app has
  a chance to react without a full page navigation.
- Keep `location.assign` only as the cross-origin fallback.

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

- `scripts/lib/p47-viewer-client.js` now uses `history.pushState` plus a
  `PopStateEvent` for same-origin dashboard swaps and reports the method in the
  navigation artifact.
- Cross-origin swaps still fall back to `window.location.assign`.
- `scripts/test-p47-viewer-client-separation.js` covers the same-origin history
  swap and fallback shape.

P59 authorizes one S6 retry after validation and live preflight.

P59-authorized S6 retry:

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
/tmp/agent-browser-p46-s6-2026-06-27T20-32-54-709Z
```

Result: passed.

Evidence:

- operator A swapped to profile B and read back
  `session:p46-s6-profile-b-2026-06-27T20-32-55-285Z`;
- operator B swapped to profile A and read back
  `session:p46-s6-profile-a-2026-06-27T20-32-55-285Z`;
- both swapped refresh controls clicked successfully;
- both swapped dashboard screenshots were captured;
- profile A and profile B finalized route-bound checkouts on distinct routes
  and displays;
- closing profile A released route A while profile B stayed ready;
- reset-after closed the remaining retained profile B session and reported zero
  active incidents.

P59 is complete. P46 continues at S7.
