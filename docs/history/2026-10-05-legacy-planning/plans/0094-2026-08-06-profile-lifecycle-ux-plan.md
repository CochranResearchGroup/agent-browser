# Plan 0094: Profile Lifecycle UX

Date: 2026-08-06
State: CLOSED
Lane: P94
Depends On:
- `docs/dev/plans/0093-2026-08-05-stored-profile-browser-opening-plan.md`

## Goal

Let an operator launch a new service-owned browser from any actionable stored
profile row and politely close any live service-owned browser from the
dashboard. Preserve access-plan, ownership, lease, and confirmation gates.

## Current Evidence

- Service status currently projects 485 profile records and 485 allocation
  rows into the dashboard launcher.
- Profile workspace nodes already expose an enabled `Launch` action, but the
  navigator does not route that action into the launcher.
- Service-owned browser nodes already expose `Close`, but navigator
  confirmation only dispatches daemon-port close and therefore does nothing
  for a browser node without a daemon port.
- The Service browser table has a working `service_browser_close` request, but
  only exposes it for the browser whose ID matches the currently selected
  daemon session.

## Authority And Safety

- Reuse the existing no-launch access-plan and exact profile identity flow.
- Opening from a profile row may select and pre-plan the matching launcher
  combination, but must not bypass eligibility or access-plan gates.
- Closing applies only to live service-owned browser records through
  `service_browser_close`; detected non-owned browsers remain lifecycle-disabled.
- Preserve the confirmation dialog and refresh service state after an accepted
  close request.
- Do not close any live browser during implementation unless a dedicated
  disposable canary is created for this plan.

## Slices

1. Add failing dashboard contracts for profile-to-launcher routing and
   service-owned close request routing.
2. Implement the two navigator action paths through existing launcher and
   service request seams.
3. Expose Service browser table Close for every live service-owned browser.
4. Update operator docs and installed skill guidance.
5. Validate focused contracts, production build, disposable live launch/close,
   installed publish, and current retained-browser preservation.

## Done Definition

- Every actionable stored profile row can open the launcher with the matching
  profile selected and its no-launch plan requested.
- Every live service-owned browser row can queue a confirmed polite close.
- Retained and detected non-owned rows stay lifecycle-disabled with reasons.
- Dashboard, docs, installed runtime, ROADMAP, RUNBOOK, commit, and pushed
  `main` agree with the result.

## Closeout

- Profile workspace actions now read `Open browser` and route the exact
  `profileId` into the existing guided launcher. The launcher selects the
  matching browser/profile row and requests its no-launch access plan without
  bypassing capability, readiness, or lease gates.
- Workspace browser Close posts `service_browser_close` through the active
  service endpoint after the shadcn confirmation dialog. Close availability is
  driven by advertised service contract actions.
- The Service browser table now offers Close for every live service-owned
  browser rather than only the browser matching the currently selected daemon
  session. Detected non-owned browser lifecycle controls remain disabled.
- Focused workspace navigator, workspace node, browser table, view stream,
  rendered row-action, selected-context, selected-chat, selected-console,
  launcher, inspector, route-confusion, and Rust output tests pass. Dashboard
  and docs production builds, Rust formatting, strict Clippy, and patch hygiene
  pass.
- The installed runtime manifest reports dashboard SHA-256
  `b2d74b07f2d649f34858c67e3830fc41427818cbbbb4a6fb4b75b0c56fabbb16`
  and executable SHA-256
  `32af83cf90e0940183f83e4e7f02ecd4f1b3b6ffaada96d6863f866c6485e3be`.
  Local and public dashboard routes return HTTP 200.
- Live review showed 485 stored profiles as 1,952 browser/profile combinations.
  The disposable `p0059-discovery-j07-disposable` profile could be selected and
  preflighted, while its missing compatibility evidence remained a visible
  launch block as designed.
- A dedicated `p94-close-canary` browser appeared as live and service-owned.
  Its Close control rendered the browser-specific retained-record explanation,
  and confirmation removed the live session. The `p94-ui-review` session was
  then closed; neither disposable session remains live.
- The operator-owned untracked `--full-page` file remained excluded and
  untouched.
