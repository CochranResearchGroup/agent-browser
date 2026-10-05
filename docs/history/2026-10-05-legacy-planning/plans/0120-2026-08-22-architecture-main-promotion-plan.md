# Plan 0120: Architecture Main Promotion

Date: 2026-08-22

State: COMPLETE

Lane: P120

Source baseline: `254c4a02c4668400774d87b7ee94a27fe99ea18e`

Depends on:

- `docs/dev/plans/0119-2026-08-22-architecture-line-reconciliation-plan.md`

## Goal

Promote the validated `architecture-deepening-20260809` integration line into
the fork's `main` branch by exact fast-forward, then retire the redundant
architecture branch without combining upstream synchronization into the same
history operation.

## Preconditions

- The source worktree is clean.
- Local and remote architecture refs both resolve to the source baseline.
- Fresh remote refs prove `origin/main` is an ancestor of the architecture
  branch with zero commits unique to `origin/main`.
- Plan 0119 validation is bound to the source baseline.
- GitHub reports no branch protection, repository ruleset, or open pull request
  that establishes a different integration path.

## Execution

1. Commit this bounded promotion receipt on the architecture branch.
2. Push the receipt and fast-forward `origin/main` to the resulting exact tip.
3. Move local `main` to the verified remote tip and switch the sole worktree to
   `main`.
4. Record the final ref identities and delete the redundant local and remote
   architecture branch only after `origin/main` is proven recoverable.

## Acceptance Criteria

1. `origin/main`, local `main`, and the worktree HEAD resolve to the same commit.
2. The worktree is clean.
3. `architecture-deepening-20260809` no longer exists locally or on `origin`.
4. The three source repair branches remain historical audit anchors only.
5. Upstream synchronization remains explicitly unperformed.

## Non-goals

- Merge, rebase, or cherry-pick `upstream/main`.
- Rewrite any shared commit.
- Install or reconcile the live runtime.
- Delete the historical source repair branches.

## Result

- Fresh remote refs proved an exact fast-forward from `origin/main` at
  `ae36b272327982e3227f4dc7c5d6dc5b4b16350c` to the promotion receipt at
  `1a91b7991fb4c55cfee463c88bf9cc0f932d1b37`.
- GitHub reported that `main` had no branch protection or repository ruleset,
  and there was no open pull request establishing another integration path.
- `origin/main` was fast-forwarded directly, preserving every structured
  architecture commit without adding a merge or rewriting shared history.
- The sole worktree was switched to local `main` after local and remote main
  resolved to the same promoted commit.
- Plan 0119 validation remains applicable because the promotion adds only this
  documentation receipt after its validated source tip.
- Upstream synchronization was not performed. At promotion time,
  `origin/main` still carried the fork's long-lived downstream history and
  remained 91 commits behind `upstream/main`.
