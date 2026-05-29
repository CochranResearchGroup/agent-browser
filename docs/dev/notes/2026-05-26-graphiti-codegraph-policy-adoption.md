# Graphiti And CodeGraph Policy Adoption

Date: 2026-05-26
State: RECORDED

## Scope

This note records a bounded patch-missing adoption of two shared policy
modules into the agent-browser repo.

## Policy Source

- Selector skill: `/home/ecochran76/workspace.local/agent-policies/repo-policy-selector/SKILL.md`
- Policy library: `/home/ecochran76/workspace.local/agent-policies/repo-policy-selector/policy-library`
- Release manifest: `repo-policy-selector` `v0.1.13`
- Source commit: `b38c90694e15562819d28a4338c5d148dc5171fd`

## Adopted Modules

- `graph-backed-memory-usage`
- `codegraph-usage`

Installed repo-local files:

- `docs/dev/policies/0011-graph-backed-memory-usage.md`
- `docs/dev/policies/0012-codegraph-usage.md`

## Selector Findings

- `scripts/select_policy.py` classified current policy coverage as
  `partial-local-policy`.
- Existing policy files `0001` through `0010` were kept.
- `AGENTS.md` remains the policy-loading entrypoint and now wires in the two
  new policy files.
- The deterministic catalog confirmed both requested modules exist in the
  installed policy library.

## Local Adaptation

- Graphiti policy names `agent_browser_main` as the repo memory group and
  `memory_atlas_main` as the reviewed cross-domain routing atlas.
- Graphiti policy keeps memory advisory and requires source verification
  against repo files, artifacts, commits, tests, or cited episodes.
- CodeGraph policy names the repo-local `.codegraph/` index and the expected
  MCP structural tools.
- CodeGraph policy requires fallback disclosure when tooling is unavailable,
  stale, or not initialized.

## Deferred Modules

The selector recommended additional modules for broader profile alignment, but
this adoption intentionally installs only the Graphiti and CodeGraph modules
requested by the maintainer.

## Validation Notes

- `graphiti-runtime doctor` reported a healthy runtime.
- `graphiti-runtime atlas-discover` routed this repo to `agent_browser_main`.
- `graphiti-runtime discover --group-id agent_browser_main` found repo-sourced
  AGENTS and README memory episodes.
- `audit_planning_contract.py` reported existing planning-contract gaps because
  this repo has `docs/dev/plans/` but no top-level `ROADMAP.md` or `RUNBOOK.md`.
  That is outside this bounded policy installation.
