# Plan 0137 Custody Reconciliation And Slice J Handoff

Date: 2026-08-29

Status: READY FOR FRESH CONTEXT | MUTATING EXECUTION NOT YET AUTHORIZED

Scope: reconcile the stale Plan 0134 topic-lane projection, preserve its
retained BILL recovery evidence, and prepare the read-only Plan 0137 Slice J
production-candidate preflight.

## Objective

Make `main` and the active-lane catalog agree that Plan 0134 is closed and
Plan 0137 owns the remaining profile-acquisition recovery defects. Preserve the
retained BILL recovery handoff from the stale Plan 0134 branch, then prepare an
exact, no-effect Slice J candidate and consumer-acceptance proposal.

The reported Odollo contractor-portal test-profile failure is
`existing_session_profile_identity_unproven`. Plan 0137 explicitly owns this
case. Do not reopen Plan 0134 implementation or weaken the identity interlock.

## Authority Order

Use these sources in descending order:

1. `AGENTS.md` and the applicable files under `docs/dev/policies/`;
2. `docs/dev/plans/0137-2026-08-28-profile-acquisition-recovery-and-lifecycle-reliability-plan.md`
   on current `origin/main`;
3. `docs/dev/plans/0134-2026-08-26-crash-epoch-and-profile-lifecycle-coherence-plan.md`
   on current `origin/main`;
4. `docs/dev/active-lanes.yaml` as a custody projection, not plan authority;
5. the note-only commits on
   `plan/crash-profile-lifecycle-coherence`; and
6. this handoff as a restart aid, not independent effect authority.

Graphiti group `agent_browser_main` was queried on 2026-08-29. The runtime was
healthy, but the query returned no current Plan 0137 or Odollo defect fact.
Repository plans, notes, Git refs, and fresh runtime readback remain
authoritative.

## Verified Starting State

The source snapshot before this handoff note was:

- `main` and `origin/main`:
  `b489ae3648ff66317b34c032ac6c73c116088821`;
- `plan/authentication-run-foundation` and its origin ref:
  `e74dda166f2cd57f1c9c458777812798ceba2735`; and
- `plan/crash-profile-lifecycle-coherence` and its origin ref:
  `2c08ed6eb62a6ed00e24d78953320e455b15a4e1`.

Plan 0134 on `main` is `CLOSED` with execution state
`installed_candidate_accepted_successor_plan_0137_owns_recovery_gaps`.
Plan 0137 on `main` is `OPEN` with execution state
`slice_i_source_complete_live_acceptance_blocked_on_exact_orphan_cleanup_authority`.

Plan 0137 Slices A through I are complete. Slice J is the next plan packet. It
installs one exact production candidate and runs bounded profile-acquisition
acceptance for Last30days, Odollo, SoyLei, fictitious-record retirement,
manual seeding, and a foreign-principal control. Slice J production effects
are not authorized by this handoff.

## Custody Defect

`docs/dev/active-lanes.yaml` currently registers the following topic lanes:

- `plan/authentication-run-foundation`; and
- `plan/crash-profile-lifecycle-coherence` as P134.

The P134 registration is semantically stale. Its branch-local copy of Plan
0134 was changed back to `OPEN`, but the canonical plan on `main` is closed and
delegates the remaining defects to Plan 0137. The catalog auditor compares the
branch-local metadata and therefore does not discover this target-branch
authority conflict by itself.

The P134 branch is four commits ahead of `main`:

- `fa7bccee` adds
  `docs/dev/notes/0135-2026-08-28-retained-bill-browser-route-recovery-handoff.md`;
- `17c976f3` adds the registry-overlay profile gate to that note;
- `20c92743` normalizes the stale branch-local Plan 0134 custody metadata; and
- `2c08ed6e` removes backticks from that stale plan's machine-readable branch
  and target fields.

Only the first two note-only commits contain evidence that should be
considered for integration. Do not merge or cherry-pick `20c92743` or
`2c08ed6e`; doing so would reopen or rewrite the closed Plan 0134 authority on
`main`.

## Foreign Worktree State

At handoff creation, the authentication worktree contains an untracked file:

`docs/dev/notes/0138-f1-2026-08-29-bill-saved-credential-fieldwork-delta.md`

This file belongs to another active context. Do not edit, move, stage, commit,
stash, clean, or delete it. Its presence causes the active-lane audit to fail
closed with `dirty_uncheckpointed` for the authentication lane. That failure
does not authorize cleanup and does not block read-only work outside the
authentication worktree.

The main and Plan 0134 worktrees were clean at the same readback.

## Startup Checks

Start in `/home/ecochran76/workspace.local/agent-browser` and run:

