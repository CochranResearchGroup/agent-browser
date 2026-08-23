# Plan 0127 Handoff: Absent Closing Lifecycle

Date: 2026-08-23

State: HANDOFF OPEN

Lane: P127

Branch: `fix/reconcile-absent-closing-lifecycle`

Published source checkpoint before this closeout: `750b17e8`

Prior handoff checkpoint: `b7561ae48e6af46446b2e9d4c88d61c46fb13f04`

Target: `main` at merged checkpoint
`7b235254`

## Outcome So Far

The source repair has six material checkpoints:

1. `71e57e1b` reconciles an exact absent-process and absent-lock
   `closing/owned` lifecycle through the existing `CompleteClose`
   compare-and-swap transition.
2. `ffd5aae6` persists the reconciled runtime-owner registry without
   overwriting a concurrent registry mutation.
3. `cd23311e` permits an exact `terminal/satisfied` owner to advance one
   generation under a collision-free new logical browser ID.
4. `e517beea` records successful auxiliary cleanup as a successful force-kill
   outcome after graceful browser exit.
5. `f11f3bd4` merges current `origin/main` and resolves the planning collision
   by preserving accepted P126 and renumbering this lane to P127.
6. `750b17e8` isolates unit tests from implicit installed-ingress state and
   aligns the unknown-session dashboard regression with current behavior.

Canonical Rust CI, strict Clippy, formatting, diff hygiene, route-confusion
gates, the selected workstation and Guacamole fixtures, docs build,
remote-view handoff docs, and installed skill parity pass. Current source
therefore permits an exact satisfied terminal owner to be replaced at the next
generation.

The immediate stale-owner blocker has cleared. The candidate is accepted in
the isolated development runtime as generation `0.28.0-b1a74a64a0dc` at
SHA-256
`b1a74a64a0dc0a80bb145a7334b741b7376c04b06829f77c72aa2ca955d9f22f`.
Development doctor and three disposable provider-free `about:blank`
open/read/close cycles pass. Post-smoke readback found zero sessions, zero
active incidents, and zero force-kill failure classifications. The remaining
unproved surface is one harmless route-bound same-profile replacement.

The workstation upgrade is complete and is not an open gate for this lane. Do
not describe another workstation upgrade as the next action. Plan 0127 remains
`OPEN` only for the bounded lifecycle and presentation repair and acceptance
described below.

## Current Runtime Interpretation

Operator directive:

> The workstation upgrade gate is closed. Continue unaffected Agent Browser
> work normally.

For this lane, continue source and provider-free lifecycle validation in the
isolated development worktree. Do not initiate another workstation upgrade or
open an authenticated provider merely to test lifecycle behavior.

Fresh evidence on 2026-08-23 established:

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
- the development publisher and smoke both report the production identity
  unchanged;
- development doctor passes on generation `0.28.0-b1a74a64a0dc`;
- three provider-free development-browser close cycles leave zero development
  sessions and zero active development incidents.

The remaining route-bound acceptance is a scoped presentation criterion. It
does not make the workstation, provider profiles, or unrelated Agent Browser
work unavailable.

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

## Resolved Companion Observation

The originating close produced an internally inconsistent shutdown
report: the close request reported success, the polite close reported success,
and the recorded process was absent, while Service Health still recorded
`browser_shutdown_force_kill_failed` and escalated to
`os_degraded_possible`.

Commit `e517beea` repairs that aggregation error, its regression passes, and
three provider-free development-browser close cycles complete without a
force-kill failure classification. This companion observation is resolved for
the provider-free close path. Route-bound replacement remains separate and
open.

The supported remedy also failed after the service browser row had already
been removed. The reconciliation repair is now the intended recovery path for
that exact absent-row state, but remedy behavior should remain a separate
contract question until tested explicitly.

## Authority Order

Use these sources in order:

1. Fresh read-only workstation, Service Health, lifecycle, supervisor, and
   presentation readback.
2. Current source and tests on this branch.
3. `docs/dev/plans/0127-2026-08-23-absent-closing-lifecycle-reconciliation.md`
   for source history. Its installation language is superseded by the accepted
   workstation state above.
4. The current `ROADMAP.md` and `RUNBOOK.md` entries on this branch.
5. This handoff note.

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

Expected Git checkpoint before this closeout commit:

- branch `fix/reconcile-absent-closing-lifecycle`;
- HEAD `750b17e8`;
- local is one commit ahead of the remote branch before this documentation
  closeout;
- worktree is clean before this closeout edit.

The installed CLI reported `agent-browser 0.28.0` on 2026-08-23. Use the
workstation-status receipt above for installed identity. The version string
alone is not identity proof, but the accepted transaction closes the existing
installation gate. Do not run another workstation upgrade for P127.

Use CodeGraph to inspect the structural flow beginning at
`handle_service_browser_close`, through runtime close and
`RuntimeLifecycleAuthority::transition`, and the admission guards in
`runtime_lifecycle.rs`. Use literal search only for the exact error codes.

## Next Bounded Packet

1. Keep the accepted workstation installation unchanged. Do not run an upgrade
   or reopen the installation handoff.
2. Re-run the validated source surface only if branch custody or source has
   changed.
3. Prove one harmless same-profile close and route-bound replacement against a
   local fixture page. Do not navigate to the provider, inspect authentication,
   or capture private page content.
4. Require immediate `terminal/satisfied` close convergence, consistent
   shutdown outcome fields, next-generation replacement admission, and a
   healthy route lease with no orphaned display.
5. Record source identity, runtime identity, lifecycle before and after,
   process and lock evidence, route and display disposition, and rollback in a
   durable validation receipt.
6. Close Plan 0127 only when the harmless
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
- Do not merge P127 while the plan remains `OPEN`.
- Do not copy tenant-specific runtime evidence into this repository.

## Handoff Recommendation

Continue on the existing P127 branch only for one bounded route-bound
acceptance packet: prove a harmless same-profile close and replacement on a
local fixture when an isolated presentation route is available. Do not reopen
the workstation upgrade, do not open the provider, and do not globally block
unrelated Agent Browser work.
