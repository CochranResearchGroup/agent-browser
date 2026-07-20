---
id: subagent-runtime-governance
title: Subagent Runtime Governance
summary: Govern spawned subagent lifecycle, status, tool access, nesting, transcript retention, and concurrency when a repo builds or operates subagent runtimes.
tags:
  - agents
  - subagents
  - runtime
  - governance
---

## Policy

- Treat subagent runtimes as operational systems with lifecycle, provenance, tool-surface, and cost controls.
- Define the authoritative status vocabulary for subagent runs, including success, failure, timeout, cancellation, and unknown outcomes.
- Require completion signals to come from runtime status, logs, or transcripts rather than model-written claims alone.
- Preserve enough run metadata for later audit or reconciliation, such as:
  - run id
  - session id or session key
  - parent run id
  - transcript or log path
  - start and finish timestamps
  - runtime status
  - token, model, and cost metadata when available
- Define the expected announce or completion payload shape, including status, result, notes, and retrieval path for deeper inspection.
- Make subagent tool access explicit.
- Deny session-management, system, destructive, credential, and live-operation tools by default unless the subagent role requires them.
- When nested subagents are allowed, define:
  - maximum spawn depth
  - maximum children per parent
  - global concurrency cap
  - which depth may orchestrate children
  - how results flow back to the primary agent
  - how cancellation cascades through children
- Prefer shallow nesting. Treat depth beyond one orchestrator layer as exceptional unless the repo exists to operate agent runtimes.
- Require timeouts or watchdog expectations for long-running subagent work.
- Treat transcript cleanup, archive, or deletion as a retention decision rather than incidental cleanup.
- Make cost and model defaults explicit for spawned work so low-risk sidecar work does not silently consume high-cost reasoning.
- Document known runtime limitations, such as best-effort announce delivery, process restarts, shared gateway resources, or missing context injection.
- Keep runtime-specific command names, config syntax, and deployment assumptions repo-local unless they generalize across multiple subagent runtimes.

## Adoption Notes

Use this module when the repo builds, configures, or operates subagent infrastructure, not merely when it occasionally delegates work.

For ordinary repos that only use subagents as a workflow technique, prefer:
- `subagent-workflow-optimization`
- `parallel-plan-design`
- `multi-agent-reconciliation`
- `validation-and-handoff`
