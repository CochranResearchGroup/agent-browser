# P55 S6 Dashboard Swap Navigation Plan

Date: 2026-06-27
State: SUPERSEDED BY P56
Lane: P55
Depends On:
- `docs/dev/plans/0046-2026-06-24-remote-view-stress-hardening-plan.md`
- `docs/dev/notes/2026-06-24-p46-stress-hardening-execution.md`
- `docs/dev/plans/0054-2026-06-27-s5-viewer-client-port-allocation-plan.md`

## Purpose

Unlock P46 S6 without a blind third retry. S6 attempt 2 failed in bounded form
when the external dashboard viewer-client CDP command `Page.navigate` timed out
during the swapped dashboard selection step. The same attempt also showed that
reset-after could miss retained browser rows whose service session rows were
already absent.

## Current Evidence

- S6 attempt 1 artifact:
  `/tmp/agent-browser-p46-s6-2026-06-27T19-49-19-793Z`.
- S6 attempt 2 artifact:
  `/tmp/agent-browser-p46-s6-2026-06-27T19-56-33-105Z`.
- Attempt 2 failure:
  `CDP command Page.navigate timed out after 30000ms`.
- Attempt 2 stopped before swapped selection artifacts were written.
- Manual cleanup closed:
  - `p46-s6-profile-a-2026-06-27T19-56-29-450Z`;
  - `p46-s6-profile-b-2026-06-27T19-56-29-450Z`.
- Final readback after manual cleanup showed zero sessions, zero browsers, zero
  tabs, zero active incidents, both route-pool entries available, and both route
  displays idle.

## Non-Negotiable Rules

- Do not run S6 again until the dashboard swap path has no-live coverage.
- Do not use service-owned `agent-browser` sessions for viewer clients.
- Preserve S6's cross-observation proof: operator A must select profile B and
  operator B must select profile A, with selected browser/session readback and
  refresh controls verified after the swap.
- Preserve reset-after cleanup proof for retained browser rows whose session
  rows are missing.

## Goal 1: Replace `Page.navigate` In The Swap Path

Work:

- Add a viewer-client helper that asks the already-loaded dashboard page to
  change `window.location` to the target dashboard URL from inside the page.
- The helper must return a JSON artifact before waiting for the new dashboard
  state.
- Keep the existing dashboard-state poll as the authority that proves the swap
  completed.

Evidence:

- `node --check scripts/lib/p47-viewer-client.js`
- `node scripts/test-p47-viewer-client-separation.js`

## Goal 2: Keep Reset Cleanup Retained-Browser Aware

Work:

- Keep `scripts/run-p46-stress-scenario.js` reset close-target discovery based
  on service sessions plus retained browser `activeSessionIds` and
  `session:<name>` browser IDs.
- Keep no-live coverage for that reset contract.

Evidence:

- `node scripts/test-p47-scenario-harness.js`

## Goal 3: Retry S6 Once After Baseline Verification

Preflight:

```bash
./cli/target/debug/agent-browser service status --json
./cli/target/debug/agent-browser service incidents --summary --json
./cli/target/debug/agent-browser install doctor --json
./cli/target/debug/agent-browser doctor remote-view --json
node scripts/smoke-rdp-guac-route-pool-readiness.js --report-only
node scripts/inspect-rdp-route-displays.js --display-content
```

Retry command:

```bash
node scripts/run-p46-stress-scenario.js \
  --scenario s6 \
  --reset-before \
  --reset-after \
  --agent-browser-command ./cli/target/debug/agent-browser \
  --require-explicit-agent-browser-command \
  --require-agent-browser-daemon-command-match
```

Pass criteria:

- Profile A and profile B are concurrently route-bound on distinct routes and
  displays.
- Operator A initially targets profile A, then swaps to profile B.
- Operator B initially targets profile B, then swaps to profile A.
- Swapped dashboard selected browser/session/tab readback matches the selected
  profile for both operators.
- Refresh controls work after the swap.
- Reset-after leaves zero sessions, browsers, tabs, routes, and active
  incidents.

## Execution Log

### 2026-06-27

Initial diagnosis completed from P46, the P46 execution note, and the S6
attempt artifacts. P46 remains locked at S6 until Goals 1 and 2 pass.

Implemented Goals 1 and 2:

- `scripts/lib/p47-viewer-client.js` now exports
  `navigateDashboardViewerClient`, which asks the already-loaded dashboard page
  to change `window.location` from inside the page instead of sending raw
  `Page.navigate` for the S6 swap.
- `scripts/run-p46-stress-scenario.js` now writes
  `operator-a-swapped-to-profile-b-navigate.json` and
  `operator-b-swapped-to-profile-a-navigate.json` before waiting for swapped
  dashboard-state readback.
- The S6 reset helper keeps the retained-browser close-target repair from the
  previous attempt.
- `scripts/test-p47-viewer-client-separation.js` covers the in-page dashboard
  navigation helper.
- `scripts/test-p47-scenario-harness.js` covers the swapped navigation
  artifacts and asserts the S6 swap path does not use raw `Page.navigate`.

Validation:

```bash
node --check scripts/lib/p47-viewer-client.js
node --check scripts/run-p46-stress-scenario.js
node scripts/test-p47-viewer-client-separation.js
node scripts/test-p47-scenario-harness.js
```

All checks passed. P46 may run one S6 retry from the explicit rebuilt-binary
lane after the live baseline preflight passes.

P55-authorized S6 retry:

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
/tmp/agent-browser-p46-s6-2026-06-27T20-06-51-508Z
```

Result: failed.

Failure:

- `operator-a-swapped-to-profile-b-navigate.json` was written with `ok: true`,
  proving the in-page URL change request returned.
- The first post-swap state poll then timed out with
  `CDP command Runtime.evaluate timed out after 30000ms`.
- Reset-after did not close the two retained browser rows because the large
  `service status` payload hit the default Node `spawnSync` output buffer and
  parsed as `null`.

Manual cleanup:

- closed `p46-s6-profile-a-2026-06-27T20-06-52-041Z`;
- closed `p46-s6-profile-b-2026-06-27T20-06-52-041Z`;
- final readback showed zero sessions, zero browsers, zero tabs, zero active
  incidents, and both route-pool entries available.

P55 did not unlock S6. P46 remains locked at S6 and follow-up repair moved to
`docs/dev/plans/0056-2026-06-27-s6-dashboard-reconnect-and-reset-buffer-plan.md`.
