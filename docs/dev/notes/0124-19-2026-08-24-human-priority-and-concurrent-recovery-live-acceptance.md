# Plan 0124 Human Priority And Concurrent Recovery Live Acceptance

Date: 2026-08-24

Plan: `docs/dev/plans/0124-2026-08-23-scalable-desktop-evidence-and-presentation-capacity-plan.md`

Authority: DEVELOPMENT RUNTIME EFFECTS | PRODUCTION READ-ONLY

Status: ACCEPTED

## Accepted Boundary

Active-human precedence, passive-viewer non-disruption, and concurrent
multi-agent desktop observation plus recovery admission are live accepted in
the isolated development runtime.

The human-priority episode used browser
`session:p124-precedence-20260824`, route `development-route-1`, display
`development-display-1`, and observer lease
`viewer:development-route-1:p124-observer-1:2026-08-25T01-19-07-707382729Z`.
An unstaged `stacking_or_occlusion` observation completed without changing the
observer. After controller takeover, a staged `passkey_chooser` request
returned `outcome=human_continuation` with reason
`human_controller_has_precedence` and durable handoff `r793522`. The request
did not mutate slot or scene authority.

The concurrent episode used three distinct live bindings:

- observation A used `session:p124-concurrent-a-v2`,
  `development-route-1`, `development-display-1`, and slot 1;
- observation B used `session:p124-concurrent-b-v2`,
  `development-route-2`, `development-display-2`, and slot 2;
- recovery used `session:p124-concurrent-recovery-v2`,
  `development-route-3`, `development-display-3`, and slot 3.

Observation job `r140094` ran from
`2026-08-25T02:33:55.350356808Z` through
`2026-08-25T02:33:56.027233739Z`. Observation job `r246560` ran from
`2026-08-25T02:33:55.563388886Z` through
`2026-08-25T02:33:56.168119203Z`. Recovery job
`mcp-service-request-service_remote_view_browser_reattach-cb87e0e7-1740-4b48-84a1-d3bd6d113843`
ran from `2026-08-25T02:33:55.668568665Z` through
`2026-08-25T02:33:55.971766902Z`. All three jobs therefore overlapped for a
bounded interval. Recovery request
`remote-view-recovery:session:p124-concurrent-recovery-v2:2026-08-25T02:33:55.817797542Z`
was admitted on slot 3 and released. Passive observer lease
`viewer:development-route-3:p124-concurrent-viewer-v2:2026-08-25T02-31-53-583022119Z`
remained observing through the episode. All browser, route, display, and slot
bindings remained exact.

## Cleanup Convergence

The live episode exposed two inverse lifecycle gaps. Route checkout did not
activate its bound capacity slot, and browser close could leave an unleased
active slot plus an observing lease after route ownership disappeared. Source
repairs now:

- activate and release capacity from exact route, display, and browser
  ownership;
- reserve recovery capacity through the existing reattach and route-switch
  interfaces;
- disconnect a viewer whose route no longer owns a ready browser;
- derive unleased warm and active slots from authoritative route and browser
  ownership;
- persist the derived capacity through reconciliation only when no concurrent
  capacity mutation occurred, preserving an in-flight recovery lease.

The final repair is present through commit `83672d09` on `main` and
`origin/main`. The relevant commits are:

- `8964dbf8` admits exact-route remote-view recovery capacity;
- `543da5e7` synchronizes route checkout and release with presentation slots;
- `4587e8f4` reconciles ownerless viewers and presentation ownership;
- `83672d09` persists capacity reconciliation through the repository merge.

The final installed development generation is
`0.28.0-32265fb11862`, with binary SHA-256
`32265fb118629999997d4d2794d1ff5f4bebeb48b0e8bdb8be203b2f89d8c6ac`.
Development doctor passed with all three development units active on that
generation. One bounded reconcile then persisted:

- zero browser records;
- zero routes with browser ownership;
- zero active viewer leases;
- four `warm_idle` slots with no browser or lease request IDs.

The older stale lease
`viewer:development-route-3:p124-concurrent-viewer:2026-08-25T02-09-12-048370321Z`
is now `disconnected` with `lastViewerEvent=route_unavailable`.

Production remained read-only and unchanged. Its selected generation remains
`0.28.0-c128349c482f-d9745dc2e128`, with binary SHA-256
`c128349c482fc049b70fe5f3dbfeadd3a9336cdd3ad5f81731dc2cb6b3d5cd63`.

## Validation

Validation passed:

- 77 focused service-health and reconciliation tests;
- 16 presentation-capacity tests;
- remote-view reattach, route-switch, and presentation-capacity focused tests
  from the preceding source slice;
- Rust formatting and clippy with warnings denied;
- documentation build and remote-view documentation guard from the preceding
  source slice;
- development workstation host, fresh-VM, Guacamole asset, PostgreSQL
  durability, and route-specific user-sync checks;
- development doctor and exact persisted-state readback.

The workstation installer fixture did not produce a result: its Node harness
remained idle after the child installer exited and was interrupted after more
than thirteen minutes. This is retained as a validator-harness issue, not
counted as acceptance evidence and not treated as a runtime failure.

## Remaining Boundary

Plan 0124 remains in progress. The next installed gate is retained
authenticated-browser survival through parking, route movement, unrelated
scale-in, reconciliation, and dashboard refresh. A complete browser-external
episode must then be repeated on the final installed generation. The remaining
legacy `development-presentation-provider-v1-2` lifecycle row also requires
safe convergence or explicit protected disposition.
