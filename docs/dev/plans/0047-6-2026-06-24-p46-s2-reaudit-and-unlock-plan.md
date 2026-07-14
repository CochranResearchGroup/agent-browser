# P47.6 P46 S2 Re-Audit And Unlock Plan

Date: 2026-06-24
State: DONE
Lane: P47.6
Parent Plan: `docs/dev/plans/0047-2026-06-24-remote-view-foundational-architecture-plan.md`

## Goal

`/goal execute P47 goal 6: after goals 1 through 5 are validated, re-audit P46 S2 from a clean runtime and either unlock one S2 retry or keep P46 locked with a new evidence-backed blocker`

## Audit Plan

1. Run no-mutation preflight:
   - install doctor;
   - remote-view doctor;
   - service status;
   - route-pool readiness;
   - route display content inspection;
   - dashboard HTTP readback;
   - P47 scenario harness validation.
2. If preflight is dirty, keep P46 locked and write the blocker.
3. If preflight is clean, run exactly one S2 retry with `--reset-before` and
   `--reset-after`.
4. Close P47.6 based on the S2 retry artifact.

## Preflight Evidence

Artifact directory:
`/tmp/agent-browser-p47-6-reaudit-2026-06-24`

Observed clean signals:

- install doctor success: true;
- remote-view doctor success: true;
- service status success: true;
- active incidents: 0;
- live service sessions: 0;
- live service browsers: 0;
- ready route-pool entries: 2;
- display inspection success: true;
- dashboard HTTP: `200 http://127.0.0.1:4848/`;
- harness validation: PASS.

## Validation

- FAIL, P46 remains locked: `pnpm test:p46-stress-scenario -- --scenario s2 --reset-before --reset-after --artifact-dir /tmp/agent-browser-p47-6-s2-retry-2026-06-24`

## S2 Retry Result

Artifact directory:
`/tmp/agent-browser-p47-6-s2-retry-2026-06-24`

Useful proof from the retry:

- one S2 target browser row: `session:default` with profile
  `p46-s2-profile`;
- route id: `guacamole:3`;
- display: `:13`;
- display state after navigation: `browser_window_visible`;
- operator A and operator B both rendered the remote viewport;
- both operators used the same frame URL:
  `http://127.0.0.1:8092/guacamole/#/client/MwBjAHBvc3RncmVzcWw=`;
- operator B refresh control clicked successfully;
- controlled browser URL after navigation:
  `https://www.iana.org/domains/reserved`;
- route display screenshot was captured;
- operator dashboard screenshots were captured;
- reset-after closed `default` and returned to zero active incidents.

Failure:

- S2 reported one active incident before reset-after:
  `remote-view-route:guacamole:3`.
- Incident message:
  `Remote route 'guacamole:3' is orphaned: orphaned display_allocation display_allocation_unavailable`.
- Related service-state row:
  `remote-view-route-pool:guacamole-rdp-a` remained in service state as
  pending `remote_view_open_acquisition`.

Decision:

P46 stays locked. The old viewer-client/extra-browser failure is no longer the
active blocker for this retry. The new blocker is route/display allocation
finalization or reconciliation drift after an otherwise functional S2 run.
