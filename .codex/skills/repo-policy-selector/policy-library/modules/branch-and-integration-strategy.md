---
id: branch-and-integration-strategy
title: Branch And Integration Strategy
summary: Choose one primary branch model, document how work lands, and be explicit about merge, rebase, and stabilization expectations.
tags:
  - git
  - branches
  - integration
  - rebase
---

## Policy

- Document one primary integration model for the repo rather than switching casually between incompatible branch conventions.
- If a repo has both conservative maintenance work and more aggressive platform or architecture work, document how those tracks coexist instead of leaving branch choice to local habit.
- Be explicit about where normal work starts and where it lands:
  - direct to `main`
  - short-lived feature branches
  - release or stabilization branches
- Prefer short-lived branches unless the repo has a documented reason for long-lived branch divergence.
- Prefer explicit track naming when different work classes coexist, for example maintenance-oriented branches versus architecture-oriented branches.
- State whether merge commits, rebased histories, or squash merges are preferred for shared history.
- State when rebasing is normal and when it is no longer appropriate because others may already depend on the branch.
- Once another lane, person, automation, or review surface depends on a published branch tip, do not rebase or otherwise rewrite it without explicit reconciliation and bounded lease protection.
- Treat branch protection, review gates, and release branching as part of the workflow contract rather than personal preference.
- Do not let local habit override the repo's documented integration model.
- When a repo supports parallel work, document whether reconciliation should happen by rebase, merge, or explicit integration branches.
- Keep branch lifecycle distinct from worktree lifecycle: a lane may remain active with a worktree, pause as a remotely preserved ref, become integration-ready, prove integration, remain temporarily cleanup-pending, archive, or receive explicit discard approval.
- Declare integration readiness only when the lane is clean, its tested checkpoint is published and matches the recorded SHA, dependencies and overlaps are reconciled, and the intended target and integration method are explicit.
- Prove merge integration by target ancestry. For squash or patch integration, preserve a durable receipt that identifies the source checkpoint and resulting target commit; do not infer integration from similar content or a closed pull request alone.
- Delete topic refs only after integration proof, verified archival, or exceptional discard approval. Routine branch cleanup must not use forced deletion to bypass missing evidence.
- Use disposable integration branches for cross-lane compatibility experiments. Do not make an exploratory integration branch a hidden source of truth for its component lanes.
- If current-behavior maintenance and future-architecture work can touch the same surface concurrently, document which class wins by default unless an approved migration slice says otherwise.

## Adoption Notes

Use this module when the repo has more than one contributor, review checkpoints, CI gates, or multiple valid ways work could land.

Repo-type guidance:
- `product-engineering`: usually benefits from explicit rules for feature branches, protected branches, and stabilization before release
- `library-cli`: often benefits from a simple default branch plus tagged releases, but may still need clear rules for release branches when compatibility is sensitive
- `workspace-agent`: often benefits from short-lived branches and explicit rebase expectations because selector, prompt, and policy changes can drift quickly
- `writing-project`: may keep a lighter branch model, but collaborative review repos still benefit from an explicit default integration path

Developer-preference guidance:
- trunk-based teams may prefer direct-to-main or very short-lived feature branches with fast validation
- review-heavy teams may prefer feature branches plus squash or rebase merges
- release-sensitive teams may require temporary stabilization branches before tags or deploys

Multi-track repo guidance:
- repos that act as both a maintenance surface and a development platform usually need:
  - one stable integration line
  - short-lived feature branches or worktrees for parallel tracks
  - explicit rules for when maintenance preserves current behavior and when migration work may intentionally replace it
