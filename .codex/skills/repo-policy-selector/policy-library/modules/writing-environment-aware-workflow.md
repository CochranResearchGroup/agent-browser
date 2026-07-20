---
id: writing-environment-aware-workflow
title: Writing Workspace Environment-Aware Workflow
summary: Keep one authoritative writing workspace across local and transient environments, and make continuity explicit when moving between them.
tags:
  - writing
  - environment
  - continuity
  - handoff
---

## Policy

- At the start of a turn, determine whether the agent is operating in the authoritative local workspace or a transient mirrored/uploaded environment.
- If work happens in a transient environment, require an explicit handoff artifact or replacement workspace bundle before turn end.
- Keep one authoritative working copy per deliverable path. Avoid competing drafts across environments unless the split is explicit and temporary.
- Record enough continuity information to resume without reconstructing document state from chat history alone.
- Treat environment changes as workflow-relevant context, not background noise.

## Adoption Notes

Use this module for writing-project repos where agents regularly move between local disks, mirrored folders, online REPL sessions, or packaged review bundles.
