# Plan 0128 Runtime Lifecycle Hotfix Installed Acceptance

Date: 2026-08-23

State: `ACCEPTED`

## Source Integration

The lifecycle and doctor collection merged through PR 8. The legacy
browserless handoff-alias compatibility repair merged through PR 9. The
shared-stream supervisor advisory repair merged through PR 10. Final merged
main is `26d83378f4af93cd6766d7f8d38af33bb1936a1c`.

## Installed Identity

- version: `0.28.0`;
- generation: `0.28.0-6b461233692c-7e71e8fd473b`;
- binary SHA-256:
  `6b461233692c0b51f67e690b50c9bf5bbf1e180c1b784b2d67fb90fd1277fdd1`;
- accepted transaction:
  `upgrade-4a2cf513-114a-4ceb-9672-0e6a30b2748d`.

The accepted transaction completed runtime census, candidate shadow
dashboard readiness, admission drain, runtime transfer, presentation
rebinding, authenticated candidate presentation, payload commit,
post-commit validation, workstation reconciliation, managed dashboard
cutover, and supervisor rebinding.

## Doctor And Multiplicity

Standalone installed doctor succeeds. Its only issue is the retained
`supervisor_stopped` warning for `last30days-home-feed`. The unit is
inactive/dead with no main PID, and the warning no longer makes the global
doctor result fail.

Every workstation readiness axis is true. Runtime multiplicity is one
dashboard process, one runtime-host process, one executable generation, and
zero legacy daemons. Runtime convergence and authenticated operator journey
are ready, and rollback remains ready.

## BILL Provider-Free Acceptance

The old unprojected BILL process group `83894` is absent and the profile lock
is absent. The broker access plan therefore selected the lawful replacement
path for the exact durable profile `bill-soylei`.

The authenticated service API launched one replacement browser at process
group `76553`, owner generation 2, and opened only a local `data:` page titled
`P128 BILL local acceptance`. It did not open BILL, QBO, or another provider.
The request reported `duplicateProcessAllowed=false`.

The same broker trace then accepted `service_browser_close`. Broker shutdown
completed, the process exited, the profile lock released, and lifecycle state
became `terminal/satisfied`. A fresh OS and service-resource readback found
zero Chrome processes and zero resources for the BILL profile.

## Preserved Independent State

The Plan 0233 QBO browser remains independently degraded because its old
polite close encountered a closed CDP connection and required force kill. It
has no live PID or service resource. Its separate owned lifecycle is
`terminal/satisfied` with absent-process and absent-profile-lock evidence.
No QBO cleanup, retry, navigation, or authentication inspection was performed.

Active AuraCall browser work was not stopped or modified. Two final install
attempts failed closed when a short-lived external Chrome renderer appeared
in the bounded census. Once that exact external process exited, supported
reconciliation reported healthy state and the next census admitted the final
candidate. No exception or process bypass was added.

## Validation

- lifecycle close and launch, runtime lifecycle, supervisor, installer,
  workstation payload, missing-projection recovery, and desktop capacity
  regressions passed in the source packet;
- all 90 workstation installer tests and all 12 supervisor tests passed;
- Rust formatting, strict Clippy, and diff hygiene passed;
- workstation install, host provision, fresh-VM, Guacamole asset,
  PostgreSQL durability, and route-specific user synchronization fixtures
  passed;
- installed doctor succeeds with seven of seven readiness axes true;
- repo and installed Agent Browser skill SHA-256 values both equal
  `6839b02cc79e9dbff4e80859c257e9046da5b48fc9f1ea2f87a2a5463c4ccbf6`.

## Boundary

This is an installed hotfix checkpoint, not a formal release. Provider and
accounting effects remain outside this acceptance. Historical retained-state
cleanup remains a separately reviewed operation.
