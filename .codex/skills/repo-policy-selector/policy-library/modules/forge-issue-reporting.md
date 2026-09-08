---
id: forge-issue-reporting
title: Forge Issue Reporting
summary: Report work through exact, permission-aware forge targets with idempotency, governed metadata, and verified receipts.
tags:
  - issues
  - github
  - gitlab
  - reporting
  - governance
---

## Policy

- Use this module with `work-item-traceability`. The tracker owns intake,
  discussion, priority, and dependencies; plans, lanes, review, Git, tests,
  deploy readback, and receipts retain their separate authority.
- Resolve every operation to an explicit forge, hostname, and canonical
  repository or project path. Do not infer an external write target solely from
  the current directory, a default remote, a similarly named fork, or a prior
  operation.
- Require all of these gates before a provider mutation:
  - current operator authority for the exact action
  - an allowlisted target and action
  - compliance with the target's contribution, issue-template, security, and
    disclosure rules
  - current provider capability for the authenticated actor or app
  - a postcondition that can be read back without repeating the mutation
- Treat authentication, repository ownership, organization membership, and a
  provider role as capability evidence, not operator intent. Read or research
  authority does not imply create, comment, edit, label, assign, close, reopen,
  transfer, board, or project authority.
- Keep a non-secret repo-local target registry with the forge, host, canonical
  path, owned-or-permissioned relationship, allowed actions, security route,
  and normalized label mappings. Verify current provider state instead of
  treating the registry as proof that access still exists.
- Classify a proposed report before creation, such as defect, feature,
  operational incident, governance gap, or security report. Include bounded
  expected-versus-observed evidence, impact, reproduction context, relevant
  version or environment, and explicit uncertainty. Redact credentials,
  private customer data, and unnecessary personal information.
- Map normalized label intent through the target registry to exact provider
  labels. Unknown mappings and missing provider labels fail closed. Applying an
  existing label, creating a label, changing label taxonomy, and applying a
  group- or organization-scoped label are separate actions and authority gates.
- Preserve the target repository's labels and workflow vocabulary. Do not
  silently substitute a similar label, create a missing label, or impose a
  source repository's taxonomy on a permissioned target.
- Search for duplicates before creating an issue. Carry a stable, non-secret
  idempotency marker in the proposed body or other supported metadata. After an
  ambiguous provider response, search and read back the marker before retrying.
- Default permissioned targets to the least-invasive allowed behavior. Do not
  assign maintainers, change labels or milestones, add planning metadata,
  transfer, close, or reopen merely because the authenticated identity can.
- Treat security-sensitive content as a separate disclosure workflow. Use the
  target's private vulnerability or security-reporting route and never publish
  vulnerability details in a normal public issue as a fallback.
- Record a receipt for each mutation with authenticated actor, host, canonical
  locator and URL, action, idempotency key, timestamp, relevant before-and-after
  state, and post-write readback. A successful request without target readback
  is not a verified effect.
- Keep issue closure distinct from implementation, validation, integration,
  deployment, and cleanup. Use closing references only when the target workflow
  permits them and the governing completion evidence will exist.

## Adoption Notes

Adopt the provider adapter for each forge in use. Keep provider commands,
target lists, label names, and operational routing in repo-local configuration
instead of copying them into this shared module.
