# Policy | Subagent Workflow Optimization

## Policy

- Delegate only concrete, bounded subtasks that materially advance the active slice.
- At the start of each non-trivial slice and after material replanning, make an
  explicit delegation decision instead of waiting for the user to request
  subagents.
- When subagent tooling and capacity are available, spawn without additional
  user prompting if at least one useful bounded lane exists, such as:
  - independent discovery or evidence collection off the immediate critical path
  - implementation with a disjoint write surface
  - context-heavy work that benefits from an isolated context window
  - independent validation, audit, or adversarial review
- If a long-running or multi-lane goal proceeds without delegation, record the
  concrete reason, such as no independent lane, unsafe overlap, coordination
  cost exceeding the expected gain, or unavailable tooling.
- For planned, multi-slice, or consequential work, leave one durable delegation
  receipt per execution packet. Record `spawned` or `not_spawned`; the bounded
  lane or non-delegation reason; available agent/run/session handle; terminal
  status; evidence returned; and how the primary agent reconciled or rejected
  the result.
- Keep urgent blocking work local when the next action depends directly on the answer.
- Give delegated work explicit ownership, expected output, and write scope.
- Prefer subagents for independent sidecar work, verification, or implementation slices with disjoint write sets.
- Do not spawn parallel work that duplicates context loading or repeats the same exploration without a clear benefit.
- Reuse prior agent context when the task is a continuation of the same bounded thread.
- Prefer fresh context when independence is part of the value: neutral review,
  adversarial audit, a newly split work unit after drift, or a handoff intended
  to shed accumulated context and assumptions.
- Keep final integration responsibility with the primary agent even when subagents perform part of the work.
- Be explicit about whether the repo optimizes for wall-clock speed, token efficiency, or a balance of the two.
- Treat spawned subagents as asynchronous runtime artifacts, not just informal delegation.
- Record the subagent run id, session id, transcript path, or equivalent handle when the runtime provides one.
- Do not assume delegated work completed until an announce payload, status check, log read, or transcript inspection confirms completion.
- A plan that merely names a subagent role is design evidence, not proof that a
  worker ran. Effectiveness claims require a runtime handle or an explicit
  unavailable-runtime receipt plus the resulting integration decision.
- For critical or high-risk delegated work, inspect the transcript or logs instead of relying only on a summarized announce.
- Prefer subagent closeout that includes status, result, notes, and available runtime, token, or cost metadata.
- Set explicit timeout expectations for long-running, slow-tool, or uncertain delegated work.
- Give each subagent a stop condition and require it to return partial evidence
  rather than self-extending into adjacent work when the bound is reached.
- Use lower-cost or lower-reasoning models for bounded sidecar work only when the quality risk is low; keep synthesis, architecture, and final integration on an appropriately capable model.
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
