# Policy | Shared Runtime Effect Coordination And Integrity

## Purpose

Coordinate concurrent Agent Browser runtime effects without turning workflow
advice into permission authority. The operator owns the system and decides
whether to build, cancel, discard, install, supersede, recover, or roll back.
Tooling makes current state and consequences visible, performs the selected
transition coherently, and prevents competing transactions from corrupting the
runtime.

## Authority Boundary

- User direction and the applicable goal or work-item authority determine
  whether an agent may request an effect. An issue, plan, branch, agent role,
  lease, recommendation, or successful check does not independently grant or
  revoke that authority.
- Treat the runtime coordination lease as an integrity mechanism, not a
  permission service. It selects the transaction currently allowed to commit
  mutations and fences superseded writers.
- Do not require a permanent coordinator agent. Any session acting within
  current user authority may inspect state and request a supported transition.
  The exact active operation, not an agent title, defines temporary execution
  custody.
- Ask for new user direction only when the requested effect exceeds current
  authority or selects a materially different outcome. Do not manufacture
  repeated approval stops for routine recovery inside an already authorized
  operation.

## Covered Effects

Apply this policy to production or staging workstation installation, recovery,
rollback, generation retirement, effectful reconciliation or garbage
collection, supervisor or ingress mutation, Service State migration, browser
or profile recovery, browser closure, route replacement, and force or
full-shutdown transitions.

Read-only status, doctor, journal, process census, Git, artifact inspection, and
build operations do not require the runtime coordination lease when they
cannot trigger repair or mutation. Builds remain isolated from production and
must not hold the install coordination record.

## Advisory Outcome Contract

- Report observed state, evidence, recommended action, supported alternatives,
  expected consequences, resource cost, and any integrity preconditions.
  Recommendations are advisory and must not be represented as permission.
- Support explicit operator-directed transitions such as start, join, wait,
  cancel, discard, queue, supersede, install, resume, recover, and rollback
  when the underlying runtime can perform them coherently.
- Reserve `authorization_required` for a genuine absence of user authority.
  Use typed operational states such as `joined_existing`, `queued`,
  `rebuild_required`, `superseded`, `recovery_required`, or
  `integrity_precondition_failed` for other outcomes. Do not collapse ordinary
  contention or drift into a generic permission denial.
- When a requested transition cannot execute safely, explain the exact
  conflicting operation or invariant and offer the supported next actions. A
  safety stop must protect a named integrity boundary, not enforce a preferred
  workflow choice.

## Transaction And Fencing Contract

- Keep at most one committing install or recovery transaction per target
  environment. Record it durably with environment, operation ID, candidate
  identity, binary and support digests, current revision, fencing generation,
  requested action, actor, timestamps, state, and last verified progress.
- Acquire or change the committing transaction through atomic compare and
  swap. Every mutating phase submits the exact operation ID, revision, and
  fencing generation. A superseded writer cannot commit after the new
  generation becomes active.
- A same-candidate request joins, observes, or resumes the existing operation
  instead of creating another. A different candidate request reports the
  active operation and supports explicit queue, cancel, discard, or
  transactional supersede choices.
- A logical transaction may span a long install, but no operating-system file
  mutex or Service State lock may span compilation, CI, artifact upload,
  network waits, process convergence waits, or large serialization. Hold each
  physical lock only for its bounded read, compare, or commit section.
- Use one documented lock order for every mutation: coordination transaction,
  install transaction, supervisor or runtime mutation, then Service State
  commit. Release physical locks in reverse order.

## Artifact Reuse

- Deduplicate builds by executable-input digest, target, toolchain, profile,
  features, and reviewed environment inputs. The first active claim builds into
  an isolated output; an equivalent request joins or waits for that claim
  instead of compiling again. Distinct builds may proceed only through the
  repository's resource-admission wrapper and must not share mutable outputs.
- Classify ordinary `ci`-profile development binaries as fast-iteration
  artifacts. They may prove behavior but are not production-promotable because
  their build profile differs from the required production profile.
- Permit a production-shaped artifact to run first in an isolated development
  runtime. Promotion reclassifies those exact sealed bytes after qualification;
  it does not rebuild or copy state from the development runtime.
- Seal a production candidate once with an immutable manifest that binds its
  source commit and tree, complete executable-input digest, target, toolchain,
  profile and resolved profile-configuration digest, features, reviewed
  environment inputs, embedded dashboard and asset digests, binary digest,
  support-manifest digest, and validation receipts.
- After protected-main integration, re-read the canonical remote and determine
  whether the sealed candidate's exact source commit entered its ancestry and
  whether any executable input changed. Reuse the exact artifact when the
  executable-input closure is equivalent, even when documentation or merge
  metadata changed.
- Rebuild only when an executable input changed, a required artifact or
  manifest is missing, or the candidate cannot be verified. Do not rebuild
  merely because a merge commit has a different identifier.
- Pin queued and active candidate generations against garbage collection until
  they become installed, discarded, superseded, or terminally failed.

## Timeout, Recovery, And Progress

- A CLI exit, connection timeout, idle agent, completed build, clean worktree,
  or stale chat report does not prove an operation terminal. Re-read the exact
  durable operation before choosing another transition.
- Resume a partially committed installation through the existing workstation
  transaction. Do not start a parallel replacement or infer `no_effect` from a
  disconnected client.
- Expiry alone does not permit a former writer to continue or a successor to
  commit. Recovery establishes the current fencing generation, process and
  transaction state, selected generation, supervisor and listener state, and
  protected runtime owners before selecting the next safe transition.
- Renew logical custody only while durable progress is observable. A stalled
  operation cannot heartbeat indefinitely. Queue entries do not hold the
  committing transaction or physical locks and are revalidated when selected.
- Record requests, recommendations, selected actions, joins, queue changes,
  cancellations, supersedes, commits, recovery, rollback, and terminal
  readback in an append-only receipt surface without secrets or tenant data.

## Development Test Coordination

- Give each active development lane a stable namespace with disjoint install,
  home, runtime, socket, port, profile, browser, and provider identities. A test
  must not borrow production state or another lane's namespace.
- Identify a test run by candidate digest, test-suite revision and selection,
  fixture digest, target platform, runtime capability manifest, and relevant
  environment inputs. An equivalent request joins an active run or reuses a
  completed receipt when the test declares itself hermetic and every identity
  still matches.
- A changed candidate, suite, fixture, target, capability, or relevant
  environment input starts a new run. Failed, cancelled, partial, quarantined,
  and provider-backed receipts are never generalized beyond their exact scope.
- Run provider-free tests concurrently only when their filesystem, process,
  port, profile, and target outputs are isolated. Serialize tests that truly
  share a browser, desktop, provider, production-like supervisor, or other
  workstation resource.
- On cancellation, timeout, or process exit, preserve the test receipt and run
  an exact task-owned residue check. Do not interpret client exit as cleanup or
  use broad process termination to make the next test pass.

## Interim Operation

Until the advisory orchestrator is implemented, perform only one user-directed
production or staging mutation at a time. Any currently authorized session may
perform it after fresh transaction and runtime readback; no permanent
coordinator designation is required. Other sessions may continue builds,
tests, inspection, and isolated development work. If the operator redirects
the active effect, reconcile or stop the current transaction first and record
the chosen handoff, cancellation, or supersede action.

## Adoption Notes

This Agent Browser repo-local extension responds to the Plan 0186 installer
collision and the subsequent policy review. It composes with active-lane
coordination, runtime-state governance, multi-agent reconciliation, and
collaborative development. Plan 0190 owns the deterministic advisory tool that
will implement this contract over the existing workstation transaction.
