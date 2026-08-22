# Plan 0121: Plan 0117 Installed Acceptance And Convergence

Date: 2026-08-22

State: COMPLETE

Lane: P121

Source baseline: `5e3d6b873751e2ce62479c10c38c8aef016bf461`

Depends on:

- `docs/dev/plans/0117-2026-08-19-runtime-lifecycle-authority-and-convergence-plan.md`
- `docs/dev/notes/0117-8-2026-08-20-runtime-lifecycle-slice-h-source-acceptance.md`
- `docs/dev/notes/0117-9-2026-08-20-runtime-lifecycle-slice-i-admission-reconciliation.md`
- `docs/dev/plans/0120-2026-08-22-architecture-main-promotion-plan.md`

## Goal

Complete Plan 0117 Slice I against the promoted `main` line. Install the
validated current candidate through the transactional workstation path,
preserve every retained browser and durable remote-view handoff, converge the
workstation to one dashboard and one runtime host on one selected immutable
generation, and reclaim only resources whose package ownership and retention
eligibility are exact.

The maintainer authorized this bounded live plan and its execution on
2026-08-22. That authority does not waive the identity, retention, rollback,
or unrelated-resource hard stops below.

## Reconciled Starting Point

- Plan 0117 Slices A through H have source acceptance receipts, although the
  plan header still reports only Slice D source acceptance.
- The earlier Slice I admission receipt is historical evidence for an older
  candidate and installed generation. It is not current runtime proof.
- Plan 0120 promoted the reconciled architecture line to `main`; local `main`,
  `origin/main`, and the worktree began this plan at the source baseline above.
- A fresh installed-runtime, operating-system process, resource, retention,
  and generation census is required before any live mutation.

## Phase A | Read-Only Admission

1. Record the source commit, source version, installed command path, installed
   version, installed executable digest, selected generation, unit identities,
   and immutable-generation inventory.
2. Read install doctor, workstation status, workstation dry-run, Service
   Status, resource inventory, service GC dry-run, and workstation GC dry-run.
3. Record a fresh operating-system census for dashboard, runtime-host,
   legacy-daemon, Chrome, display, and remote-view process trees. Bind relevant
   processes to executable paths, start tokens, process groups, and owner
   generations without publishing private command lines.
4. Record bounded memory, disk, runtime-profile, transaction, and generation
   pressure.
5. Capture logical retained-browser and durable-handoff identities before the
   transaction. Do not record provider URLs, raw Guacamole routes, cookies,
   private profile contents, or authentication material.
6. Stop before build or apply if the dry-run is not immutable, the source tree
   is dirty for an unexplained reason, current ingress is not ready, retained
   browser identity cannot be read safely, or any candidate set contains an
   unrelated or ambiguous user-owned resource.

Terminal condition: a source-backed acceptance note contains a sanitized,
current baseline and an explicit admission decision.

## Phase B | Candidate Qualification And Backup

1. Select validation from the complete changed surface since the last accepted
   source checkpoint.
2. Run Rust formatting and strict Clippy through the repository Cargo wrapper,
   plus focused runtime-host, workstation, retention, GC, service-contract,
   dashboard, and documentation checks selected by the changed surface.
3. Build the optimized candidate from the exact source commit through the
   repository Cargo wrapper and record its SHA-256 digest.
4. Re-run workstation dry-run using that exact candidate executable. Require a
   planned, non-mutating result whose candidate sets match the reviewed
   baseline.
5. Create the documented workstation backup and record its transaction-safe
   recovery locator before apply.

Terminal condition: the exact candidate is validated, its reviewed dry-run is
stable, and recovery evidence exists before live mutation.

## Phase C | Transactional Installed Convergence

1. Apply through the exact qualified candidate executable and the
   transactional workstation installer.
2. Require authenticated candidate dashboard ingress and candidate runtime-host
   readiness before selector or ownership commit.
3. Transfer every retained runtime lane with exact owner-generation,
   executable, process, profile, target, display, route, lease, and durable
   handoff evidence.
4. Preserve old-host admission and rollback authority until every committed
   lane and operator-visible journey is proven on the candidate.
5. Abort or reverse on any retained-browser, profile, target, route, display,
   lease, handoff, or operator-visible identity mismatch.

Terminal condition: the transaction is accepted, the selected generation is
the qualified candidate, every retained browser and handoff is preserved, and
rollback remains valid.

