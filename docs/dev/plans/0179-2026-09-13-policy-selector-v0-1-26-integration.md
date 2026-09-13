# Plan 0179 | Policy Selector v0.1.26 Integration

Date: 2026-09-13

State: OPEN

Lane: P179

Product lane: PL-PLATFORM

Branch: `platform/policy-selector-v0-1-26-rollout`

Target: `main`

Integration: merge

Work item: [issue #89](https://github.com/CochranResearchGroup/agent-browser/issues/89)

## Objective

Integrate the already-created repository policy selector v0.1.26 rollout
through scoped Git custody, current-main reconciliation, local validation, and
pull request review. Restore canonical `main` to exact remote authority before
continuing unrelated maintenance or bugfix work.

## Current State

Commit `133bed0de3628feb438c544fb1bef0ae3719aaf1` was created directly on local
`main` while the remote advanced. It is now preserved on the published scoped
branch. Canonical `main` is clean and equal to `origin/main` at
`ae4266429b94dedef087e91c38087c7742f3caee`. The scoped branch has merged that
remote tip without conflict and is undergoing bounded policy validation.

Checkpoint `966c61e4a721d1cd2355a5ec252ef442fc1a6828` repairs repository-specific
policy routing, installed-bundle test paths, v0.1.26 wiring, active-only audit
classification, and the three residual active-planning ledger findings.
Policy wiring, 122 selector tests with three source-checkout-only skips, the
goal audit, the active planning audit, remote-view documentation checks, and
patch hygiene pass. Pull request review and integration remain.

## Scope

- Preserve the v0.1.26 selector bundle, adopted policy modules, tests, and
  rollout receipt on one scoped branch.
- Reconcile the branch with current `origin/main` without rebasing its
  published checkpoint.
- Correct any rollout-local policy wiring or documentation defect found during
  review.
- Run selector, policy wiring, planning, goal, and changed-surface checks.
- Integrate through a pull request, then verify ancestry before retiring the
  branch and worktree.

## Non-goals

- No browser, profile, provider, tenant, installation, service, or supervisor
  mutation.
- No Plan 0178 installed acceptance or production quarantine change.
- No issue #78 implementation in this branch.
- No formal release or full CI dispatch.

## Acceptance

- The policy rollout is reachable from `origin/main` through a reviewed merge.
- Selector and policy wiring tests pass locally.
- Repository planning and goal audits pass.
- Changed-surface validation reports no unmet local gate.
- Canonical `main` is clean and synchronized after merge.
- The scoped worktree and local and remote topic refs are retired only after
  exact merge ancestry is verified.

## Next Sequence

After this plan closes, reconcile Plan 0178 source custody while preserving its
separate installed-acceptance gate. Then begin provider-free issue #78 in a new
bugfix branch and worktree.
