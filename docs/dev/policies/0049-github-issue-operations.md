# Policy | GitHub Issue Operations

## Policy

- Use this adapter with policy 0048 and identify targets by explicit GitHub
  hostname plus `OWNER/REPO`. Distinguish an owned repository, owned fork, and
  permissioned upstream before selecting the report target.
- Preflight authenticated hostname and actor, canonical repository, viewer
  permission, archive state, Issues availability, fork parent, security route,
  issue templates, and relevant labels with read-only GitHub calls.
- Evaluate every requested action separately. Issue creation, applying an
  existing label, creating a label, assigning, changing a milestone or Project,
  closing, and transferring are distinct capabilities and authority gates.
- Read the target's contribution, security, issue-form, chooser, and contact
  files before drafting. API creation must preserve required form fields.
- Resolve normalized label intent to one exact existing repository label before
  issue creation. Authority to apply labels does not imply label-creation or
  taxonomy-change authority.
- Treat issue types, Projects, milestones, sub-issues, dependencies, and
  assignees as optional extensions. Creation authority does not imply authority
  to populate them.
- Use GitHub private vulnerability reporting or the declared private security
  route for vulnerability details. A public issue may request a contact route
  only when repository instructions permit it and must not reveal the defect.
- Bind receipts to the returned repository, issue number, canonical URL, actor,
  and read-back state. On timeout or rate-limit ambiguity, search for the
  idempotency marker before another creation attempt.

## Adoption Notes

Use explicit `--hostname` targeting when the GitHub CLI operation supports it.
Keep token scopes least-privilege and never print tokens in diagnostics.
