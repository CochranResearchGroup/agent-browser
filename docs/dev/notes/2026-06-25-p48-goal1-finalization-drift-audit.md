# P48 Goal 1 Finalization Drift Audit

Date: 2026-06-25

## Finding

The P47.6 S2 retry did not fail because Guacamole viewing or browser control was absent. `remote-view-open.json` reported `operatorVisible.state = ready`, a ready browser for `session:default`, a ready view stream for `guacamole:3`, and two controllable tabs that later navigated to `https://www.iana.org/domains/reserved`.

The persisted service state diverged after checkout:

- `remoteViewAcquisitionLeases[lease].state = completed` and `phase = checked_out`.
- `routePool["guacamole-rdp-a"].state = pending` with readiness component `remote_view_open_acquisition`.
- `displayAllocations["remote-view-display:13"].state = pending` with the same lease id.
- `remoteViewRoutes["guacamole:3"].state = orphaned` with reason `display_allocation_unavailable`.
- `browsers["session:default"].health = ready` and its view stream still referenced `guacamole:3`.

The exact mismatch is that lease completion finalizes only the lease lifecycle. It does not finalize the route-pool entry, display allocation, route record, and browser stream as one route-bound ownership fact. Health reconciliation then sees the route pointing at a pending display allocation and marks the route orphaned.

## Source Notes

- `remote_view_open_begin_acquisition_lease` writes pending route, display allocation, route-pool, and acquisition lease records.
- `handle_service_remote_view_route_checkout` can produce ready checkout records, but that ready state did not remain authoritative in persisted service state after S2.
- `remote_view_open_complete_acquisition_lease` updates the lease state and phase only.
- `repair_route_pool_service_state` checks only pending leases for stale pending acquisition repair.
- `pending_acquisition_stale_reason` only flags pending acquisition drift when the browser is not ready, but S2 had a ready browser.
- `service_health` marks non-ready display allocations as route orphan causes, which converted the pending display allocation into the `display_allocation_unavailable` incident.

## Test

Added `test_repair_route_pool_service_state_characterizes_completed_lease_pending_drift` in `cli/src/native/actions.rs`.

The test is intentionally a characterization of the current gap. It reproduces a completed lease with pending route-pool and display records plus an orphaned route, then proves the current repair path reports zero stale pending acquisition and zero stale checkout candidates. Goal 2 should replace this behavior with coherent route-bound finalization.
