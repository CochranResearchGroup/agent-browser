# P57 S6 Viewer-Client CDP Swap Diagnostics Plan

Date: 2026-06-27
State: SUPERSEDED BY P58
Lane: P57
Depends On:
- `docs/dev/plans/0046-2026-06-24-remote-view-stress-hardening-plan.md`
- `docs/dev/plans/0056-2026-06-27-s6-dashboard-reconnect-and-reset-buffer-plan.md`
- `docs/dev/notes/2026-06-24-p46-stress-hardening-execution.md`

## Purpose

Diagnose the remaining S6 viewer-client swap failure before any further live
retry. P56 proved that reset-after can now clean retained browser rows, but the
viewer-client CDP target still becomes unusable after the dashboard workspace
URL swap.

## Current Evidence

- P56-authorized S6 retry artifact:
  `/tmp/agent-browser-p46-s6-2026-06-27T20-15-52-909Z`.
- `operator-a-swapped-to-profile-b-navigate.json` exists and records `ok:
  true`.
- `operator-a-swapped-to-profile-b-reconnect.json` was not written.
- Failure:
  `CDP command Page.enable timed out after 30000ms`.
- Reset-after closed:
  - `p46-s6-profile-a-2026-06-27T20-15-53-475Z`;
  - `p46-s6-profile-b-2026-06-27T20-15-53-475Z`.
- Final readback showed zero sessions, zero browsers, zero tabs, zero active
  incidents, and both route-pool entries available.

## Non-Negotiable Rules

- Do not run S6 again until this plan records a concrete diagnostic result and
  a reviewed repair.
- Do not remove the P56 reset buffer guard.
- Do not weaken S6 pass criteria to skip swapped selected-browser, selected-tab,
  refresh, and screenshot proof.

## Goal 1: Capture CDP Target Discovery Evidence

Work:

- Write a reconnect discovery artifact before sending any CDP commands to the
  rediscovered page websocket.
- Include the DevTools `/json` page list, chosen page ID, chosen page URL, page
  type, and websocket URL.
- Preserve the existing command timeout guard.

Evidence:

```bash
node --check scripts/lib/p47-viewer-client.js
node scripts/test-p47-viewer-client-separation.js
node scripts/test-p47-scenario-harness.js
```

## Goal 2: Classify Whether The Page Target Or Command Sequence Is At Fault

Work:

- Compare the selected page target before swap and after swap.
- Decide whether the repair should:
  - reuse the existing websocket without `Page.enable`;
  - connect to a different page target;
  - recreate the external viewer-client browser after swap;
  - or move the swap operation to an explicit browser-context URL open.

Evidence:

- Source-backed note in
  `docs/dev/notes/2026-06-24-p46-stress-hardening-execution.md`.
- Artifact-backed failure or pass record before another S6 retry.

## Goal 3: Authorize At Most One More S6 Retry

Only after Goals 1 and 2 produce a concrete repair, run the standard S6 command
once from the explicit rebuilt-binary lane.

## Execution Log

### 2026-06-27

Graphiti discovery was healthy but did not return current P57-specific
authority beyond older repo-policy and P46 S4 facts. Repo plans and artifacts
remain authoritative.

Implemented Goal 1:

- `scripts/lib/p47-viewer-client.js` now writes a reconnect discovery artifact
  before sending any CDP commands to the rediscovered page websocket.
- Discovery includes the DevTools `/json` page list, chosen page, previous page
  target, selected URL, selected websocket URL, page count, `samePageId`, and
  `sameWebSocketDebuggerUrl`.
- `scripts/run-p46-stress-scenario.js` now requests
  `operator-a-swapped-to-profile-b-reconnect-discovery.json` and
  `operator-b-swapped-to-profile-a-reconnect-discovery.json`.

Goal 2 diagnostic decision:

- The P56 failure was caused by the reconnect command sequence timing out on
  `Page.enable` before the helper could write a reconnect result.
- The next repair is to skip `Page.enable` and `Runtime.enable` on the
  rediscovered page target. The dashboard-state readback and screenshot steps
  use direct CDP commands and do not require event-domain enablement first.
- `scripts/test-p47-viewer-client-separation.js` now asserts reconnect does not
  send `Page.enable` or `Runtime.enable` before dashboard-state readback.

Validation:

```bash
node --check scripts/lib/p47-viewer-client.js
node --check scripts/run-p46-stress-scenario.js
node scripts/test-p47-viewer-client-separation.js
node scripts/test-p47-scenario-harness.js
```

All checks passed. P57 authorizes one S6 retry after the standard live
preflight is green.

P57-authorized S6 retry:

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
/tmp/agent-browser-p46-s6-2026-06-27T20-23-53-164Z
```

Result: failed.

Evidence:

- `operator-a-swapped-to-profile-b-navigate.json` requested profile B.
- `operator-a-swapped-to-profile-b-reconnect-discovery.json` showed the chosen
  DevTools page URL still pointed at profile A.
- `operator-a-swapped-to-profile-b-reconnect.json` was written with
  `domainEnableSkipped: true`, proving the P57 command-sequence repair ran.
- The next dashboard-state read timed out:
  `CDP command Runtime.evaluate timed out after 30000ms`.
- Reset-after closed both retained sessions and final runtime readback showed
  zero sessions, zero browsers, zero tabs, zero active incidents, and both
  route-pool entries available.

P57 did not unlock S6. Follow-up repair moved to
`docs/dev/plans/0058-2026-06-27-s6-dashboard-page-url-stabilization-plan.md`.
