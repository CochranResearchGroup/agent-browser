# Plan 0127 Hotfix And Runtime Acceptance

Date: 2026-08-23

State: ACCEPTED

Lane: P127

Branch: `fix/reconcile-absent-closing-lifecycle`

## Outcome

The absent-closing lifecycle repair is installed and accepted in production.
The selected workstation generation is
`0.28.0-a89625b870c3-1e2c09b12ebc`, built from binary SHA-256
`a89625b870c3cda3cde9b41f27271ebe36d60683b1235c4196c4be337bb39ea6`.
Transaction `upgrade-7d9a2776-2c7e-458c-8e4c-eb2bbe989c46` is accepted with
all workstation readiness axes true and rollback ready.

The isolated development runtime was not changed. It remains selected at
generation `0.28.0-b1a74a64a0dc` with SHA-256
`b1a74a64a0dc0a80bb145a7334b741b7376c04b06829f77c72aa2ca955d9f22f`.

## Repairs Added During Hotfix Qualification

- The installer shadow-dashboard timeout now covers the candidate service
  lane's bounded recovery interval.
- Exact reversed stale handoff-prepare retries are removed only when owner,
  generation, browser, profile, process, family, CDP, target set, and source
  route all match the already-reversed current owner.
- The installer performs that exact stale-descriptor cleanup before asking an
  old source daemon to prepare another transfer, avoiding a bootstrap
  dependency on candidate code.
- Managed-browser lifecycle refresh now requires an exact logical browser,
  profile identity, and process identity match. A newly launched PID cannot
  inherit a terminal binding for the exited process and must instead pass the
  existing terminal-replacement compare-and-swap.

## Acceptance Evidence

- Rust formatting and strict Clippy passed.
- Focused stale-retry, candidate-timeout, terminal-replacement admission,
  terminal-generation activation, and managed-binding identity regressions
  passed.
- The candidate dashboard returned an authenticated ready presentation receipt
  for a provider-free local fixture before ingress commit.
- The accepted runtime replaced the exact same logical browser and profile at
  owner generation 3 on a route-bound local page. Browser-build proof matched
  and operator-visible state was `ready`.
- Closing that replacement produced `terminal/satisfied` with
  `exact_process_exited` and `profile_lock_released` evidence.
- Five consecutive post-close readbacks found no matching browser or session,
  Route B available, and zero cleanup candidates.
- Runtime multiplicity converged to one current runtime host, one current
  dashboard process, and no legacy daemons. Unrelated active browsers and the
  separate feature-work checkout were not closed, restarted, or edited.

## Failed-Closed Qualification Receipts

Qualification preserved the selected old generation whenever evidence was
insufficient. It exposed and repaired the candidate-dashboard timeout, stale
handoff replay, old-daemon bootstrap, and stale in-memory lifecycle-binding
defects. A presentation attempt without a live ready handoff timed out and
rolled back normally. No failed attempt selected an unaccepted candidate.

## Remaining Nonblocking Observation

Standalone install doctor still reports one stopped historical
`last30days-home-feed` session-supervisor record even though runtime
convergence is healthy and the shared stream is reachable. This record was
outside P127 and was left unchanged.

## Next Recommendation

Review and integrate the P127 branch through the normal protected-main
workflow. Do not run another workstation upgrade for this lane unless source
or installed identity changes. Keep provider workflows and unrelated active
lanes governed by their own readiness and authority.
