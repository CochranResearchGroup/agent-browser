# Owned Project Integration Policy

Date: 2026-04-21

## Scope

This note records the project ownership decision that affects branch,
integration, and policy loading behavior.

## Finding

The project is now maintained as its own project under the
`CochranResearchGroup/agent-browser` origin. The original
`vercel-labs/agent-browser` repository may remain useful as a historical or
reference remote, but it is no longer the normal PR target.

## Policy Change

- `AGENTS.md` now states that normal project work lands in this project's
  `origin` repository.
- `docs/dev/policies/0011-upstream-fork-maintenance.md` was retired because it
  framed the repo as a downstream fork with private deltas on top of a
  non-owned active upstream.

## Practical Guidance

Default integration work should target this project's branch and PR flow.
Only prepare an upstream contribution when a maintainer explicitly asks for
one.

## Validation

Checked remotes on 2026-04-21:

```text
origin   https://github.com/CochranResearchGroup/agent-browser
upstream https://github.com/vercel-labs/agent-browser
```

This matches the desired model: `origin` is the owned project, while
`upstream` is reference-only unless explicitly requested otherwise.
