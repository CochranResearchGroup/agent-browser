---
id: tenant-isolation-and-operator-state
title: Tenant Isolation And Operator State
summary: Keep one runtime profile per tenant or operator environment, with isolated state, resources, artifacts, and logs.
tags:
  - tenant
  - runtime
  - isolation
  - operations
---

## Policy

- Treat one runtime profile as one tenant or one operator environment.
- Keep state, resources, memories, artifacts, and action history isolated per profile.
- Bind any persistent local state to the exact runtime target it belongs to, such as profile name, tenant label, base URL, database, or equivalent identity.
- Refuse silent reuse of one tenant's local state against another tenant or environment.
- Keep tenant-specific secrets, mailbox bindings, connector identities, and deploy-time resources outside the repo in the runtime home.
- Prefer explicit readiness checks before tenant-specific write workflows.
- Record durable tenant facts in tenant-scoped runtime memories rather than repo docs when those facts are private, environment-specific, or operationally sensitive.
- Keep cross-tenant product behavior in product code and shared docs; keep tenant-specific operational facts in isolated runtime state.

## Adoption Notes

Use this module when a repo manages more than one live customer, tenant, environment, or operator-facing runtime target.
