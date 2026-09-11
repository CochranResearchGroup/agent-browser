# Plan 0164: Forward-Only Workstation Upgrade Recovery

Date: 2026-09-10

State: OPEN

Consolidation: required

Lane: production workstation upgrade repair

Branch: fix/forward-only-workstation-upgrade

Target: main

Integration: merge

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
Its presentation timeout attempted rollback, the rollback could not reverse the
committed owner transfer, and the transaction entered
`operator_recovery_required`. Inspection incorrectly exposes `recover`, whose
old-generation preservation path correctly refuses the still-live candidate.

The selected production generation remains the old generation. The active
admission drain belongs to this exact transaction. No further rollback or
candidate termination is permitted.

## Consolidated batch

This batch contains one interdependent outcome:

1. classify committed or finalized cooperative lane ownership as a durable
   forward effect;
2. make guarded actions, automatic prior-install convergence, and failure
   handling consistently select forward resume after that effect;
3. reconstruct candidate dashboard custody when resuming the durable phase;
4. retain authenticated candidate presentation as readiness evidence without
   invoking rollback after the forward boundary; and
5. prove the repaired behavior locally, merge it, resume the exact stranded
   transaction, then perform one repaired production reinstall and verify one
   supervisor-owned runtime topology.

Changing provider capacity, weakening runtime identity checks, bypassing the
authenticated presentation receipt, and deleting retained generations are
outside this batch.

## Delivery sequence and budget

1. Add one public transaction-behavior regression: committed cooperative
   ownership exposes `resume` and rejects rollback. Implement only the shared
   forward-effect predicate needed to pass it.
2. Add one resume regression for the operator-recovery transition and candidate
   dashboard reconstruction. Implement the minimal durable replay path.
3. Add one regression proving presentation failure preserves forward recovery
   rather than calling rollback. Implement the minimal failure transition.
4. Run focused tests during repair, then one changed-surface qualification and
   one optimized candidate build after source freeze.
5. Merge the qualified commits to `main`. Use the repaired controller to resume
   the exact existing transaction with compare-and-swap evidence. Perform at
   most one subsequent repaired-candidate production reinstall.

The complete batch is bounded to 90 minutes of active work, four vertical TDD
cycles, one review/rework cycle, and two production forward transactions. The
first guarded resume demonstrated that dashboard rehydration rejected the
already-removed candidate before restaging, so that exact live red result
authorizes one affected-surface requalification and one replacement optimized
build. Any further source failure stops the production path for disposition.
Any live mismatch in transaction identity, revision, candidate digest,
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
| Source qualification | Validation selector, format, strict workspace Clippy, focused installer tests | All touched Rust surfaces pass once on the frozen source |
| Installed production state | Transaction receipts, binary digest, supervisor and process census, install doctor | Repaired generation is selected, exactly one production runtime host and one dashboard generation remain, and no duplicate fixture process exists |

RUNBOOK.md remains the sole current execution status. Close this plan only when
the source is merged, the repaired binary is installed, the exact production
transaction history proves no rollback was accepted, and fresh process plus
doctor readback proves the singular supervised topology.
