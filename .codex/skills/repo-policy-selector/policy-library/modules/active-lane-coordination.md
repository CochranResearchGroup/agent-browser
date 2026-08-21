---
id: active-lane-coordination
title: Active Lane Coordination
summary: Keep a default-branch projection of concurrent off-main work while preserving branch-local execution plans and Git as custody evidence.
tags:
  - git
  - planning
  - worktrees
  - coordination
---

## Policy

- Use this contract in repositories where several projects, agents, branches, or worktrees may remain active at once. Keep lighter repositories on proportional planning and Git policy without requiring a lane catalog.
- Keep a compact machine-readable active-lane catalog on the canonical default branch, normally `docs/dev/active-lanes.yaml`. A documented equivalent path is allowed.
- Treat the catalog as a discovery projection. A roadmap owns priority, a branch-local plan owns execution detail, a runbook owns chronological history, review tooling owns review state, and Git refs plus receipts prove custody and integration.
- Give each lane one stable id and one branch owner. Record its objective, plan path and source ref, branch, target, plan state, custody state, published checkpoint, remote ref, integration method, dependencies, overlaps, reconciliation date, and any blocker or disposition.
- Keep plan outcome state separate from Git custody state. Use a small plan vocabulary such as `PLANNED`, `OPEN`, `BLOCKED`, `CLOSED`, and `CANCELLED`, and a custody vocabulary such as `ACTIVE_WORKTREE`, `PAUSED_REF`, `INTEGRATION_READY`, `INTEGRATED`, `ARCHIVED`, and `DISCARD_APPROVED`.
- Keep detailed plans with their topic branches. Expose deterministic metadata for lane, state, branch, target, integration method, dependencies, overlaps, and base or checkpoint evidence so an auditor can read it from an explicit ref without checkout.
- Do not put absolute worktree paths, ephemeral agent identifiers, secrets, tenant data, or private runtime details in the shared catalog. Derive local worktree locations during reconciliation.
- Reconcile the catalog against current worktrees, bounded local and remote refs, branch-local plan metadata, checkpoint SHAs, target ancestry, receipts, dependencies, and overlap before planning, handoff, integration, or cleanup decisions. Prefer catalog-only discovery when the catalog is the complete authorized population; use exact repeated branch selectors for bounded unregistered-lane discovery. Prefix discovery is an explicit broader survey and should not be the default in repositories with large historical branch namespaces.
- For active worktree custody, classify equal, local-ahead, remote-ahead, and diverged local/remote tips explicitly. Local-ahead, remote-ahead, and diverged state fail closed until the lane owner reconciles and publishes the intended checkpoint.
- Fetching is a caller-controlled operation. A lane auditor must remain read-only and must not fetch, merge, rebase, push, delete refs, remove worktrees, edit plans, or infer authority from a clean report.
- Register normal work before parallel execution begins. An urgent lane may start first only when delay creates greater risk; register and publish its first recoverable checkpoint at the earliest safe boundary.
- Do not silently resolve catalog conflicts. Duplicate lane ids, two lanes claiming one branch, missing custody, stale checkpoints, active local/remote mismatch, plan/catalog drift, and unresolved overlaps fail closed until reconciled.
- Keep the catalog current through the repository's protected-default-branch workflow. A lane branch may propose its own registration, but it is not globally discoverable until that projection lands on the configured default ref.

## Adoption Notes

Adopt this module by default for multi-track product-engineering and operations-platform repositories. Add it to other profiles only when concurrent worktree, off-main planning, branch-registry, or multi-project signals justify the coordination cost.

Repositories may provide a deterministic auditor such as `audit_active_lanes.py`. Its report is evidence for reconciliation, not permission to merge, publish, discard, or clean up.
