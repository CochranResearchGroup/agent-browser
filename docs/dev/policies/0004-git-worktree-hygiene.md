# Policy | Git / Worktree Hygiene

## Policy

- Start branch-sensitive work by checking `git status`.
- Inventory all registered worktrees with `git worktree list --porcelain` before creating, closing, pruning, or reassigning one; the current checkout alone is not the repository topology.
- Before creating a worktree, decide whether an existing clean checkout already
  owns the intended branch and lane. Continue in the correct existing worktree
  when it is safe; do not create duplicate checkouts merely to avoid orienting
  to current custody.
- Create a new worktree only when the work needs an isolated branch, a separate
  concurrent checkout, or continuity beyond the current session. Give it one
  clear branch and purpose, and do not repurpose another active lane's checkout
  by switching its branch or mixing in unrelated work.
- Treat pre-existing dirty state as a real constraint.
- Keep one bounded branch or worktree scope per execution slice or roadmap lane, consistent with the repo's documented integration model.
- When parallel work is needed, prefer `git worktree` over a second full clone.
- Do not call work merge-ready while the intended changes are still uncommitted.
- Treat the worktree as a checkout, the branch or detached commit as local custody, and a verified remote or archive ref as shared custody. Removing a worktree does not preserve uncommitted changes and does not prove the commits remain discoverable.
- Before removing a worktree, require a clean status, a named branch or explicitly preserved detached commit, an exact checkpoint SHA, and verified durable custody on the intended remote ref or on matching local and remote archive refs.
- Close a worktree promptly when its branch is integrated, its work is durably
  handed off without needing the checkout, or its preserved branch is paused or
  archived. Do not accumulate idle worktrees as informal reminders or confuse a
  retained branch with a need to retain its checkout.
- Normal closure uses `git worktree remove` without `--force`. Forced removal is exceptional recovery work: first inventory the exact path, preserve any recoverable diff and commit, establish a durable ref, record the reason, and verify the retained SHA.
- After removal, verify the exact path is absent from the registered worktree
  inventory. Prune only stale administrative entries whose checkout absence and
  branch custody have been established; pruning is not a substitute for closing
  a live worktree deliberately.
- Do not delete an unmerged branch merely because its worktree is gone. Prove integration, archival, or explicit discard approval separately.
- If overlapping dirty work exists across branches or worktrees, open a reconciliation step rather than calling it a normal merge.
- Keep branch scope narrow and avoid mixing unrelated lanes unless the active slice requires it.

## Adoption Notes

Use this module in repos where multiple lanes, multiple worktrees, or parallel agents regularly overlap.

This module governs local git cleanliness and overlap handling. Use `branch-and-integration-strategy` to choose whether the repo prefers direct-to-`main`, short-lived feature branches, or another integration model.
