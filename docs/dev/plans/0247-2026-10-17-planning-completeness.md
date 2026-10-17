# Plan 0247: Selected planning completeness

Status: OPEN
Product lane: PL-BUGFIX
Work item: CochranResearchGroup/agent-browser#299
Owner: ecochran76
Branch: fix/planning-completeness
Base: 0cf8e5bc8d057eb8b5856dcfceb856bf6b19c1a3

## Current State

The active audit passes while excluding current Status: COMPLETED plans. Existing
Git custody and archival audits do not check plan/note discoverability.

## Outcome and scope

Extend the existing planning auditor with explicit catalog lane selection. For
selected active or completed outcomes, require plan links in startup authorities,
matching product lane and accountable ownership, and corresponding evidence-note
metadata/index links. Closed/cancelled records need no current execution links.
Unknown lane/state and missing selected artifacts must fail visibly. Do not infer
execution authority from completion or migrate unrelated historical records.

## Execution and acceptance

One serial owner edits auditor, disposable fixtures and developer guidance. P246
owns its validation runner and P240 owns resource code; neither is edited here.
No delegated track is needed. Reproduce missing-link/ownership failures, implement
one slice at a time, then run auditor fixtures and selected local checks. The
accepted P245 structure must pass. Review Standards and Spec against the base.
No runtime, CI, login, installation or other-repository effects.

## Terminal condition

Selected P245 and this packet pass, negative fixtures catch omissions, existing
planning tests remain green, and a reviewable source change with validation is
published through the repository workflow.
