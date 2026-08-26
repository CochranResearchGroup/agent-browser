# Plan 0131 Slice E Production Candidate Preflight

Date: 2026-08-25

Status: CANDIDATE QUALIFIED | PRODUCTION TRANSACTION NOT APPLIED

Authority: SLICE E PRODUCTION AUTHORITY GRANTED WITH ONE BOUNDED CARGO GUARD
OVERRIDE

## Outcome

The production-capable source and release candidate are qualified. The
installed production selector, provider configuration, browsers, routes,
displays, handoffs, and processes were not changed.

The production transaction and controlled fixture remain deferred for two
current hard gates:

1. `docs/dev/notes/0133-2026-08-25-operator-visible-window-focus-gap-handoff.md`
   prohibits another workstation upgrade while the operator-visible focus-gap
   repair is active.
2. The current production resource projection has zero cleanup candidates and
   zero apply-safe retained displays. Both Guacamole routes retain live
   workload bindings. Obtaining a fixture route would require route expansion,
   takeover, or workload displacement outside the granted guard bypass.

This is a qualified candidate packet, not production fixture acceptance and
not Plan 0110 live Foundation Acceptance.

## Authority And Guard Boundary

The operator explicitly granted Slice E production authority and separately
authorized bypassing the swap-free Cargo admission guard. Every Cargo command
continued to use `scripts/ci/cargo-safe.sh`, four jobs, the repository WSL
cgroups, per-invocation memory limits, and aggregate slice limits. The only
override was `AGENT_BROWSER_CARGO_MINIMUM_SWAP_FREE_KIB=0`.

The authority did not grant route expansion, controller takeover, browser
closure, workload parking, opportunistic cleanup, or displacement. No live
effect was attempted after the runtime census showed that one of those actions
would be required.

## Candidate Identity

- Branch: `feature/plan0131-production-candidate`
- Source baseline: `d07b481159a0de8a3e69b0dd7a327d8fb7fef8e9`
- Source commit: `9ec6a2b4`
- Candidate path: `cli/target/release/agent-browser`
- Candidate version: `0.28.0`
- Candidate SHA-256:
  `2202890a31370f6693f8f50db06448b5a4b2b1b36d930538afe34d910b6fc245`

The candidate admits the controlled X11 provider only from an exact production
runtime-generation manifest with production environment, matching provider,
capability, recipe, and binary hash. Development admission retains its separate
schema and environment. Caller-selected provider routing remains unavailable.

## Source And Candidate Validation

The following gates passed from the dedicated Slice E worktree:

- focused provider admission, desktop interaction, desktop control
  coordinator, desktop capture, service resources, workstation install, and
  configured-provider dispatch tests;
- the unattended service-GC single-clock regression;
- Rust formatting and strict Clippy with warnings denied;
- the complete repository Rust test runner, including 1,543 parallel-safe
  tests and every serial environment-mutating partition;
- service client, service contract no-launch, and service API/MCP parity tests;
- dashboard inspector, view-stream, browser-row, and browser-table tests;
- WSL Cargo safety and remote-view handoff documentation tests;
- dashboard and documentation production builds;
- the source-free workstation installation fixture;
- the release build that produced the candidate identity above.

The actions architecture check remains red for the existing P0101 and P0108
baseline classifications and counts. Running the same check at canonical main
commit `d07b4811` produced the same failure, so it is retained as a baseline
finding rather than attributed to Slice E.

An earlier comprehensive run exposed one stale dispatch assertion that expected
only the former provider-unavailable code. The assertion was narrowed to the
typed provider-admission family while preserving the zero-confirmation-effect
check. The first failure is retained, and the repaired focused test plus the
final comprehensive run passed.

## Installed Runtime Re-anchor

The selected installed production generation remains
`0.28.0-05d9da26035e-7fa3fbcb7248`. Its latest workstation transaction remains
accepted with rollback generation `0.28.0-80d87ab7be0d-5926db67f48a` and
transaction identity `upgrade-3a9d3ace-cd02-48aa-851d-f1452c0832f5`.
Workstation readiness reports payload, selected generation, runtime
convergence, dashboard ingress, operator journey, and rollback ready.

Installed doctor reports one issue only:
`path_command_workspace_binary_mismatch`. This is expected because the
qualified workspace release candidate was intentionally not installed. It is
not evidence that the candidate transaction occurred.

The service resource projection reports:

- 231 total processes and about 18.0 GB total resident memory;
- 177 protected processes, 54 observed processes, and zero cleanup candidates;
- one observed unowned Agent Browser process, so resource pressure remains
  active;
- four viable modeled browsers and two attention records whose live PID is
  absent;
- zero apply-safe retained display allocations;
- Guacamole route 1 still bound to the retained Last30Days browser on live
  display 10 despite an orphaned route record and contradictory available pool
  projection;
- Guacamole route 2 ready and checked out to the retained Bill SoyLei browser
  on live display 11 with active observing viewer leases.

A fresh operating-system census independently observed the installed service,
browser, Guacamole, RDP, X11, and concurrent Plan 0133 validation processes.
No process was stopped, signaled, adopted, or reclassified.

## Remaining Transaction Gate

Before production execution resumes, re-anchor all candidate and installed
identities and require both of these conditions:

1. The Plan 0133 workstation-upgrade and production-provider hard stop is
   explicitly cleared or superseded by current source-backed authority.
2. One exact controlled fixture route is apply-safe without route expansion,
   controller takeover, browser closure, workload parking, or displacement.

Only then may the existing workstation hot-upgrade transaction install the
exact candidate. After installation, re-read the selected generation and
provider admission before any fixture input. Any identity drift, uncertain
effect, missing rollback readiness, or route ambiguity stops the transaction
and follows the existing transactional rollback contract.

## Effects Not Taken

- no production installation, selector change, or rollback;
- no production provider admission or input event;
- no controlled fixture browser or route allocation;
- no controller takeover, browser closure, workload parking, or route release;
- no process cleanup or retry;
- no real authentication, credential, private profile, or tenant-content use.
