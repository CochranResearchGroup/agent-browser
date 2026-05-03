# Profile Lease Observability Checkpoint

Date: 2026-05-03

## Decision

The profile lease waiting lane is complete enough to pause and treat as a
closed observability slice. It should remain backend-first: the dashboard and
clients consume service-owned trace and job state rather than inventing their
own profile contention model.

## What Changed

The service control plane now has a full profile lease wait path:

- `profileLeasePolicy: "wait"` keeps blocked service requests queued while the
  worker remains available for unrelated jobs.
- Waiting jobs are persisted as `waiting_profile_lease`.
- Service status reports profile lease wait pressure.
- Profile lease wait start and end events are retained in service state.
- `service trace` includes a structured `summary.profileLeaseWaits` rollup with
  outcome, timing, conflict sessions, and service, agent, and task labels.
- CLI text output renders a `Profile lease waits` block.
- HTTP, MCP, generated service observability client types, dashboard trace
  cards, README, docs site, and the installed agent-browser skill all consume
  the same backend trace contract.
- A live profile lease wait smoke proves a held exclusive lease produces a
  retained wait record in `service_trace.summary.profileLeaseWaits`.

## Evidence

Relevant commits on `main`:

- `5ece4ed` added service profile lease wait policy.
- `65756de` queued profile lease waits outside worker execution.
- `01ceacf` surfaced profile lease wait job state.
- `8aa4030` summarized profile lease waits in service status.
- `b79685e` recorded profile lease wait events.
- `e4b91bc` summarized profile lease waits in trace text output.
- `9f1d47d` added structured `summary.profileLeaseWaits`.
- `2bcb850` showed profile lease waits in the dashboard trace explorer.
- `352cfe5` covered dashboard and client trace consumers.
- `a8211c7` asserted HTTP and MCP profile lease wait trace parity.
- `719145f` added the live profile lease wait smoke.
- `208f5cf` documented the smoke as a manual full-CI and release-gating check.

## Validation Now Available

Use the targeted local checks for ordinary implementation closeout:

```bash
pnpm test:dashboard-trace
pnpm test:service-client
pnpm test:service-contracts-no-launch
pnpm test:mcp-stdio
git diff --check
```

Use the live profile lease smoke for manual full-CI or release-gating checks
when a change touches profile selection, profile lease waiting, service
request, trace summary, dashboard trace, or service observability client
behavior:

```bash
pnpm test:service-profile-lease-wait-live
```

This live smoke is intentionally not part of ordinary CI because it starts local
service infrastructure and exercises a slower contention path.

## What This Proves

The service can own profile contention deterministically enough for operators,
agents, API clients, MCP clients, and the dashboard to reason from one shared
state model:

- wait requests are queued rather than blocking the worker
- wait pressure is visible in status and job state
- wait history is visible in events and trace summaries
- dashboard and clients render the backend summary directly
- HTTP and MCP remain aligned on the trace payload

## Residual Gaps

- The live smoke currently proves the timeout path for a held lease. The
  success-after-release path is covered by Rust control-plane tests, but not yet
  by a live HTTP smoke.
- Dashboard rendering is build and fixture validated, but not browser-visually
  inspected after this slice.
- Profile lease summaries are trace-window dependent. Very small trace limits
  can omit one side of a wait if callers request too narrow a window.

## Best Next Slice

Return to the backend-first service roadmap and work on richer service-owned
profile and session state before adding more dashboard panels. The next useful
slice is a profile/session allocation view model that can answer, from service
state alone:

- which service, agent, and task currently holds a profile
- which target identities the profile is meant to satisfy
- whether the lease is shared, exclusive, released, waiting, or conflicted
- which sessions or jobs are blocking a new request
- what action an operator should take to release, reuse, or redirect the
  profile

Only after that model is authoritative should the dashboard get a larger
profile-management surface.
