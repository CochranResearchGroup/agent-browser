# Policy | Roadmap / Runbook Governance

## Policy

- `ROADMAP.md` owns priority and lane lifecycle. Plans under `docs/dev/plans/`
  own scope, acceptance, dependencies, and strategy. `RUNBOOK.md` is the sole
  current execution status and links evidence and history. Retrospective
  progress records are not planning authorities; no new progress file is needed.
- Preserve existing roadmap lane and plan identifiers. Do not reprioritize
  lanes without operator direction or a narrow correction needed for authorized
  work. Wire active plans into their roadmap lane and the current runbook.
- Keep a compact current entry at the top of `RUNBOOK.md`: acceptance state,
  cumulative effort when relevant, blocker, and next action or stop reason.
  Link previous evidence rather than copying retry transcripts into plans or
  new runbook entries. Roadmap current-state notes summarize lane lifecycle and
  link the runbook; they need not change after every implementation step.
- Keep the active `RUNBOOK.md` at or below 200 lines. Replace superseded current
  status rather than prepending full execution transcripts. At a material
  closeout, archive older detail as needed, retaining a short index and all
  still-applicable stop instructions in the active file.
- Preserve archive contents, relative-link behavior, and known incoming anchor
  links. Prefer an archive beside the original file to preserve relative paths;
  provide compatibility anchors or update known incoming links if needed.
- Do not reread entire archives for routine policy loading or startup. Read a
  specific historical entry only when it bears on the current question.
- Existing historical clutter does not require a migration before ordinary
  work. Preserve history and scope archival separately from implementation.

## Semantic Compaction

- Reconcile meaning as well as length. Keep one authoritative current statement
  per requirement and remove contradictory stale states from the active runbook.
  Preserve superseded detail in linked history rather than squeezing outdated
  instructions into the line limit. Check evidence links, applicable stop
  conditions and the next action after compaction.

## Adoption Notes

Adapted from LitScout policy 0006 at `03f6f4026bef63bdbc4a5ba472ce140fb5c5d442`.
Local policy 0006 remains branch and integration strategy. This module adds no
new plan numbering, release authority, or historical schema migration.
