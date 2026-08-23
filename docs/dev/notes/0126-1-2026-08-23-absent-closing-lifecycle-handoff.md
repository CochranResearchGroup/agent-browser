# Plan 0126 Handoff: Absent Closing Lifecycle

Date: 2026-08-23

State: HANDOFF OPEN

Lane: P126

Branch: `fix/reconcile-absent-closing-lifecycle`

Published checkpoint: `cd23311e2fcd0934a1d5e9d7b3a6b93cf4d0f847`

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
Service Health tests, formatting, strict Clippy, and diff hygiene. It also
records one bounded live reconciliation that moved the affected lifecycle to
`terminal/satisfied` without launching or killing a browser and without
modifying the authenticated profile.

This is not yet installed acceptance. A consuming tick proved that the first
repair restored launch planning, then exposed the logical-browser-ID rotation
defect repaired by `cd23311e`. That final source checkpoint has not been
transactionally installed, and no post-install consuming tick has been run.
Plan 0126 therefore remains `OPEN`.

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
Do not close this observation by inference. Either:

- prove with a provider-free close fixture that successful polite exit
  terminalizes the lifecycle and does not classify a force-kill failure, or
- open a separate bounded repair lane if that fixture reproduces the mismatch.

The supported remedy also failed after the service browser row had already
been removed. The reconciliation repair is now the intended recovery path for
that exact absent-row state, but remedy behavior should remain a separate
contract question until tested explicitly.

## Authority Order

Use these sources in order:

1. Current source and tests on this branch.
2. `docs/dev/plans/0126-2026-08-23-absent-closing-lifecycle-reconciliation.md`.
3. The current `ROADMAP.md` and `RUNBOOK.md` entries on this branch.
4. Cross-repo consuming evidence in the books-receipts repository at commit
   `f5ab836df2f1d636a92130ad50b45f8738ed76d1`, file
   `docs/dev/validation/0232-i2r-auth-probe-and-sandbox-target-gate.md`.
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
```

Expected Git checkpoint at handoff creation:

- branch `fix/reconcile-absent-closing-lifecycle`;
- HEAD `cd23311e2fcd0934a1d5e9d7b3a6b93cf4d0f847` before this note;
- local and remote divergence `0 0`;
- worktree clean before this note was added.

The installed CLI reported `agent-browser 0.28.0` on 2026-08-23. A version
string does not prove that the P126 candidate is installed. Re-read the
transactional upgrade receipt and installed generation before any installed
acceptance claim.

Use CodeGraph to inspect the structural flow beginning at
`handle_service_browser_close`, through runtime close and
`RuntimeLifecycleAuthority::transition`, and the admission guards in
`runtime_lifecycle.rs`. Use literal search only for the exact error codes.

## Next Bounded Packet

1. Re-run the focused source validation from Plan 0126 against the published
   checkpoint if branch custody or source has changed.
2. Transactionally install the exact candidate only under explicit installed
   acceptance authority. Bind the receipt to the candidate commit and binary
   identity.
3. Run one provider-free close fixture first. Require immediate lifecycle
   convergence and consistent polite-close and force-kill outcome fields.
4. Re-read the exact lifecycle and access plan. Require one owner, one cleanup
   obligation, no pending transfer, and launch admission at the next
   generation.
5. Run at most one bounded consuming tick only if the previous gates pass and
   the workflow owner still authorizes provider access.
6. Record installed identity, lifecycle before and after, process and lock
   evidence, launch admission, consuming result, and any rollback in a durable
   validation receipt.
7. Close Plan 0126 only when acceptance criterion 7 and the normal-close
   lifecycle invariant are both evidenced. Otherwise record the exact blocker
   or open the companion close-outcome repair lane.

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
- the consuming workflow passes launch admission without profile mutation.

## Hard Stops

- Do not edit the user-scoped Service State or runtime-owner registry by hand.
- Do not kill a PID, delete a profile, remove a profile lock, reauthenticate,
  or reseed a browser to make acceptance pass.
- Do not retry a provider workflow merely to diagnose lifecycle behavior.
- Do not install or upgrade Agent Browser without exact candidate identity,
  rollback evidence, and explicit installed-acceptance authority.
- Do not merge P126 while the plan remains `OPEN` or while the normal-close
  companion observation lacks a disposition.
- Do not copy tenant-specific runtime evidence into this repository.

## Handoff Recommendation

Continue on the existing P126 branch. The best next action is the
provider-free normal-close regression fixture, followed by transactional
installed acceptance of `cd23311e` only if that fixture passes. This preserves
the already-published lifecycle repair while testing the unresolved edge that
created the stale record in the first place.