## Phase D | Reviewed Reclamation

1. Re-run Service resource and workstation retention inventories after the
   accepted transaction.
2. Apply unattended reconciliation only to identity-proven package-owned stale
   process trees and policy-eligible expired ephemeral profiles.
3. Review the exact remaining profile, transaction, and generation plan. Apply
   only candidates whose review token and references remain unchanged.
4. Preserve named persistent profiles, the current generation, the immediate
   healthy rollback generation, all referenced resources, and all ambiguous
   resources.
5. Record recovery instructions for every retained ambiguity instead of
   widening cleanup scope.

Terminal condition: no overdue apply-safe candidate remains, while every
protected or ambiguous resource has a typed reason.

## Phase E | Terminal Acceptance And Closeout

1. Re-run doctor, workstation status, reconciliation, Service Status, resource
   inventory, both GC dry-runs, dashboard readiness, durable handoff
   resolution, profile locks, generation inventory, operating-system process
   census, disk usage, and process RSS.
2. Prove exactly one healthy dashboard process and one healthy runtime-host
   daemon process in steady state, both on the selected current immutable
   generation. No legacy per-session daemon may remain.
3. Prove one current generation plus at most one healthy rollback generation is
   retained on disk, with no historical transaction pinning an otherwise
   unreferenced generation.
4. Prove no unreferenced package-owned browser process tree, no overdue
   reclaimable ephemeral profile, a current monitor summary, no PAM or keyring
   event caused by the operation, and zero unexplained readiness drift.
5. Reconcile the Plan 0117 header and receipts, record the installed acceptance
   note, update the roadmap and runbook, validate the final diff, create
   structured truthful commits, push `main`, and verify the remote readback.

Terminal condition: all Plan 0117 acceptance criteria are current and
source-backed, the worktree is clean, and `main` equals `origin/main` at the
pushed closeout commit.

## Completion Receipt

Plan 0121 completed the authorized installed convergence on 2026-08-22.
Runtime implementation commit `9fb73d15` produced candidate binary SHA-256
`aa21c5fe8a6dd75f1422bd84147756f984ea8662fc5d9a1ea3afac1c37eed452`.
Transaction `upgrade-52684512-bfc2-4c30-971b-ab166eaa5364` was accepted after
an authenticated resolution of durable handoff `r520477` on the candidate
dashboard. The accepted transaction was reviewed and finalized, then
generation GC removed only obsolete generation
`0.28.0-bcfab70c2be9-7ad9e5b748d3`.

The terminal doctor reports one dashboard process, one runtime host, one
executable generation, zero legacy daemons, converged runtime status, and no
issues. Service GC reports zero candidates. The operating-system readback
shows one package-managed browser root, the preserved
`last30days-facebook` browser, and the durable handoff responds with HTTP 200.
Detailed evidence and validation are recorded in
`docs/dev/notes/0117-11-2026-08-22-installed-runtime-convergence-acceptance.md`.

## Hard Stops

- Do not signal a process without exact root PID, start token, executable,
  process group, owner generation, launch identity, and profile identity.
- Do not move or delete a profile with a process, browser, session, lease,
  handoff, transaction, rollback, quarantine, or other live reference.
- Do not delete a generation with a live executable, selected-generation,
  supervisor, transaction, rollback, failure-evidence, or unclosed reference.
- Do not commit runtime-host ownership for ambiguous, missing, or wrong lanes.
- Do not replace the dashboard before authenticated candidate ingress and its
  durable presentation journey are ready.
- Do not apply a candidate list containing an unrelated resource or a resource
  whose package ownership is inferred only from name, path shape, age, or PID.
- Do not continue if rollback cannot preserve every retained browser and
  durable handoff.
- Do not provide, request, store, or automate an operator password. Stop at an
  intentional interactive privilege boundary if the installed helper is not
  sufficient.
- Do not edit private Service State, owner registries, transaction ledgers, or
  retention metadata by hand.

## Non-Goals

- A formal release or upstream pull request.
- Deleting named persistent profiles merely to reduce disk use.
- Disturbing unrelated Chrome, desktop, Guacamole, RDP, or user workloads.
- Treating a doctor advisory alone as authority to launch, terminate, or
  reclaim a browser.
- Combining upstream synchronization or branch cleanup with installed-runtime
  convergence.
