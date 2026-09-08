---
id: gitlab-issue-operations
title: GitLab Issue Operations
summary: Apply forge issue-reporting controls to GitLab projects, roles, work items, labels, templates, and confidential reporting.
tags:
  - gitlab
  - glab
  - issues
  - labels
  - permissions
---

## Policy

- Use this adapter with `forge-issue-reporting` and identify targets as an
  explicit GitLab hostname plus the full `GROUP/SUBGROUP/PROJECT` path. Preserve
  both the project id and project-local issue IID in receipts when available.
- Preflight the authenticated hostname and actor, canonical
  `path_with_namespace`, archive state, issue access level, effective
  project-or-group role, project URL, and relevant labels with read-only `glab`
  or API calls. Support GitLab.com and explicitly allowlisted self-managed
  hosts without treating them as interchangeable.
- Evaluate each action against current GitLab permissions. Guest issue creation
  does not imply authority to edit general issue metadata, manage boards or
  milestones, create labels, move issues, or operate on another reporter's
  issue.
- Read the target's contribution and security guidance plus
  `.gitlab/issue_templates/` before drafting. Because `glab issue create
  --template` loads from the local checkout, prove that the local template
  belongs to the exact target revision or fetch the target template explicitly.
- Resolve normalized label intent to an exact existing project or group label.
  Record the resolved scope, and do not create or modify group-scoped labels as
  a side effect of reporting to one project.
- Treat boards, milestones, epics, tasks, work-item types, weights, time
  tracking, linked issues, and linked merge requests as optional GitLab
  extensions with separate authority. Do not force them into GitHub-equivalent
  semantics.
- Treat `confidential` as an explicit visibility choice, not an automatic
  security-disclosure guarantee. Follow the target's security workflow and
  verify who can read the confidential issue before submitting sensitive
  details.
- Bind mutation receipts to the returned project id, issue IID, canonical URL,
  actor, and read-back state. On timeout or API ambiguity, search for the
  idempotency marker before issuing another create request.

## Adoption Notes

Use an explicit `--hostname` for authentication and API discovery. Avoid
commands that display tokens, and URL-encode nested project paths for REST API
calls.
