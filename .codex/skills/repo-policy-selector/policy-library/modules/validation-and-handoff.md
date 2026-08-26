---
id: validation-and-handoff
title: Validation And Handoff
summary: Run relevant verification before commit or handoff and leave concise evidence-backed closeout notes.
tags:
  - validation
  - tests
  - handoff
  - verification
---

## Policy

- Run the relevant validation for the touched surface before commit, handoff, or merge preparation.
- Prefer targeted verification that matches the changed area, and widen to broader suites when the impact is user-visible or cross-cutting.
- Include concrete pass/fail evidence in the handoff or closeout note.
- For off-main work, bind validation to the exact checkpoint SHA and report branch, remote custody, worktree status, target, integration method, and remaining cleanup or archival disposition.
- Keep handoff notes concise, explicit about remaining risk, and clear about the next recommended action.
- When live or manual smoke matters for the changed surface, record whether it was run and what it proved.
- Prefer validation receipts that bind the result to a durable commit, artifact,
  installed version, endpoint response, or other current-state identifier.
  Temporary paths alone are not durable handoff evidence; preserve or publish
  the necessary artifact in a repo-approved location, or record why the proof
  is intentionally ephemeral and how it can be reproduced.
- Distinguish validation run by the primary agent from validation reported by a subagent or delegated worker.
- If validation was delegated, record whether the primary agent independently verified the result or accepted the delegated evidence as-is.
- For failed, timed-out, incomplete, or unknown subagent statuses, state what was trusted, what was ignored, and what remains unverified.
- Use an independent evaluator when fresh judgment materially reduces risk or
  uncertainty, or when an explicit acceptance contract requires it. Duration or
  plan count alone does not make independent review mandatory for routine,
  low-risk work.
- Treat evaluator output as candidate evidence, not an automatic veto. The
  primary agent owns adjudication and records each candidate as `blocking`,
  `nonblocking_backlog`, `rejected`, or `needs_evidence` against the frozen
  objective, acceptance criteria, non-goals, and applicable safety controls.
- A reviewer is not an approver. Work may continue on unaffected in-scope units
  while candidates are adjudicated, and only an accepted blocking finding may
  block the action or criterion it actually affects.
- Require each candidate finding to state the criterion, evidence, consequence,
  reproducer, confidence, and suggested disposition. A useful independent
  review may return no findings; novelty and finding count are not quality
  metrics.
- When both conformance and objective correctness matter, report them as
  separate review axes: one for repository standards and one for the frozen
  specification or acceptance contract. Do not let a pass on one axis mask a
  failure on the other, and do not let the separation bypass primary-agent
  evidence review and disposition.
- Separate review modes. Use at most one broad fresh-context `drift_discovery`
  pass when observed drift, consequence, or uncertainty justifies it. After
  adjudication, use `closed_world` remediation
  verification limited to accepted blocking findings and critical regressions
  introduced by their fixes. Do not reopen broad discovery merely because a
  new evaluator performs final verification.
- Bound review and rework at the goal level, not only per plan version. Prefer
  one consolidated candidate set and one bounded remediation pass; if accepted
  blocking findings still fail verification, split, reframe, or block the unit
  instead of continuing an open-ended evaluator/optimizer loop. Record
  nonblocking concerns in backlog without silently expanding the active plan.
- A review or rework bound ending triggers primary-agent disposition, local
  reframe, or a scoped block. It does not consume goal authority or require user
  approval when another safe in-scope action remains.
- Validate the resulting outcome and current external state, not only the
  transcript, diff shape, test count, or agent's narrative of progress.
- Distinguish `validated`, `integration-ready`, `integrated`, and `cleanup-complete`; none implies the next. Verify target ancestry or a squash/patch receipt before claiming integration, and verify retained refs before removing a worktree or branch.
- Treat fail-closed gates as successful policy execution when they prevent an
  unsafe or disproven change from integrating. Report the blocked outcome and
  evidence instead of grading effectiveness only by shipped changes.

## Adoption Notes

Use this module when the repo:
- has multiple test or smoke surfaces with different scopes
- expects evidence-backed closeout notes
- needs clear verification and residual-risk communication before review or release
