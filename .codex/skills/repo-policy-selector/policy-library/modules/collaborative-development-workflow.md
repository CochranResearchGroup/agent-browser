---
id: collaborative-development-workflow
title: Collaborative Development Workflow
summary: Keep multi-user agent-assisted development visible in the shared forge while allowing accountable authors to self-check and merge routine work.
tags:
  - collaboration
  - contributors
  - pull-requests
  - git
  - deployment
---

## Policy

- Use this contract when more than one human contributor, or agents acting for
  different humans, can change the same repository. The shared forge is the
  coordination and source-custody system of record; chat and local agent plans
  are supporting context, not substitutes for shared state.
- Configure one canonical remote integration branch for the repository. Normal
  work starts from a freshly fetched readback of that ref, and only a merged
  pull request may modify it. Do not push directly to the canonical branch.
- Give every substantive change one accountable human owner. An agent may plan,
  implement, test, and self-review within delegated scope, but it does not
  silently widen the objective, resolve a disagreement between humans, or
  acquire deployment authority merely by performing the work.
- Before substantive implementation, search open work items and pull requests
  for the problem and affected surface. Reuse an existing item when it already
  represents the intent; otherwise create one in the repository's configured
  tracker. Claim it with an accountable owner and an `in progress` state before
  editing so another contributor or agent can discover the active lane.
- Keep the work-item projection concise. It must identify the objective,
  accountable owner, current status, affected surface, risk or live effect,
  branch, durable plan locator when one exists, known overlaps or dependencies,
  and pull request when opened. Update material scope or ownership changes;
  do not mirror the full implementation plan into tracker prose.
- Use one scoped branch per independently mergeable intent. Put simultaneous
  work in separate physical checkouts or worktrees, and never let two people or
  agent sessions edit the same checkout. Several small related edits may share
  one work item and branch; this policy does not require an issue per commit.
- If discovery shows another active item, branch, or pull request touching the
  same behavior, do not begin a competing implementation silently. Link the
  overlap and let the accountable humans choose ownership, sequencing, or an
  integration strategy. Unrelated work may continue.
- Push coherent checkpoints soon enough for shared discovery, recovery, and CI.
  Before handoff or review, verify that the intended remote branch resolves to
  the reported commit. A local branch, clean worktree, chat message, plan, or
  live runtime does not prove shared custody.
- Every change to the canonical branch goes through a pull request linked to its
  work item. Keep the description short: objective, affected surface, risk or
  deployment effect, durable plan locator when applicable, validation evidence,
  and unresolved overlap. Draft status may communicate incomplete work but does
  not excuse stale remote custody or hidden scope.
- A second human review is optional unless another repository, security,
  financial, legal, or live-effect rule explicitly requires it. The accountable
  author may self-check and merge their own pull request after CI and applicable
  checks pass. Self-check means inspecting the published diff, validation, base,
  scope, and deployment effect; it is not represented as a separate GitHub
  approval by the author.
- Close the work item only when its stated outcome is complete. Link the merged
  pull request and record any deferred work separately. A closed issue, merged
  pull request, successful test, deployment, or observed outcome proves only its
  own boundary.
- Production deployment is allowed only from an exact commit on the configured
  canonical remote branch. Fetch the remote immediately before release, resolve
  the candidate SHA from the remote ref rather than a local branch, and verify
  the commit entered the branch through a merged pull request. Normal deployment
  uses the current verified remote tip; an authorized rollback may select a
  previously merged commit on that branch with an incident or rollback record.
- Record the repository, canonical remote ref, exact commit, actor, target
  environment, validation, authorization, and post-deploy readback. Deployment
  automation must fail closed when the candidate is local-only, the remote
  readback differs, pull-request provenance is absent, or required checks fail.
- Treat direct edits to a live CMS, commerce system, asset store, or other
  database-backed surface as state changes that still require a reproducible
  migration, export, or reconciliation artifact when the repository owns that
  state. Do not let live state become the only surviving copy of a change.
- Run urgent fixes through the same accelerated issue, branch, pull-request,
  merge, and canonical-branch deployment path. Urgency may shorten discussion
  and validation to the safest relevant subset, but it does not authorize an
  unpublished deployment. If the forge path is unavailable, apply separately
  authorized operational mitigation and preserve evidence; reconcile source
  before resuming normal deployment.
- Use one tracker as the canonical coordination ledger. Do not duplicate active
  state in GitHub Issues and Jira unless a documented cross-system need and a
  reliable ownership or synchronization rule justify the second system.

## Adoption Notes

For a small team, the minimum useful flow is:

1. search or open the work item and claim it;
2. create a scoped branch in an isolated checkout;
3. publish the branch and link its pull request;
4. run CI and self-check the published diff;
5. merge without waiting for peer review when no separate approval applies;
6. deploy only the exact verified canonical remote commit.

Use repository-local guidance for the canonical ref, contributor names, branch
format, issue states and labels, plan locations, CI requirements, deployment
commands, high-risk approval rules, and emergency contacts. Where provider
branch protection is unavailable, enforce the same contract through contributor
policy and a fail-closed deployment preflight until provider enforcement exists.
