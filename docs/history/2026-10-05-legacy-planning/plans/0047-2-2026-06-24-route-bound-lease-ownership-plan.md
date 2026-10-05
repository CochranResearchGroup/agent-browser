# P47.2 Route-Bound Lease Ownership Plan

Date: 2026-06-24
State: DONE
Lane: P47.2
Parent Plan: `docs/dev/plans/0047-2026-06-24-remote-view-foundational-architecture-plan.md`

## Goal

`/goal execute P47 goal 2: deepen route-bound lease ownership so route, display, browser, tab, proof, finalize, and rollback are owned by one typed module instead of reinterpreted across remote-view code paths`

## Audit Findings

- `RemoteViewAcquisitionLease` persists `state` and `phase` as strings, so ownership state is not typed at the model boundary.
- `remote_view_open_begin_acquisition_lease`, `remote_view_open_complete_acquisition_lease`, and `remote_view_open_rollback_acquisition_lease` mutate lease state directly.
- `remote_view_open` already has a defensible order: plan, reserve, display access, browser launch or reuse, tab acquire, focus, visible proof, checkout, then complete. The missing foundation is a shared lifecycle gate that prevents callers from publishing a finalized target from partial state.

## Implementation Plan

1. Add a typed route-bound lease lifecycle module with states from `requested` through `finalized`, `rolled_back`, and `failed_diagnostic`.
2. Add unit tests for ordered success, illegal skips, rollback, and persisted state/phase round trip.
3. Wire begin, complete, and rollback to stamp persisted `RemoteViewAcquisitionLease` values from the typed lifecycle instead of duplicated string literals.
4. Keep the external JSON contract compatible by preserving current persisted `state` and `phase` values for reserved, checked-out, and rollback-complete leases.

## Validation

- PASS: `cargo test --manifest-path cli/Cargo.toml remote_view_lease -- --test-threads=1`
- PASS: `cargo test --manifest-path cli/Cargo.toml remote_view_open_acquisition_lease_rollback -- --test-threads=1`
- PASS: `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- PASS: `git diff --check -- cli/src/native/remote_view_lease.rs cli/src/native/mod.rs cli/src/native/actions.rs docs/dev/plans/0047-2-2026-06-24-route-bound-lease-ownership-plan.md`

## Closeout

Added `cli/src/native/remote_view_lease.rs` as the typed lifecycle authority for
route-bound acquisition lease state. Wired begin, complete, and rollback
mutation boundaries through the lifecycle while preserving the existing
external JSON `state` and `phase` contract.
