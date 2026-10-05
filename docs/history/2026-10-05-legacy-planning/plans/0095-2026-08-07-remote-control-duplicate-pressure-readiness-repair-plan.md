# Plan 0095: Remote-Control Duplicate-Pressure Readiness Repair

Date: 2026-08-07
State: CLOSED
Lane: P95
Depends On:
- `docs/dev/notes/0095-2026-08-07-last30days-remote-control-duplicate-pressure-handoff.md`
- `docs/dev/plans/0026-2026-06-02-browser-resource-safety-plan.md`
- `docs/dev/plans/0027-2026-06-05-minimal-runtime-profile-reuse-plan.md`

## Goal

Let the single-route remote-control readiness aggregate proceed when the only
install-doctor issue is duplicate-profile pressure and the service reports zero
readiness-impacting cleanup candidates. Preserve the install warning, all
other install blockers, and the target-profile duplicate guard enforced by the
actual remote-view open request.

## Current Evidence

- Installed version `0.28.0` reports one install issue,
  `service_duplicate_profile_pressure`, with zero cleanup candidates, zero
  readiness-impacting candidates, and two duplicate-pressure warnings.
- The remote-control aggregate reports every route, gateway, display-access,
  and browser-launch prerequisite ready, but still reports `ready: false` and
  recommends `repair_install_drift` because it equates aggregate install-doctor
  success with remote-control install readiness.
- The service resource report attributes both warnings to the unrelated
  `default` profile. The requested `last30days-facebook` profile has no live
  browser or active lease.
- The remote-view open request already rejects a same-profile retained browser
  or conflicting profile lease unless the caller supplies an explicit bounded
  override. This remains the target-scoped safety authority.

## Authority And Safety

- Keep `agent-browser install doctor` unchanged. Its duplicate-pressure issue
  remains visible and its aggregate success remains false.
- Treat duplicate pressure as nonblocking only inside remote-control readiness,
  only when it is the complete install issue set, the install doctor returned
  structured issue data, and readiness-impacting candidates equal zero.
- Fail closed for timeouts, missing or malformed issue data, any additional
  issue code, or any positive readiness-impacting candidate count.
- Keep the raw install-doctor result and the effective remote-control
  classification distinct in JSON output.
- Keep the top-level next action and remote-view issue list consistent with the
  effective classification. A nonblocking warning must not become a repair
  instruction or a blocking remote-view issue.
- Do not close browsers, release leases, apply resource cleanup, create a
  browser, visit Facebook, or alter stored profile state during this plan.

## Public Contract

- `remoteControl.installDoctorReady` reports the raw embedded install-doctor
  success value.
- `remoteControl.installReady` reports effective install readiness for the
  single-route remote-control aggregate.
- `remoteControl.nonBlockingInstallIssueCodes` lists issue codes accepted by
  that narrow classifier. It is empty when the install doctor is ready or when
  any issue remains blocking.
- When duplicate pressure is the sole issue and readiness-impacting candidates
  are zero, the remote-control aggregate may become ready and recommend its
  live gate while the embedded install report remains unsuccessful and retains
  the warning.
- All other install failures preserve their existing recovery actions and
  blocking issue records.

## Slices

1. Add failing unit coverage for the raw-versus-effective classification, the
   top-level recommendation, and the remote-view issue projection.
2. Implement one shared remote-control install-readiness classifier and use it
   at all three seams.
3. Update CLI help, README, installed skill guidance, docs site guidance, and
   inline source comments for the new JSON distinction.
4. Run focused Rust tests, formatting, strict Clippy, selected changed-surface
   validation, and the relevant broader test gate.
5. Install the reviewed candidate through the normal user-scoped checkpoint
   flow and re-read the installed doctor output without opening or closing a
   browser.

## Done Definition

- A regression fixture matching the observed duplicate-pressure payload proves
  raw install-doctor failure, effective remote-control install readiness, no
  blocking remote-view install issue, and the correct next action.
- Mixed, malformed, timed-out, and readiness-impacting install failures remain
  blocking.
- The installed runtime reports all current single-route prerequisites ready
  without recommending install repair for unrelated duplicate pressure.
- Pre-validation and post-validation service readbacks show that existing
  browser and session ownership was not changed by this plan.
- Source, docs, installed runtime, ROADMAP, RUNBOOK, and validation evidence
  agree with the result.

## Closeout

- One shared classifier now distinguishes raw install-doctor success from the
  effective single-route remote-control gate. It accepts only a complete issue
  set containing `service_duplicate_profile_pressure` with exactly zero
  readiness-impacting candidates.
- The classifier is used by `remoteControl`, the top-level next-action
  recommendation, and remote-view issue projection. Mixed, malformed,
  timed-out, and readiness-impacting reports remain blocking.
- JSON output now exposes `installDoctorReady`, effective `installReady`, and
  `nonBlockingInstallIssueCodes`. CLI help, README, repo and installed skill
  guidance, docs-site guidance, and inline source documentation agree.
- The regression was observed failing before implementation. All 44
  remote-view doctor tests, the same-profile retained-browser guard test, the
  canonical partitioned Rust suite, strict Clippy, Rust formatting, docs build,
  patch hygiene, and skill sync pass.
- The installed `0.28.0` executable SHA-256 is
  `8582bf0900b4d974994846c4ff3985746dcbbf5ee2136699f68e56ea5e73726b`.
  Its workstation payload, dashboard manifest, and runtime inventory converge.
- Current installed doctor output has no resource warning because the publish
  handoff retired inactive daemon listeners. The exact original warning-only
  payload remains covered by the regression fixture. Current remote control
  reports all prerequisites ready and recommends `run_remote_view_open_live_gate`.
- The publisher reported a `p0065` resume error after the replacement daemon
  had already attached. Direct readback proved the current daemon carries the
  installed SHA-256, owns the original browser PID `19675`, and returns the
  existing URL. The LitScout lane also returns its existing ChatGPT URL. All
  four pre-publish ready browser records remain ready after convergence.
- No browser was opened or closed, no profile lease was released, and no
  resource cleanup was applied for this plan. Unrelated untracked files remain
  excluded and untouched.
