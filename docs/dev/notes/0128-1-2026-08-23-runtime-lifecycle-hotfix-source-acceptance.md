# Plan 0128 Runtime Lifecycle Hotfix Source Acceptance

Date: 2026-08-23

Branch: `hotfix/runtime-lifecycle-collection`

Source commit: `8d2c6fe8`

State: `SOURCE_ACCEPTED_INSTALL_PENDING`

## Accepted Repairs

- The accepted absent-closing lifecycle branch is reconciled with current
  main, including collision-free next-generation terminal replacement,
  terminal owner-binding suppression, exact absent-process completion, and
  corrected graceful shutdown aggregation.
- Auto-launch and explicit owned launch share one persistence obligation. A
  lifecycle or service projection failure invokes exact cleanup once and
  clears the in-memory launch only after the owned close result is recorded.
- When lifecycle registration succeeded before a later persistence failure,
  the binding is retained early enough for cleanup to use the authorized
  managed close transition.
- A live managed browser with no service browser or session projection remains
  recoverable only through the existing orphan-adoption seam. The regression
  requires an orphaned exact owner, matching durable handoff presentation
  receipt, one exact runtime-profile process identity, CDP endpoint, and target
  evidence. Mismatch and ambiguity remain rejected.
- Install doctor retains warning observations but computes success from
  blocking issues. A stopped supervisor is advisory only when it has no live
  process or stream expectation. Active, starting, drifted, conflicted,
  restart-exhausted, or unavailable supervisor state remains blocking.

## Validation

- `native::action_runtime::runtime::close_launch_tests`: 9 passed.
- `native::runtime_lifecycle::tests`: 12 passed.
- `session_supervisor::tests`: 12 passed.
- serial `install_doctor_` selection: 20 passed.
- `workstation_payload_status`: 4 passed.
- exact missing-projection durable recovery regression: passed.
- Rust formatting and strict Clippy: passed.
- workstation install fixture, host-provision fixture, fresh-workstation VM
  harness, Guacamole assets, PostgreSQL durability, route-specific user sync,
  and remote-view handoff documentation: passed.
- documentation production build: passed.
- diff hygiene: passed.

## Runtime Boundary

The installed workstation dry-run reported `mode=dry-run`, `mutated=false`,
and no runtime census transaction. It did not authorize installation. No
production browser, provider, profile, service, or accounting state changed.

## Remaining Gate

Integrate the reviewed source through protected main, build the exact candidate,
and run the transactional installer census. Stop if the `bill-soylei` process,
profile, CDP, target, owner generation, runtime host, or durable handoff evidence
is ambiguous. After accepted apply, prove one harmless local tab acquisition and
release without navigating to BILL or QBO.
