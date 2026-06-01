# Retained Orphan Profile Cleanup Plan

Date: 2026-05-30
State: CLOSED
Lane: P10
Outcome: PASSED

## Purpose

Reduce retained service-state noise caused by one-off custom profile paths
without deleting reusable runtime profiles or live browser evidence.

The audit found that the large count is persisted service state, not tracked
repository data. The risky set is `custom:*` profile records derived from
temporary `--profile` paths whose user data directories are already gone and
whose IDs are not referenced by retained sessions, browsers, jobs, events, or
profile handoffs.

## Non-Goals

- Do not delete physical browser profile directories.
- Do not prune named runtime profiles such as `default`, `google-login`, or
  `stealthcdp-default`.
- Do not make process-exited browser cleanup implicit.
- Do not hide retained-state history without a dry-run review step.

## Slices

### Slice A | Backend Cleanup Contract

Add an opt-in `orphanedProfiles` retained-prune option and CLI
`--orphaned-profiles` flag.

Exit criteria:

- Dry-run reports orphaned profile candidates and counts.
- Apply removes only reviewed profile records from service state.
- Existing prune defaults remain conservative.

### Slice B | Dashboard Operator Flow

Expose orphaned-profile cleanup through the Service dashboard retained-state
warning.

Exit criteria:

- Dry-run includes orphaned-profile candidates.
- Summary copy includes profile candidates and removals.
- Apply remains disabled until a dry-run result is visible.

### Slice C | Docs, Contracts, And Validation

Update user-facing docs, generated client types, command help, and focused
tests for the changed service request contract.

Exit criteria:

- Rust command/parser and prune tests pass.
- Dashboard static smoke covering the retained cleanup controls passes.
- Generated client and docs mention orphaned profile cleanup.

## Result

Implemented an opt-in `--orphaned-profiles` retained-prune path for `custom:*`
profile records whose ephemeral user-data directories are missing and whose
profile IDs are not referenced by retained service records.

The Service dashboard retained-state flow now sends `orphanedProfiles: true`
for dry-run and apply requests, shows orphaned profile candidate counts, and
keeps apply gated on a visible dry-run result.

Rendered dashboard QA confirmed the updated summary copy is visible. The live
installed service used for that QA did not yet include the new backend field,
so it reported `0 orphaned profiles`; the current source binary dry-run reports
the new candidate set before installation or service replacement.
