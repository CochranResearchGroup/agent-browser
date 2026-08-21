# Policy | Goal Execution Governance

## Policy

- Apply this policy when autonomous work is expected to span multiple bounded
  slices, context windows, sessions, or human/runtime gates.
- Preserve the user-approved objective as the stable goal contract. Do not
  silently narrow, expand, or rewrite it to match the work already completed.
- Treat that approved goal as standing authority for ordinary in-envelope
  implementation, validation, repair, retest, worker replacement, integration,
  and bounded successor packets. A packet hard stop ends that execution window;
  it does not revoke the approved goal or create a new approval gate by itself.
- Default to action, not permission seeking. When the next useful action is
  clearly implied by the objective, stays within the same target and mutation
  class, and is low risk, readily reversible, or already covered by explicit
  authorization and safeguards, take it without asking the user to approve the
  step or first manufacturing another packet. Record the decision afterward
  when it matters for continuity.
- Ask for new authorization only for a material departure: changing the
  objective, acceptance criteria, or non-goals; adding a new system, tenant, or
  private-data class; widening mutation scope; crossing into a destructive,
  external, legal, financial, publication, release, or public action not already
  clearly authorized; materially raising
  cost or resource ceilings; weakening a safety control; or choosing among
  materially different outcomes the user must decide. Preserve stricter gates
  only when they are explicitly established by higher-priority instructions,
  the user, or applicable repo/runtime policy and apply to the exact action
  contemplated. A broad label such as `runtime`, `provider`, `review`, or
  `live` is not an approval gate by itself.
- An approval stop must identify the exact proposed action, the applicable
  authority or safety boundary, and why no safe in-envelope default can make
  progress. Missing bookkeeping, a packet counter reaching zero, reviewer
  novelty, or generalized uncertainty is not sufficient by itself.
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
- At goal start and after a material change, establish one concise control
  record: current authority, unmet acceptance criteria, owned worktree scope,
  current evidence, ready and blocked work, checkpoint cadence, and exact
  applicable gates. Add delegation detail only when relevant. Do not recreate
  this inventory for every packet.
- Choose concrete bounds before starting: work-unit attempts, review/rework
  cycles, consecutive hardening/no-progress checkpoints, and maximum time,
  slices, or available runtime budget between durable checkpoints. If one
  metric is unavailable, another observable bound must still cover the loop.
  Repo-local defaults may supply these values; an individual packet need not
  restate them, and missing packet metadata does not block a first safe attempt.
  Bounds prevent runaway work; they are not consumable approval tokens. When a
  local bound is reached, first reassess, split the unit, change tactics, or
  continue a different safe ready unit under the same authority. Escalate only
  when no meaningful safe action remains or an exact action-specific gate is
  reached.
- Keep one primary orchestrator responsible for authority, the critical path,
  work-unit selection, integration, progress classification, and the final
  completion claim.
- Apply `subagent-workflow-optimization` when delegation offers a useful
  authorized lane. Apply `validation-and-handoff` for proportionate independent
  review and final outcome verification. Neither a worker nor a reviewer is an
  approval authority unless an explicit external contract says otherwise.
- At every durable checkpoint, compare the current state with the prior
  checkpoint and classify movement as:
  - `outcome_progress`: current evidence advances an acceptance criterion
  - `blocker_reduction`: a verified blocker or material risk was removed
  - `hardening`: resilience improved without changing acceptance state
  - `no_progress`: the goal state did not materially change
  - `regression`: evidence, safety, or alignment worsened
- Checkpoint at material state transitions, before context handoff, before a
  risky or gated mutation, at closeout, and at the configured cadence backstop.
  Routine low-risk steps inside one coherent unit do not each require a durable
  checkpoint or approval-like receipt. Record the state transition, current
  acceptance state, progress classification, evidence, material blockers, and
  next action or stop reason; add review or delegation detail only when those
  events occurred.
- A failed closed-world verification of an accepted blocking finding transitions
  the unit to split, reframe, block, or escalation; it does not silently reopen
  an unbounded review cycle.
- Allow at most one broad fresh-context drift-discovery pass per approved goal,
  and run it only when consequence, uncertainty, or observed drift makes
  independent discovery useful. After the primary adjudicates candidate
  findings, verification is closed-world against accepted findings plus
  critical regressions introduced by remediation. Plan versions and successor
  packets inherit the maximum rather than resetting it.
- Scope every drift guard to what its evidence actually affects. Stale evidence
  blocks the associated current claim; an unsafe dirty overlap blocks mutation
  of that overlap; an accepted blocking finding blocks the affected criterion
  or integration; and an exhausted loop bound blocks repeating that same loop.
  Continue unrelated safe work when it can still advance the approved goal.
- Stop autonomous execution only when no meaningful safe in-envelope action
  remains, an exact applicable gate requires a user decision, or the objective
  is complete, cancelled, or disproven. Repeated hardening or no-progress first
  requires a local tactic change or bounded reframe; it does not automatically
  require operator approval.
- Continue automatically whenever a useful in-scope action is available and no
  exact applicable gate blocks it. A recent checkpoint may support that choice,
  but creating another checkpoint is not a prerequisite for taking an obvious
  low-risk next step.
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
max_work_unit_attempts: 2
max_review_rework_cycles: 1
max_hardening_checkpoints: 2
checkpoint_interval: 3 slices or 90 minutes
authorization_gate: material_departure_or_explicit_action_gate_only
continuation_default: execute_obvious_in_scope_low_risk
bound_exhaustion_mode: local_replan_before_escalation
max_review_discovery_passes: 1
review_verification_mode: closed_world_if_reviewed
checkpoint_mode: material_boundary_with_cadence_backstop
checkpoint_record_fields: state_transition, acceptance_state, progress_classification, evidence, material_blockers, next_action_or_stop_reason
```

The selector bundle's planning auditor supports `--goal-only` to verify this
contract without requiring roadmap/runbook governance.

Recommended companion modules:

- `planning-discipline`
- `parallel-plan-design`
- `subagent-workflow-optimization`
- `validation-and-handoff`
- `commit-and-push-cadence`
