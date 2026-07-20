---
id: runtime-state-governance
title: Runtime State Governance
summary: Govern when user-scoped runtime state should itself become a managed artifact set, and separate tracked durable state from ephemeral or secret runtime material.
tags:
  - runtime
  - state
  - versioning
  - redaction
---

## Policy

- Treat user-scoped runtime state as a separate surface from the product repo.
- Do not version all runtime output by default; classify runtime material first:
  - durable authoritative state
  - durable but derived state
  - ephemeral caches and previews
  - secrets and sensitive logs
- Version runtime state only when it is:
  - expensive to reconstruct
  - intentionally curated over time
  - needed across machines or operators
  - important for audit, rollback, or continuity
- Keep secrets, raw credentials, and highly sensitive logs out of versioned runtime state even when other runtime artifacts are tracked.
- Prefer a dedicated tracked runtime-state root or repo over mixing tracked and untracked concerns indiscriminately across one large runtime home.
- Use explicit ignore rules so caches, temporary artifacts, bulk exports, and machine-local scratch data do not pollute the tracked runtime state history.
- Prefer structured JSON, YAML, or similarly inspectable files for tracked runtime state over opaque blobs when practical.
- Be explicit about which runtime artifacts are authoritative versus rebuildable.
- When runtime state schema changes, migrate it deliberately and record the change rather than silently rewriting old state in incompatible ways.
- If the runtime state is version controlled, use clear commit hygiene, pruning rules, and archival rules so the state history remains understandable and recoverable.

## Adoption Notes

Use this module when a repo's user-scoped runtime home contains durable tenant, operator, or environment state that may itself need version control, backup, migration, or cross-machine continuity.

This module complements:
- `runtime-vs-product-boundary`, which keeps runtime state out of the product repo
- `tenant-isolation-and-operator-state`, which keeps runtime state isolated per tenant or operator environment
