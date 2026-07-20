---
id: fieldwork-productization
title: Fieldwork Productization
summary: Start live customer or tenant interventions as bounded fieldwork, then explicitly classify what becomes product, what stays local, and what should be retired.
tags:
  - fieldwork
  - productization
  - operations
  - migration
---

## Policy

- Treat reactive live work for one tenant, customer, or operator as fieldwork until it proves reusable.
- Start fieldwork on an explicit bounded branch, note, or equivalent execution surface whenever practical.
- Fieldwork notes should capture:
  - tenant or operator goal
  - systems touched
  - expected write surfaces
  - artifact locations
  - starting branch or commit state
  - the provisional roadmap lane or product area
- During fieldwork, allow pragmatic code changes when needed to solve the live problem, but keep evidence, endpoint notes, and idempotency notes close to the field note.
- Before merging fieldwork into normal product history, classify outcomes explicitly:
  - keep as product
  - refactor before keep
  - archive as note only
  - discard
- When fieldwork reveals a repeatable workflow, decide whether its durable home is:
  - core product code
  - operational runtime config
  - a skill or playbook
  - or a local operator note
- Do not let one tenant's urgent workflow silently define the long-term product architecture without an explicit productization pass.

## Adoption Notes

Use this module when a repo is developed partly through live tenant, customer, or operator interventions that may later become reusable product behavior.
