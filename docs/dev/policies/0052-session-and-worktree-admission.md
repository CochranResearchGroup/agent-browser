# Policy | Session And Worktree Admission

## Purpose

Make the live worktree population a deliberate projection of the active
top-level development portfolio without assigning permanent permission power
to a coordinator agent. The operator chooses the portfolio. Sessions observe
and record it, serialize conflicting Git transitions, and keep every checkout
attributable and disposable.

## Portfolio Contract

- Keep one canonical integration checkout and at most one primary worktree for
  each active top-level development session. Four active development sessions
  normally correspond to four primary lane worktrees plus the canonical
  checkout, not every unused slot under a repository-wide ceiling.
- Treat six substantive active worktrees as a hard warning threshold, not a
  target or permission allocation. The expected population derives from the
  user-directed active portfolio and may be lower.
- Bind each primary worktree to one work item, product lane, bounded outcome,
  branch, expected write surfaces, known overlaps, and terminal disposition.
  Use stable work-item and branch locators in shared records rather than
  ephemeral session identifiers or absolute local paths.
- No agent title grants worktree authority. The session performing an admission
  is temporarily responsible for the inventory, collision check, creation
  receipt, and handoff. The user may redirect that responsibility at any time.
- Treat tracker state and worktree admission as separate decisions. A READY
  item is eligible for assignment but does not reserve a session or authorize a
  checkout. BLOCKED umbrellas, live operational gates, and TRIAGE evidence
  items remain coordination records until their exact blocker or scope is
  resolved.
- Use a subagent within an admitted lane only when the task can return to the
  lane owner without independent Git or runtime custody. If the work needs its
  own durable branch, merge decision, candidate build, or terminal disposition,
  it is a top-level lane and must pass this admission procedure.

## Admission Procedure

- Before creating or assigning a primary checkout, refresh the canonical
  remote, inventory `git worktree list --porcelain`, read every checkout's
  dirty state, compare local and remote refs, inspect active work items and
  pull requests, and reconcile `docs/dev/active-lanes.yaml`.
- Serialize the inventory, collision decision, work-item or lane update, and
  `git worktree add` operation under one repository-scoped admission lock when
  the helper exists. The lock prevents two sessions from acting on the same
  snapshot; it does not decide whether the operator may change the portfolio.
- Until the helper exists, perform one foreground admission at a time and
  publish its branch and work-item assignment before another admission. A lane
  session should reuse its existing primary worktree instead of creating a
  replacement merely because another session is idle, blocked, or unavailable.
- Unexplained, duplicate, detached, dirty auxiliary, unregistered, or
  catalog-contradicting worktrees trigger reconciliation advice and suspend
  automatic creation. An agent stops before an unrequested additional checkout
  and reports the exact population. The operator may still direct a specific
  creation or disposition after reviewing that evidence.

## Auxiliary Checkouts

- A reviewer, benchmark, red-team pass, test fixture, build, or subagent does
  not receive another primary worktree. Prefer disposable target, profile, and
  fixture directories or read-only Git operations.
- Create an auxiliary only when the exact task genuinely requires another
  checked-out tree. Bind it to one parent lane, purpose, exact ref, creation
  receipt, dirty-state owner, and expected removal point.
- Use a temporary path and detached commit when branch mutation is unnecessary.
  Move any valuable edit into the parent lane through a reviewed commit or
  explicit recoverable patch rather than leaving it hidden in the auxiliary.
- Remove an auxiliary in the same bounded slice after proving it clean or
  preserving its exact dirty state. If interruption prevents removal, report it
  as the parent lane's first cleanup obligation and make it visible to future
  admission advice.

## Handoff And Closure

- A top-level session handoff reports its primary worktree, local and remote
  checkpoint, pull request or branch disposition, and every parent-owned
  auxiliary. A missing report is drift evidence, not proof that the checkout
  may be removed.
- Close a clean primary worktree promptly after integration or durable handoff
  when the next owner does not need the checkout. A paused branch may retain
  durable Git custody without retaining a worktree.
- Reconcile the complete inventory after every lane merge, session termination,
  interrupted auxiliary operation, and portfolio change. Integrated, archived,
  paused, missing, and active claims should agree across work items, plans,
  catalog entries, refs, and actual checkouts.
- Never force-remove unexplained or dirty worktrees. Establish ownership and
  preserve recoverable work before exact cleanup under policy 0004.

## Advisory Tool Contract

- The future admission helper reports expected primaries, the hard warning
  threshold, registered primaries, auxiliaries, unexplained checkouts, dirty
  paths, stale catalog claims, missing expected worktrees, recommendations, and
  supported operator choices.
- The helper may prevent accidental concurrent creation by locking the Git
  transition. It must not become a general permission service or claim that a
  warning overrides explicit user direction.
- Every operator-directed create, retain, handoff, archive, or close action
  returns a receipt and refreshes the advice from current state.

## Adoption Notes

This Agent Browser repo-local extension followed a four-session inventory that
contained six non-canonical worktrees. It composes with Git hygiene,
active-lane coordination, multi-agent reconciliation, and the multi-session
operating model. Runtime build and install coordination remains separate under
policy 0051 and Plan 0190.
