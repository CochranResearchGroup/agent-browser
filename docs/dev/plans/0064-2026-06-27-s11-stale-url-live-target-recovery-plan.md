# S11 Stale URL Live-Target Recovery Plan

Date: 2026-06-27
State: COMPLETE
Lane: P64
Parent: `docs/dev/plans/0046-2026-06-24-remote-view-stress-hardening-plan.md`

## Problem

P46 S11 is locked by the two-consecutive-failure rule.

S11 must prove that dashboard refresh, stale workspace URLs, direct Guacamole
readback, reconnect, and viewport refresh remain stable for a route-bound
service-owned browser.

The first two live attempts failed in the harness before S11 evaluation:

- `/tmp/agent-browser-p46-s11-2026-06-27T23-02-14-303Z` pushed stale target
  `target:p46-s11-stale-target`, but the dashboard immediately rewrote the
  page URL back to the live tab target. The harness incorrectly waited for the
  exact stale URL to persist.
- `/tmp/agent-browser-p46-s11-2026-06-27T23-05-10-207Z` waited for explicit
  stale-recovery notice text, but the dashboard recovered by rewriting to live
  target `target:734BA856384987537291FC56C628FFBE` with a healthy iframe and
  no password prompt.

Both attempts reset cleanly and reported zero active incidents after
reset-after.

## Repair

The S11 harness now accepts both valid stale URL recovery forms:

- explicit stale-target recovery notice with a healthy viewport;
- immediate rewrite from the requested stale target to a current live target
  with the same browser/session, a healthy iframe, refresh control, and no
  password prompt.

This is intentionally scoped to S11 by using the new
`allowRecoveredLiveTab` viewer-client helper option. Default dashboard-state
matching remains strict for other scenarios.

## Validation Gate

Before another live S11 retry, run:

```bash
node --check scripts/run-p46-stress-scenario.js
node --check scripts/lib/p46-scenario-harness.js
node scripts/test-p47-scenario-harness.js
git diff --check -- scripts/lib/p47-viewer-client.js scripts/lib/p46-scenario-harness.js scripts/run-p46-stress-scenario.js scripts/test-p47-scenario-harness.js docs/dev/plans/0046-2026-06-24-remote-view-stress-hardening-plan.md docs/dev/plans/0064-2026-06-27-s11-stale-url-live-target-recovery-plan.md docs/dev/notes/2026-06-24-p46-stress-hardening-execution.md RUNBOOK.md
/home/ecochran76/.local/bin/agent-browser --json service incidents --summary
```

Authorize exactly one retry only if the no-live gate passes and active
incidents are zero:

```bash
node scripts/run-p46-stress-scenario.js --scenario s11 --reset-before --reset-after --agent-browser-command /home/ecochran76/.local/bin/agent-browser --require-explicit-agent-browser-command --require-agent-browser-daemon-command-match
```

## Green Preflight

The no-live gate passed:

```bash
node --check scripts/run-p46-stress-scenario.js
node --check scripts/lib/p46-scenario-harness.js
node --check scripts/lib/p47-viewer-client.js
node scripts/test-p47-scenario-harness.js
node scripts/test-p47-viewer-client-separation.js
git diff --check -- scripts/lib/p47-viewer-client.js scripts/lib/p46-scenario-harness.js scripts/run-p46-stress-scenario.js scripts/test-p47-scenario-harness.js scripts/test-p47-viewer-client-separation.js docs/dev/plans/0046-2026-06-24-remote-view-stress-hardening-plan.md docs/dev/plans/0064-2026-06-27-s11-stale-url-live-target-recovery-plan.md docs/dev/notes/2026-06-24-p46-stress-hardening-execution.md RUNBOOK.md
/home/ecochran76/.local/bin/agent-browser --json service incidents --summary
```

The incident summary reported `count=0`, `matched=0`, and `total=0`.

One P64-authorized S11 retry is now permitted.

## Pass Conditions

The retry must prove all of the following:

- dashboard reload restores the selected route-bound browser with a remote
  viewport iframe;
- stale tab URL is rejected or recovered to a live target for the same
  browser/session;
- viewer-client reconnect after stale URL recovery succeeds;
- viewport refresh remains clickable and preserves a healthy iframe;
- direct Guacamole frame URL is reachable before and after stale URL recovery;
- route display remains `browser_window_visible`;
- route-bound finalization remains complete;
- reset-after leaves zero active incidents.

## Completion

The P64-authorized S11 retry passed:

```text
/tmp/agent-browser-p46-s11-2026-06-27T23-09-57-372Z
```

Evidence:

- dashboard reload restored the route-bound browser viewport;
- stale URL request for `target:p46-s11-stale-target` recovered to live target
  `target:2AE5C8E4CED37D0C1A77771CD3CA9F58`;
- `staleRecovered: true`;
- `staleRecoveredLiveTab: true`;
- viewer-client reconnect after stale URL recovery succeeded;
- viewport refresh after stale URL recovery clicked successfully;
- direct Guacamole frame URL returned
  `200 http://127.0.0.1:8092/guacamole/#/client/MwBjAHBvc3RncmVzcWw=`;
- route display `:13` remained `browser_window_visible`;
- route-bound finalization was complete with no blockers;
- reset-before and reset-after both ended with zero active incidents.

Command metadata caveat: the live retry used an explicit
`--agent-browser-command /home/ecochran76/.local/bin/agent-browser`, and daemon
realpath matching was enforced and passed. The explicit-command guard flag was
misspelled in this retry, so the summary reports `requireExplicit: false`,
but also reports `explicit: true` and the expected installed binary realpath.

P64 is complete. P46 may continue at S12.
