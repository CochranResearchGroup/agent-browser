# Plan 0126 Handoff: Absent Closing Lifecycle

Date: 2026-08-23

State: HANDOFF OPEN

Lane: P126

Branch: `fix/reconcile-absent-closing-lifecycle`

Published source checkpoint: `cd23311e2fcd0934a1d5e9d7b3a6b93cf4d0f847`

Prior handoff checkpoint: `b7561ae48e6af46446b2e9d4c88d61c46fb13f04`

Target: `main` at source baseline
`e3945810b3e15c507c00dd0218656735f266fcc0`

## Outcome So Far

The source repair is published, clean, and equal to its remote branch. It
contains three checkpoints:

1. `71e57e1b` reconciles an exact absent-process and absent-lock
   `closing/owned` lifecycle through the existing `CompleteClose`
   compare-and-swap transition.
2. `ffd5aae6` persists the reconciled runtime-owner registry without
   overwriting a concurrent registry mutation.
3. `cd23311e` permits an exact `terminal/satisfied` owner to advance one
   generation under a collision-free new logical browser ID.

The plan records passing source validation for 12 lifecycle tests, 50 focused
Service Health tests, formatting, strict Clippy, and diff hygiene. Current
source therefore permits an exact satisfied terminal owner to be replaced at
the next generation.

The immediate stale-owner blocker has cleared. Fresh live readback on
2026-08-23 found the affected lifecycle `terminal/satisfied`, no incident for
that lifecycle, and no global lifecycle blocking incident. This does not prove
the complete close and replacement lifecycle. The false force-kill
classification remains unproven as fixed, and no successful same-profile,
route-bound replacement has completed.

The workstation upgrade is complete and is not an open gate for this lane. Do
not describe another workstation upgrade as the next action. Plan 0126 remains
`OPEN` only for the bounded lifecycle and presentation repair and acceptance
described below.

## Current Runtime Interpretation

Operator directive:

> The workstation upgrade gate is closed. Continue unaffected Agent Browser
> work normally. Pause only work requiring the affected lifecycle or
> presentation route. P126 should continue in its development worktree with
> provider-free lifecycle and route acceptance; do not initiate another
> workstation upgrade.

Fresh read-only evidence on 2026-08-23 established:

- workstation transaction
  `upgrade-0df91191-ad9b-4eb9-aa85-2f92e9729563` is accepted;
- selected generation is
  `0.28.0-4b975a51aa89-d0782705d5ff`;
- installed binary SHA-256 is
  `4b975a51aa892241ea73cc6e8acef42bb67d781c8b9be43edbc1086f4d7956f8`;
- workstation `ready=true`, every readiness axis is true, and admission is not
  draining;
- the service reports one current runtime-host process and no lifecycle
  blocking incident;
- the shared runtime-host supervisor unit is stopped, even though the current
  runtime-host process is still observed;
- two presentation incidents are active. The P126 maintenance packet owns the
  orphaned display allocation on `guacamole:1`. The separate route-pool
  exhaustion incident belongs to another active presentation scope and must
  not be silently folded into this lane.

These axes are independent. The supervisor warning and route incident require
bounded runtime maintenance, but they do not make all Agent Browser work
unavailable. Only work requiring the affected presentation route or the
unvalidated lifecycle should pause.

## Incident Contract

The product-level failure is independent of any one tenant or provider:

1. A service-owned managed browser entered close.
2. Its process group exited and its service browser row disappeared.
3. The runtime-owner registry remained `closing/owned`.
4. Reconciliation preserved the stale lifecycle instead of completing it.
5. A later launch correctly failed closed with
   `runtime_lifecycle_existing_owner_requires_explicit_transition`.
6. After bounded reconciliation reached `terminal/satisfied`, replacement
   under a different logical browser ID failed with
   `runtime_lifecycle_terminal_replacement_rejected`.

The reusable invariant is:

> An exact managed owner whose close is in progress may become terminal only
> when owner ID, generation, profile identity, absent process-group evidence,
> and absent profile-lock evidence all agree. A satisfied terminal owner may
> then be replaced exactly once at the next generation without duplicating the
> profile lifecycle or cleanup obligation.

Live processes, present locks, missing profiles, identity mismatches,
generation mismatches, pending transfers, and logical-ID collisions remain
fail closed.

## Unresolved Companion Observation

The originating close also produced an internally inconsistent shutdown
report: the close request reported success, the polite close reported success,
and the recorded process was absent, while Service Health still recorded
`browser_shutdown_force_kill_failed` and escalated to
`os_degraded_possible`.

Plan 0126 repairs deterministic convergence from the resulting stale state. It
does not, by itself, prove that the ordinary close path now avoids an
unnecessary force-kill attempt or writes `terminal/satisfied` immediately.
Do not close this observation by inference. The next packet must reproduce or
disprove it with a provider-free close fixture, repair the classification if
reproduced, and prove one harmless same-profile close and route-bound
replacement without opening the target provider.

The supported remedy also failed after the service browser row had already
been removed. The reconciliation repair is now the intended recovery path for
that exact absent-row state, but remedy behavior should remain a separate
contract question until tested explicitly.

## Authority Order

Use these sources in order:

1. Fresh read-only workstation, Service Health, lifecycle, supervisor, and
   presentation readback.
2. Cross-repo upgrade acceptance in the books-receipts repository, file
   `docs/dev/validation/0232-i2q-agent-browser-upgrade-and-profile-gate-accepted.md`.
