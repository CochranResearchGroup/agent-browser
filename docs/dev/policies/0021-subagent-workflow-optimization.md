# Policy | Subagent Workflow Optimization

## Policy

- Delegate only concrete, bounded subtasks that materially advance the active slice.
- At batch planning, choose the primary route and any independent worker lanes
  together, using the least expensive capable model for each lane. Start with
  deterministic tools and repository evidence where they can answer the
  question directly.
- At the start of non-trivial work and after material replanning, consider
  whether delegation would create a genuinely useful independent lane. This is
  an execution choice, not a user-approval event.
- When subagent tooling and capacity are available, spawn without additional
  user prompting if at least one useful bounded lane exists, such as:
  - independent discovery or evidence collection off the immediate critical path
  - implementation with a disjoint write surface
  - context-heavy work that benefits from an isolated context window
  - independent validation, audit, or adversarial review
- If a useful independent lane is available and capacity permits, run it during
  the batch. Do not delegate merely to fill capacity, repeat exploration, or
  execute a trivial command.
- Record a non-delegation reason only when a plan expected a worker or the lack
  of delegation materially affects timing, independence, or evidence. Do not
  create a `not_spawned` receipt for every routine packet.
- When delegation occurs, leave a durable receipt for consequential work:
  record the bounded lane, available agent/run/session handle, terminal status,
  evidence returned, and the primary agent's reconciliation decision.
- Keep urgent blocking work local when the next action depends directly on the answer, except for a bounded specialist consultation whose demonstrated capability is more likely to resolve that exact obstacle than local continuation.
- Give delegated work explicit ownership, expected output, and write scope.
- Every worker packet must state the exact task, exact write scope, evidence to
  return, and stop condition. Keep the context compact enough to avoid paying
  for unrelated history.
- Prefer subagents for independent sidecar work, verification, or implementation slices with disjoint write sets.
- Do not spawn parallel work that duplicates context loading or repeats the same exploration without a clear benefit.
- Reuse prior agent context when the task is a continuation of the same bounded thread.
- Prefer fresh context when independence is part of the value: neutral review,
  adversarial audit, a newly split work unit after drift, or a handoff intended
  to shed accumulated context and assumptions.
- For a fresh reviewer, provide a frozen review packet: objective, acceptance
  criteria, non-goals, target identity or commit, applicable gates, review mode,
  and—during remediation verification—the accepted finding ledger. Ask for
  evidence-shaped candidate findings and explicitly permit a no-finding result.
- A reviewer detects drift; it does not own scope, finding disposition, goal
  authority, or operator approval. The primary agent reconciles the result and
  may reject, backlog, or seek evidence for a candidate that does not satisfy
  the frozen contract.
- Do not turn reviewer completion, reviewer agreement, or a second reviewer
  opinion into a prerequisite for obvious low-risk progress unless an explicit
  acceptance or safety contract requires that review.
- Use broad fresh context for the initial drift scan. Use closed-world prompts
  for later verification and carry the same finding identifiers across worker
  replacement, plan revisions, and successor packets so review discovery does
  not restart accidentally.
- Keep final integration responsibility with the primary agent even when subagents perform part of the work.
- The primary agent retains production effects and final acceptance; worker
  output is evidence to evaluate, not an acceptance decision.
- Be explicit about whether the repo optimizes for wall-clock speed, token efficiency, or a balance of the two.
- Treat spawned subagents as asynchronous runtime artifacts, not just informal delegation.
- Record the subagent run id, session id, transcript path, or equivalent handle when the runtime provides one.
- Do not assume delegated work completed until an announce payload, status check, log read, or transcript inspection confirms completion.
- A plan that merely names a subagent role is design evidence, not proof that a
  worker ran. Effectiveness claims require a runtime handle or an explicit
  unavailable-runtime receipt plus the resulting integration decision.
- For critical or high-risk delegated work, inspect the transcript or logs instead of relying only on a summarized announce.
- Prefer subagent closeout that includes status, result, notes, and available runtime, token, or cost metadata.
- Evaluate the batch by accepted results, elapsed wall time, and rework as well
  as cheap calls or summed worker effort.
- Set explicit timeout expectations for long-running, slow-tool, or uncertain delegated work.
- Give each subagent a stop condition and require it to return partial evidence
  rather than self-extending into adjacent work when the bound is reached.
- Use `model-selection-and-calibration` for model, reasoning, escalation, and calibration decisions. Architecture decisions may use a bounded specialist; the primary retains integration and acceptance.
- Treat subagent cleanup and transcript retention as deliberate choices when later evidence or reconciliation may matter.

## Adoption Notes

Use this module when repos actively rely on delegation or subagent orchestration rather than single-agent execution.

Execution-bias guidance:
- `max-dev-speed`: delegate earlier, parallelize more independent work, and accept some coordination overhead to reduce wall-clock time
- `balanced`: delegate bounded sidecar work and verification, but keep tightly coupled or critical-path work local
- `max-token-efficiency`: delegate only when the subtask is clearly independent and the expected gain exceeds the added context and reconciliation cost
- `max-token-efficiency` still requires the explicit delegation decision; it
  changes the threshold for spawning, not whether delegation is considered

Use `subagent-runtime-governance` as a companion module when the repo builds, configures, or operates the subagent runtime itself.
