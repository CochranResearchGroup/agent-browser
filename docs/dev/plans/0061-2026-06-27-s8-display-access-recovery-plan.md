# S8 Display Access Recovery Plan

Date: 2026-06-27
State: COMPLETE
Lane: P61
Parent: `docs/dev/plans/0046-2026-06-24-remote-view-stress-hardening-plan.md`

## Problem

S8 requires proof that display-access failures are diagnosed, fail closed, and
recover through the supported open path without leaving fake live rows,
terminal fallback, or stale route-pool state.

The scenario must be safe to run on the shared workstation. It must not mutate
host X11 permissions merely to create a failure.

## Implementation

- Added S8 metadata to the P46 scenario harness.
- Added a safe display-access denial fixture to the runner by prepending a
  temporary `timeout` shim to `PATH` for the denial probe only.
- Captured denied-open, post-denial service status, incident summary, display
  inspection, and remote-view doctor artifacts.
- Reran the same route-bound open with normal display access as the recovery
  proof.
- Added evaluator checks for typed display-access blocker, no retained denied
  browser row, clean route-pool rollback, no terminal fallback, successful
  recovery open, route-bound finalization, and display-access grant evidence.
- Added no-live test guards for S8 metadata, fixture use, denial artifacts,
  recovery artifacts, and evaluator requirements.

## Validation

```bash
node --check scripts/run-p46-stress-scenario.js
node scripts/test-p47-scenario-harness.js
git diff --check -- scripts/lib/p46-scenario-harness.js scripts/run-p46-stress-scenario.js scripts/test-p47-scenario-harness.js
node scripts/run-p46-stress-scenario.js --scenario s8 --reset-before --reset-after --agent-browser-command ./cli/target/debug/agent-browser --require-explicit-agent-browser-command --require-agent-browser-daemon-command-match
```

Live pass artifact:

```text
/tmp/agent-browser-p46-s8-2026-06-27T21-07-22-844Z
```

## Result

S8 passed. The denied open returned `display_access_grant_failed` with cleanup
before browser launch, no retained denied-profile browser row, route-pool
entries restored available, and terminal-free displays. The recovery open
reported `displayAccessGrant.state: already_ready`, became
browser-window-visible on route A, finalized route-bound state, and reset-after
closed the repair session with zero active incidents.

P46 may continue at S9.
