# Plan 0124 Browser External Live Acceptance

Date: 2026-08-24

Plan: `docs/dev/plans/0124-2026-08-23-scalable-desktop-evidence-and-presentation-capacity-plan.md`

Authority: DEVELOPMENT RUNTIME EFFECTS | PRODUCTION READ-ONLY

Status: PARTIAL ACCEPTED

## Accepted Boundary

The development runtime completed one bounded `passkey_chooser` Desktop
Evidence Episode against a local WebAuthn fixture. The exact service-owned
browser, tab, RDP route, private display, process generation, presentation
slot, scene generation, and geometry epoch remained bound through admission,
staging, trigger, paired page-absence evidence, X11 capture, verification,
restoration, release, and cleanup.

The successful episode ran on development generation
`0.28.0-44d9b41d9b2c`. Its installed binary SHA-256 was
`44d9b41d9b2c1a491b3da754bfe7f98710a2d11e9bc2167e7a5d44c20a892cf5`.
The browser was `session:p124-passkey-live-20260824-f`, the route was
`development-route-1`, the display allocation was `development-display-1`
on `:12`, and `operatorVisible.state` was `ready` before the trigger.

Two independent Service State refreshes preserved the checked-out route,
display, and slot ownership. The episode returned `outcome=desktop` with:

- admission receipt
  `presentation-admission:p124-passkey-live-20260824-f:slot:development-slot-1`;
- scene-stage receipt `scene-stage:p124-passkey-live-20260824-f`;
- bounded trigger receipt with digest
  `4f762e8f7195a47c810635b1d2c416bf6a248b057a349c9ce15f1b3f946a8fa8`;
- paired page-absence receipt with digest
  `ff719c555b21647d0636352ef01e1febae904ec18d65fc98b96c4f6b4295489a`;
- fresh ephemeral PNG receipt `desktop-frame-d1198134455acc64a95b0b0a`;
- verification, restoration, slot-release, and cleanup receipts.

The frame was 1280 by 633 pixels and 29,953 bytes. Its receipt reported
`persisted=false` and content SHA-256
`d21b6614b44067cd7afc6b20dd6a7962a7b998a88d51a5b08e655b46b79b2e75`.
Persisted job `r879269` contained the redacted context, frame receipt, and
episode receipts but no frame bytes, page DOM, WebAuthn request content, or
prompt text.

The episode initially exposed three runtime integration defects, all repaired
and retained as regressions:

1. configured provider inventory erased checked-out route and display
   ownership during Service State refresh;
2. stale durable handoff history for absent browsers created phantom
   presentation pressure;
3. closed browsers left orphaned provider ownership instead of converging
   back to inventory-owned available capacity.

The final convergence repair was installed as development generation
`0.28.0-57fe9956d43f`, binary SHA-256
`57fe9956d43f76cc6e9f1553b91ebf2544f5c0ce2dba45ac8bc5c2fa0b58fa76`.
A separate minimal open and close cycle on that generation proved both
directions: a live checkout survived two status refreshes, then close and
reconciliation cleared route and display ownership, returned the pool entry
to `available`, and left zero ready development browsers.

Production remained read-only throughout. The stable production binary
remained SHA-256
`c128349c482fc049b70fe5f3dbfeadd3a9336cdd3ad5f81731dc2cb6b3d5cd63`.

## Source And Validation Evidence

The repair is present through commit `0e93657c` on `main` and `origin/main`.
The relevant structured commits are:

- `faa1430c` binds desktop evidence to the checked-out route session;
- `de386786` preserves only active route checkout state;
- `61087313` retains live checkout across provider inventory refresh;
- `369cea23` ignores stale handoff occupancy when its browser and route stream
  are absent;
- `0e93657c` clears orphaned provider ownership after browser close.

Validation passed:

- 56 focused `desktop_evidence` tests;
- checked-out configuration overlay preservation and closed-owner convergence;
- provider inventory refresh preservation and closed-owner convergence;
- stale durable handoff occupancy regression;
- Rust formatting;
- Rust clippy with warnings denied;
- development-only transactional install with production unchanged.

## Remaining Boundary

Plan 0124 remains in progress. This acceptance does not yet prove active human
controller precedence, passive-viewer non-disruption, two concurrent desktop
observations plus recovery reservation, ordinary CDP-only slot neutrality, or
retained authenticated-browser survival through route movement and unrelated
scale-in. The successful browser-external episode and the final-generation
close convergence are separate receipts; the full browser-external episode
was not repeated after the final cleanup-only overlay change.

