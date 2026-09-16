# Plan 0199 | Compatible Access Profile Selection

Date: 2026-09-16

Plan version: 1

State: OPEN

Consolidation: required

Product lane: PL-BUGFIX

Lane: P199

Work item: `CochranResearchGroup/agent-browser#67`

Branch: `fix/p199-compatible-access-planning`

Target: `main`

Integration: merge through the protected `main` workflow after provider-free
regression and changed-surface validation

Source baseline: `faee8887fb420e8e46bd5a165ce6c44fa10059d2`

## Objective

Make unauthenticated access planning choose a retained profile only when the
profile is compatible with the browser capability selected for the request.
When an explicitly requested retained profile is incompatible, return a typed,
no-effect rejection instead of copying that profile into an executable browser
request.

## Current State

Issue #67 is claimed and marked in progress. The preserved Plan 0176 field note
shows an unrelated retained task profile winning a public, unauthenticated
request before capability posture was reconciled. Current source confirms that
catalog profile ranking runs before browser capability evidence is joined. The
later launch preflight has an exact `profile_compatibility_missing_or_blocked`
gate, but the access plan can already have selected and recommended the
incompatible profile.

P198 owns retained owner and inventory projection under issue #143. Its code
surface is disjoint, but it has an unpublished transition in the shared plan
catalog, ROADMAP, and RUNBOOK files. P198 remains primary writer for that
transition. P199 records the overlap and must reconcile the published catalog
before integration. P199 will not edit P198 lifecycle-owner code or its dirty
worktree.

## Consolidated Batch

1. Add one provider-free access-plan fixture where deterministic profile rank
   currently prefers an incompatible retained profile over a compatible
   candidate for the selected browser capability. Prove the fixture fails
   before changing policy.
2. Minimize the failure and test ranked hypotheses at the public no-launch
   access-plan seam.
3. Add the narrow capability-compatibility predicate to access-plan selection.
   Preserve explicit profile intent by rejecting an incompatible explicit
   profile rather than silently substituting another identity.
4. Return a typed no-effect planning outcome and ensure no provider, browser,
   profile, or Service State mutation is reachable from the rejected plan.
5. Run focused tests during repair, then complete changed-surface validation
   once against the frozen batch. Publish, review, integrate, and close only
   after exact-head CI is green.

## Scope And Effect Boundary

Expected write surfaces are `cli/src/native/service_access.rs`, the narrow
capability-compatibility predicate it consumes, provider-free fixtures, and the
minimum public contract and documentation needed to describe the typed
planning result. This plan and its branch-local lane projection are in scope.

This plan does not authorize Chrome launch, profile mutation, provider access,
Service State migration, installation, supervisor or shared-runtime mutation,
production effect, or release. It must not change custom filesystem-profile
identity (#131), retained owner or inventory projection (#143), authentication
execution (#96), or the core Service State persistence algorithm (#76).

## Delivery Sequence And Budget

- Attempt 1: red-capable fixture and minimized selection evidence.
- Attempt 2: one access-plan selection repair and focused regression tests.
- Attempt 3: one bounded correction if changed-surface validation exposes a
  defect introduced by this packet.
- Maximum work-unit attempts: 3.
- Maximum review and rework cycles: 1.
- Reassess after two checkpoints or 30 active minutes without outcome progress.
- Overall effort ceiling: 180 active minutes through protected integration.

No subagent, browser worker, runtime operator, benchmark checkout, fixture
worktree, or provider operator is assigned.

## Worker Assignments

The P199 lane owner holds the critical path, plan, branch, fixture, source
repair, validation, integration, and closeout. P198 remains primary writer for
its pending shared-authority transition. No parallel writer is assigned within
P199.

## Evidence And Exit

Exit requires current evidence that:

- the public provider-free access-plan fixture is red for the original
  incompatible selection and green after the repair;
- implicit selection deterministically chooses the compatible profile for the
  selected browser capability;
- an incompatible explicitly requested retained profile returns a stable typed
  rejection and no executable browser request;
- unrelated profiles and retained browser records are unchanged;
- no browser launch, provider call, profile mutation, Service State write, or
  protected retry occurred;
- focused regression tests, formatting, strict workspace Clippy, and every
  additional check selected from the complete changed surface pass; and
- the shared catalog is reconciled with P198 before the published source
  checkpoint enters `main` through the linked pull request with applicable CI
  green.

## Stop Condition

Stop and route a dependency through `PL-PLATFORM` if compatibility cannot be
decided from the existing browser capability registry contract. Stop before
editing P198 lifecycle-owner surfaces, Service State persistence, authentication
execution, custom profile identity, or any live runtime. Do not weaken explicit
profile intent, capability gates, fencing, or the no-effect contract.
