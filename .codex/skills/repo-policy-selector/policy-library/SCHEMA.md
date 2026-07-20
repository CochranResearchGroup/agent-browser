# Policy Schema

This repo is intentionally small. The schema is lightweight so humans and scripts can both work with it.

## Repository Objects

There are three first-class objects:

1. `module`
2. `profile`
3. `catalog`

Repo-local `AGENTS.md` files are downstream entrypoints, not objects in this repo.
The adopted repo-local policy should live under `docs/dev/policies/` and be wired in from `AGENTS.md`.

## Repo Purpose And Workflow Subtype

Policy selection is purpose-aware.

Each target repo should be classified by:

1. `repo_purpose`
2. optional `workflow_subtype`
3. optional `execution_bias`

Initial purpose families:

- `product-engineering`
- `operations-platform`
- `website-maintenance`
- `course-workspace`
- `library-cli`
- `seminal-workspace`
- `workspace-agent`
- `writing-project`

Examples of `workflow_subtype` under `writing-project`:

- `grant-proposal-writing`
- `grant-proposal-review`
- `journal-article-writing`
- `patent-application-writing`
- `technical-report-writing`

Rules:
- purpose should describe the repo's operating model, not only its subject matter
- subtype is optional and should refine the operating model when it changes policy needs materially
- execution bias is optional and should describe whether the repo prefers lower wall-clock time, lower coordination/token cost, or a balance of the two
- selector output should always include purpose; subtype is recommended when confidence is adequate
- selector output should include execution bias when repo policy or workflow signals make it clear
- selector output should include `policy-management` as the first adopted policy in selector-managed repos

Execution-bias vocabulary:

- `max-dev-speed`
- `balanced`
- `max-token-efficiency`

## Planning Contract

For repos that adopt bounded planning discipline:

- actionable plans should live under `docs/dev/plans/` unless the repo has a clearly documented alternate location
- plan filenames should use a deterministic serial-plus-date prefix, for example `0001-2026-04-09-slice-name.md`
- planning migration for active repos should include both:
  - structural migration for canonical files, naming, and wiring
  - semantic reconciliation against actual shipped state and recent history
- plan states should come from a small fixed vocabulary such as:
  - `PLANNED`
  - `OPEN`
  - `CLOSED`
  - `CANCELLED`
- plans in an active state such as `OPEN` should include a short `Current State` section
- if the repo uses `ROADMAP.md`, it should be the master plan and should change cautiously
- if the repo uses `RUNBOOK.md`, it should be a dated turn log of what happened, not a second roadmap
- if the repo uses roadmap lanes, lane ids should use one canonical naming pattern, for example `P## | <Lane Title>`
- if the repo uses roadmap lanes, lanes in an active state such as `OPEN` should include a short `Current State` note
- if the repo uses roadmap lanes, active lanes such as `OPEN` should normally have at least one actionable plan
- if the repo uses `ROADMAP.md` and `RUNBOOK.md`, plan wiring into them should be auditable by deterministic helpers
- deterministic audits should report whether the planning contract is
  applicable from adopted policy rather than failing every repo that lacks the
  strict roadmap/runbook profile
- repos with documented alternate authority paths should pass those paths to
  the audit rather than copying artifacts solely to satisfy defaults
- active-only audits should report excluded closed or unclassified legacy plans;
  they are steady-state gates, not evidence that historical migration is done

## Notes And Memories Contract

For repos that adopt notes/memory continuity discipline:

- dated notes should live under `docs/dev/notes/`
- durable memories should live under `docs/dev/memories/`
- notes and memories should use the same deterministic serial-plus-date filename prefix as plans, for example `0001-2026-04-10-slice-name.md`
- deterministic helpers should be used to enumerate, audit, or manage notes and memories when the repo provides them
- policy adoption or upgrade feedback that should inform future shared policy work should be recorded as dated notes instead of left only in chat history
- one dated note may satisfy upgrade tracking, adoption feedback, and continuity capture when it records the decision and reusable lesson clearly
- when a repo also uses a durable memory system, notes and memories remain the place for richer human-readable continuity unless a more specific shared memory-usage module says otherwise

## Module Contract

Modules live under `modules/*.md`.

Each module should contain:

1. YAML frontmatter
2. a short `## Policy` section
3. optional `## Adoption Notes`

Required frontmatter fields:

```yaml
id: git-worktree-hygiene
title: Git / Worktree Hygiene
summary: Keep branch scope narrow and treat overlap as reconciliation.
tags:
  - git
  - worktrees
  - merge
```

Rules:
- `id` must be stable and kebab-case.
- `summary` should describe the reusable idea, not one repo's local wording.
- `tags` should support search and profile composition.
- module body text should be reusable across multiple repos.
- avoid repo-local paths, commands, and product nouns unless they are essential to the policy itself.

