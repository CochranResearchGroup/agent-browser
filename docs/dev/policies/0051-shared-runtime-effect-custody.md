# Policy | Shared Runtime Effect Custody

## Purpose

Prevent concurrent sessions from racing on one shared Agent Browser runtime.
Source ownership, operator authority, process serialization, and live-effect
custody are separate claims. None implies another.

## Covered Effects

Apply this policy before any command that can mutate a shared production or
staging Agent Browser environment, including:

- workstation install, resume, recovery, rollback, or generation retirement;
- effectful runtime reconciliation or garbage collection;
- runtime or dashboard supervisor start, stop, restart, takeover, or ingress
  promotion;
- shared Service State migration, browser or profile recovery, browser closure,
  or route replacement; and
- any force or full-shutdown mode that can replace, terminate, or supersede a
  shared runtime owner.

Read-only status, doctor, journal, process-census, Git, and build operations do
not require effect custody when they cannot trigger repair or mutation. Builds
remain isolated from production and confer no install authority.

## Single-Owner Contract

- Keep at most one active effect-custody lease per target environment. The
  environment's runtime control plane is the authority for that lease. An issue
  comment, chat message, plan, branch, active-lane entry, shell PID, file lock,
  or agent status is discovery evidence only.
- Acquire custody through an atomic compare-and-swap before the first covered
  effect. A command must submit the exact lease ID and revision, and the target
  runtime must reject absent, stale, expired, released, transferred, or
  mismatched custody before effect.
- Bind the lease to the environment, accountable principal, execution session,
  work item, bounded operation, allowed action set, source commit, candidate
  binary digest when applicable, acquisition time, expiry, heartbeat, revision,
  and state. Do not store secrets or tenant payloads in the lease.
- Treat operating-system and installer file locks as short critical-section
  serialization only. Acquiring a file lock does not acquire, renew, transfer,
  or prove effect custody.
- Keep one controller responsible for the complete effect sequence. A helper or
  second session may build, inspect, or review, but it must not mutate the same
  environment under the controller's lease.

## Validation And Renewal

- Immediately before each covered effect, re-read the authoritative lease and
  verify its environment, holder, revision, state, allowed action, source
  commit, candidate digest, and expiry. Re-read relevant runtime transaction,
  admission-drain, supervisor, listener, and protected-owner evidence in the
  same preflight.
- Renew only by compare-and-swap from the current revision. A heartbeat extends
  time, not scope. Changing the candidate, action set, environment, or owner
  requires a new lease or an atomic transfer.
- A CLI process exit, timed-out client request, idle agent, completed build,
  clean worktree, or stale chat report does not release custody. The lease
  remains authoritative until a terminal release, transfer, expiry plus
  successful reclamation, or environment reset produces a durable receipt.
- After a timeout or ambiguous command result, inspect the exact operation and
  lease receipts before retrying. Do not infer `no_effect` from client exit or
  loss of connection.

## Transfer, Release, And Recovery

- Transfer custody atomically from the current holder to one named successor.
  Bind the transfer to the prior lease revision, a single-use nonce, the same
  environment, the intended remaining action set, and both parties' readback.
  The old lease becomes unusable in the same state transition that activates
  the successor.
- Release only after the covered operation is terminal or deliberately
  stopped, no task-owned effect remains in flight, and the release receipt
  records the final transaction, supervisor, listener, candidate, and protected
  owner state relevant to the operation.
- Reclaim an expired or apparently abandoned lease only after positive evidence
  proves the former controller and its effectful child processes absent, no
  covered command or transaction remains active, and current runtime state is
  coherent. Record the stale-owner evidence and the new lease revision.
- If two sessions claim custody, or a covered effect occurs without a valid
  lease, stop new effects. Preserve processes and profiles, reconcile the
  authoritative lease and runtime receipts, select one controller, and resume
  only through a new or transferred lease. Do not solve the collision with a
  force flag, broader shutdown, or blind retry.

## Coordination And Audit

- Work items and active lanes state which owner may request live-effect custody
  and name known overlaps. They must not project a local lease as globally
  active authority unless the runtime confirms the exact current revision.
- Record acquisition, renewal, denied attempts, transfer, release, reclamation,
  and mismatch events in an append-only audit surface. Each receipt includes
  environment, lease and revision, holder, operation, candidate identity when
  applicable, timestamp, result, and redacted reason.
- Installation and runtime tooling must eventually enforce this contract. Until
  that enforcement is installed and proven, serialize shared production and
  staging effects through one coordinator session and require an explicit
  current custody checkpoint plus a fresh runtime readback before every covered
  command. This procedural fallback reduces risk but is not equivalent to the
  runtime-enforced contract.

## Adoption Notes

This is an Agent Browser repo-local extension identified after the Plan 0186
installer collision. It composes with active-lane coordination, runtime-state
governance, multi-agent reconciliation, and collaborative development. It is a
candidate for a reusable shared-policy module after runtime enforcement proves
the lease schema and recovery semantics.
