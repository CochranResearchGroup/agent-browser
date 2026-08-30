# Plan 0142 Structured Client Recourse Checkpoint

Date: 2026-08-29

Plan: `docs/dev/plans/0142-2026-08-29-service-state-concurrency-and-client-recourse-reliability-plan.md`

Progress classification: `outcome_progress`

State transition: `slice_a_contract_and_regression_in_progress` to
`slice_b_structured_recourse_checkpoint_complete_lock_refactor_next`

## Accepted Behavior

Failed Service worker responses now preserve the legacy error and add a
versioned `failure` object before job persistence and client delivery.

The first accepted mappings are:

- `service_state_lock_timeout: process mutation lock` returns the Service State
  axis, process-mutex-wait phase, uncertain effect state,
  inspect-before-retry disposition, no reuse, and hard stops against blind
  retry and duplicate profile launch;
- `runtime_lifecycle_existing_owner_requires_explicit_transition` returns the
  lifecycle-owner axis, launch-admission phase, zero-effect state,
  refresh-access-plan disposition, no inferred reuse, no inferred Recovery
  Plan, and a hard stop against duplicate profile launch; and
- a viewport failure at `1440 x 1000` is represented as an effect-uncertain
  failed job because the browser mutation may have happened before Service
  State persistence reported contention.

The generated JavaScript client exposes:

- `getServiceFailureRecourse(response)` for non-throwing branching;
- `requireServiceRequestSuccess(response)` for callers that want a success
  assertion; and
- `ServiceOperationError`, which retains both the original response and its
  structured recourse.

Existing clients remain compatible because failed responses and retained jobs
keep their original `error` field.

## Evidence

RED evidence:

- the initial Rust classifier test failed because no structured failure module
  existed;
- the lifecycle-owner test failed against the generic fallback; and
- the client test failed because `ServiceOperationError` was not exported.

GREEN evidence:

```text
native::service_failure::tests: 3 passed
viewport_lock_failure_persists_the_same_structured_recourse_returned_to_clients: 1 passed
pnpm test:service-client-contract: passed
pnpm test:service-client-types: passed
pnpm test:service-request-client: passed
pnpm test:service-observability-client: passed
pnpm test:service-api-mcp-parity: passed
```

## Material Blockers

- Service State snapshot reads still serialize behind the process mutation
  mutex and exclusive file lock.
- Lock errors do not yet carry measured wait and hold telemetry.
- The lifecycle classifier does not yet attach exact reuse hints or a sealed
  Recovery Plan from current acquisition evidence.
- README, CLI help, docs-site, dashboard, and skills guidance remain pending
  until the underlying response and lock behavior is complete.

## Next Action

Implement the P142-D and P142-E lock telemetry and read-path decontention tracer
bullets while preserving the four-file transaction tests.
