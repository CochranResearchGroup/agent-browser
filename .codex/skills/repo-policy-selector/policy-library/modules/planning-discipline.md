---
id: planning-discipline
title: Planning Discipline
summary: Keep stable high-level outcome plans while deriving bounded execution packets, explicit definitions of done, and parallel workstreams as work becomes ready.
tags:
  - planning
  - slices
  - parallelism
  - states
---

## Policy

- Use bounded plan artifacts under `docs/dev/plans/` or an equivalent plans directory, not ad hoc note files scattered through the repo.
- Plan filenames should use a deterministic serial-plus-date prefix such as `0001-YYYY-MM-DD-plan-slug.md`.
- If the repo uses a canonical long-range plan such as `ROADMAP.md`, treat it as the source of truth for priority.
- If the repo uses a canonical live execution log such as `RUNBOOK.md`, treat it as the source of truth for what happened turn by turn.
- When `RUNBOOK.md` is present, maintain it as a dated turn log with deterministic headings such as `Turn N | YYYY-MM-DD`.
- Treat planning migration for active repos as two phases:
  - structural migration to establish canonical files, naming, and wiring
  - semantic reconciliation to align plan text and lane status with the actual shipped state
- Configure deterministic planning audits to the repo's documented authority
  paths. Do not assume `docs/dev/plans`, `ROADMAP.md`, or `RUNBOOK.md` when the
  repo has an explicit equivalent such as `doc/dev/plans`.
- Each plan should carry an explicit deterministic state from a small fixed vocabulary, for example:
  - `PLANNED`
  - `OPEN`
  - `CLOSED`
  - `CANCELLED`
- For any plan in an active state such as `OPEN`, require a short `Current State` section that says what already exists and what still remains.
- Use bounded plan artifacts with explicit scope, non-goals, acceptance criteria, and definition of done.
- Keep plan altitude proportional to its horizon. A campaign or `/goal` plan may
  remain high-level when it preserves the objective, milestones, dependencies,
  gates, and outcome evidence; derive detailed implementation packets just in
  time instead of pretending every future step is knowable up front.
- Separate the stable objective and milestone plan from mutable execution state.
  Record material replanning as an explicit revision or successor plan rather
  than silently rewriting the goal to fit current progress.
- Give each active execution packet one bounded outcome, owner, expected write
  surface, required inputs, validation evidence, and terminal condition.
- When a task is large enough to plan, explicitly separate:
  - parallelizable low-conflict tracks
  - critical-path serialized work
- Keep one critical-path owner visible even when subagents or parallel workers are used.
- Do not let one plan artifact accumulate endless follow-on polish; close it or open a new bounded slice.
- Reconcile plan state promptly when implementation lands, a successor
  supersedes the plan, or a gate blocks integration. Stale `OPEN` labels are
  authority drift even when the implementation itself is sound.
- Do not equate plan activity with progress. Require current evidence that a
  slice advances an acceptance criterion or removes a verified blocker.
- If the repo adopts roadmap/runbook governance, keep plan wiring and plan state aligned with those canonical files.
- When the planning contract changes in a way that affects validation, update the deterministic audit helper in the same slice.
- Separate steady-state enforcement from legacy migration. A current/active
  audit may ignore closed or unclassified historical artifacts only when the
  report names what it excluded and the repo retains a bounded migration or
  baseline decision for that debt.

## Adoption Notes

Use this module when the repo has:
- multiple active lanes
- bounded plan artifacts or equivalent planning discipline

Use `roadmap-runbook-governance` as the stricter companion module when the repo keeps canonical `ROADMAP.md` and `RUNBOOK.md` authority.

Use `goal-execution-governance` when the plan will drive autonomous work across
multiple slices, sessions, context windows, or gates.
