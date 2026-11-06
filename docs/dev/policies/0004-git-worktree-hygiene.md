# Policy | Git / Worktree Hygiene

## Policy

- Start branch-sensitive work by checking `git status`.
- Inventory all registered worktrees with `git worktree list --porcelain` before creating, closing, pruning, or reassigning one; the current checkout alone is not the repository topology.
- In this repository, apply `0052-session-and-worktree-admission.md` before
  creating or assigning any primary or auxiliary worktree. Serialize the exact
  Git transition from current inventory evidence; no agent role grants or
  withholds the operator's authority to change the portfolio.
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
- Treat worktree closeout as one repository-scoped transaction. Key it by the
  common Git repository identity and the exact worktree incarnation, then bind
  its request to the expected path, HEAD, ref, dirty-state digest, pinned
  candidates, and selected dispositions. An identical request joins or replays
  the existing operation; a changed request is a typed conflict.
- Before removing a checkout that owns pinned candidate artifacts, select
  retain, archive, or discard for every pin. Retain leaves the checkout and
  candidate in place. Archive verifies byte-for-byte publication and a durable
  locator outside the checkout. Discard removes only the digest-pinned files.
  Missing selection is actionable evidence, not a generic denial.
- Close a worktree promptly when its branch is integrated, its work is durably
  handed off without needing the checkout, or its preserved branch is paused or
  archived. Do not accumulate idle worktrees as informal reminders or confuse a
  retained branch with a need to retain its checkout.
- Normal closure uses `git worktree remove` without `--force`. Forced removal is exceptional recovery work: first inventory the exact path, preserve any recoverable diff and commit, establish a durable ref, record the reason, and verify the retained SHA.
- After removal, verify the exact path is absent from the registered worktree
  inventory. Prune only stale administrative entries whose checkout absence and
  branch custody have been established; pruning is not a substitute for closing
  a live worktree deliberately.
- Use the repository closeout helper for governed closure when available. Its
  inspection and consequence preview are read-only, and effects require an
  explicit apply choice. Raw Git remains available to the operator, but bypasses
  the helper's candidate custody, serialization, recovery, and terminal receipt.
- Do not delete an unmerged branch merely because its worktree is gone. Prove integration, archival, or explicit discard approval separately.
- If overlapping dirty work exists across branches or worktrees, open a reconciliation step rather than calling it a normal merge.
- Keep branch scope narrow and avoid mixing unrelated lanes unless the active slice requires it.

## Recurring Integration And Storage Review

- At session startup, before another checkout or candidate build, and after
  every merge, reconcile registered worktrees, branch divergence, open pull
  requests, dirty state, candidate pins, and active-lane claims. During ongoing
  development repeat this review at least weekly; record its date and the next
  review date in the lane handoff.
- Finish each bounded slice by integrating validated work through its normal
  pull request, or naming the exact remaining gate, owner, checkpoint and next
  review date. Review idle branches after seven days. Age triggers review and
  never substitutes for validation, merge authority or preservation.
- In the same slice as integration, close clean lane and auxiliary checkouts
  that are no longer needed through the governed helper. Retain the canonical
  integration checkout and any checkout with a recorded continuing purpose.
  For paused work, preserve verified remote branch custody and
  archive pinned candidates before closing a checkout that is no longer needed.
  An open pull request can retain its branch without retaining its checkout.
- Include allocated build-directory sizes across all registered worktrees and
  their declared external target directories in this review. Count shared
  targets once. Review aggregate repository build storage above 20 GiB before
  adding another build. Prune qualified,
  disposable incremental and abandoned fixture outputs first. Keep an explicit
  exception with an owner and review date when active work needs more space.
- Before cache deletion, record exact paths and allocated sizes outside Git,
  check active builders and executable, mapped-file and descriptor references,
  reject symlinks and nested mounts, and hold the actual Cargo target lock with
  Cargo-compatible locking. Skip a busy target; do not stop another build to
  reclaim its files. Recheck identities immediately before deletion.
- Preserve source, lockfiles, build receipts, sealed candidates, candidate
  manifests, installed runtimes and active fixture evidence. Candidate custody
  uses explicit retain, archive or discard dispositions. Cache age alone does
  not authorize deletion of those artifacts.
- Record before and after filesystem usage, removed paths, archive locators,
  remaining exceptions and rebuild cost. Cleanup is complete only after fresh
  worktree and storage readback. These rules require recurring session review;
  they do not install an unattended deletion or automatic merge service.

## Adoption Notes

Use this module in repos where multiple lanes, multiple worktrees, or parallel agents regularly overlap.

This module governs local git cleanliness and overlap handling. Use `branch-and-integration-strategy` to choose whether the repo prefers direct-to-`main`, short-lived feature branches, or another integration model.