3. Cross-repo probe and sandbox evidence in the books-receipts repository,
   file
   `docs/dev/validation/0232-i2r-auth-probe-and-sandbox-target-gate.md`.
4. Current source and tests on this branch.
5. `docs/dev/plans/0126-2026-08-23-absent-closing-lifecycle-reconciliation.md`
   for source history. Its installation language is superseded by the accepted
   workstation state above.
6. The current `ROADMAP.md` and `RUNBOOK.md` entries on this branch.
7. This handoff note.

Chat, Graphiti recall, and old runtime snapshots are locators, not current
proof. Keep tenant identifiers, profile paths, authentication state, and raw
browser artifacts outside this repository.

## Fresh-Agent Startup

Suggested skills: `repo-policy-selector`, `graphiti-discovery`,
`codegraph-workspace`, `agent-browser-service`, and `diagnosing-bugs`.

Run read-only checks first:

```bash
git status --short --branch
git rev-parse HEAD
git rev-list --left-right --count @{upstream}...HEAD
agent-browser --version
agent-browser install workstation status --json
agent-browser install doctor --json
agent-browser --json service status
agent-browser --json service incidents --state active --limit 50
```

Expected Git checkpoint before this correction:

- branch `fix/reconcile-absent-closing-lifecycle`;
- HEAD `b7561ae48e6af46446b2e9d4c88d61c46fb13f04`;
- local and remote divergence `0 0`;
- worktree clean before this correction was added.

The installed CLI reported `agent-browser 0.28.0` on 2026-08-23. Use the
workstation-status receipt above for installed identity. The version string
alone is not identity proof, but the accepted transaction closes the existing
installation gate. Do not run another workstation upgrade for P126.

Use CodeGraph to inspect the structural flow beginning at
`handle_service_browser_close`, through runtime close and
`RuntimeLifecycleAuthority::transition`, and the admission guards in
`runtime_lifecycle.rs`. Use literal search only for the exact error codes.

## Next Bounded Packet

1. Keep the accepted workstation installation unchanged. Do not run an upgrade
   or reopen the installation handoff.
2. Re-run the focused source validation from Plan 0126 if branch custody or
   source has changed.
3. Use the isolated development runtime and provider-free fixtures to
   reproduce the polite-close and force-kill contradiction. Repair the close
   classification if it reproduces.
4. Diagnose the stopped supervisor and the orphaned `guacamole:1` display as
   separate runtime-maintenance and presentation axes. Preserve the observed
   runtime host and every unrelated browser, route, and controller lease.
5. Prove one harmless same-profile close and route-bound replacement against a
   local fixture page. Do not navigate to the provider, inspect authentication,
   or capture private page content.
6. Require immediate `terminal/satisfied` close convergence, consistent
   shutdown outcome fields, next-generation replacement admission, and a
   healthy route lease with no orphaned display.
7. Record source identity, runtime identity, lifecycle before and after,
   process and lock evidence, supervisor state, route and display disposition,
   and rollback in a durable validation receipt.
8. Keep the consuming Books Receipts mutation saga blocked. The sandbox
   inventory contains zero disposable transactions. A human must explicitly
   designate or create one disposable sandbox card transaction and separately
   authorize the mutation test.
9. Close Plan 0126 only when the normal-close lifecycle and harmless
   route-bound replacement are evidenced. Otherwise record the exact scoped
   blocker without globally blocking unrelated Agent Browser work.

## Acceptance Readback

The final acceptance must prove all of the following:

- an exact stale `closing/owned` record converges only with matching absent
  process and absent lock evidence;
- live or ambiguous owners remain unchanged;
- reconciliation persists without clobbering concurrent registry state;
- terminal replacement moves the lifecycle to a collision-free new logical
  browser ID at exactly the next owner generation;
- successful polite close does not generate a false force-kill failure;
- the service browser row and runtime lifecycle cannot disagree after normal
  close;
- the shared runtime-host supervisor is active and ready, or has an explicit
  nonblocking disposition consistent with the observed runtime host;
- the `guacamole:1` orphaned display is reconciled without disturbing an
  unrelated route or browser;
- route, display, browser, and profile ownership are released or retained
  according to their explicit contracts;
- a harmless local fixture passes same-profile route-bound replacement without
  provider navigation or profile mutation;
- unrelated Agent Browser work remains available when its own acquisition and
  presentation axes are ready.

## Hard Stops

- Do not edit the user-scoped Service State or runtime-owner registry by hand.
- Do not kill a PID, delete a profile, remove a profile lock, reauthenticate,
  or reseed a browser to make acceptance pass.
- Do not retry a provider workflow merely to diagnose lifecycle behavior.
- Do not run another workstation upgrade in this packet. The installation gate
  is accepted.
- Do not open the target provider during lifecycle or presentation acceptance.
- Do not prune an unrelated retained display, release another route, close
  another browser, or restart a shared component without exact scoped evidence
  and authority.
- Do not merge P126 while the plan remains `OPEN` or while the normal-close
  companion observation lacks a disposition.
- Do not copy tenant-specific runtime evidence into this repository.

## Handoff Recommendation

Continue on the existing P126 branch with one bounded maintenance packet: fix
or disprove the false force-kill classification, reconcile the stopped
supervisor and orphaned `guacamole:1` display within their own axes, then prove
a harmless same-profile close and route-bound replacement on a local fixture.
Do not reopen the workstation upgrade, do not open the provider, do not claim
the empty sandbox is a lawful mutation target, and do not globally block
unrelated Agent Browser work.
