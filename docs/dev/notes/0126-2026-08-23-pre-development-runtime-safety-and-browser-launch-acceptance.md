# P126 Runtime Safety And Browser Launch Acceptance

Date: 2026-08-23

Result: ACCEPTED

Authority: SOURCE AND DEVELOPMENT RUNTIME EFFECTS | REVIEWED TERMINAL PROCESS CLEANUP | PRODUCTION READ-ONLY

## Accepted Repairs

Development service resource classification now requires positive current
Runtime Environment ownership before any global process can become a garbage
collection candidate. A process bound to a development browser, development
profile root, selected development generation, or development command namespace
can remain eligible. An uncorrelated process is protected with
`runtime_environment_ownership_unproven`. Production classification behavior
is unchanged.

The development publisher now selects and validates one host-compatible browser
executable. Installed generation `0.28.0-b3dc87dcc29a`, its stable launcher,
runtime host, dashboard backend, dashboard unit, generation manifest, status,
and doctor all agree on `/opt/google/chrome/chrome`. A caller can still supply
an explicit one-command diagnostic override.

## Browser Launch Acceptance

Before repair, this exact development flow failed three times in under two
seconds with Chrome exit code zero before DevTools and empty stderr:

```bash
agent-browser-dev --session p126-fresh-launch \
  --runtime-profile p126-fresh-launch --json open about:blank
```

The same flow with an explicit Linux Chrome path succeeded, isolating the cause
from profile locks, sandboxing, and workstation pressure. After development-only
publication, the original command succeeded without an override. The installed
smoke then completed three consecutive disposable cycles. Every cycle opened
`about:blank`, read back `about:blank`, closed the exact named session, found no
matching process, and moved its profile to trash. The smoke's production
before-and-after identity assertion passed.

## Garbage Collection Acceptance

The original development dry-run proposed production-protected Xvfb PIDs 8319,
30094, and 68300 because process discovery was global while correlation used
only development Service State. The reviewed token was never applied.

Regression fixtures prove that an uncorrelated display is protected in
development, an exact development-owned lifecycle remains eligible, and the
equivalent production orphan behavior remains unchanged. Fresh installed
dry-runs now report:

- development candidates: 0;
- production candidates: 0;
- development policy:
  `requiresRuntimeEnvironmentOwnershipForCandidates=true`.

Follow-up resolution on 2026-08-23: remote-display classification now requires
the sampled executable basename to be exactly `xvfb`; arbitrary command
arguments cannot supply process kind. A red regression reproduced the
diagnostic-shell false positive, then passed with the repair while real Xvfb
and retained-display fixtures remained green. Installed development generation
`0.28.0-16cf2763100d` excluded a live zsh process carrying
`Xvfb-diagnostic` in argv from both service resources and GC. Production
remained on its prior generation and the only before-and-after changes were
live CPU, RSS, lease timestamp, and readiness timestamp observations.

## Reviewed Terminal Cleanup

The two-day-old Chrome root PID 72225 was bound to temporary test home
`/tmp/agent-browser-managed-runtime-devtools-file-home-71134-1787349850851845576`.
Immediately before termination, the root still had start token `12280196`,
executable `/opt/google/chrome/chrome`, process group 72225, and the exact test
profile path. Production and development status and resource correlation had no
browser ID, session, profile ID, or retained lifecycle owner for the root.

The whole exact process group exited after `SIGTERM`; no force signal was used.
No group member or profile reference remained. The exact test home and the
closed `p126-fresh-launch` and `p126-linux-control` diagnostic profiles were
moved to desktop trash and are recoverable.

## Production Non-Interference

Production stayed read-only and remains on selected generation
`0.28.0-4b975a51aa89-d0782705d5ff` with runtime host PID 75310 and dashboard
PIDs 87827 and 87877. The runtime multiplicity readback is steady with one
generation, one runtime host, one dashboard process, and zero legacy daemons.
Three retained browsers, 63 sessions, nine retained displays, handoff state,
and route state survived the development publication and browser smoke.

The installed production doctor reports source-versus-installed binary drift
because P126 source was intentionally installed only into development. It also
reports the pre-existing optional supervisor warning. Neither condition changed
the selected production runtime, and the final production GC dry-run is empty.

## Final Pressure Readback

The final OS census observed 88 Chrome-family processes across eight root
trees, using approximately 6.45 GiB RSS. The visible roots are mixed-owner
production Agent Browser, AuraCall, and other browser workloads. Development
reports 119 observed service-resource processes using approximately 7.01 GiB,
zero candidates, and six observed unowned Agent Browser-family processes.

This pressure remains an observation, not cleanup authority. P126 reclaimed the
exact package test residue and left active, retained, foreign, and unknown
workloads untouched. At the final readback the host had 70 GiB memory, 51 GiB
available, and 10 GiB free swap.

## Validation

The accepted gates are:

- development publisher fixtures;
- development environment ownership and production-behavior Rust fixtures;
- three-cycle installed development browser smoke;
- installed development doctor;
- fresh production and development GC dry-runs;
- Rust formatting and strict Clippy;
- documentation production build;
- remote-view handoff documentation guard;
- release asset verifier fixture;
- repository and installed skill equality;
- source diff hygiene.

P124 remains unstarted. Its next authority is Slice A source and provider-free
fixtures only.
