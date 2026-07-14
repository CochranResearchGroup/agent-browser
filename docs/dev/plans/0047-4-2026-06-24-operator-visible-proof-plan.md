# P47.4 Operator-Visible Proof Plan

Date: 2026-06-24
State: DONE
Lane: P47.4
Parent Plan: `docs/dev/plans/0047-2026-06-24-remote-view-foundational-architecture-plan.md`

## Goal

`/goal execute P47 goal 4: deepen operator-visible proof so remote-view success returns one ready proof or one typed blocker covering CDP target, route display, Guacamole route, dashboard viewport, and selected tab freshness`

## Audit Findings

- `remote_view_open_operator_visible` already returns one `operatorVisible`
  object with route, display, browser, tab, stream, and Guacamole components.
- The state-priority rules were embedded in `actions.rs`, which makes the
  proof contract harder to test without exercising the whole command path.
- Dashboard inventory already consumes structured proof states such as
  `wrong_tab`, `cdp_target_unavailable`, `stale_route_record`, and
  `guacamole_route_unavailable`.

## Implementation Plan

1. Add a runtime proof module for the operator-visible state decision.
2. Keep the existing JSON response shape intact.
3. Cover success, route blocker, display blocker, wrong-tab blocker,
   Guacamole blocker, and missing target cases with unit tests.
4. Re-run the existing `remote_view_open_operator_visible` regression so the
   extracted proof state still reaches callers as before.

## Validation

- PASS: `cargo test --manifest-path cli/Cargo.toml remote_view_proof -- --test-threads=1`
- PASS: `cargo test --manifest-path cli/Cargo.toml remote_view_open_operator_visible -- --test-threads=1`
- PASS: `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- PASS: `git diff --check -- cli/src/native/remote_view_proof.rs cli/src/native/mod.rs cli/src/native/actions.rs docs/dev/plans/0047-4-2026-06-24-operator-visible-proof-plan.md`

## Closeout

Added `cli/src/native/remote_view_proof.rs` for the operator-visible proof
state decision and wired `remote_view_open_operator_visible` through it. The
existing response shape stays intact while success and blocker priority are
unit-tested independently from the command path.
