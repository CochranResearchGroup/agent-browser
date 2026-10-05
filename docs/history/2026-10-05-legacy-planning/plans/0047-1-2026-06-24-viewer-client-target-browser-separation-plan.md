# Viewer Client Target Browser Separation Plan

Date: 2026-06-24
State: DONE
Lane: P47.1
Parent Plan:
- `docs/dev/plans/0047-2026-06-24-remote-view-foundational-architecture-plan.md`

## Goal

Separate viewer-client behavior from target-browser behavior so dashboard
operator browsers can observe and exercise viewport UX without becoming
service-owned route-bound browsers, consuming route leases, or publishing
service-owned browser rows.

Compatible `/goal` objective:

```text
/goal execute P47 goal 1: separate viewer-client from target-browser so dashboard operator browsers can observe and control viewport UX without becoming service-owned route-bound browsers or consuming route leases
```

## Audit Findings

P46 S2 failed for a structural reason:

- first S2 attempt used `agent-browser` sessions as dashboard operator
  browsers;
- those operator sessions became service-owned target browsers;
- they attempted route/display acquisition and generated service incidents;
- the second S2 attempt moved to external Chromium, but the implementation was
  still embedded in `scripts/run-p46-stress-scenario.js`;
- launch metadata and readiness diagnostics were added after the lock, but
  viewer-client behavior still does not have its own module or test surface.

Source inspection for this slice found:

- `scripts/run-p46-stress-scenario.js` mixes target-browser operations
  (`remote-view open`, `open`, `get url`, route display inspection) with
  dashboard viewer-client operations (external Chromium launch, dashboard
  auth, viewport refresh, screenshot);
- old helper names still describe operator browsers as external dashboard
  operators rather than viewer clients;
- no focused no-live test proves that the viewer-client path cannot call
  `agent-browser`, `remote-view open`, service session launch, or route
  checkout;
- P46 S2 remains locked, so this slice must not perform a live S2 retry.

Graphiti discovery for `agent_browser_main` was healthy but returned only broad
service-control-plane context. CodeGraph tools were not exposed in this
session, so direct focused source reads were used.

## Desired Module

Create a dedicated viewer-client harness module under `scripts/lib/` with a
small interface:

- resolve the external browser executable;
- build viewer-client launch metadata;
- launch an external dashboard viewer over CDP;
- authenticate against dashboard without writing secrets to artifacts;
- load a workspace URL;
- inspect dashboard viewport state;
- click viewport refresh;
- capture a screenshot;
- close and clean up the external browser process and profile directory.

This module must not import or call `agent-browser`, service session launch,
`remote-view open`, route checkout, route-pool mutation, or service-owned
browser publication.

## Implementation Plan

1. Add `scripts/lib/p47-viewer-client.js`.
2. Move or recreate external Chromium viewer-client behavior from the P46
   runner behind that module.
3. Export pure helpers for no-live validation:
   - dashboard workspace URL builder;
   - dashboard state script builder;
   - verified Chromium executable resolver;
   - viewer-client launch descriptor builder;
   - service-ownership command detector.
4. Update `scripts/run-p46-stress-scenario.js` S2 to use the module.
5. Keep P46 S2 locked. Do not run `pnpm test:p46-stress-scenario -- --scenario
   s2`.
6. Add `scripts/test-p47-viewer-client-separation.js` to prove:
   - viewer-client module source contains no `agent-browser` command usage;
   - launch descriptor role is `viewer-client`;
   - descriptor executable resolves to verified `stealthcdp_chromium` when
     install-doctor says it is ready;
   - descriptor arguments expose CDP and a profile directory but do not contain
     route, session, service, or remote-view commands;
   - service-ownership command detector flags contaminated command vectors.
7. Add a package script for the focused no-live test.

## Validation Plan

Run:

```bash
node --check scripts/lib/p47-viewer-client.js
node --check scripts/run-p46-stress-scenario.js
pnpm test:p47-viewer-client-separation
git diff --check -- scripts/lib/p47-viewer-client.js scripts/run-p46-stress-scenario.js scripts/test-p47-viewer-client-separation.js package.json docs/dev/plans/0047-1-2026-06-24-viewer-client-target-browser-separation-plan.md
agent-browser --json service status
```

The service status check is read-only. It must show no sessions, no browsers,
and zero active incidents after this no-live slice.

## Completion Criteria

This slice is complete when:

- the viewer-client module exists and is used by S2;
- the no-live test proves viewer-client launch metadata and source do not use
  service-owned browser commands;
- P46 S2 remains locked and unrerun;
- runtime service state is clean after validation;
- this plan is updated to `DONE` with validation evidence.

## Execution Result

Completed on 2026-06-24.

Changes:

- added `scripts/lib/p47-viewer-client.js` as the dedicated viewer-client
  harness module;
- updated `scripts/run-p46-stress-scenario.js` so S2 dashboard operator
  browsers use `launchDashboardViewerClient`, `clickDashboardRefresh`,
  `captureDashboardScreenshot`, and `closeViewerClients` from the
  viewer-client module;
- removed the old in-script dashboard operator path based on agent-browser
  session evaluation helpers;
- added `scripts/test-p47-viewer-client-separation.js`;
- added package script `test:p47-viewer-client-separation`.

Validation:

```bash
node --check scripts/lib/p47-viewer-client.js
node --check scripts/run-p46-stress-scenario.js
node --check scripts/test-p47-viewer-client-separation.js
pnpm test:p47-viewer-client-separation
agent-browser --json service status
```

Results:

- parse checks passed;
- no-live viewer-client separation test passed;
- service status after the no-live slice had no sessions, no browsers, and
  zero active incidents.

P46 S2 was not rerun. P46 remains locked for live stress execution.

## Stop Conditions

Stop and update this plan instead of continuing if:

- the only available viewer-client implementation requires `agent-browser`
  session launch;
- dashboard credentials would need to be written into artifacts;
- the no-live test cannot distinguish viewer-client launch from target-browser
  launch;
- validation leaves active service sessions, browsers, or incidents.
