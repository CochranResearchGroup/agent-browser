# Plan 0124 Configured Scene And Capture Source Acceptance

Date: 2026-08-24

Plan: `docs/dev/plans/0124-2026-08-23-scalable-desktop-evidence-and-presentation-capacity-plan.md`

Authority: SOURCE-ONLY

Status: ACCEPTED

## Accepted Boundary

The observation-only Desktop Evidence Episode now has a configured read-only
window-semantic provider and a configured desktop-frame provider.

The window provider resolves the exact service-owned browser, process
generation, route, display allocation, presentation-slot lease, scene
generation, viewer posture, and controller posture. Its native X11 probe reads
the active window, authoritative stacking order, work-area geometry, exact PID
ownership, and intersecting occlusion without changing focus, stacking,
geometry, route state, or input state.

Capture-ready evidence now includes frame dimensions, full-frame crop, scale,
coordinate space, and the same geometry epoch used by the configured desktop
capture path. The frame adapter rejects any browser, route, display, geometry,
dimension, crop, scale, or coordinate-space drift. Capture failures are typed
terminal episode failures that still restore when authority permits, release
the slot, and complete cleanup.

## Safety Outcomes

- An active human controller blocks the episode before capacity reservation.
- A passive viewer permits only an already capture-ready unstaged observation.
- Missing `_NET_CLIENT_LIST_STACKING` evidence cannot prove topmost or
  unoccluded posture.
- An unowned intersecting window fails capture readiness.
- A non-maximized browser fails authorized-geometry proof.
- Service authority is read before and after the native scene probe; any
  process, route, display, slot, scene, viewer, or controller drift fails
  closed.
- Configured production input remains unavailable pending independent Plan
  0110 live acceptance.

## Validation

- `scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml x11_scene::tests -- --nocapture`
  passed 5 tests.
- `scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml desktop_evidence_configured::tests -- --nocapture`
  passed 7 tests.
- `scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml desktop_evidence::tests -- --nocapture`
  passed 20 tests.
- `scripts/ci/cargo-safe.sh fmt --manifest-path cli/Cargo.toml -- --check`
  passed.
- `scripts/ci/cargo-safe.sh clippy --manifest-path cli/Cargo.toml -- -D warnings`
  passed after the one bounded readability repair.
- `git diff --check` passed.

## Remaining Boundary

One product caller still needs the configured paired-CDP, staging,
verification, durable-handoff, release, and cleanup adapters composed around
these accepted capacity, scene, and frame providers. Installed human, passive
viewer, CDP-only, and retained-browser acceptance remains unrun. No installed
runtime or live browser state changed in this slice.
