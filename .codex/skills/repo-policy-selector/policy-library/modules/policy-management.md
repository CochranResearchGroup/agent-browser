---
id: policy-management
title: Policy Management
summary: Install policy first, keep adopted repo-local policy under docs/dev/policies, and wire it in deterministically from AGENTS.md.
tags:
  - policy
  - governance
  - installation
  - wiring
---

## Policy

- When a repo adopts shared policy, install the policy library before running selection or adoption workflows.
- Enumerate available profiles, modules, and catalog metadata deterministically from the installed policy library rather than relying on chat history or sibling checkout layout.
- Keep the adopted repo-local policy under `docs/dev/policies/`.
- Keep exactly one active repo-local policy file per shared module identity.
  Treat the module id, not the ordinal filename, as the stable identity; a new
  serial must not turn an upgrade into a second active copy.
- Keep `AGENTS.md` as the entrypoint that wires the adopted repo-local policy into the repo contract.
- Treat `AGENTS.md` as a policy-loading contract, not just a static pointer.
- Treat repo-local policy as one section of `AGENTS.md`, not the whole file.
- Keep repo-specific commands, environment prerequisites, and operating constraints in `AGENTS.md` or adjacent local docs even after shared policy is installed.
- Keep `AGENTS.md` thin relative to the full durable policy body; do not turn it into the full policy dump if the repo can keep policy files under `docs/dev/policies/`.
- Make each policy pointer name both the target and the condition that should
  trigger reading it. Required policy behind a vague or stale pointer is not
  reliably wired.
- Keep each rule in one authoritative location. Use `AGENTS.md` for routing and
  repo-specific constraints, and use linked policy files for the durable body;
  do not duplicate the same rule across both surfaces for emphasis.
- Re-read the relevant adopted policy files at the start of any non-trivial turn.
- Re-read the relevant adopted policy files when task scope changes mid-session.
- Treat policy installation, policy enumeration, and `AGENTS.md` wiring as deterministic setup work rather than ad hoc prose copying.
- Validate policy identity and wire-in uniqueness deterministically. Duplicate
  identities must name every conflicting path and fail closed until a
  maintainer reconciles them; tooling must not silently choose a winner.
- When the repo uses an installable selector bundle, ensure the selector ships with the policy library it depends on.

## Adoption Notes

Use this module as the first adopted policy when a repo is managed through the shared policy selector workflow.
