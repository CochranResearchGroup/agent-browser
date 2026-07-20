---
id: parallel-plan-design
title: Parallel Plan Design
summary: Design plans so parallelizable work is explicit, low-conflict lanes are separated from critical-path blockers, and ownership stays clear.
tags:
  - planning
  - parallelism
  - agents
  - coordination
---

## Policy

- Give each parallel lane a clear owner, bounded scope, and expected write surface.
- Keep the critical path visible so parallel work does not hide the real blocker.
- Prefer plan slices that minimize cross-lane file overlap and reconciliation cost.
- Call out integration points explicitly when multiple lanes must converge before completion.
- Express non-trivial execution as inspectable work units and dependency edges,
  including fan-out, join, review, retry, and terminal transitions. A table or
  plan section is sufficient; a graph framework is not required.
- Do not open parallel lanes just because tools allow delegation; open them only when the work can move independently.
- If a lane becomes coordination-heavy, collapse it back into the critical path or redefine the lane boundary.
- Declare the intended active-agent concurrency before spawning many subagents or parallel workers.
- Cap active subagents per plan lane unless the repo explicitly optimizes for `max-dev-speed` and has strong reconciliation rules.
- Avoid nested subagents by default.
- Use nested or orchestrator subagents only when the plan names the parent orchestration role, child scopes, result-flow path, and synthesis responsibility.
- Treat high fan-out as a plan smell unless the subtasks are independent, low-conflict, and cheap to verify.
- Put a semantic exit condition and a hard bound on every review, retry, repair,
  or agent-handoff edge that can cycle back to prior work.
- When a work unit cannot be bounded or has too many coupled write surfaces,
  return it for split/reframe before spawning workers.

## Adoption Notes

Use this module when repos regularly use subagents, parallel contributors, or multiple active implementation lanes.

Execution-bias guidance:
- `max-dev-speed`: open more parallel lanes when ownership and write surfaces are clear enough to keep wall-clock time down
- `balanced`: parallelize bounded sidecar work but keep urgent blockers and tightly coupled work on the critical path
- `max-token-efficiency`: keep fewer active lanes, prefer larger local ownership, and avoid parallel decomposition that duplicates context or creates heavy reconciliation work
