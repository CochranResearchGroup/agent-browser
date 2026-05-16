# CI Rust PID Fixture RCA

## Summary

Fast CI failed repeatedly in the `Rust` job while local no-launch client and docs checks passed. The failing gate was the Rust unit partition `native::control_plane::tests`.

## Root Cause

`native::control_plane::tests::service_status_response_combines_worker_and_service_state` used PID `42` as a dead persisted browser PID. On GitHub Ubuntu runners, PID `42` can exist. When it exists, `refresh_browser_record_health()` treats the process as live, probes the persisted CDP endpoint, and reports `cdp_disconnected` instead of `process_exited`.

The test expected `process_exited`, so the result depended on host process state rather than deterministic fixture data.

## Fix

The test fixture now uses high positive PID `2147483647`, which avoids the ordinary hosted-runner PID range and keeps the process-exit branch deterministic without changing production health reconciliation behavior.

## Validation

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml native::control_plane::tests::service_status_response_combines_worker_and_service_state -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml native::control_plane::tests -- --test-threads=1`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `git diff --check`
