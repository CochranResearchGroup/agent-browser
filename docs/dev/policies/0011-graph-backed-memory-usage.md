# Policy | Graph-Backed Memory Usage

## Policy

- Treat Graphiti as durable retrievable context, not as a scratchpad for every
  turn.
- Use Graphiti for compact, stable cross-turn facts such as user preferences,
  project decisions, durable entity relationships, and recurring operational
  context that later turns should retrieve quickly.
- Do not store ephemeral material in Graphiti, including temporary debugging
  notes, one-off command output, transient errors, raw reasoning traces,
  secrets, tokens, passwords, credential material, browser auth artifacts,
  cookies, screenshots, private site state, or raw logs.
- Before re-asking the user for likely durable repo context, prefer a bounded
  Graphiti read.
- At the start of non-trivial planning, debugging, architecture, audit,
  policy adoption, policy upgrade, harvest, or handoff work, run the documented
  Graphiti discovery workflow when prior context may exist.
- Use the `graphiti-discovery` skill as the repo-local discovery entrypoint.
- Prefer the repo memory group `agent_browser_main` for agent-browser work.
- When the task crosses repos, tenants, or domains, query the reviewed atlas
  group `memory_atlas_main` first and inspect retrieval, privacy, export, and
  audience policy before descending into source groups.
- Treat Graphiti results as advisory until verified against repo files,
  artifacts, commits, tests, or cited episodes.
- Keep richer narrative rationale, long-form handoff, and human-readable
  change history in repo notes, plans, or validation artifacts under
  `docs/dev/`; use Graphiti for compact retrieval-oriented facts and
  relationships.
- Prefer compact, factual, retrieval-friendly writes over conversational
  filler or repeated paraphrases of the same fact.
- Avoid memory spam:
  - do not write the same preference or project fact every turn
  - prefer one good durable memory over many near-duplicate entries
  - if a durable fact changed, record the new durable state rather than every
    intermediate thought
- Seed or refresh memory only from curated source-backed artifacts such as
  `AGENTS.md`, roadmap notes, bounded plans, validation reports, release
  checkpoints, and dated notes under `docs/dev/notes/`.
- Do not seed or refresh Graphiti from raw command logs, private browser
  artifacts, auth state, private target-site data, or every small commit.
- Verify Graphiti runtime health before debugging against it or assuming it is
  available during normal work.
- Treat destructive memory-maintenance tools as explicit cleanup or repair
  operations, not casual day-to-day commands.
- Treat graph-backed memory as durable retrievable context, not as a scratchpad for every turn.
- Use graph-backed memory for compact, stable cross-turn facts such as:
  - user preferences
  - project decisions
  - durable entity relationships
  - recurring operational context that later turns should retrieve quickly
- Do not store ephemeral material in graph-backed memory, including:
  - temporary debugging notes
  - one-off command output
  - transient errors unless they represent a durable incident worth tracking
  - raw reasoning traces
  - secrets, tokens, passwords, or credential material
- Before re-asking the user for likely durable context, prefer a bounded graph-memory read.
- At the start of non-trivial work, make one lightweight discovery decision:
  - `use` when prior decisions, user preferences, runtime history, cross-repo
    context, or avoided repeated investigation could materially affect the work
  - `skip` when the task is trivial or self-contained and supplied content or
    current authoritative sources are sufficient
  - `unavailable` when the memory system or required retrieval surface is not
    healthy; continue from repo-native evidence and state the fallback only
    when it materially limits confidence
- For non-trivial planning, debugging, architecture, audit, adoption, upgrade,
  harvest, or handoff work, default to `use` when prior context may exist. Do
  not make performative memory calls when the decision is `skip`.
- Keep discovery bounded to the narrowest relevant group and one or two focused
  reads before widening. Discovery is read-first and does not authorize a
  memory write.
- Query the repo-named memory group first when repo policy names one.
- When the right memory group is unclear, or when the task crosses repos, tenants, or domains, query a reviewed atlas or routing layer first and inspect retrieval, privacy, export, and audience policy before descending into source groups.
- Prefer compact, factual, retrieval-friendly writes over conversational filler or repeated paraphrases of the same fact.
- Avoid memory spam:
  - do not write the same preference or project fact every turn
  - prefer one good durable memory over many near-duplicate entries
  - if a durable fact changed, record the new durable state rather than narrating every intermediate thought
- Use partitions, groups, namespaces, or equivalent separation mechanisms deliberately so unrelated projects, tenants, or domains do not bleed into one another.
- Treat destructive memory-maintenance tools as explicit cleanup or repair operations, not casual day-to-day commands.
- Verify the memory system's availability or health before debugging against it or assuming it is available during normal work.
- Treat memory-derived claims as advisory until verified against repo files, artifacts, commits, tests, or cited episodes.
- Keep richer narrative rationale, long-form handoff, and human-readable change history in repo notes or memories; use graph-backed memory for compact retrieval-oriented facts and relationships.

## Repo-Local Commands

- Runtime health: `~/.local/bin/graphiti-runtime doctor`
- Repo discovery: `~/.local/bin/graphiti-runtime discover --group-id agent_browser_main "<query>"`
- Atlas discovery:
  `~/.local/bin/graphiti-runtime atlas-discover --atlas-group-id memory_atlas_main "<query>"`
- Queue status, when reads or writes appear blocked:
  `~/.local/bin/graphiti-runtime queue --full`

## Adoption Notes

This policy adopts the shared `graph-backed-memory-usage` module with
agent-browser-specific group and privacy boundaries. Graphiti is a discovery
and continuity aid for this repo, not an authority over source, tests, live
services, or release state.
Keep product-specific tool names, partition-key semantics, and runtime assumptions repo-local unless they clearly generalize across multiple graph-memory systems.

Repo-local policy should name the primary memory group when one exists and should identify the memory-discovery skill, tool, or command agents are expected to use. For Graphiti, prefer the `graphiti-discovery` skill and record the repo's primary `group_id`; if the repo does not use Graphiti, preserve the same `use` / `skip` / `unavailable` decision contract with its actual memory system.
