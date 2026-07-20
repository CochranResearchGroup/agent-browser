---
id: memory-service-runtime-governance
title: Memory Service Runtime Governance
summary: Govern installed memory-service runtimes with clear client/server boundaries, health checks, queue visibility, and live smoke verification.
tags:
  - memory
  - runtime
  - mcp
  - operations
---

## Policy

- Treat an installed memory service as an operational runtime, not just a library import or agent-side feature.
- Keep the client/server boundary explicit:
  - agent clients invoke the service
  - the service owns its model, embedding, storage, and queue backends
  - client configuration should not be assumed to supply the service's provider credentials or runtime state
- Keep reusable code, scripts, docs, schemas, and redacted examples in the repo; keep live service config, credentials, queues, databases, caches, and operator state in the runtime home.
- Verify the authoritative runtime state before diagnosing behavior, including:
  - installed release or manifest identity
  - active process or service manager state
  - current runtime config source
  - health endpoint or equivalent readiness check
  - bound listener or transport endpoint
- When the service accepts asynchronous memory work, expose durable job status rather than relying on fire-and-forget behavior.
- Use a small, explicit queue vocabulary such as pending, running, succeeded, failed, retrying, and dead-lettered.
- Make dead-letter inspection and recovery deliberate operations, with separate list, requeue, and drop paths when the runtime supports them.
- After install, restart, migration, or backend changes, run a live read-after-write smoke that proves the service can accept memory input, process it, and retrieve the resulting record.
- Wait for readiness before smoke checks so startup races are not confused with broken releases.
- Check for stale processes or port/listener conflicts before assuming the installer, service manager, or new release is faulty.
- Treat destructive memory or queue maintenance as explicit repair or cleanup work, not normal exploration.
- When docs and runtime factory/config code disagree about supported providers or backends, verify against the executable runtime source or a live health/smoke check before giving setup guidance.
- Keep provider names, runtime paths, service names, port numbers, database choices, and exact tool names repo-local unless they clearly generalize across multiple memory-service runtimes.

## Adoption Notes

Use this module when a repo builds, installs, or operates a durable memory service, especially when agents reach it through MCP or a similar service boundary.

This module complements:
- `graph-backed-memory-usage`, which governs what agents should read and write as durable graph memory
- `runtime-vs-product-boundary`, which separates reusable product code from private runtime state
- `runtime-state-governance`, which decides when runtime artifacts themselves need backup, migration, or versioned tracking

For repos that only consume a memory service through normal agent workflow and do not operate the service runtime, prefer `graph-backed-memory-usage`.
