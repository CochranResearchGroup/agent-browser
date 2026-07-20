---
id: notes-and-memories
title: Notes And Memories
summary: Keep durable notes and memories under docs/dev with deterministic naming and use deterministic helpers to manage them.
tags:
  - notes
  - memories
  - continuity
  - governance
---

## Policy

- Routinely look for opportunities to record durable notes under `docs/dev/notes/` when a slice leaves behind findings, migration lessons, semantic mismatches, or operational lessons worth preserving.
- Routinely look for opportunities to record durable memories under `docs/dev/memories/` when a repo accumulates stable context, conventions, or recurring facts that future sessions should not have to rediscover.
- Prefer notes for dated observations tied to a specific slice or event.
- Prefer memories for stable context that should persist across many slices.
- Use the same deterministic serial-plus-date filename prefix for notes and memories that the repo uses for plans, for example `0001-YYYY-MM-DD-slug.md`.
- Keep notes and memories discoverable and auditable through deterministic helpers rather than relying on chat history or ad hoc filenames.
- When a repo adopts this policy, check existing `docs/dev/notes/` and `docs/dev/memories/` before starting work and record new entries when the current slice produces reusable context.
- Do not create multiple overlapping notes for one event when one well-scoped dated note already captures the decision, evidence, and reusable lesson.
- When a repo also uses a durable memory system, keep the boundary explicit: use notes and memories for richer human-readable continuity, and use the memory system for compact retrieval-oriented facts and relationships.
- When a repo has an explicit graph-memory group, consider mirroring compact source-cited summaries after durable notes or memories are created so future agents can discover the context without rereading every file.
- Do not treat graph-memory writes as a substitute for repo-file continuity. Write the durable note, memory, plan, release note, or artifact first, then mirror only the stable retrieval-oriented facts.

## Adoption Notes

Use this module when a repo benefits from durable continuity beyond plans alone, especially for migrations, policy evolution, operational knowledge, or recurring maintainer context.

Use `graph-backed-memory-usage` as a companion module when the repo relies on an installed graph-backed memory system during normal work.
