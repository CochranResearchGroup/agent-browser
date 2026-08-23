# Plan 0123: Exact Profile Compatibility Installed Admission

Date: 2026-08-23

State: OPEN

Execution state: `source_published_read_only_admission_accepted_live_apply_requires_explicit_authorization`

Lane: P123

Candidate source commit: `5fd4be88b2e35b4a6fe3e9e16f20ece8e20301f4`

Depends on:

- `docs/dev/plans/0122-2026-08-23-exact-profile-capability-compatibility.md`
- `docs/dev/plans/0121-2026-08-22-plan-0117-installed-acceptance-and-convergence-plan.md`

## Goal

Promote the exact-profile compatibility repair through the transactional
workstation path, preserve every current runtime lane and operator journey,
then prove the installed access plan and executable no-launch preflight agree
for the same selected profile before any browser acquisition.

## Current Read-Only Admission

- Candidate source commit `5fd4be88b2e35b4a6fe3e9e16f20ece8e20301f4`
  and its Plan 0123 admission commit `f697969de3ec89e631d3148626348fa9777c7ea8`
  are published on `origin/main`; exact remote readback matched `f697969d`.
- The remote baseline is
  `8c81de89e8103f9d990af7fbb7bb752d6473d1e9`.
- The optimized candidate reports version `0.28.0` and SHA-256
  `ae49edfd9d71161543c8378c06688876984f891b46cedca5272de1e77ca2f811`.
- The installed executable remains SHA-256
  `aa21c5fe8a6dd75f1422bd84147756f984ea8662fc5d9a1ea3afac1c37eed452`
  in generation `0.28.0-aa21c5fe8a6d-25828e3b8aed`.
- Installed doctor returns success with zero issues, one dashboard, one runtime
  host, one executable generation, zero legacy daemons, and converged runtime
  state. Dashboard ingress and the authenticated operator journey are ready.
- Candidate workstation dry-run returns `success=true`, `state=planned`,
  `mode=dry-run`, and `mutated=false`. Host admission reports Ubuntu 24.04
  x86_64, no missing commands, effective `agent-browser` and `docker` groups,
  and more than the six GiB disk minimum.
- The dry-run intentionally does not create an upgrade transaction or runtime
  census receipt. Real apply must collect two matching read-only census rounds
  before candidate staging, admission drain, or ownership transfer.
- Service resource inventory reports zero GC candidates, zero warnings, five
  owned cleanup obligations, and zero unknown or transferring obligations.
  Service GC and generation GC dry-runs both report zero candidates; one
  selected generation is retained.
- The retained Service baseline contains four browser records: three ready and
  one degraded. The exact managed retained browser is
  `session:plan0117-final-runtime` on profile `last30days-facebook`; external or
  ambiguous rows remain preservation-only inputs, not cleanup candidates.

## Remaining Execution

1. Obtain explicit authority for the live transactional workstation apply.
2. Re-read source, installed, runtime, retained-browser, durable-handoff,
   process, and disk state immediately before apply. Stop on drift or ambiguity.
3. Create the supported workstation backup and retain its recovery locator.
4. Apply the exact candidate binary. Require stable closed-world census,
   candidate dashboard readiness, preserved browser and handoff identities,
   accepted transaction state, and rollback viability.
5. Re-run installed doctor, workstation status, runtime census, resources, and
   operating-system process readback. Do not perform opportunistic GC.
6. Run the attributed access plan and its recommended no-launch capability
   preflight for the consuming profile. Require exact compatibility identity
   agreement and `wouldLaunch=false` before any browser or provider request.

## Acceptance

- The remote source commit is exact and the installed binary digest binds to
  the reviewed candidate source.
- The workstation transaction is accepted with no missing or changed runtime
  identity and no operator-recovery state.
- Every retained browser, tab, profile, lease, display, route, handoff, and
  unrelated observed browser remains preserved.
- Installed doctor is green and runtime multiplicity returns to one dashboard,
  one runtime host, one executable generation, and zero legacy daemons.
- The installed access plan and no-launch preflight agree on the selected
  profile-host-executable compatibility row.

## Hard Stops

- Do not treat this read-only admission as live apply authority.
- Do not apply after source, candidate hash, installed state, census, browser,
  handoff, process, or disk drift without a fresh reviewed admission.
- Do not kill, close, detach, reprofile, or garbage-collect a runtime to make
  census pass.
- Do not launch a browser or contact a provider during installed qualification.
- Do not expose private command lines, profile paths, provider URLs,
  credentials, or raw handoff routes in tracked evidence.

## Publication Receipt

Push `8c81de89..f697969d` advanced `origin/main` successfully. A direct remote
readback returned
`f697969de3ec89e631d3148626348fa9777c7ea8`, and local divergence became zero.
Publication changed source history only; the installed generation and running
service remained unchanged.
