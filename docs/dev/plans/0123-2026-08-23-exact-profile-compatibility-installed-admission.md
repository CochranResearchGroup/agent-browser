# Plan 0123: Exact Profile Compatibility Installed Admission

Date: 2026-08-23

State: OPEN

Execution state: `read_only_admission_accepted_live_apply_requires_explicit_authorization`

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

- Local `main` is clean and two commits ahead of `origin/main` at candidate
  source commit `5fd4be88b2e35b4a6fe3e9e16f20ece8e20301f4`.
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

## Remaining Execution

1. Publish the accepted Plan 0122 source and this admission plan through the
   normal `main` integration path, then verify the remote commit.
2. Obtain explicit authority for the live transactional workstation apply.
3. Re-read source, installed, runtime, retained-browser, durable-handoff,
   process, and disk state immediately before apply. Stop on drift or ambiguity.
4. Create the supported workstation backup and retain its recovery locator.
5. Apply the exact candidate binary. Require stable closed-world census,
   candidate dashboard readiness, preserved browser and handoff identities,
   accepted transaction state, and rollback viability.
6. Re-run installed doctor, workstation status, runtime census, resources, and
   operating-system process readback. Do not perform opportunistic GC.
7. Run the attributed access plan and its recommended no-launch capability
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
