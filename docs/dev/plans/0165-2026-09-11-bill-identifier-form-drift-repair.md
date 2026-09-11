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

The recipe and shared-task-lease repairs are integrated at `5c247f9d` and
installed as generation `0.28.0-c0cc081c1c6b-8e9cb639ccc3`. Focused tests,
the full Authentication Run suite, formatting, and strict Clippy pass. The
retained `bill-soylei` browser profile was relaunched without reset or reseed.

The live run now recognizes the visible `loginEmail` plus `Continue` form and
is durably paused at `awaiting_identifier`, transition 1, with one observation,
zero action receipts, and no pending effect. Its exact resume failed before a
credential action because Auth Vault profile `bill-soylei` does not exist. The
provider returned `effect_uncertain` plus hard stop `blind_retry`, so the live
sequence stopped without retry. Restoring or explicitly seeding that separate
sealed-vault entry is the remaining consumer gate; browser profile continuity
does not prove Auth Vault continuity.

## Consolidated batch

The batch contains two joined consumer repairs: accept the exact tenant-neutral selector
`input#login-email-input[name='loginEmail']` and the identifier control label
`Continue`, while preserving the existing selector and `Save` variants. A
single classifier regression protects both forms. The normal shared-profile
broker path also returns a `shared` task lease beneath the exclusive profile
owner; Authentication Run accepts that service-serialized exact-tab lease as
well as the legacy `exclusive` task lease, while continuing to reject released,
expired, conflicted, and human-takeover leases. No broader login provider or
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
| Compatibility | Focused form and task-lease tests | Existing email-selector plus `Save` remains accepted; shared or exclusive exact-tab task leases work and inactive leases fail closed |
| Source qualification | Authentication-focused tests, format, validation selector, diff check | All required touched-surface checks pass |
| Installed repair | Workstation transaction and exact binary/generation readback | Candidate is selected without profile or browser replacement |
| Consumer outcome | Sealed run receipt and retained URL/title read | Exact BILL organization authenticates in `bill-soylei` |
| Downstream boundary | Read-only Books Receipts case inspection | Result is recorded without BILL or accounting mutation |

`RUNBOOK.md` remains the sole current execution status.
