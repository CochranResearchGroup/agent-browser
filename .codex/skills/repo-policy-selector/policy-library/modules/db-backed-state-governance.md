---
id: db-backed-state-governance
title: DB-Backed State Governance
summary: Distinguish file-backed artifacts from database-backed website state, keep recovery and reproducibility explicit, and avoid pretending git is the history of mutable CMS state.
tags:
  - website
  - database
  - cms
  - migrations
---

## Policy

- Distinguish clearly between state that can live as files in git and state that exists only in a database or CMS runtime.
- Put file-backed assets in git when the repo owns them, such as custom code, migrations, deployment scripts, configuration templates, and durable operational docs.
- Do not try to force mutable CMS or database-backed state into git when the repo cannot represent it faithfully.
- For DB-backed changes, classify each change as one of:
  - recoverable state
  - repeatable configuration
  - editorial or operational content
- Recoverable state must be covered by the repo's backup and recovery workflow.
- Repeatable configuration should be promoted into reproducible artifacts when possible, such as:
  - migration scripts
  - command sequences
  - sanitized SQL patches
  - export/import helpers
  - documented operator procedures
- Keep migrations narrowly scoped, reviewable, and safe to rerun where possible.
- Separate local-only and live-targeting migrations when production execution risk differs materially.
- When a change cannot be represented safely as code, be explicit that backup/recovery rather than git is the authoritative recovery path.
- Do not let undocumented DB-backed admin changes become silent durable state when they should instead be captured as repeatable artifacts.

## Adoption Notes

Use this module when repos manage websites, CMS-backed applications, or other systems where important operational state lives outside normal source-controlled files.
