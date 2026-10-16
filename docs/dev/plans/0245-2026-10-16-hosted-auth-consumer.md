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

Integration branch started from fresh main c911bd358 in the reused root; foreign worktrees are preserved. U1, U2 and U2N are validated at their stated boundaries. Producer PR56 is installed as candidate3c97417d/sourceb2480ba. Independent native consumer-mode positive reconciliation and clean failed-login retirement both pass, with pinned receipts and zero owned process residue. The original real QBO request was retired through supported readback only after independent native proof and durable zero password/factor attempts. One separately recorded bounded attempt reached actual email MFA, then became terminal uncertain/capture-held before any password or factor fill. Exact service-environment reproduction identifies the private host's unresolved msgcli executable as exit127; an absolute-path staged host passes the same mailbox lookup and code-format checks under that environment. This candidate is not active and the terminal request is not resumed or replaced. A fresh supported readback returns auth_unavailable: the current exact profile process owns the same native window, but the host pins the earlier PID/start token. Current-owner rebind and terminal private-step continuation remain unsupported in this configured host; do not accept stale ownership or infer replay authority. U3 account/company acceptance remains incomplete, U4 BILL is pending, and U5 final review/publication remains open.

## Integration ledger

| Unit | Blockers | State | Owner and write scope | Required evidence |
| --- | --- | --- | --- | --- |
| U1 version2 client | Producer51 source inspected | validated | coordinator, client/login/helper transport/public tests | red/green distinct identity, wrong binding/scope, source no-effect and no-replay |
| U2 installed helper/dummy | U1 validated | validated | coordinator, host adapter and fixture; private operator config outside repo | normal installed hosted service, independent credential/principal, native privacy, restart/ack and zero residue/peers |
| U2N native staged dummy | shared receiving sequence and native port source checks passed | validated | coordinator, existing installed-host fixture and native receiving module | installed native identifier/password journey, independent company, native safety, canary scans, restart/no-replay and zero owned residue; final receipt native-staged-dummy.json |
| U3 QBO | U2 validated; U2N before protected input; exact owner metadata and grants | running | coordinator, QBO SiteGuide/routine/receiving mapping | existing profile, identifier/password, authorized MFA/native human return, intended company |
| U4 BILL | U3 validated | pending | coordinator, BILL SiteGuide consuming same adapter | existing account/company and actual provider result, no credential duplication |
| U5 docs/review | docs after U1/U2N; final review after U3/U4 | running | coordinator, help/skill/docs, frozen integration diff | selected checks, one Standards/Spec review and one bounded remediation, full acceptance audit |

Note0245 records red/green commands, failed receipts, exact tip/result SHA and runtime handles. Ready means blockers validated on this branch, not tracker closure or worker assertion. Required missing/false/unsafe evidence and real-provider limitations remain incomplete. No new ticket graph or second backlog; these units deliver the existing bounded issue.

## Recovery extension and independent documentation

Public consumer-mode native reconciliation now passes with the frozen hashes in consumer-native-reconciliation.json. Five-surface source guidance and relevant quality/docs gates pass; no shared-skill or Agent Browser CLI publication is claimed. Documentation can progress independently while final review still waits for actual U3/U4. The original real request remains held, no password/factor attempt. Producer Plan0010/PR56 adds clean-failure retirement from durable untouched-input plus independent known-login/native proof. Canonical installation and the independent native negative fixture now pass in native-untouched-login.json. Actual original-request retirement and one separately bounded attempt are recorded in note0245; that attempt is terminal at MFA, with no protected input. The diagnosed mail executable correction is staged and privately qualified, not activated. A configuration correction does not resume a terminal request; preserve its original checkpoint and do not repeat sign-in or resend a challenge. Original checkpoints/grants/history remain preserved; no replay of a retired or unresolved request and no new auth extraction. Goal limits and full acceptance are unchanged.

## Seams and validation

Public createLoginPlaybook.advance proves request/checkpoint composition but not native privacy. Installed helper SDK/MCP plus existing browser workflow proves actual transport/private receiving; independent dummy website proves password/account. Actual existing-profile QBO then BILL prove intended company and supported real states. Tests cover two different task IDs, wrong company, stale prepared controls, owned popup, human same-request continuation and unknown-ack no-replay. Use existing runners, red then minimal green per slice; do not replace required real outcomes with synthetic checks.

## Custody

One integration branch in the existing clean root, no new worktree. Coordinator owns shared login/helper contract and serial integration. Read-only qbo_bill_metadata290 owns no source, branch or runtime effects. Integrated P242/P237 behavior is consumed from main; foreign checkout and catalog records preserved. Relevant governance planning/implementation/runtime packets retained; no conflicts or new approval requirement manufactured. Private account/source IDs, credentials, factors, cookies and bodies stay outside product files. Formal release, production replacement and broader auth extraction are not inferred from this plan.
