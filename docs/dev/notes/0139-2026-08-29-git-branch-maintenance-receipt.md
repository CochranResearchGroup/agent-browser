# Git Branch Maintenance Receipt

Date: 2026-08-29

Status: COMPLETE

Scope: local branches, origin branches, worktrees, active-lane custody, and the
single pre-existing stash

## Outcome

The repository now has one default branch and two explicitly cataloged active
lanes. Local and origin branch tips are equal, all three worktrees are clean,
no worktree is prunable, and no stash remains.

The authoritative active-lane projection is `docs/dev/active-lanes.yaml`. Its
catalog-only audit passes against `refs/remotes/origin/main` and records:

- `plan/authentication-run-foundation` at
  `e74dda166f2cd57f1c9c458777812798ceba2735`; and
- `plan/crash-profile-lifecycle-coherence` at
  `2c08ed6eb62a6ed00e24d78953320e455b15a4e1`.

Both lanes have clean assigned worktrees and equal local and origin refs.

## Ref Retirement

Before deletion, the origin inventory contained 282 branch refs including
`main` and the two retained lanes. There were no open pull requests.

The maintenance operation deleted 279 origin topic branches:

- 259 exact name-and-SHA duplicates of upstream branches;
- 9 origin-specific branches whose tips were ancestors of `origin/main`; and
- 11 origin-specific unmerged branches archived first under
  `archive/remote-origin-20260829/<former-branch>`.

The 11 archive tags were pushed and verified to peel to the former branch
tips before their branch refs were deleted. The local merged Plan 0131 and
workstation hotfix branches were deleted through ancestry-safe deletion. The
two local superseded Plan 0131 branches were deleted only after their tips
matched the corresponding remote archive tags.

## Stash Disposition

The sole stash contained one redacted repository note about a legacy session
profile-routing mismatch. Its exact stash commit
`91d32fc9e7e7a86af34105a8f769f987fa69ae4c` was preserved and verified at the
remote tag `archive/stash-20260821-legacy-session-routing-note` before the
stash entry was dropped.

## Final Readback

- local branches: 3;
- origin branches: 3;
- active worktrees: 3;
- prunable worktrees: 0;
- stashes: 0;
- open pull requests: 0; and
- divergence for each retained branch: 0 behind, 0 ahead of its matching
  origin ref.

No live browser, provider, tenant, installed runtime, release, or upstream ref
was changed.
