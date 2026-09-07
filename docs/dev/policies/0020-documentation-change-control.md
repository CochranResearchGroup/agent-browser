# Policy | Documentation Change Control

## Policy

- Update the document that owns the changed fact in the same slice: roadmap
  for priority and lane lifecycle, plan for scope and acceptance, runbook for
  current execution, and narrow contracts for behavior.
- If scope, semantics, service contracts, or operator workflows change, update the corresponding user-facing or governing docs before handoff.
- Preserve completed or superseded plans as durable history instead of deleting them outright.
- Do not rely on chat history as the authoritative explanation of why a change happened; record it in the repo docs.
- When a change affects a narrow contract document, update that contract doc in the same slice rather than deferring it to later cleanup.
- Keep one current execution status in `RUNBOOK.md`, with links to evidence
  and history. Do not duplicate every repair, test, and retry across roadmap,
  plan, runbook, and retrospective progress records.
- Retrospective progress records describe meaningful completed outcomes, not a
  second live status. Routine implementation steps need no separate checkpoint
  unless a material fact changes; batch updates at meaningful boundaries.
- Preserve history and scope archival separately. Follow
  `0043-roadmap-runbook-governance.md` for active runbook compaction.

## Adoption Notes

Use this module when the repo:
- has roadmap, runbook, journal, contract, or execution-plan docs that steer work
- needs docs to stay aligned with semantics or operator behavior
- benefits from explicit anti-drift rules for plan and doc maintenance
