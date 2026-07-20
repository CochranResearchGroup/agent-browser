---
id: policy-harvest-loop
title: Policy Harvest Loop
summary: Capture reusable policy from successful sessions and normalize it back into reusable modules rather than leaving it trapped in one repo.
tags:
  - policy
  - harvesting
  - normalization
---

## Policy

- When a repo develops a strong local policy, decide whether it is:
  - repo-local only
  - reusable enough for the shared policy repo
- Prefer normalizing reusable rules into small modules rather than copying giant `AGENTS.md` sections wholesale.
- Preserve the original repo-specific wording only when the local context is essential.
- Harvest from:
  - repo `AGENTS.md`
  - repeated session behavior
  - runbook or antidrift patterns
  - branch and merge discipline that proved useful in practice
  - dated adoption feedback, notes, memories, and release notes
  - compact graph-memory facts when they are source-cited and verified against repo artifacts
- For a multi-repo harvest, define the checkout inventory and exclusions before
  comparing repos. Classify worktrees, aliases, backups, smoke clones, public
  exports, and the policy source repo separately so they do not inflate
  downstream adoption or effectiveness counts.
- Keep three evidence stages distinct:
  - `available`: the policy or behavior appears in a repo artifact
  - `adopted`: the active entrypoint wires it into the repo contract
  - `evidenced`: a current plan, runbook, closeout, or runtime receipt shows it
    affecting execution
- Do not call a policy effective from file presence or keyword counts alone.
  Review direct source paths for strong, partial, and weak cases, including
  counterexamples where tests passed but a gate correctly stopped integration.
- When deterministic audits are used as fleet evidence, record applicability,
  configured paths, audit scope, excluded legacy/unclassified artifacts, and
  false-positive limitations. A large problem count from an incompatible
  contract is migration evidence, not proof that execution quality is poor.
- When a repo has an explicit graph-memory group, query it before starting substantial harvest work so prior policy decisions and repeated friction are not rediscovered from scratch.
- After a harvest changes shared modules, profiles, selector behavior, or schema, mirror a compact source-cited summary into the policy memory group when the repo's Graphiti write workflow is available and safe.
- Do not harvest directly from unsourced memory facts. Treat graph memory as discovery and routing evidence until verified against repo files, artifacts, commits, or cited episodes.

## Adoption Notes

Use this module in policy repos and skill repos that curate reusable agent behavior.

Fleet-level harvests should produce a reproducible inventory/scorecard plus a
dated synthesis note. Keep exact repo scores and checkout paths in the audit
artifact rather than in this reusable module.
