---
id: monolith-extraction-discipline
title: Monolith Extraction Discipline
summary: Treat oversized command trunks, service files, or workflow modules as liabilities that require explicit extraction boundaries and staged decomposition.
tags:
  - architecture
  - modularity
  - refactor
  - debt
---

## Policy

- When one file or module becomes the default landing zone for new behavior, treat that as a design smell rather than normal growth.
- Define extraction seams before adding more unrelated behavior into an oversized trunk file.
- Prefer moving stable responsibilities into focused modules such as:
  - command groups
  - service layers
  - workflow packages
  - runtime or tenant helpers
  - rendering or output helpers
- Keep extraction work incremental and behavior-preserving; do not require a single all-or-nothing rewrite before improvement can start.
- Record the intended long-term boundaries in a bounded plan before a large decomposition effort.
- When live operations pressure encourages short-term additions to the monolith, record the debt explicitly and schedule the extraction slice rather than treating the temporary shortcut as a permanent structure.
- Use tests and explicit validation to hold behavior steady during extraction.

## Adoption Notes

Use this module when a repo has one oversized command file, orchestrator, or service module that is accumulating unrelated product and operational responsibilities.