```bash
git fetch --prune origin
git status --short --branch
git worktree list --porcelain
git -C /home/ecochran76/workspace.local/agent-browser-authentication-run status --short --branch
git -C /home/ecochran76/workspace.local/agent-browser-plan0131 status --short --branch
git rev-parse main origin/main
git rev-parse plan/authentication-run-foundation origin/plan/authentication-run-foundation
git rev-parse plan/crash-profile-lifecycle-coherence origin/plan/crash-profile-lifecycle-coherence
python .codex/skills/repo-policy-selector/scripts/audit_active_lanes.py \
  --repo-root /home/ecochran76/workspace.local/agent-browser \
  --default-ref refs/remotes/origin/main \
  --catalog-only \
  --json
```

Re-read the current Plan 0137 Slice I checkpoint and Slice J contract before
planning any candidate work. If any ref has moved, recompute the branch delta
and do not reuse the SHAs above as current authority.

## Bounded Next Packet

The recommended first packet is custody reconciliation plus read-only Slice J
preparation:

1. Revalidate that `fa7bccee` and `17c976f3` touch only the retained BILL
   handoff note.
2. Integrate those two note-only commits onto current `main`, or reproduce
   their exact final note through `apply_patch` if cherry-pick ancestry is no
   longer clean.
3. Confirm the integrated note contains no credential, capability, tenant
   identifier, profile path, raw provider URL, or durable handoff URL.
4. Update `docs/dev/active-lanes.yaml` to remove the stale P134 topic-lane
   registration. Do not modify the authentication lane or its foreign
   untracked file.
5. Record the custody disposition in ROADMAP or RUNBOOK only if their current
   Plan 0137 sections require it. Do not duplicate the full Plan 0137 contract.
6. Run documentation hygiene and the active-lane audit. The audit may remain
   fail-closed solely for the already-known foreign authentication file; report
   that exact residual condition rather than cleaning it.
7. Publish the reconciled main checkpoint through the normal repository
   workflow.
8. Before deleting the P134 branch or worktree, revalidate the exact tip,
   create and verify a recoverable archive ref, and require task authority that
   explicitly covers ref and worktree retirement.
9. Prepare, but do not apply, a Plan 0137 Slice J effect manifest containing
   the exact candidate commit, binary SHA-256, generation, dashboard assets,
   support payload, migration digest, backup receipt, rollback evidence,
   current Service State revision, process census, lease doctor, route
   inventory, handoff readiness, proposed consumer cases, and every hard stop.

The packet is complete when the retained note is durably preserved on `main`,
the stale P134 catalog entry is removed, the remaining catalog state is
truthfully reported, and one reviewable no-effect Slice J proposal exists.

## Slice J Effect Gate

Stop and obtain explicit authority before any of the following:

- installing, selecting, committing, rolling back, or closing a production
  candidate generation;
- mutating production Service State, leases, owners, sessions, routes,
  displays, providers, profiles, processes, or installation transactions;
- retiring fictitious records or applying orphan cleanup;
- acquiring or recovering the Odollo, SoyLei, Last30days, or any other real
  tenant profile;
- launching a browser, opening a provider site, entering credentials,
  navigating FedEx, submitting a tracking number, or inspecting private page
  content; or
- forcing shutdown, process cleanup, route cleanup, profile cleanup, or
  provider mutation.

A successful dry run or ready candidate is not effect authority. For the
Odollo contractor-portal case, preserve the distinction between an exact
principal/profile reconciliation defect and terminal-owner supersession.

## Validation Expectations

For the custody packet, run at minimum:

```bash
git diff --check
pnpm validation:select -- --base <pre-packet-main-sha>
python .codex/skills/repo-policy-selector/scripts/audit_active_lanes.py \
  --repo-root /home/ecochran76/workspace.local/agent-browser \
  --default-ref refs/remotes/origin/main \
  --catalog-only \
  --json
git status --short --branch
git branch -vv
git worktree list
```

Also verify local and origin equality for every retained branch after fetch.
No Rust or browser test is required for a note-and-catalog-only change unless
the validation selector or resulting diff expands the touched surface.

For Slice J, use the full validation and acceptance contract in Plan 0137.
Keep source qualification, candidate readiness, installation, consumer
acceptance, and live provider authority as separate proof boundaries.

## Suggested Skills

- `graphiti-discovery` for advisory prior-context lookup in
  `agent_browser_main`, followed by repository verification;
- `repo-policy-selector` for the catalog-only active-lane audit and custody
  reconciliation;
- `agent-browser-service` before any Service control-plane inspection or
  operation;
- `diagnosing-bugs` if the current source still reproduces an acquisition
  defect after the custody correction; and
- `handoff` if the packet stops at the explicit Slice J effect gate.

## Best Next Recommendation

Complete the custody-only packet first. Then present the exact Slice J
production effect manifest to the operator for authorization. Keep the
authentication-run lane paused and untouched until its current owner publishes
or disposes the foreign fieldwork note.
