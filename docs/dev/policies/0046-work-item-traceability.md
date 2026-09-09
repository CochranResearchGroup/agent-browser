# Policy | Work Item Traceability

## Policy

- Use the issue tracker or governed repo-local backlog as the intake,
  discussion, prioritization, and dependency surface. Do not make it the sole
  authority for detailed execution, Git custody, review, or completion proof.
- Keep authority roles explicit:
  - roadmap or portfolio view: initiative priority and sequencing;
  - work item: problem or outcome, owner, state, priority, and dependencies;
  - bounded plan: execution scope, non-goals, acceptance criteria, and next action;
  - active-lane catalog: concurrent branch and custody projection;
  - review system: review state and findings;
  - Git, tests, deployment readback, and receipts: implementation and completion evidence.
- Give every actionable work item a stable locator, one bounded outcome,
  acceptance evidence, an owner or owning lane, current state, and explicit
  blockers or dependencies. Preserve the locator across plans, branches,
  changes, handoffs, and completion receipts.
- Use a small workflow vocabulary with explicit local mappings, normally
  `TRIAGE`, `READY`, `IN_PROGRESS`, `BLOCKED`, `DONE`, and `CANCELLED`. Do not
  infer implementation or integration from a label, assignee, comment, or
  closed issue alone.
- Keep unrefined ideas in `TRIAGE`. Move an item to `READY` only when its
  outcome and acceptance signal are understandable. Move it to `IN_PROGRESS`
  only when an owner has accepted it and any substantive implementation lane is
  discoverable.
- Set and periodically review an explicit work-in-process limit for substantive
  `IN_PROGRESS` items or active lanes. When the limit is reached, finish,
  unblock, split, cancel, or pause existing work before starting another lane;
  do not raise the limit merely to hide contention.
- Split oversized work into outcome-oriented child items or plans and record
  blocking relationships. Avoid parallel children whose expected write
  surfaces substantially overlap unless reconciliation is part of the plan.
- Link delivery changes to their governing work item. Use automatic closing
  only when integration into the intended target really satisfies the item;
  otherwise use a non-closing reference and close from verified completion
  evidence.
- Close an item only when its acceptance evidence is recorded and its
  implementation, integration, rollout, cleanup, or non-code disposition is
  truthful for the item type. Reopen or create a linked corrective item when
  rollback, regression, or missing evidence invalidates that claim.
- Triage stale, duplicate, blocked, and abandoned items on a documented
  cadence. Merge or link duplicates, retain decision context, and cancel work
  explicitly rather than leaving an indefinitely active shadow backlog.
## Adoption Notes

Use this module when a repository has enough concurrent or deferred work that
chat, branch names, and plan files no longer provide a reliable intake and
priority surface. The contract is tracker-neutral: GitHub Issues and Projects,
another governed tracker, or a repo-local backlog may supply the work-item
locator.

When `active-lane-coordination` is also adopted, record one or more work-item
locators in each substantive lane and enable required work-item validation in
the lane catalog after existing entries are migrated.
