#!/usr/bin/env bash
set -euo pipefail

manifest_path="${CARGO_MANIFEST_PATH:-cli/Cargo.toml}"
profile="${CARGO_TEST_PROFILE:-ci}"

# These modules mutate process-global environment variables or user-scoped
# runtime state during tests. Keep them out of the parallel pass, then run them
# serially in the same job so coverage is preserved without duplicate compile
# work across CI jobs.
serial_filters=(
  "agent_env::tests"
  "connection::tests"
  "flags::tests"
  "native::actions::tests"
  "native::auth::tests"
  "native::cdp::chrome::tests"
  "native::control_plane::tests"
  "native::parity_tests"
  "native::policy::tests"
  "native::providers::tests"
  "native::service_health::tests"
  "runtime_profile::tests"
)

skip_args=()
for filter in "${serial_filters[@]}"; do
  skip_args+=(--skip "$filter")
done

echo "Running parallel-safe Rust tests"
cargo test --profile "$profile" --manifest-path "$manifest_path" -- "${skip_args[@]}"

echo "Running env-mutating Rust tests serially"
for filter in "${serial_filters[@]}"; do
  echo "Running serial Rust test partition: $filter"
  cargo test --profile "$profile" --manifest-path "$manifest_path" "$filter" -- --test-threads=1
done
