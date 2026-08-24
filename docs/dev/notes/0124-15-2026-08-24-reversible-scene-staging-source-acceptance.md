# Plan 0124 Reversible Scene Staging Source Acceptance

Date: 2026-08-24

Plan: `docs/dev/plans/0124-2026-08-23-scalable-desktop-evidence-and-presentation-capacity-plan.md`

Authority: SOURCE-ONLY

Status: ACCEPTED

## Accepted Boundary

The configured Desktop Evidence Episode now supports one reversible X11 scene
staging transaction for an exact service-owned presentation slot. Before any
native mutation, the adapter records the browser process identity, display,
active window, authoritative client stacking order, browser-owned window
geometry, and horizontal and vertical maximize state in process-local memory.
Native window identities never enter durable Service State or a public receipt.

Visible staging is restricted to `private_virtual_display`. A shared or ambient
display fails before the native staging provider is invoked. The provider
maximizes and raises the exact PID-owned browser window, asks the window manager
to activate it, and returns only after active-window, topmost-window, work-area
geometry, and unoccluded-region evidence is capture ready.

The presentation slot advances through `staging`, `capture_ready`, and
`restoring` with one incremented scene generation and a pending-restoration
obligation. Completion restores prior maximize state, geometry, relative stack,
and focus, then returns the exact leased slot to its prior `reserved` or
`active` state. A failed stage is rolled back before returning. Any rollback,
restoration, or durable restoration commit that cannot be verified quarantines
the exact slot with an attributable cleanup obligation instead of making it
available for reuse.

An active retained browser remains attached to its slot throughout staging and
restoration. Release of the observation lease does not park or terminate the
browser. Current route, display, controller, and scene authority are rechecked
before restoration so newer human or route intent is not overwritten.

## Validation

- `scripts/ci/cargo-safe.sh fmt --manifest-path cli/Cargo.toml -- --check`
  passed.
- `scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml desktop_evidence -- --nocapture`
  passed 45 tests after the private-display regression was added.
- `scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml desktop_evidence_configured -- --nocapture`
  passed 13 tests after the private-display guard was added.
- `scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml presentation_capacity -- --nocapture`
  passed 15 tests.
- `scripts/ci/cargo-safe.sh clippy --manifest-path cli/Cargo.toml -- -D warnings`
  passed.
- `git diff --check` passed.

## Remaining Boundary

This slice does not claim a real browser-external trigger or paired CDP absence
receipt. It does not install a development runtime or mutate a live browser.
Live X11 convergence, human-viewer precedence, retained-browser preservation,
and installed binary identity remain controlled acceptance work. Configured
production desktop input remains unavailable pending independent Plan 0110
acceptance.
