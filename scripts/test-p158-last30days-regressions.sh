#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/.." && pwd)"
cd "$repo_root"

rust_tests=(
  "native::service_request::tests::route_bearing_tab_new_is_rejected_before_job_creation"
  "native::service_access::tests::service_request_route_hints_reuse_compatible_live_browser"
  "native::service_access::tests::shared_local_cold_profile_session_round_trips_without_owner_proof"
  "native::action_runtime::runtime::route_host_tests::test_shared_profile_attach_target_honors_exact_service_route_hints"
  "native::service_failure::tests::access_plan_route_conflict_refreshes_before_exact_reuse"
  "native::service_request::tests::route_conflict_failure_record_preserves_the_exact_actionable_code"
  "native::stream::http::tests::service_request_rejection_response_exposes_route_conflict_recourse"
  "native::control_plane::tests::shared_profile_identity_rejection_keeps_provenance_recourse_and_terminal_event"
  "native::stream::dashboard::tests::durable_handoff_resolution_uses_daemon_relay_without_an_http_lane"
  "native::remote_view::tests::acquisition_plan_avoids_released_display_number_identity_collision"
  "native::remote_view_handoff::tests::complete_open_finalizes_lease_and_returns_opened_response"
  "native::actions::remote_view_route_tests_one::test_remote_view_open_dry_run_plans_route_bound_launch_without_existing_display"
  "native::actions::remote_view_route_tests_two::test_remote_view_open_dry_run_prefers_inline_route_pool_identity_over_stale_state"
  "native::remote_view::open::route_action_helper_tests::test_remote_view_open_acquisition_lease_rollback_quarantines_until_cleanup_confirmation"
)

for test_name in "${rust_tests[@]}"; do
  scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml "$test_name" -- --exact --test-threads=1
done

node scripts/test-service-request-client.js
node scripts/test-remote-view-handoff-docs.js

echo "P158 Last30Days route acquisition regressions passed"
