# S9 Stale Target Recovery Plan

Date: 2026-06-27
State: COMPLETE
Lane: P62
Parent: `docs/dev/plans/0046-2026-06-24-remote-view-stress-hardening-plan.md`

## Problem

P46 S9 is locked. The scenario requires duplicate same-origin tabs and an
`about:blank` tab to stay independently addressable through dashboard selected
target recovery.

The S9 runner now records the dashboard recovery path, but the corrected live
run showed that the dashboard keeps rewriting a requested blank-tab URL back to
the recovered duplicate tab after the blank tab is navigated by CLI.

## Evidence

Artifacts:

- first attempt:
  `/tmp/agent-browser-p46-s9-2026-06-27T21-32-27-404Z`;
- corrected attempt:
  `/tmp/agent-browser-p46-s9-2026-06-27T21-42-54-990Z`.

Observed from the corrected attempt:

- requested blank tab:
  `target:92B1ABA4B645E77E3C72BE117CD14832`;
- duplicate tab A:
  `target:1650B99DC1120C753CE97BFE43050090`;
- duplicate tab B:
  `target:AD26C70D100CF68CAD54C74B5D225325`;
- operator C initial dashboard state reported
  `recoveredStaleTab: true` and the recovery notice for the blank target;
- CLI tab selection and navigation of the blank tab succeeded, with final URL
  `https://www.iana.org/domains/reserved?p46=s9-blank-recovered`;
- when operator C was returned to the blank-tab dashboard URL, the dashboard
  page URL stayed on duplicate tab A;
- reset-after closed `default` and the final runtime readback reported zero
  sessions, zero browsers, zero tabs, and zero active incidents.

## Goals

1. Diagnose selected-target recovery for blank tabs after they become live.
2. Decide whether the dashboard should preserve the requested tab identity,
   rebind it after target refresh, or expose a typed recovery state that the
   runner can validate without URL equality.
3. Update product code or the S9 harness contract accordingly.
4. Add no-live checks that prevent the undefined-helper regression and preserve
   the stale-recovery evidence path.
5. Authorize exactly one S9 retry only after the no-live checks pass and the
   runtime preflight is clean.

## Validation Gate

Before another live S9 retry, run:

```bash
node --check scripts/lib/p47-viewer-client.js
node --check scripts/run-p46-stress-scenario.js
node scripts/test-p47-viewer-client-separation.js
node scripts/test-p47-scenario-harness.js
git diff --check -- scripts/lib/p47-viewer-client.js scripts/lib/p46-scenario-harness.js scripts/run-p46-stress-scenario.js scripts/test-p47-viewer-client-separation.js scripts/test-p47-scenario-harness.js
./cli/target/debug/agent-browser --json service incidents --summary
```

Then run one S9 retry from the explicit rebuilt-binary lane:

```bash
node scripts/run-p46-stress-scenario.js --scenario s9 --reset-before --reset-after --agent-browser-command ./cli/target/debug/agent-browser --require-explicit-agent-browser-command --require-agent-browser-daemon-command-match
```

## Result

P62 is complete. The dashboard selected-target contract now distinguishes
between two cases:

- an explicitly selected live blank tab is preserved as the selected tab so a
  later status refresh can prove the same target became navigated;
- a missing, dead, or otherwise stale tab selection still recovers to a current
  live tab and records stale-target recovery evidence.

Implementation:

- changed `packages/dashboard/src/components/workspace-remote-viewport.tsx` so
  `selectedTabForBrowser` honors an explicitly selected live blank tab while
  still marking it as stale-selection evidence;
- loosened the S9 viewer-client recovery helper so recovered stale-tab evidence
  no longer requires a URL rewrite to a different tab;
- changed the S9 evaluator to accept either exact initial blank-target
  selection or typed stale-target recovery before requiring final navigated
  blank-target readback;
- updated no-live source guards in `scripts/test-dashboard-view-streams.js`,
  `scripts/test-p47-viewer-client-separation.js`, and
  `scripts/test-p47-scenario-harness.js`.

Validation:

```bash
node --check scripts/lib/p47-viewer-client.js
node --check scripts/run-p46-stress-scenario.js
node scripts/test-dashboard-view-streams.js
node scripts/test-p47-viewer-client-separation.js
node scripts/test-p47-scenario-harness.js
git diff --check -- packages/dashboard/src/components/workspace-remote-viewport.tsx scripts/lib/p47-viewer-client.js scripts/run-p46-stress-scenario.js scripts/test-dashboard-view-streams.js scripts/test-p47-viewer-client-separation.js scripts/test-p47-scenario-harness.js
pnpm publish:local-dashboard -- --skip-smoke --json
agent-browser --json install doctor
node scripts/smoke-local-dashboard-runtime.js --dashboard-url http://127.0.0.1:4848/ --agent-browser-bin /home/ecochran76/.local/bin/agent-browser --skip-browser --json
node scripts/run-p46-stress-scenario.js --scenario s9 --reset-before --reset-after --agent-browser-command /home/ecochran76/.local/bin/agent-browser --require-explicit-agent-browser-command --require-agent-browser-daemon-command-match
```

Live pass artifact:

```text
/tmp/agent-browser-p46-s9-2026-06-27T22-03-14-950Z
```

S9 passed with exact initial blank-target selection, blank-tab navigation,
duplicate same-origin tab isolation, browser-window-visible route display,
route-bound finalization, one default-profile browser row, and zero active
incidents after reset-after.

P46 may continue at S10.
