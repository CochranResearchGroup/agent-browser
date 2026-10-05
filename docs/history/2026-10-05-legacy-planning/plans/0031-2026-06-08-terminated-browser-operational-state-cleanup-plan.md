# Plan 0031: Terminated Browser Operational State Cleanup

## Goal

Stop writing post-termination browser records back into service operational state. Browser shutdown and process-exit history should remain in events, incidents, traces, and logs, while `ServiceState.browsers`, linked sessions, and live tab references describe only current or recoverable work.

## Evidence

- `persist_closed_browser_health_in_repository` records a successful operator close as a retained `NotStarted` browser row.
- `persist_process_exited_browser_health_in_repository` records unexpected process exit as a retained `ProcessExited` browser row.
- `reconcile_service_state` can detect a dead PID, mark the browser `ProcessExited`, and merge that retained terminal browser record back into persisted state.
- Plan 0030 already hid post-termination records in the dashboard, but the backend still hoards newly terminated rows.

## Scope

1. Add a shared service-state cleanup helper for terminated browser operational rows.
2. Use it after successful operator close and unexpected process-exit recording.
3. Make reconciliation remove browser records it proves are process-exited after recording the health transition event.
4. Preserve degraded/faulted close outcomes because those still represent recoverable or diagnostic action.
5. Update Rust unit tests to assert events remain while browser/session/tab operational rows are removed.

## Validation

- `cargo test --manifest-path cli/Cargo.toml native::service_health`
- `cargo test --manifest-path cli/Cargo.toml native::control_plane`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm validation:select -- --base HEAD`
- `git diff --check`

## Result

Implemented on 2026-06-08. Successful operator close and unexpected process-exit recording now preserve browser-health events and release or orphan remote-view state, then remove the terminated browser from `ServiceState.browsers`. Linked tabs and empty linked sessions are removed from operational state at the same boundary. Reconciliation records health and tab lifecycle transitions first, removes process-exited browser history, and merges those removals back into persisted state when the target rows did not change concurrently.

Validation passed:

- `cargo test --manifest-path cli/Cargo.toml native::service_health -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml native::control_plane -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml service_health -- --test-threads=1`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm validation:select -- --base HEAD`
- `git diff --check`
