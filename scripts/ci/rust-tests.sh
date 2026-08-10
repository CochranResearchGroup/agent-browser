#!/usr/bin/env bash
set -euo pipefail

manifest_path="${CARGO_MANIFEST_PATH:-cli/Cargo.toml}"
profile="${CARGO_TEST_PROFILE:-}"
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

cargo_test=("$script_dir/cargo-safe.sh" test --manifest-path "$manifest_path")
if [[ -n "$profile" ]]; then
  cargo_test+=(--profile "$profile")
fi

# These modules mutate process-global environment variables or user-scoped
# runtime state during tests. Keep them out of the parallel pass, then run them
# serially in the same job so coverage is preserved without duplicate compile
# work across CI jobs.
serial_filters=(
  "agent_env::tests"
  "connection::tests"
  "flags::tests"
  "mcp::tests"
  "native::action_runtime::cancellation::tests"
  "native::action_runtime::runtime::close_launch_tests"
  "native::action_runtime::runtime::dispatch_runtime_tests"
  "native::action_runtime::runtime::route_host_tests"
  "native::actions::confirmation_tests"
  "native::actions::dependent_batch_tests"
  "native::actions::dispatch_tests"
  "native::actions::remote_view_route_tests_one"
  "native::actions::remote_view_route_tests_two"
  "native::actions::runtime_route_host_tests"
  "native::actions::service_activity_tests"
  "native::actions::service_config_tests"
  "native::actions::service_health_tests"
  "native::actions::service_incident_mutation_tests"
  "native::actions::service_incidents_tests"
  "native::actions::service_inventory_tests"
  "native::actions::service_jobs_tests"
  "native::actions::service_reconcile_tests"
  "native::actions::service_trace_tests"
  "native::actions::state_tests"
  "native::auth::action_tests"
  "native::browser_input::action_tests"
  "native::browser_inspection::evaluation_action_tests"
  "native::browser_lifecycle::action_tests"
  "native::interaction::action_tests"
  "native::network::action_tests"
  "native::remote_view::helper_action_tests"
  "native::remote_view::open::tests"
  "native::remote_view::open::route_action_helper_tests"
  "native::remote_view::open::visibility_action_tests"
  "native::service_health::action_helper_tests"
  "native::service_health::reconcile_action_helper_tests"
  "native::service_incidents::action_mutation_helper_tests"
  "native::service_resources::tests"
  "native::service_retained_state::inventory_action_helper_tests"
  "native::service_store::tests"
  "native::stream_runtime::action_tests"
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
"${cargo_test[@]}" -- "${skip_args[@]}"

echo "Running env-mutating Rust tests serially"
for filter in "${serial_filters[@]}"; do
  echo "Running serial Rust test partition: $filter"
  "${cargo_test[@]}" "$filter" -- --test-threads=1
done
