# Plan 0170 Service Job Timestamp Ordering Receipt

Date: 2026-09-12
Branch: `maintenance/plan-0170-job-timestamp-ordering`
Base: `a4ba32ee2137cbaebd03fcec94114cc1bbccb3ea`

## Observed Defect

Last30days Plan 0068 recorded a successful Agent Browser
`tab_handle_release` job whose `startedAt` preceded `submittedAt` by about
1.881 seconds. The control plane sampled public lifecycle fields from the
adjustable wall clock independently. Its separate monotonic submission instant
continued to enforce elapsed deadlines correctly, but did not protect display
and audit timestamp ordering.

## Repair

- compare RFC 3339 lifecycle candidates with the preceding persisted boundary;
- clamp running `startedAt` to `submittedAt` when the wall clock moves backward;
- clamp terminal `completedAt` to `startedAt`, or `submittedAt` when no start
  exists;
- leave monotonic queue, cancellation, and timeout deadlines unchanged.

## Validation

- `lifecycle_timestamp_never_precedes_its_prior_boundary`: passed;
- `submit_returns_command_response`: passed;
- `service_job_timeout_marks_running_job_timed_out`: passed;
- `cargo fmt --check`: passed;
- workspace `cargo clippy` with warnings denied: passed.

The first focused launch failed before reaching product code because the Cargo
slice reported 947 of 1,024 tasks and process/thread creation returned OS error
11. Fresh readback showed 959 processes, 8,803 threads, 39 GiB available memory,
and 30 GiB swap in use. A single bounded rerun with one Cargo job and cache
disabled avoided the task ceiling and preserved the subsequent red/green
result. No process cleanup was performed.

No production runtime, browser, profile, provider, tenant, service state, or
timeout configuration was changed.
