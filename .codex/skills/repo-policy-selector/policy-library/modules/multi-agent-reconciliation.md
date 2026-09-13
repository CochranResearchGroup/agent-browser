---
id: multi-agent-reconciliation
title: Multi-Agent Reconciliation
summary: Reconcile overlapping agent work explicitly, preserve ownership signals, and use history to resolve conflicts instead of silently overwriting edits.
tags:
  - agents
  - git
  - reconciliation
  - blame
---

## Policy

- Treat overlapping agent changes as reconciliation work, not as normal silent merge cleanup.
- Prefer disjoint write scopes before parallel execution, and record ownership when multiple agents are active.
- Declare shared integration surfaces before concurrent work and give each one a
  single coordination owner. Other lanes may propose changes, but they must not
  independently rewrite shared schemas, coordination catalogs, or equivalent
  cross-lane authorities without explicit reconciliation.
- Reconcile against the default-branch active-lane projection before assigning a new branch or worktree when the repository adopts that contract. Record dependencies and expected overlap before concurrent edits begin.
- When integrating conflicting edits, inspect history directly rather than assuming the most recent edit is correct.
- Use commit history, branch context, and `git blame` or equivalent file-history inspection when authorship and intent need to be reconstructed.
- Preserve useful ownership signals such as named commits, clear branch purpose, or explicit closeout notes when they help later reconciliation.
- Preserve subagent run ids, session keys, transcript paths, or equivalent provenance when integrating delegated work.
- When delegated output changes code, policy, or durable docs, cite the delegated source in closeout, commit context, or the relevant plan/handoff note.
- If delegated outputs conflict, inspect logs or transcripts before deciding which result to keep.
- Do not treat summarized announce messages as sufficient reconciliation evidence for high-risk changes.
- Do not rewrite another agent's work without first understanding the intended change surface.
- Keep accountability with the lane owner even when agents or subagents perform
  bounded work. Session topology is an implementation choice; ownership,
  evidence, and reconciliation must remain clear across session replacement.
- If a collision reveals weak lane boundaries, update the plan or policy so the same overlap is less likely next time.
- Do not treat an agent session ending as Git closure. The responsible owner must leave a clean published checkpoint and an explicit custody or integration disposition before its worktree can be removed safely.

## Adoption Notes

Use this module when repos regularly use multiple agents or contributors on adjacent surfaces and need explicit conflict-resolution discipline.

Execution-bias guidance:
- `max-dev-speed`: tolerate more concurrent work, but require stronger reconciliation rules and clearer ownership tracking
- `balanced`: prefer disjoint writes first and use reconciliation discipline when overlap still occurs
- `max-token-efficiency`: minimize overlap up front because post-hoc reconciliation is expensive in both time and context tokens
