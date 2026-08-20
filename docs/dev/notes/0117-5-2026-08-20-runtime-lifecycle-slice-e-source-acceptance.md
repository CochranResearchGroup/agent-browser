# P117 Slice E Source Acceptance

Date: 2026-08-20

Plan: `docs/dev/plans/0117-2026-08-19-runtime-lifecycle-authority-and-convergence-plan.md`

## Accepted Boundary

Slice E is accepted at the source, isolated-fixture, and disposable-browser
boundary. This slice did not install the candidate binary, replace live user
units, mutate authenticated profiles, or converge the workstation's retained
legacy session daemons. Those effects remain behind Slice F and later explicit
live reconciliation authority.

## Single Runtime Host

Named sessions now resolve to one `runtime-host` socket, process identity,
authentication record, executable generation, and supervisor service. Each
session is an independently serialized logical lane with its own control-plane
worker, browser state, runtime profile configuration, stream server, and event
attribution. A fixed bound prevents unbounded lane admission.

The Linux supervisor writes one `agent-browser-runtime-host.service`. The
legacy `agent-browser-session@.service` template is now a forwarding-only
oneshot adapter that admits a lane through `session supervisor run-lane`; it
cannot execute a legacy per-session daemon. Install fails closed when an active
legacy template, a mixed executable generation, or a fixed stream-port conflict
requires transaction-bounded transfer.

Supervisor restart preloads every valid lane manifest and restores its fixed
stream port. Removing one lane closes only that lane. The shared host stops
only after the final supervised lane is removed.

## Ingress and Discovery

CLI commands share the host endpoint and carry their logical lane and cached
lane configuration in the authenticated command envelope. HTTP, MCP, dashboard
relay, stream delivery, remote-headed actions, and service requests enter the
same per-lane control plane. Dashboard discovery now validates each lane stream
against `runtime-host.pid` while host admission is enabled, so a healthy
single-host topology remains visible instead of being filtered out for lacking
obsolete per-session PID files.

Cold CLI startup now waits for the initial lane stream to become ready before
returning from daemon establishment. This closes a race where the host socket
could be bound before its router and initial stream were ready, causing the
first real browser command to time out while later commands succeeded.

## Stress and Browser Evidence

The disposable three-lane stress smoke proves:

- concurrent alpha, beta, and gamma admission uses one PID and one socket;
- duplicate beta admission is idempotent;
- a hanging alpha navigation does not delay beta or gamma;
- cancellation reaches the running alpha job and the lane accepts a follow-up
  browser navigation;
- lane close does not terminate unrelated lanes; and
- final-lane close permits a clean host restart under a new PID.

The supervisor smoke independently proves two manifest-backed lanes retain
their fixed ports across host restart and remain visible through
`/api/sessions`. The no-launch host smoke proves concurrent lane creation,
bounded topology, lane-scoped close, and idle host exit. Real Chrome launch,
navigation, evaluation, cancellation recovery, and close passed with explicit
disposable profiles. The live default profile and its authenticated browser
were not acquired.

## Validation

- `scripts/ci/cargo-safe.sh fmt --manifest-path cli/Cargo.toml -- --check`
- `scripts/ci/cargo-safe.sh clippy --manifest-path cli/Cargo.toml -- -D warnings`
- focused session-supervisor unit tests: 10 passed
- session-supervisor integration test: 1 passed
- runtime-host supervisor no-launch smoke: passed
- runtime-host no-launch smoke: passed
- runtime-host multi-lane real-Chrome stress smoke: passed
- real-Chrome launch, navigation, evaluation, and close E2E: passed
- real-Chrome running-navigation cancellation and recovery E2E: passed
- service-state independent-writer exclusion and bounded lock timeout: passed
- running service-job cancellation: passed
- canonical partitioned Rust suite through `scripts/ci/rust-tests.sh`: passed
- docs build: all 35 pages generated
- dashboard production build: all 7 pages generated
- remote-view handoff documentation checks: passed
- repository and installed Agent Browser skills: byte-identical
- patch whitespace check: passed

An initial raw parallel full-suite run passed 2,208 tests before the
env-mutating service-monitor test collided with parallel shared state. The test
passed alone and in the canonical serial partition. That failed run left one
disposable headless Chrome fixture holding the Cargo lock descriptor. Its exact
`/tmp/agent-browser-managed-runtime-devtools-file-*` profile and test-owned PID
were verified before termination. No live or authenticated browser was
targeted.

## Accepted Commits

- `57c3526e` establishes shared-host supervision and forwarding-only lanes.
- `970ebaea` closes cold initial-lane readiness.
- `80af9c02` adds the multi-lane real-browser stress gate.

## Next Boundary

Execute Slice F by extending the P116 transaction and census to the complete
runtime-lane set, starting one observation-only candidate host, and transferring
lane ownership with rollback receipts before any live legacy daemon is retired.
