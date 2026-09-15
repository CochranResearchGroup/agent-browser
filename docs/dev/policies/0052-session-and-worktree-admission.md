# Policy | Session And Worktree Admission

## Purpose

Make the live worktree population a deliberate projection of the admitted
top-level development portfolio. Agent availability, reviewer convenience, a
free product-lane slot, or a need for a second checkout does not independently
authorize another durable worktree.

## Portfolio Contract

- Keep the canonical checkout under the coordinator session. Each explicitly
  admitted top-level development session may own exactly one primary lane
  worktree. Four admitted development sessions therefore permit four primary
  lane worktrees, not six, even when the repository-wide hard ceiling is six.
- Treat the repository-wide ceiling as a maximum, not a target or standing
  allocation. The operative primary-worktree cap is the smaller of that
  ceiling and the number of currently admitted top-level development sessions.
- Bind each admission to one work item, product lane, bounded outcome, branch,
  primary worktree purpose, accountable owner, expected write surfaces, known
  overlaps, and terminal disposition. Use stable work-item and branch locators;
  do not put ephemeral session identifiers or absolute local paths in the
  shared catalog.
- A subagent, reviewer, benchmark, red-team pass, test fixture, or build does
  not receive another primary worktree. It works through the owning lane or an
  auxiliary checkout governed below.

## Atomic Admission

- Only the coordinator creates or assigns primary worktrees. Before opening
  concurrent top-level sessions, the coordinator serially admits the intended
  portfolio and gives each session its existing branch and checkout locator.
  Lane sessions must not run `git worktree add` to create their own primary or
  replacement checkout.
- Before every admission, refresh the canonical remote, inventory
  `git worktree list --porcelain`, read dirty state for every checkout, compare
  local and remote branches, search active work items and pull requests, and
  reconcile `docs/dev/active-lanes.yaml` against the default branch.
- Perform the inventory, slot decision, catalog or work-item claim, and
  worktree creation as one serialized coordinator transaction. When an
  admission helper exists, it must hold a repository-scoped lock across the
  decision and creation and return a receipt that identifies the admitted work
  item, lane, branch, base commit, and resulting checkout.
- Until that helper is installed, the coordinator itself is the single
  admission serializer. It must finish and publish each portfolio assignment
  before another top-level session may create or receive a worktree.
- Fail closed on any unexplained, duplicate, detached, dirty auxiliary,
  unregistered, or catalog-contradicting worktree. Existing admitted lane work
  may continue when its ownership and write surface remain unambiguous, but no
  new primary or auxiliary checkout may be admitted until reconciliation.
- Reuse the exact existing primary worktree when the intended work item and
  branch already own one. Do not create a replacement because its session is
  idle, blocked, slow, or difficult to contact.

## Auxiliary Checkouts

- Prefer a disposable target directory, temporary profile, fixture directory,
  or read-only Git operation over an auxiliary worktree. Create an auxiliary
  only when the exact task genuinely requires a second checked-out tree, such
  as comparing two commits whose source must be built independently.
- Bind every auxiliary to one admitted parent lane, a purpose, exact ref,
  creation receipt, dirty-state owner, and same-slice removal condition. An
  auxiliary does not consume or enlarge the primary portfolio allocation and
  must not become an informal branch, reminder, or handoff surface.
- Use a temporary path and detached commit when branch mutation is unnecessary.
  Do not leave generated, benchmark, reviewer, or red-team edits in an
  auxiliary checkout. Preserve any valuable change through the parent lane's
  reviewed branch or an explicit recoverable patch before cleanup.
- Remove the auxiliary in the same bounded slice that created it after proving
  it clean or preserving its exact dirty state safely. If interruption prevents
  removal, the parent lane records the residual checkout as its first cleanup
  obligation; further worktree admission remains blocked.

## Handoff And Closure

- A top-level session is not ready for handoff, replacement, idle closure, or
  completion until it reports its primary worktree status, local and remote
  checkpoint, pull request or branch disposition, and every parent-owned
  auxiliary checkout.
- Close a clean primary worktree promptly after integration or durable handoff
  when the next owner does not require that checkout. A paused branch may
  retain durable Git custody without retaining a worktree.
- The coordinator reconciles the worktree inventory after every lane merge,
  session termination, interrupted auxiliary operation, and portfolio change.
  Integrated, archived, paused, missing, and active states must agree across
  the work item, plan, catalog, Git refs, and actual checkout inventory.
- Never force-remove unexplained or dirty worktrees. Freeze new admission,
  establish ownership, preserve recoverable work, and apply policy 0004 before
  exact cleanup.

## Enforcement And Audit

- A clean active-lane catalog alone is insufficient because unregistered and
  detached worktrees can exist outside it. Admission validation must inspect
  the complete local `git worktree list --porcelain` population and classify
  every non-canonical checkout as one primary lane worktree or one temporary
  parent-owned auxiliary.
- The admission audit must report the admitted primary count, hard ceiling,
  canonical checkout, registered primaries, auxiliary checkouts, unexplained
  checkouts, dirty paths, stale catalog claims, and missing expected
  worktrees. Any unexplained population or count excess is a blocking result.
- Treat policy-only enforcement as an interim control. The durable repair is a
  repository helper that performs locked admission and closeout, plus a
  deterministic audit that checks the full worktree population. Until those
  tools land, coordinator-only creation and a fresh full inventory before each
  assignment are mandatory.

## Adoption Notes

This is an Agent Browser repo-local extension identified after four intended
top-level development sessions produced six non-canonical worktrees. It
composes with Git and worktree hygiene, active-lane coordination, multi-agent
reconciliation, and the multi-session development operating model. Shared
runtime effects remain governed separately by policy 0051.
