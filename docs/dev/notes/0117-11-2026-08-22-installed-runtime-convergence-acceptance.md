# Plan 0117 terminal installed runtime convergence acceptance

Date: 2026-08-22

## Outcome

Plan 0117 is accepted at the current source and installed-runtime boundaries.
Implementation commit `9fb73d15` built candidate binary SHA-256
`aa21c5fe8a6dd75f1422bd84147756f984ea8662fc5d9a1ea3afac1c37eed452`.
The installed selected generation is
`0.28.0-aa21c5fe8a6d-25828e3b8aed`.

Accepted transaction `upgrade-52684512-bfc2-4c30-971b-ab166eaa5364`
completed every workstation phase through managed candidate dashboard and
session-supervisor rebinding. The terminal transaction result is `accepted`.
The reviewed finalization transition marked the prior generation retirable.

## Runtime convergence

The final install doctor succeeded with no issues and reported:

- one logical dashboard backend behind the stable ingress
- one runtime-host daemon
- one executable generation across managed Agent Browser processes
- zero legacy per-session daemons
- steady-state multiplicity and converged runtime status
- ready dashboard ingress and authenticated operator journey

The stable ingress and selected backend execute the same installed generation.
The extra stable-ingress router process is not a second logical dashboard.

## Browser and handoff preservation

The managed `last30days-facebook` Chromium process retained PID 27742 and its
original process start identity throughout the accepted transaction. Durable
handoff `r520477` resolved on the candidate dashboard with
`operatorVisible.state=ready`, retained logical browser
`session:plan0117-final-runtime`, and returned HTTP 200 at its opaque handoff
URL after convergence. No replacement browser or replacement operator handoff
was created.

## Repairs proven by the live transaction

- managed launch admission now rejects a conflicting live owner before Chrome
  starts and rolls back a just-launched browser if later persistence fails
- lifecycle registration preserves the canonical logical browser across
  transferred daemon aliases
- idle lanes on a shared runtime host defer process retirement to host cutover
- accepted cutover finalizes browser lanes, then retires the old shared host
  once through exact generation, socket, PID, and process-start evidence
- supervisor manifests rebind to the accepted selected generation
- only the latest accepted transaction can pin rollback retention

The final shared-host retirement repair has a live acceptance receipt because
the old host exited while the transferred Chromium process, target, profile,
route, display, and durable handoff stayed ready.

## Garbage collection and pressure

Service GC dry-run reported zero candidates and no warnings. Workstation
generation GC dry-run identified only the obsolete rollback generation
`0.28.0-bcfab70c2be9-7ad9e5b748d3`; reviewed apply removed exactly that
generation and retained the selected live generation. The final generation
directory inventory contains only the selected generation.

A fresh operating-system readback found one package-managed browser root, PID
27742 on the named persistent profile. No second default-profile browser root
remained. Ambiguous and unrelated browser workloads were not terminated.

## Validation

- Rust formatting check passed
- strict Clippy with warnings denied passed
- 85 workstation-install tests passed
- 12 lifecycle, 8 retention, and 11 supervisor focused tests passed
- source-free workstation install fixture passed
- workstation host-provision fixture passed
- fresh-workstation VM harness contract passed
- Guacamole asset, PostgreSQL durability, and route-user synchronization gates
  passed
- remote-view handoff documentation guard passed
- production docs build passed
- repository and installed Agent Browser skills have matching SHA-256
  `2769b9ecf967a4efa375590d1418bbf0f8344ce541ecf4ab856fa42ee7aca367`
- final install doctor passed after reviewed generation GC

## Residual boundary

This checkpoint is not a formal release. Release preparation remains subject
to the repository's explicit many-to-many Guacamole/RDP milestone and manual
release workflow.
