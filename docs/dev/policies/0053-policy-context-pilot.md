---
id: policy-context-pilot
title: Policy Context Pilot
summary: Pilot structured, scoped policy reads through a governance MCP while canonical Markdown remains authoritative and direct reads remain a tested fallback.
category: pilots
status: pilot
tags: [pilots, policy, mcp, context, measurement]
---

## Pilot Status

This module governs an opt-in experiment. Selection means the repository is a
candidate for a bounded pilot; it does not establish a default policy-loading
method, prove token savings, or authorize fleet rollout.

## Policy

- Keep canonical repository policy in human-readable Markdown. Treat a policy
  context manifest, compiled response, cache, capsule, or MCP output as a
  derived read surface.
- Start with one repository, one declared policy profile, and a small set of
  explicit task kinds. Do not infer applicability from free-form task text in
  the first pilot.
- Keep a complete inventory of policy-bearing files in the pilot manifest,
  including the repository policy entrypoint. Record content hashes for the
  declared canonical sources.
- For each profile, declare exact source sections, supported task kinds, scope,
  reread triggers, canonical fallback paths, and a versioned decision packet.
  The packet must declare cited required gates, prohibited effects, approval
  requirements, validation requirements, receipt requirements, precedence, and
  unresolved conflicts. The tool may address document structure internally;
  agents should request the profile and task kind rather than line numbers.
- Assemble policy context from current canonical sections at read time. Return
  source paths, hashes, cited obligations, a stable policy digest, and a
  decision digest. Keep section text out of the default compact result and
  hydrate it only on explicit request.
- Keep current-action facts separate from canonical policy. Accept only bounded
  caller-supplied facts with an ID, kind, statement, and source; echo them in
  the packet and never infer user authorization, requested effects, lineage,
  exclusions, rollback requirements, or runtime observations from static
  policy.
- Return unresolved conflicts explicitly. Reject unknown obligation or citation
  references and any manifest that claims a conflict resolution outside the
  reviewed precedence declarations.
- Fail closed without policy text when a source is missing or changed, the
  inventory differs, a selector is invalid, the profile is unknown, or the
  task kind is outside the declared scope. Direct the agent to the named
  canonical fallback paths.
- Wire `AGENTS.md` with a precise trigger: for a supported task, call
  `gov_policy` once with the declared profile and task kind; on any unavailable,
  stale, invalid, or out-of-scope result, read the canonical fallback files.
- Preserve ordinary direct Markdown reading throughout the pilot. The MCP must
  not become the only path to repository governance.
- Refresh hashes only after reviewing the canonical policy change. Treat
  profile scope, section selection, task kinds, and reread triggers as
  reviewable policy decisions rather than mechanical manifest maintenance.

## Pilot Validation

- Run a fail-closed availability preflight before admitting a repository to a
  real task. Persist a receipt proving the intended installed version, exact
  registered root and required includes, fresh-session `gov_policy` discovery,
  a current profile result, and readable canonical fallback. Selector
  installation or an `AGENTS.md` pointer alone is not availability evidence.
- Before real work, exercise four deterministic cases: current profile, changed
  source, missing or added policy file, and out-of-scope task kind.
- Run at least one fresh-host task through the MCP path and verify the resulting
  action or edit against the canonical policy. Record tool calls and whether a
  canonical fallback occurred.
- Compare representative ordinary tasks in paired conditions: direct canonical
  reads and the policy-context path. Hold the task, host configuration, policy
  revision, and correctness criteria stable where practical.
- Record total host input, cached input when available, output, tool calls,
  serialized policy bytes, fallback frequency, latency, and correctness. Do not
  use response byte count alone as a token-efficiency result.
- Record startup policy and skill context separately when it dominates total
  usage. Report inconclusive measurements as inconclusive.

## Adoption And Exit

- Record the installed governance-context version or immutable ref, manifest
  version, selected profile, task kinds, canonical sources, host-registration
  and discovery receipt, validation receipt, and pilot owner in a dated
  repository note.
- Continue only when the repository shows equal policy correctness and a useful
  reduction in total context cost or operational friction for its measured
  tasks. Narrow or revise the pilot when fallback frequency or maintenance cost
  erases the benefit.
- Remove the pilot when it produces stale guidance, obscures authority, cannot
  be measured, or adds more context than direct reads. Removal includes the
  `AGENTS.md` trigger, MCP configuration, and manifest; canonical policies stay
  in place and direct reads must still work.
- Treat wider adoption, default-profile inclusion, and savings claims as later
  decisions supported by evidence from more than one suitable repository.

## Adoption Notes

Use this module as a custom composition when the target repository already has
or explicitly requests a policy-context MCP pilot, such as a
`.governance/policy-context.json` manifest or `gov_policy` loading rule. Do not
add it merely because a repository uses MCP tools, has large policy files, or
wants general token efficiency.