## Profile Contract

Profiles live under `profiles/*.yaml`.

Required fields:

```yaml
id: repo-product-engineering
summary: Full planning and branch-discipline profile for multi-lane product repos.
modules:
  - planning-discipline
  - roadmap-runbook-governance
  - git-worktree-hygiene
  - turn-closeout
overrides:
  expects_runbook: true
  expects_roadmap: true
  parallel_execution_bias: true
```

Rules:
- profiles compose modules; they should not duplicate module prose
- `overrides` are hints for the selector/adopter, not a second prose policy layer
- profile ids should reflect repo archetypes, not one named repository
- profiles should declare their intended `repo_purpose`
- profiles may declare supported `workflow_subtypes`

## Catalog Contract

The catalog is a lightweight index:
- `modules[].id`
- `modules[].path`
- `modules[].tags`
- `profiles[].id`
- `profiles[].path`
- `profiles[].tags`

It exists to support simple deterministic tools without requiring a richer database.

For deployment and installation:
- the policy library should be installable alongside the selector as a self-contained bundle
- profile and module enumeration should come from deterministic library artifacts such as `catalog.yaml`
- adoption wiring into target repos should be deterministic:
  - adopted policy files live under `docs/dev/policies/`
  - `AGENTS.md` acts as the entrypoint that wires those files into the repo contract
  - `AGENTS.md` should tell agents when to re-read relevant adopted policy files, especially at the start of non-trivial turns and when scope changes

## Harvesting Contract

Harvesting should classify policy into one of four outcomes:

1. `reuse-existing-module`
2. `update-existing-module`
3. `propose-new-module`
4. `keep-repo-local`

Preferred default:
- if a rule fits an existing concept, update or reuse that concept
- only propose a new module when the rule is genuinely reusable and conceptually distinct
- adoption feedback from downstream repos should be treated as a first-class harvest input when it identifies repeatable fit problems, missing modules, or over-prescriptive profiles
- multi-repo harvests should define an inventory and explicit exclusion classes
- availability, active adoption/wiring, and enacted behavior are separate
  evidence stages and should not be collapsed into one keyword score
- audit incompatibility and excluded legacy artifacts should be reported as
  limitations or migration debt, not silently counted as policy failure

Harvesting should also respect purpose boundaries:
- do not mix `workspace-agent` memory/heartbeat rules into `product-engineering` profiles
- do not force heavyweight engineering roadmap governance onto lightweight local-ops or simple library repos
- treat `writing-project` policies as deliverable-oriented rather than codebase-architecture-oriented

For `writing-project`, likely reusable policy themes include:
- environment-aware workspace continuity
- authoritative deliverable discipline
- runbook or handoff continuity for long document workflows
- document safety rules for DOCX/PDF/editing pipelines
- review and evidence-pack organization

Those should remain separate from `product-engineering` modules unless a rule is clearly shared across both families.

For `product-engineering`, likely reusable policy themes include:
- architecture or service-boundary guardrails
- codegraph-backed source discovery and impact analysis before non-trivial code edits or refactors
- documentation-change control when plans, semantics, or operator surfaces move
- validation and handoff discipline with explicit verification before commit or handoff
- preview artifact review when generated reports, local builds, review packets, or visual outputs need browser-based human inspection before approval
- subagent runtime provenance when delegated work produces code, policy, or validation evidence
- long-running goal execution with stable objectives, bounded work packets,
  durable checkpoints, explicit state transitions, convergence tests, and stop
  rules

Those should stay distinct from `writing-project` modules even when both families use planning or closeout rules.

For `website-maintenance`, likely reusable policy themes include:
- environment and surface targeting across canonical, staging, local, and deprecated web surfaces
- governance for DB-backed state that cannot be represented faithfully in git
- live-drift reconciliation before release
- backup and recovery discipline as part of operational completeness
- visual release QA and reusable web-interface quality rules
- codegraph-backed source discovery when the website repo includes owned application code, themes, plugins, build systems, or deploy tooling
- preview artifact review when local builds, screenshots, PDFs, or generated review packets should be surfaced as one browser-review session

Those should stay distinct from general `product-engineering` modules when the repo's main operating risk is live website change management rather than service architecture evolution.

For `operations-platform`, likely reusable policy themes include:
- boundary discipline between reusable product code and private runtime state
- codegraph-backed source discovery and impact analysis for product/runtime code before changing shared operator workflows
- governance for when runtime state itself should become a managed artifact set
- tenant or environment isolation for local state, artifacts, and resources
- fieldwork productization after live customer or operator interventions
- extraction discipline for oversized command or orchestration trunks
- release and validation rules that account for both publishable software and private operational runtimes
- graph-backed memory usage when a durable memory service becomes part of normal operator workflow
- memory-service runtime governance when the repo builds or operates the installed durable memory service itself
- subagent lifecycle and tool-surface governance when live operator workflows depend on spawned agents
- preview artifact review when operator handoffs, approval packets, dry-run outputs, or generated local artifacts require human review before mutation

