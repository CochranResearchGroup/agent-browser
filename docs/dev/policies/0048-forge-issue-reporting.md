# Policy | Forge Issue Reporting

## Policy

- Use this policy with `0046-work-item-traceability.md`. The tracker owns
  intake, discussion, priority, and dependencies; plans, lanes, review, Git,
  tests, deploy readback, and receipts retain separate authority.
- Resolve every operation to an explicit forge, hostname, and canonical
  repository. Do not infer a write target from the current directory, default
  remote, similarly named fork, or prior operation.
- Before a provider mutation require current operator authority for the exact
  action, an allowlisted target and action, target-rule compliance, current
  provider capability, and a readable postcondition.
- Treat authentication, ownership, organization membership, and provider role
  as capability evidence, not operator intent. Read authority does not imply
  create, comment, edit, label, assign, close, reopen, transfer, or planning
  authority.
- Keep a non-secret repo-local target registry with forge, host, canonical
  repository, relationship, allowed actions, security route, and normalized
  label mappings. Verify provider state rather than treating the registry as
  proof that access remains current.
- Classify a report before creation. Include bounded expected and observed
  evidence, impact, reproduction context, relevant version or environment, and
  explicit uncertainty. Exclude credentials, customer data, and unnecessary
  personal information.
- Map normalized label intent through the registry to an exact provider label.
  Unknown mappings and missing labels fail closed. Applying a label, creating a
  label, changing taxonomy, and applying organization-scoped metadata are
  separate actions.
- Preserve the target's workflow vocabulary. Never silently substitute a
  similar label or create a missing label without separate authority.
- Search for duplicates before issue creation. Carry a stable non-secret
  idempotency marker in the body. After an ambiguous response, search and read
  back that marker before retrying.
- Default permissioned targets to the least-invasive behavior. Do not assign,
  milestone, plan, transfer, close, or reopen merely because capability exists.
- Route security-sensitive content through the target's private disclosure
  workflow. Never publish vulnerability details in a normal public issue as a
  fallback.
- Record each mutation with actor, host, canonical locator and URL, action,
  idempotency key, timestamp, relevant before and after state, and readback.
- Keep issue closure distinct from implementation, validation, integration,
  deployment, and cleanup.

## Adoption Notes

GitHub-specific commands and checks are owned by policy 0049. Agent Browser's
exact target and labels are configured in `docs/dev/forge-issue-targets.json`.
