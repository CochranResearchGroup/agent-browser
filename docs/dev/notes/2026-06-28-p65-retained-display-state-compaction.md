# P65 Retained Display State Compaction

Date: 2026-06-28

## Outcome

P65 added explicit retained display-allocation classification and a
service-owned cleanup path.

Implemented surfaces:

- `agent-browser service prune-retained --display-allocations --dry-run`
- `agent-browser service prune-retained --display-allocations --apply`
- `service status` JSON/text `retainedDisplayAllocations`

Classifier classes:

- `live`
- `diagnostic-retained`
- `safe-orphan-display`
- `stale-route-reference`
- `historical-placeholder`
- `unknown`

Apply removes only apply-safe `safe-orphan-display`,
`stale-route-reference`, and `historical-placeholder` candidates. Route-pool
entries are not changed by this cleanup path.

## Live Result

Artifact directory:

- `/tmp/agent-browser-p65-retained-display-20260628T174225Z`

Live dry-run result:

- `display-prune-dry-run.json` reported zero apply-safe display allocation
  candidates.
- Apply was skipped because there were no safe candidates to remove.
- Final service status retained 22 display allocations: 16
  `diagnostic-retained`, 6 `live`, 0 apply-safe.
- Remaining records include explicit `candidateReasons`.

Final proof:

- `final2-service-status.json`: success.
- `final2-incidents-summary.json`: success, incident count 0.
- `final2-install-doctor.json`: success, no issues.
- `final2-remote-view-doctor.json`: success, status `ready`, no issues.
- `final2-route-pool-readiness.json`: success, status `ready`.

Runtime convergence:

- Rebuilt and installed the local debug binary.
- Ran `pnpm publish:local-dashboard -- --skip-browser --json`.
- Removed two stale deleted-executable default daemon listeners reported by
  install doctor after binary replacement.

## Validation

- `cargo test --manifest-path cli/Cargo.toml service_prune_retained -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml test_prune_retained_service_state_classifies_display_allocations -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml test_service_status_via_actions_does_not_launch_browser -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml test_format_service_status_text_includes_profile_and_session_summaries -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml service_status_and_collection_response_contracts_match_wire_shape -- --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm test:service-client-contract`
- `pnpm test:service-client-types`
- `pnpm --dir docs build`

## Next Step

No display allocation apply is needed until a future dry-run reports
apply-safe candidates.
