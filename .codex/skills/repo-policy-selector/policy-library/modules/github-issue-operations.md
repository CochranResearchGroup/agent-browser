---
id: github-issue-operations
title: GitHub Issue Operations
summary: Apply forge issue-reporting controls to GitHub roles, repositories, issue forms, labels, and security routes.
tags:
  - github
  - issues
  - labels
  - permissions
  - security
---

## Policy

- Use this adapter with `forge-issue-reporting` and identify targets as an
  explicit GitHub hostname plus `OWNER/REPO`. Distinguish an owned repository,
  an owned fork, and a permissioned upstream before choosing the report target.
- Preflight the authenticated hostname and actor, canonical `nameWithOwner`,
  viewer permission, archive state, Issues availability, fork parent, security
  policy, issue templates, and relevant existing labels with read-only `gh` or
  API calls.
- Evaluate the requested action against the current GitHub role and token or
  app permissions. Opening an issue, applying an existing label, creating a
  label, assigning, changing a milestone or Project, closing, and transferring
  are distinct capabilities.
- Read the target's `CONTRIBUTING.md`, `SECURITY.md`, issue forms, issue-template
  chooser, and contact links before drafting. Required target fields remain
  required when the issue is created through an API rather than the web form.
- Resolve every normalized label to one exact existing repository label before
  creation. A target may authorize application of existing labels without
  authorizing label creation or taxonomy changes; never collapse those actions.
- Treat issue types, Projects, milestones, sub-issues, dependencies, and
  assignees as optional GitHub extensions. Creation authority does not imply
  authority to populate them.
- Use GitHub private vulnerability reporting or the repository's declared
  private security route for vulnerability details. A public issue may ask for
  a contact route only when the repository directs reporters to do so and must
  not contain the vulnerability.
- Bind mutation receipts to the returned repository and issue number, canonical
  URL, actor, and read-back state. On timeout or rate-limit ambiguity, search
  for the idempotency marker before issuing another create request.

## Adoption Notes

Prefer explicit `--hostname` or host-qualified targeting for GitHub Enterprise.
Keep fine-grained token scopes least-privilege and do not print tokens in
diagnostic output.