Those should stay distinct from both `website-maintenance` and general `product-engineering` modules when the repo's main operating risk is mixing product evolution with live tenant operations.

For repos that build or operate subagent runtimes, likely reusable policy themes include:
- status and completion signals derived from runtime state instead of model claims alone
- run ids, session ids, transcript paths, announce payloads, and provenance for delegated work
- tool allow/deny policy by agent role and spawn depth
- concurrency, fan-out, nesting, timeout, and cascade-stop limits
- model, token, cost, archive, cleanup, and retention expectations

Those should stay distinct from ordinary workflow delegation rules; most repos need `subagent-workflow-optimization`, while only runtime-oriented repos need `subagent-runtime-governance`.

For repos that use `/goal` or other long-horizon autonomous execution, likely
reusable policy themes include:

- a stable goal objective plus high-level milestone/campaign plan
- just-in-time bounded execution packets rather than premature low-level plans
- explicit states, dependencies, joins, bounded feedback cycles, and terminal outcomes
- automatic delegation decisions and fresh-context worker/auditor roles
- checkpointed evidence, progress classification, convergence guards, and
  deterministic stop or escalation conditions

Those belong in `goal-execution-governance`. Exact time, token, slice, command,
and runbook thresholds should remain repo-local.

For repos that rely on installed durable memory systems, likely reusable policy themes include:
- when to read memory before re-asking the user
- active memory discovery before non-trivial planning, debugging, audit, adoption, upgrade, harvest, or handoff work
- repo-named memory groups and atlas/routing behavior when the right group is unclear
- what belongs in graph-backed memory versus notes and memories docs
- when to mirror compact source-cited policy, adoption, or harvest summaries into graph memory after repo-file artifacts exist
- duplicate-write and memory-spam avoidance
- cautious use of destructive maintenance tools
- verification of memory-derived claims against repo files, artifacts, commits, tests, or cited episodes

Graph-backed memory usage is part of the default starter profiles because durable context retrieval has become a normal agent workflow surface. Repo-local policy should still name the actual memory group, discovery skill, privacy boundary, and write discipline for that repo.

For repos that contain code and have an indexed codegraph available, likely reusable policy themes include:
- consulting codegraph before non-trivial code edits, architecture claims, trace analysis, or refactor planning
- using structural queries such as context, trace, callers, callees, impact, and indexed file listings before broad manual search loops
- treating codegraph output as discovery evidence that still requires source reads and tests
- accounting for index freshness after edits
- keeping exact sibling checkout paths, MCP tool names, and index refresh commands repo-local

For repos that build or operate installed memory-service runtimes, likely reusable policy themes include:
- client/server provider boundaries, especially when agent clients do not own service backend credentials
- installed-release, runtime-config, service-manager, health, and listener verification before diagnosis
- durable async queue status, retry, and dead-letter visibility
- live read-after-write smoke checks after install, restart, migration, or backend changes
- keeping product code separate from private runtime state, credentials, queues, and databases

Those should stay distinct from repo-local tool names, partition-key semantics, and deployment assumptions unless those behaviors clearly generalize.

For `course-workspace`, likely reusable policy themes include:
- course identity, term, workspace, archive, and generated-artifact governance
- LMS CLI read-before-write discipline, live course target checks, and post-write validation
- cloud-drive placeholder and provider-native document handling for course workspaces
- student-data, assessment, answer-key, grading, and private-feedback safety
- validation and handoff rules that distinguish read-only inspection from live course mutation
- preview artifact review when course materials, PDFs, Office documents, generated packets, or galleries need browser review before sharing or LMS writes

Those should stay distinct from `writing-project` modules when the repo's main operating risk is live course operation and student-data handling rather than producing a writing deliverable.

## Repo-Local Override Rule

The target repo's repo-local policy remains authoritative after adoption.
By default, that policy should live under `docs/dev/policies/`, with `AGENTS.md` acting as the wire-in entrypoint.

That means:
- shared policy modules are a source library
- profiles are starter bundles
- repo-local policy files under `docs/dev/policies/` are the durable adopted layer
- `AGENTS.md` should keep repo-specific guidance and treat policy entry as one section, not the whole document
- `AGENTS.md` should act as a policy-loading contract, not just a one-time pointer
- repo-local command lists, paths, and domain-specific rules usually stay local
- selector and harvester outputs should draft or recommend changes, not silently replace local policy
