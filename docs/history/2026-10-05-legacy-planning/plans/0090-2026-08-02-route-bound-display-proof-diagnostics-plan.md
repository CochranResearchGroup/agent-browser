# Plan 0090: Route-Bound Display Proof Diagnostics

State: CLOSED

Created: 2026-08-02

Lane: last30days X successor hard-stop diagnosis

Source evidence:

- last30days Plan 0018 checkpoint C63
- last30days receipt `docs/dev/notes/0034-x-profile-handoff-repair-and-successor-stop.json`
- retained agent-browser acquisition leases at 2026-08-02T21:22:07Z and
  2026-08-02T21:22:51Z

## Goal

Remove the redundant ambient-display discovery gate from an already
route-bound visible-window proof and preserve its exact bounded probe failure
through CLI error rendering.

## Bounds

- Keep arbitrary dashboard and unbound display probes guarded by configured
  route discovery.
- Allow only an exact display carried by a normalized route binding to use the
  bound-route probe path.
- Preserve the capped underlying display-probe reason in visible-window proof
  failures.
- Preserve typed remote-view proof errors in AI-friendly CLI output.
- Add focused no-launch Rust regressions before implementation.
- One implementation pass, one review and rework cycle, and no live browser or
  last30days source attempt in this packet.
- Preserve the operator-owned untracked `--full-page` file.
- No push, release, tag, or publication.

## Acceptance

1. A route-bound display probe does not fail merely because ambient route
   discovery does not list that exact display.
2. Unbound display probes retain their current configured-route guard.
3. A display-probe failure reports its capped underlying reason.
4. AI-friendly error rendering preserves the structured remote-view proof
   failure rather than replacing it with a generic page timeout.
5. Focused tests, formatting, strict Clippy, and relevant Rust validation pass.

## Next Action

- Return to last30days Plan 0018 and create one separately reviewed, bounded X
  successor checkpoint. The installed agent-browser runtime and route-bound
  proof gate are now satisfied; do not reuse the failed C63 run or bypass its
  exhausted attempt budget.

## Candidate Result

- The already normalized remote-view binding now probes its exact display
  without repeating ambient route discovery; unbound probes retain the
  configured-route guard.
- Visible-window failures preserve a sanitized, 240-character maximum probe
  reason, and AI-friendly rendering preserves the structured failure.
- Forty-seven focused tests and all twenty-nine `remote_view_open` tests pass.
  Rust formatting and strict Clippy pass.
- The first independent review found one blocking coverage gap: the probe-error
  regression did not exercise sanitization or the 240-character cap. The one
  allowed rework cycle now covers multiline, quoted, over-limit input and
  asserts the sanitized bounded result; thirty remote-view unit tests and all
  twenty-nine `remote_view_open` tests pass after rework.
- The complete Rust suite reported 1,755 passed, 57 ignored, and two failures.
  The authenticated-target failure passed alone. The unknown-command fixture
  remains sensitive to the installed browser-recovery retry state and still
  fails alone before reaching its expected unknown-command assertion; it does
  not exercise the files or paths changed by this plan.
- No browser or source attempt was launched. The operator-owned untracked
  `--full-page` file remains untouched.
- Independent re-review passed exact commit `116ee810`.
- The no-browser local publisher built candidate binary SHA-256 `a99728c56a57a80bd89ad1bc4e8c8d4a1d1af7bc08e2d52919ea0e384a5d7211`
  but could not quiesce the active `litscout-plan0311` daemon. It rolled back
  the executable handoff and resumed the prepared `litscout-0312` session.
  Install doctor then passed with the supported prior executable SHA-256
  `cc22abe43a069e55e2dd46598b3eaa4954ffd4b8859388f646d7761c6c05da60`;
  all three active daemons match that installed executable and the dashboard
  service is active. The reviewed repair is therefore implemented and
  validated in the repo, but not installed.
- After the litscout owner paused its workflow, the no-browser publisher
  succeeded and installed candidate SHA-256 `a99728c56a57a80bd89ad1bc4e8c8d4a1d1af7bc08e2d52919ea0e384a5d7211`.
  Dashboard smoke passed and `litscout-0312` reattached ten targets. Install
  doctor then failed closed because the source-free manifest still records the
  previous executable and the bundled privilege helper differs from the
  installed root-owned helper. Workstation reconciliation requires interactive
  sudo and was not completed in this non-interactive turn.
- The operator subsequently completed the interactive workstation
  reconciliation. `agent-browser install doctor --json` now passes with no
  issues, the installed executable and sole live daemon share SHA-256
  `a99728c56a57a80bd89ad1bc4e8c8d4a1d1af7bc08e2d52919ea0e384a5d7211`,
  the dashboard is ready, and runtime convergence reports no stale or
  diagnostic runtimes.
- `agent-browser doctor remote-view --json` is ready with both route displays
  accessible, the route pool ready, and external Guacamole ingress ready. A
  direct installed-binary Route B gate opened the existing dashboard through
  `guacamole:2`, proved `browser_window_visible`, operator visibility, and
  attachability in one target readback, then closed the disposable session;
  the post-close doctor remained ready and no route allocation remained.
- The repository fixture harness separately stalled its in-process HTTP
  fixture at navigation because its synchronous CLI child blocks that same
  Node event loop. It nevertheless reached the repaired visible-window proof
  before the unrelated navigation timeout and cleaned up its disposable
  browser, session, route, and display. Treat that harness defect as separate
  follow-up; it does not invalidate the installed product-path gate above.
