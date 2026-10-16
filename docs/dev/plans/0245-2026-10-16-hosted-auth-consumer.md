# Plan 0245: Existing-host QBO and BILL authentication

Status: IN_PROGRESS
Product lane: PL-AUTH
Work item: CochranResearchGroup/agent-browser#290
Owner: ecochran76
Branch: auth/shared-helper-290
Base: c911bd358fe0c43f3342418bfde1b8dd2b3e72b4
Spec: Issue290 current body; installed producer auth-external.md version2

## Objective and bounds

Deliver all nine issue290 acceptance items: installed independent dummy journey, then actual existing-profile SoyLei QBO and BILL account/company verification. Ordinary caller identity and exact receiving task/profile/browser/tab remain distinct. No standalone service, duplicate credential logic, vault enumeration, guessed references, accounting writes, profile reset, unsolicited enrollment/saving or new viewer/authority ceremony. Source, installed/runtime and real acceptance remain separate.

Goal began2026-10-16T14:20:50Z; stop/checkpoint before2000000 cumulative tokens or20:20:50Z. Safety checkpoint1850000tokens or20:05:50Z. Successors/retries do not reset limits.

## Current State

Integration branch started from fresh main c911bd358 in reused clean root; foreign worktrees are preserved. U1, U2 and U2N are validated at their stated boundaries. U3 has now used the actual installed helper and existing protected source: one source read and one identifier fill succeeded. The request ended uncertain/capture-held at identifier submission, without password or MFA input. Read-only owner metadata identified a LastPass extension popup covering the submit control. One exact-owner native Escape dismissed that popup and restored the submit hit test without submitting the form. The original request remains uncertain; the installed producer has no supported uncertain-request reconciliation operation. Preserve that request and privacy hold while repairing continuation rather than replaying authentication or editing its store. The temporary real host configuration remains installed with its original-byte rollback preserved. QBO principal/company, BILL and final documentation/review remain incomplete.

## Integration ledger

| Unit | Blockers | State | Owner and write scope | Required evidence |
| --- | --- | --- | --- | --- |
| U1 version2 client | Producer51 source inspected | validated | coordinator, client/login/helper transport/public tests | red/green distinct identity, wrong binding/scope, source no-effect and no-replay |
| U2 installed helper/dummy | U1 validated | validated | coordinator, host adapter and fixture; private operator config outside repo | normal installed hosted service, independent credential/principal, native privacy, restart/ack and zero residue/peers |
| U2N native staged dummy | shared receiving sequence and native port source checks passed | validated | coordinator, existing installed-host fixture and native receiving module | installed native identifier/password journey, independent company, native safety, canary scans, restart/no-replay and zero owned residue; final receipt native-staged-dummy.json |
| U3 QBO | U2 validated; U2N before protected input; exact owner metadata and grants | running | coordinator, QBO SiteGuide/routine/receiving mapping | existing profile, identifier/password, authorized MFA/native human return, intended company |
| U4 BILL | U3 validated | pending | coordinator, BILL SiteGuide consuming same adapter | existing account/company and actual provider result, no credential duplication |
| U5 docs/review | U1-U4 validated | pending | coordinator, help/skill/docs, frozen integration diff | selected checks, one Standards/Spec review and one bounded remediation, full acceptance audit |

Note0245 records red/green commands, failed receipts, exact tip/result SHA and runtime handles. Ready means blockers validated on this branch, not tracker closure or worker assertion. Required missing/false/unsafe evidence and real-provider limitations remain incomplete. No new ticket graph or second backlog; these units deliver the existing bounded issue.

## Seams and validation

Public createLoginPlaybook.advance proves request/checkpoint composition but not native privacy. Installed helper SDK/MCP plus existing browser workflow proves actual transport/private receiving; independent dummy website proves password/account. Actual existing-profile QBO then BILL prove intended company and supported real states. Tests cover two different task IDs, wrong company, stale prepared controls, owned popup, human same-request continuation and unknown-ack no-replay. Use existing runners, red then minimal green per slice; do not replace required real outcomes with synthetic checks.

## Custody

One integration branch in the existing clean root, no new worktree. Coordinator owns shared login/helper contract and serial integration. Read-only qbo_bill_metadata290 owns no source, branch or runtime effects. Integrated P242/P237 behavior is consumed from main; foreign checkout and catalog records preserved. Relevant governance planning/implementation/runtime packets retained; no conflicts or new approval requirement manufactured. Private account/source IDs, credentials, factors, cookies and bodies stay outside product files. Formal release, production replacement and broader auth extraction are not inferred from this plan.
