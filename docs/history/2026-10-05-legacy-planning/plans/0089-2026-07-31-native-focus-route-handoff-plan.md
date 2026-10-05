# Plan 0089: Native Focus for Route-Bound Handoff

State: CLOSED

Created: 2026-07-31

Lane: cold-start successor to Plan 0088

## Goal

Keep route-bound remote-view handoff from stalling on redundant CDP window
focus after exact-target navigation has already succeeded.

## Evidence

- The installed warm LinkedIn feed gate returned operator-visible ready.
- Fresh service and exact cold CLI runs both navigated the feed, then failed at
  roughly fourteen seconds during handoff focus.
- Route-bound handoff already proves the exact active target and independently
  requires visible X11 display content before checkout.

## Bounds

- Use native X11 focus only for the internal route-bound handoff command.
- Preserve ordinary `view_focus` CDP behavior and fail-closed semantics.
- Preserve exact-target, visible-window, operator-visible, checkout, and
  rollback proofs.
- One implementation/build/install/cold-gate cycle; no push or release.

## Acceptance

1. Focused tests prove route-bound focus requests native-only behavior.
2. Formatting, clippy, route-confusion, and remote-view parser gates pass.
3. An installed cold LinkedIn feed gate returns operator-visible ready.

## Result

Focused tests, production clippy, parser tests, and route-confusion gates pass.
The installed cold LinkedIn feed opened in 5.5 seconds with exact-target,
visible-window, and operator-route proof ready. Ordinary `view_focus` retains
its CDP behavior.
