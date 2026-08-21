# P117 Slice G Source Acceptance

Date: 2026-08-20

Plan: `docs/dev/plans/0117-2026-08-19-runtime-lifecycle-authority-and-convergence-plan.md`

## Accepted Boundary

Slice G is accepted at the source, deterministic-fixture, and disposable
runtime boundary. The candidate has not been installed into the live
workstation. Controlled installed convergence, generation reclamation, and
authenticated-profile readback remain Slice I.

## Automatic Reconciliation

The enabled user timer and operator-triggered command now enter the same
bounded reconciler. Process cleanup automatically applies only to package-owned
unattended classes with exact executable, process-group, start-time, lifecycle,
and owner-generation evidence. Review-gated and ambiguous resources remain
protected. Repeated effect failures use bounded backoff and produce one typed
incident rather than an unbounded retry loop.

The reconciler also runs profile and generation retention through their shared
authorities. Install doctor rejects stale monitor evidence, unexplained runtime
multiplicity, generation drift, missing cleanup obligations, and unknown
ownership under pressure.

## Compatibility Deletion

Cold per-session daemon creation is retired. An existing reachable legacy
socket remains addressable only as a migration adapter for explicit handoff;
when no runtime host has been admitted, a new launch fails with the typed
`runtime_host_admission_required` error. The private child-process entry now
identifies a runtime host and the old `AGENT_BROWSER_DAEMON` entry fails closed.

The standalone local-runtime convergence controller and its process-per-session
fixtures were deleted. Runtime-host, lifecycle, reconciliation, and retention
tests now own that behavior.

## Operator Readback

The authenticated dashboard health endpoint projects shared multiplicity and
monitor authorities. Its persistent summary distinguishes steady state from an
active transaction-bounded convergence window and reports dashboard, host,
legacy-daemon, and generation counts; protected, reclaimable, and unowned RSS;
cleanup obligations; retention effects; monitor freshness; and blocking
incidents.

The multiplicity census adds one bounded `ss -xlpn` probe to the existing
ten-second dashboard health poll. It does not run reconciliation, cleanup, or
browser discovery. Automatic effects remain on the five-minute monitor
cadence.

## Validation

- `scripts/ci/cargo-safe.sh fmt --manifest-path cli/Cargo.toml -- --check`
- `scripts/ci/cargo-safe.sh clippy --manifest-path cli/Cargo.toml -- -D warnings`
- focused P117 lifecycle tests: 7 passed
- focused runtime-health and resource-summary tests: passed
- runtime-host convergence no-launch smoke: passed
- runtime-host multi-lane real-Chrome stress smoke: passed
- workstation install source-free fixture: passed
- service API/MCP parity, client contract, and client type gates: passed
- dashboard navigator and inspector-action gates: passed
- dashboard production build: passed
- docs production build and remote-view documentation gate: passed
- patch whitespace check: passed

## Accepted Commits

- `d6e10c2b` deletes the temporary lifecycle compatibility facades.
- `62e99bde` bounds unattended profile and generation retention authority.
- `20fd80c9` authorizes exact unattended process-tree reclamation.
- `c1caa599` enables bounded automatic reconciliation and typed incidents.
- `e4eed22a` preserves legacy registry readers across the helper repair.
- `7b5a5f17` retires new legacy daemon launch, deletes the obsolete convergence
  controller, and adds shared dashboard runtime summaries.

## Next Boundary

Execute Slice H by aligning multiplicity, lifecycle, reconciliation, retention,
and incident readback across CLI, HTTP, MCP, generated client, dashboard, and
Service State, then update the operator and release documentation.
