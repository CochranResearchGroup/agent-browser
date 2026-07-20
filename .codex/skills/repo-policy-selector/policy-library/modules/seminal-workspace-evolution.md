---
id: seminal-workspace-evolution
title: Seminal Workspace Evolution
summary: When a repo does not fit existing policy families well, treat it as a formative workspace, capture fit and friction deliberately, and use that evidence to shape a future repo type.
tags:
  - policy
  - archetypes
  - feedback
  - evolution
  - continuity
---

## Policy

- If a repo does not fit an existing shared policy family cleanly, treat its selected profile as provisional rather than final.
- Record why the current repo does not fit the existing families well enough, including:
  - which candidate profiles were considered
  - what fit cleanly
  - what felt over-prescriptive
  - what important workflow or artifact surfaces remain uncovered
- After meaningful turns, record dated fit notes that capture:
  - what policy worked well
  - what required local override or reinterpretation
  - what was missing from the shared library
  - whether the repo is converging toward a stable new archetype
- Promote repeated stable conclusions into `docs/dev/memories/`; keep turn- or slice-specific refinement notes in `docs/dev/notes/`.
- When the same missing pattern appears repeatedly, propose:
  - a new reusable module
  - a refined selector rule
  - a new profile
  - or a new `repo_purpose`
  rather than letting the workspace remain an indefinite one-off.
- Keep the current repo-local policy explicit while the archetype is still forming; do not pretend the repo is well served by an existing family just because a weak nearest match exists.
- When the workspace is serving as the first strong example of a new repo type, treat those fit notes as first-class harvest input for the shared policy repo.

## Adoption Notes

Use this module when a repo is a formative or seminal workspace whose operating model is not yet covered well by the existing shared policy families.

This module complements:
- `policy-adoption-feedback-loop`, which captures adoption and upgrade feedback
- `notes-and-memories`, which defines where formative notes and stable conclusions live
- `policy-harvest-loop`, which governs how those conclusions become reusable shared policy
