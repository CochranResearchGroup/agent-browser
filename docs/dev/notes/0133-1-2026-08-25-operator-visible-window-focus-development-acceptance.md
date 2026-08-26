# Plan 0133 Development Acceptance

Date: 2026-08-25

State: DEVELOPMENT ACCEPTED

Authority: isolated development runtime only. Production remained read-only.

## Qualified Identity

- Runtime source commit: `0d2a30e710b5b29401abb097c256c333f8185178`.
- Branch head after smoke-gate repairs: `336c8f2d`.
- Version: `0.28.0`.
- Candidate SHA-256:
  `a50697a7f4f758c35b4e7e7f659c62d86ce1307a04ef26c1f1e7b79cadc86653`.
- Installed development generation: `0.28.0-a50697a7f4f7`.
- `development-runtime:doctor` passed every runtime, unit, manifest, skill,
  provider, route, display, secret-mode, and port check.
- The transactional development installer reported `Production unchanged:
  true`.

The later branch commit changes only the live fixture harness. It does not
change the qualified Rust executable.

## Source Validation

- The minimized-scene fixture first failed against the old permissive proof.
- Missing process-bound evidence first failed to compile before the strict
  helper existed.
- The redundant new-target switch fixture failed red with
  `switch_target, navigate_target, refresh_targets` and passed green with
  `navigate_target, refresh_targets`.
- All 18 `native::remote_view::open::tests` passed.
- Route action and route open focused modules passed.
- Formatting passed.
- Clippy passed with warnings denied.
- `pnpm test:route-confusion-gates` passed.
- `pnpm test:remote-view-handoff-docs` passed.
- The docs production build passed.
- The full Rust suite recorded 2,449 passes and one unrelated parallel
  temporary-directory cleanup race. The isolated rerun of that existing test
  passed.

## Development Runtime Acceptance

The accepted fixture used a self-contained non-private data page on the
isolated development provider. It did not open a tenant service.

The initial direct open completed in 1.8 seconds and returned:

- browser `session:plan0133-development-data-acceptance`;
- profile `plan0133-development-data-acceptance-profile`;
- browser PID `46817`;
- route `development-route-1`;
- route-pool entry `development-slot-1`;
- display allocation `development-display-1` on display `:12`;
- `operatorVisible.state=ready`;
- `displayContent.state=browser_window_visible`.

The independent operator-presentation observation proved every required
predicate:

- `processBound=true`;
- `mapped=true`;
- `minimized=false`;
- `visibleWorkspace=true`;
- `activeWindowOwned=true`;
- `topmostWindowOwned=true`;
- `authorizedGeometry=true`;
- `captureRegionUnoccluded=true`.

The explicit browser reattach returned `status=reattached`. A separate service
state readback retained PID `46817`, the same browser, profile, session, route,
display allocation, and stream. The refreshed readiness component was
`operator_visible_window`, and attachability was `attached_ready` with the
same strict presentation proof.

Closing the disposable session succeeded. Post-close service state contained
no matching browser, route 1 had no browser or session owner and remained
ready, and slot 1 returned to `available`.

## Acceptance Harness Findings

Two initial `test:remote-view-open-fixture-live` attempts produced misleading
near-timeout failures because the Node process hosted a local fixture server
and then blocked its own event loop with `spawnSync`. The runtime jobs
eventually recorded success when the harness timeout released the event loop.
The fixture now uses a self-contained data page, and repeat opens carry the
exact route-pool entry identity.

The full helper later reached its durable handoff URL assertion. The isolated
development provider intentionally has no published external ingress, so that
assertion is outside this acceptance boundary. The durable URL contract was
not weakened.

The development state also contains older available route aliases keyed
`development-route-1` through `development-route-4` alongside the authoritative
`development-slot-1` through `development-slot-4` identities. Exact slot
routing avoids ambiguity. They were not removed because the supported
route-pool repair action repairs stale checkouts and pending acquisitions but
does not delete available definitions, and direct state-file editing is not an
authorized cleanup mechanism.
