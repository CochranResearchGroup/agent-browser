---
id: architecture-guardrails
title: Architecture Guardrails
summary: Keep changes aligned with explicit boundaries and prevent unplanned repository and dependency sprawl.
tags:
  - architecture
  - boundaries
  - semantics
  - change-control
---

## Policy

- Derive implementation boundaries from the live architecture and current service seams, not from aspirational or superseded layouts by default.
- Do not add new top-level workflows, endpoints, abstractions, or major aliases unless the governing plan or roadmap is updated in the same slice.
- Prefer tightening semantics and ownership boundaries over widening the surface area opportunistically.
- Keep provider-specific or deployment-specific heuristics at the narrowest layer that can own them cleanly.
- When a change would blur current architecture boundaries, stop and update the governing plan before proceeding.
- Keep a discoverable current architecture map at the level needed to answer
  which top-level areas exist, what each owns, and which dependency directions
  are allowed. Prefer generated dependency evidence where practical, but keep
  ownership and intent human-reviewable.
- Give each top-level package, service, application, or durable workflow one
  primary responsibility and an explicit owner. New top-level areas require a
  stated responsibility, dependency position, owner, and retirement condition;
  directory creation alone is not an architecture decision.
- Record a short architecture decision record for changes that materially alter
  system structure, key quality attributes, ownership, or a hard-to-reverse
  dependency. Capture context, decision, alternatives, consequences, status,
  and supersession; do not turn ordinary implementation detail into ADR churn.
- Keep accepted decision records immutable apart from factual corrections.
  Supersede a changed decision with a linked successor so the reason for the
  old structure remains recoverable.
- Prefer small, independently valid structural changes. Separate broad moves,
  renames, dependency inversions, or mechanical refactors from behavior changes
  when that makes review, rollback, and overlap reconciliation clearer.
- Treat repeated cross-boundary edits, circular dependencies, duplicate
  responsibilities, import-layer violations, orphaned entrypoints, and large
  change fan-out as structure-health signals. Diagnose ownership and boundary
  causes before adding another facade, shared helper, or top-level directory.
- Define deprecation and deletion paths for superseded modules, flags,
  workflows, and compatibility layers. A new abstraction is incomplete when it
  leaves the old path active without an owner, consumer inventory, or retirement
  disposition.

## Adoption Notes

Use this module when the repo:
- has a service or architecture seam that must stay coherent
- has active refactors or staged migration work
- frequently risks structural drift through ad hoc feature additions
