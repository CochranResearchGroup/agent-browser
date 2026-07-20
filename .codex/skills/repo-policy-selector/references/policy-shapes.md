# Policy Shapes

Prefer three adoption shapes.

## 1. Starter profile

Use when the repo has weak or inconsistent policy and the shared profile fits well.

## 2. Profile plus overrides

Use when the repo clearly matches a profile, but has local needs such as:
- stronger roadmap governance
- stricter git/worktree handling
- repo-specific command or data rules
- a mixed maintenance-plus-platform operating model that needs branch-policy refinement without creating a whole new generic profile

## 3. Custom composition

Use when the repo has strong local policy already or spans multiple operating modes.

Examples that often need custom composition or a profile plus overrides:
- repos that are both a conservative maintenance surface and a forward-looking development platform
- repos where one maintainer preserves the current production scheme while another is building migration architecture

## Drafting guidance

- Keep shared policy wording concise.
- Preserve repo-local commands, paths, and naming.
- Avoid copying policy-repo metadata or YAML frontmatter into repo-local policy files.
- Keep `AGENTS.md` thin when possible and use it to wire in policy that lives under `docs/dev/policies/`.
- Preserve purpose-specific local rules:
  - codebase build/test/release rules for engineering repos
  - memory/heartbeat/social conduct for workspace-agent repos
  - deliverable and evidence organization rules for writing-project repos
