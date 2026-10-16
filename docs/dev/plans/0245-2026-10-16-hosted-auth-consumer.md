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

Integration branch started from fresh main c911bd358 in reused clean root; foreign worktrees are preserved. U1 separates native caller from receiving browser task, and U2 passed through the actual installed hosted service. The restored host is healthy; its ordinary external receiver remains unconfigured. U3 has reached the real identifier-first email challenge and established exact nonsecret LastPass record references in private operator evidence. The matching modern record is a passkey; a separate matching password record has an old recovery-path URL and is not yet credential-qualified. No password or verification code has been retrieved or submitted. The installed fresh-form receiver assumes simultaneous username/password controls and a POST form; observed QBO needs multiple states and native activation. These are implementation gaps, not completed real acceptance. Disk recovery and earlier failed receipts remain in note0245.

## Integration ledger

| Unit | Blockers | State | Owner and write scope | Required evidence |
| --- | --- | --- | --- | --- |
| U1 version2 client | Producer51 source inspected | validated | coordinator, client/login/helper transport/public tests | red/green distinct identity, wrong binding/scope, source no-effect and no-replay |
| U2 installed helper/dummy | U1 validated | validated | coordinator, host adapter and fixture; private operator config outside repo | normal installed hosted service, independent credential/principal, native privacy, restart/ack and zero residue/peers |
| U3 QBO | U2 validated; exact owner metadata and grants | running | coordinator, QBO SiteGuide/routine/receiving mapping | existing profile, identifier/password, authorized MFA/native human return, intended company |
| U4 BILL | U3 validated | pending | coordinator, BILL SiteGuide consuming same adapter | existing account/company and actual provider result, no credential duplication |
| U5 docs/review | U1-U4 validated | pending | coordinator, help/skill/docs, frozen integration diff | selected checks, one Standards/Spec review and one bounded remediation, full acceptance audit |

Note0245 records red/green commands, failed receipts, exact tip/result SHA and runtime handles. Ready means blockers validated on this branch, not tracker closure or worker assertion. Required missing/false/unsafe evidence and real-provider limitations remain incomplete. No new ticket graph or second backlog; these units deliver the existing bounded issue.

## Seams and validation

Public createLoginPlaybook.advance proves request/checkpoint composition but not native privacy. Installed helper SDK/MCP plus existing browser workflow proves actual transport/private receiving; independent dummy website proves password/account. Actual existing-profile QBO then BILL prove intended company and supported real states. Tests cover two different task IDs, wrong company, stale prepared controls, owned popup, human same-request continuation and unknown-ack no-replay. Use existing runners, red then minimal green per slice; do not replace required real outcomes with synthetic checks.

## Custody

One integration branch in the existing clean root, no new worktree. Coordinator owns shared login/helper contract and serial integration. Read-only qbo_bill_metadata290 owns no source, branch or runtime effects. Integrated P242/P237 behavior is consumed from main; foreign checkout and catalog records preserved. Relevant governance planning/implementation/runtime packets retained; no conflicts or new approval requirement manufactured. Private account/source IDs, credentials, factors, cookies and bodies stay outside product files. Formal release, production replacement and broader auth extraction are not inferred from this plan.
