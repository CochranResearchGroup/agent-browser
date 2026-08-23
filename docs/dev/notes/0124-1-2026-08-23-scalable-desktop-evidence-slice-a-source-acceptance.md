# Plan 0124 Slice A Source Acceptance

Date: 2026-08-23

Status: SOURCE ACCEPTED

Plan: `docs/dev/plans/0124-2026-08-23-scalable-desktop-evidence-and-presentation-capacity-plan.md`

Source baseline: `7b2352548db80dd051064849b7e8c452756cc17b`

## Outcome

Slice A freezes provider-free contracts and fixtures for scalable presentation
capacity and bounded Desktop Evidence Episodes. It does not connect the new
contracts to a browser, display, route, provider, dashboard, or installed
runtime.

The evidence coordinator keeps page and supported modal evidence on CDP,
rejects generic CDP failure as desktop authority, routes sensitive surfaces to
typed human continuation, and selects paired desktop evidence for a
browser-external passkey chooser. Capture-ready proof binds browser and process
generation, route, display allocation, slot, scene, geometry, focus, topmost
state, occlusion, frame mapping, viewer and controller posture, and freshness.

The presentation contracts preserve arbitrary route and slot counts of zero,
one, two, four, six, and eight. The four-slot profile protects one human and one
recovery reserve. Queued requests expose a typed limiting resource, position,
and next safe action, remain FIFO within a priority class, and gain bounded
aging without outranking active human work.

The lifecycle contract deletes only exact owned identities. Ambiguous identity
or any browser, lease, viewer, controller, handoff, rollback, recovery,
restoration, or cleanup reference produces a quarantine receipt with no
deletions. Three deterministic provision and reclaim cycles converge to the
two-slot fixture warm minimum with no live owned resource identity.

## Architecture Guard

`scripts/test-presentation-capacity-architecture.js` proves its detector
recognizes fixed two-entry truncation and alphabetic route configuration. It
then enforces an exact migration baseline over the current route inventory
owners. New assumptions fail the guard, while removing an existing assumption
requires an explicit baseline reduction. Slice B is responsible for driving
that baseline toward the single compatibility adapter.

## Validation

- Evidence fixtures: 9 passed.
- Presentation inventory, capacity, and lifecycle fixtures: 10 passed.
- Presentation capacity architecture guard: passed.
- Rust formatting check: passed.
- Strict default-target Clippy with `-D warnings`: passed.
- Release asset verifier selected by the repository validator: passed.
- `git diff --check`: passed.

An additional non-required all-test-target Clippy diagnostic found existing
warnings in unrelated test modules. The required default-target Clippy gate and
all focused Slice A test targets are green.

## Scope Boundary

No browser, profile, display, route, Guacamole connection, RDP target, process,
dashboard, installed generation, or production state was changed. The modules
remain test-only contract surfaces until their implementation slices establish
the real integration seams. Slice B may generalize static route inventory only;
it does not authorize dynamic provisioning or live provider effects.
