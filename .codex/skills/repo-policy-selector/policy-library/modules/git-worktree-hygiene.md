---
id: git-worktree-hygiene
title: Git / Worktree Hygiene
summary: Keep branch scope narrow, check dirty state early, and treat overlapping work as reconciliation instead of a normal merge.
tags:
  - git
  - branches
  - worktrees
  - merge
---

## Policy

- Start branch-sensitive work by checking `git status`.
- Inventory all registered worktrees with `git worktree list --porcelain` before creating, closing, pruning, or reassigning one; the current checkout alone is not the repository topology.
- Treat pre-existing dirty state as a real constraint.
- Keep one bounded branch or worktree scope per execution slice or roadmap lane, consistent with the repo's documented integration model.
- When parallel work is needed, prefer `git worktree` over a second full clone.
- Do not call work merge-ready while the intended changes are still uncommitted.
- Treat the worktree as a checkout, the branch or detached commit as local custody, and a verified remote or archive ref as shared custody. Removing a worktree does not preserve uncommitted changes and does not prove the commits remain discoverable.
- Before removing a worktree, require a clean status, a named branch or explicitly preserved detached commit, an exact checkpoint SHA, and verified durable custody on the intended remote ref or on matching local and remote archive refs.
- Normal closure uses `git worktree remove` without `--force`. Forced removal is exceptional recovery work: first inventory the exact path, preserve any recoverable diff and commit, establish a durable ref, record the reason, and verify the retained SHA.
- Do not delete an unmerged branch merely because its worktree is gone. Prove integration, archival, or explicit discard approval separately.
- If overlapping dirty work exists across branches or worktrees, open a reconciliation step rather than calling it a normal merge.
- Keep branch scope narrow and avoid mixing unrelated lanes unless the active slice requires it.

## Adoption Notes

Use this module in repos where multiple lanes, multiple worktrees, or parallel agents regularly overlap.

This module governs local git cleanliness and overlap handling. Use `branch-and-integration-strategy` to choose whether the repo prefers direct-to-`main`, short-lived feature branches, or another integration model.

Use `active-lane-coordination` when several off-main lanes need default-branch discovery and custody reconciliation.
