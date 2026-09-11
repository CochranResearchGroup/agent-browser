# Plan 0165: BILL Identifier Form Drift Repair

Date: 2026-09-11

State: OPEN

Consolidation: required

Lane: sealed authentication consumer repair

Branch: `fix/bill-login-email-continue`

Target: `main`

Integration: short-lived branch, then fast-forward or reviewed merge to `main`

Parent: Books Receipts Plan 0240 consumer acceptance

## Objective and authority

Update the repository-owned `bill-login-v1` recipe for the observed BILL
identifier form, install one qualified production candidate through the
canonical workstation transaction, and prove one sealed reauthentication on
the retained `bill-soylei` profile. The operator authorized this packet.

The packet must preserve the profile directory, authenticated browser, tabs,
cookies, credentials, and unrelated runtime owners. It does not authorize a
profile reset, transaction mutation, receipt upload, review, QuickBooks sync,
or accounting effect.

## Current state

The installed recipe classifies the visible `loginEmail` field with a
`Continue` control as `unsupported_challenge`. Two prior bounded runs stopped
before credential or challenge-provider effects. The retained browser and
profile remain available on the installed generation recorded in `RUNBOOK.md`.

## Consolidated batch

The batch contains one repair: accept the exact tenant-neutral selector
`input#login-email-input[name='loginEmail']` and the identifier control label
`Continue`, while preserving the existing selector and `Save` variants. A
single classifier regression protects both forms. No broader login provider or
website interaction feature is included.

## Delivery sequence and budget

1. Produce focused red and green classifier evidence.
2. Run authentication-focused tests, formatting, the changed-surface selector,
   and diff checks against the frozen source.
3. Commit and integrate the repair, build one production candidate, and apply
   it once through `install workstation`.
4. Verify installed identity and retained profile/browser continuity, then run
   one sealed BILL reauthentication and a read-only consumer inspection.

Overall active-work ceiling is 45 minutes. The batch permits one source-frozen
production build, one workstation install transaction, one sealed
reauthentication run, and one repair cycle if a source defect is demonstrated.
Any profile, owner, generation, or transaction mismatch stops live effects.

## Worker assignments

The primary owns source, validation, integration, installation, and live
read-only acceptance. No worker is assigned because the patch and retained
runtime custody are one serialized critical path.

## Evidence and exit

| Requirement | Evidence | Exit condition |
| --- | --- | --- |
| Current form support | Focused red then green classifier test | Exact `loginEmail` plus `Continue` classifies as identifier form |
| Compatibility | Same focused test | Existing email-selector plus `Save` form remains accepted |
| Source qualification | Authentication-focused tests, format, validation selector, diff check | All required touched-surface checks pass |
| Installed repair | Workstation transaction and exact binary/generation readback | Candidate is selected without profile or browser replacement |
| Consumer outcome | Sealed run receipt and retained URL/title read | Exact BILL organization authenticates in `bill-soylei` |
| Downstream boundary | Read-only Books Receipts case inspection | Result is recorded without BILL or accounting mutation |

`RUNBOOK.md` remains the sole current execution status.
