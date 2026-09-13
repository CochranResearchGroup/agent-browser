---
id: development-runtime-isolation
title: Development Runtime Isolation
summary: Isolate concurrent development runtimes from production and from one another, and keep live external effects separately governed.
tags:
  - runtime
  - development
  - isolation
  - environments
  - safety
---

## Policy

- Treat production, staging, and development runtimes as distinct environment
  identities. A Git branch or local configuration change does not itself create
  an isolated environment or authorize promotion between them.
- Give each concurrently executing development lane its own runtime identity
  when it needs service execution. Isolate configuration roots, databases,
  writable data, ports or sockets, logs, process or service identities, and
  other mutable resources that could collide with another lane or production.
- Bind runtime identity to the lane and exact source checkpoint through a
  readable health, status, or startup record. Do not claim isolation from naming
  convention or intended configuration alone.
- Fail closed when resource separation cannot be proved. Do not inherit
  production credentials, schedules, browser profiles, writable data, or other
  effect-bearing state into a development runtime merely for convenience.
- Prefer provider-free fixtures and disposable development data for ordinary
  validation. Treat authenticated provider calls, browser sessions, shared
  hardware, rate limits, and other singleton resources as serialized unless an
  explicit operating contract proves independent identities and effect
  boundaries.
- Keep runtime provisioning and teardown exact and reversible. Teardown must
  target only the named lane runtime and must verify that production and other
  active lanes remain intact.
- Promote from lane evidence to integration and then through the repository's
  staging and production gates. A passing local suite or healthy development
  runtime proves neither staging acceptance nor production readiness.

## Adoption Notes

Use this module when multiple branches or agents may run services concurrently,
or when development execution could otherwise reuse production-like state. Keep
provider names, credential stores, ports, service commands, environment paths,
and deployment topology in repository-local guidance.
