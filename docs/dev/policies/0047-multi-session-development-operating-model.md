# Policy | Multi-Session Development Operating Model

## Purpose

Make independent product lanes fast, restart-safe, and reconcilable without
allowing parallel agents, branches, runtimes, or external effects to blur
ownership and evidence.

## Operating Unit

- Use one top-level Codex session as the owner of each substantive development
  lane. A lane session owns one work item, one bounded plan, one short-lived
  branch, one worktree, its acceptance evidence, and its handoff.
- Keep one separate coordinator session for intake, roadmap priority,
  dependency joins, shared-contract decisions, active-lane reconciliation, and
  final integration. Small conflict resolution and shared-authority edits are
  the coordinator's normal coding boundary.
- Use subagents inside a lane only for concrete, bounded support work. The lane
  owner reconciles every result and remains accountable for the outcome.
- Keep routine delegation one level deep. A lane owner's subagent must not
  spawn another subagent unless the active plan names the exceptional topology,
  maximum depth and fan-out, result flow, and cancellation flow.
- A single coordinator with support workers is acceptable for governance
  bootstrap or a short tightly coupled batch. It is not the default topology
  for several long-lived implementation lanes.

## Lane Portfolio And Priority

- Use the five product lanes in `docs/dev/product-lanes.md`: `PL-BUGFIX`,
  `PL-AUTH`, `PL-CHALLENGE`, `PL-RECIPES`, and `PL-PLATFORM`.
- Keep the normal repository-wide limit at six substantive active worktrees:
  one per product lane plus one additional disjoint bugfix worktree.
- Reserve bugfix capacity for production-impacting defects. Pause the
  lowest-priority conflicting lane when needed, start the repair from current
  `origin/main`, integrate it first, then reconcile affected feature branches.
- Do not fill unused capacity merely because an agent slot exists. Dependencies,
  overlapping writes, live-effect serialization, and review load may require
  fewer concurrent lanes.

## Git And Shared Authority

- Keep `main` releasable and use short-lived lane branches. Use one session and
  one branch per worktree; do not share mutable worktree custody across sessions.
- Register substantive lanes in `docs/dev/active-lanes.yaml` before parallel
  execution and publish a recoverable remote checkpoint at the earliest safe
  boundary.
- The coordinator owns `ROADMAP.md`, `RUNBOOK.md`,
  `docs/dev/active-lanes.yaml`, issue taxonomy, shared schemas, and other
  declared integration surfaces. Lane sessions may propose changes but must not
  independently rewrite those authorities.
- Freeze shared schemas and interfaces before parallel implementations depend
  on them. Record dependencies and overlaps rather than discovering them at
  merge time.
- GitHub work items are the coordination surface after policy 0046 migration.
  Chat and session summaries remain handoff aids, not portfolio authority.

## Runtime Topology

- The target topology is one production runtime, one staging runtime, and one
  isolated development runtime for each implementation lane that needs service
  execution. Do not claim an environment exists until current readback proves it.
- A lane runtime needs a unique configuration root, database, socket or port,
  logs, and service identity. It fails closed when isolation is unproven and
  must not inherit production credentials, schedules, profiles, or writable
  data by convenience.
- Until per-lane isolation is proven, serialize use of any shared development
  runtime. Runtime availability is capacity evidence, not effect authority.
- Serialize every shared production or staging mutation through the atomic
  custody contract in `0051-shared-runtime-effect-custody.md`. Source-lane
  ownership, an active issue, a custody comment, and an installer file lock do
  not grant that live-effect lease.
- Serialize authenticated browser or provider canaries unless an explicit plan
  proves separate profiles and independent effect boundaries. Multiple runtimes
  do not authorize parallel external effects.
- Promotion flows from lane evidence to integration, staging acceptance, and
  production under the applicable release and effect authority.

## Request And Delivery Discipline

- Use the lifecycle: work item to bounded plan to lane owner to worktree and
  branch to isolated development runtime when needed to review and acceptance
  to coordinator integration to the separate staging or production gate.
- Every work item and plan names its outcome, non-goals, owner, dependencies,
  expected writes, acceptance evidence, stop condition, and effect boundary.
- Handoffs state the current ref and checkpoint, worktree and branch locator,
  governing plan and work item, accepted evidence, outstanding gate, and one
  bounded next action.
- Reserve live mutations, credential use, provider retries, deployments, and
  production changes for explicit authority. Planning readiness or an open lane
  does not grant external-effect authority.
- Close a lane only after work-item, plan, branch custody, integration,
  validation, and runtime claims agree. Preserve failed and partial evidence.

## Adoption Notes

Read this policy before starting or resuming substantive development, assigning
top-level session ownership, delegating work, creating a lane runtime,
integrating parallel branches, or entering the hotfix path.
