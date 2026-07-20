---
id: goal-execution-governance
title: Goal Execution Governance
summary: Keep long-running agent goals convergent with stable objectives, explicit execution states, durable checkpoints, bounded feedback loops, and evidence-based stop rules.
tags:
  - goals
  - agents
  - orchestration
  - antidrift
  - checkpoints
---

## Policy

- Apply this policy when autonomous work is expected to span multiple bounded
  slices, context windows, sessions, or human/runtime gates.
- Preserve the user-approved objective as the stable goal contract. Do not
  silently narrow, expand, or rewrite it to match the work already completed.
- Allow the campaign plan to stay high-level and derive bounded execution
  packets just in time under `planning-discipline`.
- Model execution as explicit states and transitions even when no graph
  framework is used. At minimum distinguish ready, active, awaiting-review,
  awaiting-gate, blocked, complete, failed, and cancelled states.
- Use `parallel-plan-design` to make dependencies, fan-out, joins, and retry
  edges inspectable. Every feedback cycle that can repeat model calls, tool
  calls, agent runs, mutations, or context growth must have one named
  controller, a semantic exit condition, and a hard bound.
- Treat material replanning as a new plan version or bounded successor packet.
  Preserve what changed and why instead of mutating execution history in place.
- Before execution, record the current authority, unmet acceptance criteria,
  owned worktree scope, current evidence, ready work units, blocked units,
  delegation plan, checkpoint cadence, and human/runtime/security gates.
- Choose concrete hard bounds before starting: work-unit attempts, review/rework
  cycles, consecutive hardening/no-progress checkpoints, and maximum time,
  slices, or available runtime budget between durable checkpoints. If one
  metric is unavailable, another observable bound must still cover the loop.
- Keep one primary orchestrator responsible for authority, the critical path,
  work-unit selection, integration, progress classification, and the final
  completion claim.
- Apply `subagent-workflow-optimization` at each execution packet and record the
  delegation decision. Apply `validation-and-handoff` for independent review
  and final outcome verification.
- At every durable checkpoint, compare the current state with the prior
  checkpoint and classify movement as:
  - `outcome_progress`: current evidence advances an acceptance criterion
  - `blocker_reduction`: a verified blocker or material risk was removed
  - `hardening`: resilience improved without changing acceptance state
  - `no_progress`: the goal state did not materially change
  - `regression`: evidence, safety, or alignment worsened
- Checkpoint after each validated execution packet and before context handoff,
  risky mutation, independent audit, human gate, or closeout. Record owned
  changes, validation evidence, state transitions, remaining criteria, and the
  next ready unit or exact stop reason in a durable repo artifact.
- A failed final review transitions the unit to split, reframe, block, or
  escalation; it does not silently reopen an unbounded review cycle.
- Stop autonomous execution when any configured drift guard fires, including:
  repeated hardening without outcome movement; repeated failure on the same
  invariant; stale evidence being reused for a current claim; an oversized or
  cyclic unit without a covering bound; an unresolved critical audit finding;
  an unsafe or unowned dirty worktree; a required human/runtime/security gate;
  or remaining work that is unbounded polish rather than goal capability.
- A goal may continue only when the latest checkpoint shows outcome progress or
  verified blocker reduction and names a bounded ready unit. Otherwise close,
  block, cancel, or obtain explicit approval for a new plan version.
- Completion requires current evidence for every acceptance criterion. Token
  spend, elapsed time, test count, schema growth, documentation volume, and
  completed slice count are not completion evidence by themselves.

## Adoption Notes

Use this module for repos that run `/goal`, unattended campaigns, multi-session
agent work, or other long-horizon autonomous execution.

Before calling adoption complete, adopting repos must define concrete checkpoint
and drift thresholds plus the required checkpoint-record fields in repo-local
policy. When a deterministic planning/runbook audit exists, extend it to verify
goal-plan versioning, checkpoint identifiers, progress classification, and the
configured bounds. Keep exact token counters, time windows, command names, and
runbook schemas repo-local.

Use a machine-checkable repo-local section such as:

```text
## Local Goal Bounds
max_work_unit_attempts: <positive integer>
max_review_rework_cycles: <positive integer>
max_hardening_checkpoints: <positive integer>
checkpoint_interval: <positive integer> <minutes|slices|tokens>
checkpoint_record_fields: plan_version, state_transition, progress_classification, evidence, subagent_status, next_action_or_stop_reason
```

The selector bundle's planning auditor supports `--goal-only` to verify this
contract without requiring roadmap/runbook governance.

Recommended companion modules:

- `planning-discipline`
- `parallel-plan-design`
- `subagent-workflow-optimization`
- `validation-and-handoff`
- `commit-and-push-cadence`
