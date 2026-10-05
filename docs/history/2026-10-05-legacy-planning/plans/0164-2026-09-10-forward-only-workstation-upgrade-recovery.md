# Plan 0164: Forward-Only Workstation Upgrade Recovery

Date: 2026-09-10

State: COMPLETE

Consolidation: required

Lane: production workstation upgrade repair

Branch: main

Target: main

Integration: direct `main`

Parent: [Plan 0160](0160-2026-09-06-production-profile-identity-and-operational-readiness.md), A3

## Objective and authority

Repair the workstation installer so an upgrade that has durably committed a
runtime-lane ownership transfer can only proceed forward. Then qualify one
candidate, merge it to `main`, resume the exact stranded production transaction
without rollback, and reinstall the repaired candidate with a singular
supervised runtime topology.

The operator explicitly authorized the source repair, merge when advised, and
production reinstall. The operator explicitly prohibited rollback during the
reinstall. This plan does not authorize browser termination, broad process
cleanup, provider mutation, release publication, or tenant effects.

## Current state

Production transaction
`upgrade-bc9935eb-9425-426b-a8bf-fe8e2d00fc14` has one committed,
unfinalized cooperative runtime handoff and one live candidate runtime host.
Its original presentation timeout attempted rollback, the rollback could not
reverse the committed owner transfer, and the transaction entered
`operator_recovery_required`. The first repair corrected that classification
and made inspection expose `resume`. Live resume then exposed two additional
upgrade defects: a newer controller synthesized an older candidate's dashboard
asset identity from its own embedded assets, and installation blocked on an
operator-presentation receipt that the current profile state cannot produce.

The selected production generation remains the old generation. The active
admission drain belongs to this exact transaction. No further rollback or
candidate termination is permitted.

## Consolidated batch

This batch contains one interdependent outcome:

1. classify committed or finalized cooperative lane ownership as a durable
   forward effect;
2. make guarded actions, automatic prior-install convergence, and failure
   handling consistently select forward resume after that effect;
3. reconstruct candidate dashboard custody when resuming the durable phase and
   observe the candidate's own executable-bound manifest;
4. commit a healthy dashboard deployment independently of authenticated
   operator presentation, while retaining presentation as a visible readiness
   axis that may converge later; and
5. prove the repaired behavior locally, resume the exact stranded
   transaction, then perform one repaired production reinstall and verify one
   supervisor-owned runtime topology.

Changing provider capacity, weakening runtime identity checks, bypassing the
truthfulness of presentation receipts, and deleting retained generations are
outside this batch. No receipt is synthesized: an absent operator journey stays
reported as absent and no longer blocks executable deployment.

## Delivery sequence and budget

1. Add one public transaction-behavior regression: committed cooperative
   ownership exposes `resume` and rejects rollback. Implement only the shared
   forward-effect predicate needed to pass it.
2. Add one resume regression for the operator-recovery transition and candidate
   dashboard reconstruction. Implement the minimal durable replay path.
3. Add one regression proving presentation failure preserves forward recovery
   rather than calling rollback. Implement the minimal failure transition.
4. Add candidate-owned manifest and health-only deployment regressions, then
   decouple installation readiness from the still-visible presentation axis.
5. Run focused tests during repair, then one changed-surface qualification and
   one optimized candidate build after source freeze. Use the repaired controller to resume
   the exact existing transaction with compare-and-swap evidence. Perform at
   most one subsequent repaired-candidate production reinstall.

The complete batch is bounded to 90 minutes of active work, four vertical TDD
cycles plus two live-derived regression cycles, one review/rework cycle, and
two production forward transactions. Prior optimized builds are diagnostic;
only one final source-frozen build may be installed. Any live mismatch in
transaction identity, revision, candidate digest,
admission-drain ownership, or process identity stops effects without rollback.

## Worker assignments

The primary owns source edits, transaction semantics, validation, git
integration, production mutation, and final acceptance. No worker is assigned;
the transaction seam is concentrated in one Rust module and production custody
must remain with the primary.

## Evidence and exit

| Requirement | Required evidence | Exit condition |
| --- | --- | --- |
| Forward-only classification | Focused red then green Rust test | Committed cooperative ownership exposes only `inspect` and `resume`; rollback returns a typed forward-only refusal |
| Durable replay | Focused isolated resume test | Exact operator-recovery transaction returns to its durable phase without requiring a full-shutdown receipt |
| Presentation failure | Focused failure-path test | Candidate and drain are retained in forward recovery; no rollback transition is recorded |
| Candidate manifest identity | Focused dashboard test | A newer controller accepts the sealed candidate's own live embedded-asset manifest only when its executable path and digest match |
| Presentation decoupling | Focused ingress and post-commit tests | Healthy deployment selects the candidate with `operatorJourneyReady=false`; no presentation receipt is invented |
| Source qualification | Validation selector, format, strict workspace Clippy, focused installer tests | All touched Rust surfaces pass once on the frozen source |
| Installed production state | Transaction receipts, binary digest, supervisor and process census, install doctor | Repaired generation is selected, exactly one production runtime host and one dashboard generation remain, and no duplicate fixture process exists |

RUNBOOK.md remains the sole current execution status.

## Completion evidence

The source repair is committed on `main` through `8070505d`. In addition to the
planned forward-only, durable resume, candidate-manifest, and presentation
decoupling work, live qualification exposed and repaired three successor
compatibility defects: post-commit generation selection now makes rollback
unavailable, runtime-monitor backoff is scoped to the executable SHA that
failed, and missing operator-journey proof remains a visible doctor warning
without making the installed workstation nonzero.

Final transaction `upgrade-4026dffd-af10-4f33-a519-2a430929fb6e` accepted
generation `0.28.0-6d4e6085c1de-e6cab967af18` at revision 13 with terminal
result `accepted`, no stop reason, and zero outstanding owner obligations. The
installed binary SHA-256 is
`6d4e6085c1dee1a6e135e473f221277b0cf1ca4fd48fc79d16c767437e234381`.
No rollback command was used for the final transaction.

Fresh installed doctor returned success with no blocking issues. The deferred
operator journey is the sole warning. Runtime multiplicity is
`steady_current`: one dashboard process, one runtime host, one executable
generation, and zero legacy daemons. The session supervisor is ready on the
selected executable with zero restarts. The runtime monitor is healthy, has
zero consecutive failures, and is bound to the installed SHA. The leaked test
daemon PID 39694 was proven to own only
`/tmp/ab-service-collections-no-launch-IKTKS5/s/runtime-host.sock`, terminated
by exact PID, and verified absent. The production Agent Browser skill is
synchronized with the repository copy.

Focused live-derived regressions, Rust formatting, and strict workspace Clippy
passed. Earlier batch qualification also passed the workstation installer
fixture, source-free fixture, dashboard/docs checks, and the partitioned
installer tests recorded in the structured commit history. This plan is
complete; Plan 0160 remains authoritative for wider A1 to A4 and AX work.
