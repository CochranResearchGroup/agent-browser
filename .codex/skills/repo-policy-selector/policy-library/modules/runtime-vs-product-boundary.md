---
id: runtime-vs-product-boundary
title: Runtime Vs Product Boundary
summary: Keep reusable product code and templates in the repo while pushing live user, tenant, and operator state into a user-scoped runtime home.
tags:
  - runtime
  - product
  - boundary
  - state
---

## Policy

- Keep the repo publishable without private runtime state.
- Store reusable code, schemas, templates, docs, redacted fixtures, and deterministic examples in the repo.
- Store live user data, tenant data, secrets, caches, artifacts, action logs, and operator memories in a user-scoped runtime home outside the repo.
- Treat repo-local config files as templates or examples unless they are intentionally non-sensitive defaults.
- Resolve private runtime state through stable selectors such as profile ids, tenant ids, resource ids, or artifact ids rather than by hardcoding private local paths into product code.
- Do not let personal testing data, ad hoc exports, or one tenant's artifacts become the canonical product interface by accident.
- A good boundary test is: deleting the runtime home should still leave a coherent public repo.

## Adoption Notes

Use this module when a repo is both:
- a reusable software project
- and a tool used against live private operator or tenant state during development
