---
id: policy-adoption-feedback-loop
title: Policy Adoption Feedback Loop
summary: Record what worked, what did not, and what should change upstream after policy adoption or upgrade so reusable lessons do not disappear into chat history.
tags:
  - policy
  - feedback
  - upgrades
  - notes
---

## Policy

- After first policy adoption, the first substantive execution under that
  policy, a major policy upgrade, or meaningful policy friction, record a dated
  feedback artifact in the adopting repo.
- The feedback artifact should identify at least:
  - installed policy bundle version or immutable ref, or an explicit statement
    that provenance is unknown and must be repaired
  - selected profile
  - modules adopted
  - modules deferred, retired, or overridden locally
  - what worked cleanly
  - what created friction or ambiguity
  - what should remain repo-local
  - what may warrant an upstream module, profile, or selector change
- Distinguish installation, active wiring, and enacted behavior. Cite the
  `AGENTS.md` entrypoint for wiring and a current plan, runbook entry, closeout,
  audit receipt, or runtime readback for behavioral evidence.
- If no substantive execution has exercised the policy yet, record `not yet
  evidenced` rather than calling adoption successful or ineffective.
- Prefer storing dated adoption feedback in the repo's normal durable continuity surface, such as:
  - `docs/dev/notes/`
  - `docs/dev/memories/`
  - bounded plans plus matching runbook entries
  - another documented local equivalent
- Do not leave important adoption lessons only in chat history, commit messages, or oral maintainer knowledge.
- When feedback appears reusable across repos, route it into the shared policy repo through a deterministic harvest path rather than treating it as one repo's private observation.
- If the repo uses a pinned installed selector bundle, tie feedback to that pinned version so later maintainers can interpret it correctly.
- When a repo adopts local overrides instead of the exact starter profile, record why; those reasons are often the best signal for future shared policy refinement.
- When a repo upgrades policy, compare the new experience to prior adoption notes so repeated friction becomes visible over time.
- Record stale local-policy prose, invalid local facts, and audit-contract
  incompatibilities as adoption defects even when the underlying work outcome
  was successful.
- When a repo has an explicit graph-memory group, mirror compact source-cited adoption feedback into that group after the dated feedback artifact exists, especially when it identifies reusable friction, missing modules, profile-fit issues, or selector behavior changes.
- Keep graph-memory feedback entries small and source-anchored. They should point future agents to the dated artifact or release note, not replace it.
- A single dated artifact may satisfy this module, `policy-upgrade-management`, and `notes-and-memories` when it captures both the upgrade or adoption decision and the resulting feedback clearly.

## Adoption Notes

Use this module when repos adopt shared policy from an external source library and want a durable loop between downstream adoption experience and upstream policy improvement.

This module complements `notes-and-memories` and `policy-harvest-loop`:
- `notes-and-memories` defines where continuity artifacts live
- `policy-harvest-loop` governs how a policy repo normalizes reusable rules
- `policy-adoption-feedback-loop` governs how adopting repos capture feedback that can later be harvested
