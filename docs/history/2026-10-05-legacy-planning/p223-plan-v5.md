# Plan 0223 | Finish Agent Browser Acceptance
Date: 2026-10-04
Plan version: 5
State: OPEN
Consolidation: required
Product lane: PL-PLATFORM
Lane: P222
Predecessor: P222, CLOSED by supersession while incomplete
Work item: [agent-browser#202](https://github.com/CochranResearchGroup/agent-browser/issues/202)
Branch: platform/p220-remote-view-consumer
Target: main
Owner: primary Agent Browser owner
## Current State
Milestones 1, 2 and 3 are accepted within the evidence recorded in [RUNBOOK](../../../RUNBOOK.md). The installed native handoff restores the retained browser, and AuraCall's unchanged launcher survives immediate isolated service restart with the same profile, browser and handoff. Authenticated reads match the account fingerprint; the native viewer shows the browser with active control. Testing remains in p221.
Shared operation now includes unchanged Odollo, separate tabs/sessions, two profiles on the native desktop, duplicate-free replay and peer-preserving normal cleanup. Finite link retention and extension now have installed evidence, including expiry rejection with native viewer and automation continuity. Later cross-client idle recovery selected the wrong browser executable for the actual default profile. The installed per-profile repair now passed cross-client recovery before and after runtime-host restart, authenticated continuation and peer-preserving task cleanup; RUNBOOK records the bounded evidence. The remaining recovery/capacity work and reproducible delivery remain open. Main integration remains open. The synthetic p221-auracall profile is historical evidence, not the authentication acceptance profile.
AuraCall's existing launcher and browser profiles may be used; no AuraCall changes, integration or service adoption are required.

## Consolidated batch
Finish Agent Browser launch, navigation, authenticated durable handoff, every supported Remote View control, continued automation and explicit end-of-work cleanup using those existing profiles.
Preserve existing accounts and authenticated profiles. Use the actual AuraCall default-profile handoff for login persistence; retain earlier handoffs as evidence. Current operator authentication is available, so do not request new credentials to continue.
Operator clarification: the handoff is an authenticated deep link directly to the Remote View desktop. No dashboard workspace, duplicate viewer or extra Open desktop click. Reuse Remote View qualification within its proven scope. Prefer a chooser restricted to Agent Browser-owned desktops; the native viewer uses Remote View operator visibility rules.
Operator correction: remove the extra expiring Agent Browser viewer-grant requirement. Desktop handoffs use Remote View native authenticated desktop routes, joined to current Agent Browser desktop ownership, independent of the original physical tab. Existing Remote View authentication, including production Authelia, remains in place. No additional consumer viewer-grant issuance, renewal or expiry is an acceptance requirement.
Demonstrate the complete workflow through normal Agent Browser interfaces, then prove owned-resource release and peer preservation with a fresh OS census.

## Delivery sequence and budget
1. Preserve the qualified full-controls and explicit-cleanup results. Fix opening the durable handoff so it recovers an idle-cleaned retained browser and shows the browser on the desktop; verify continued automation and peer preservation.
2. Prove reviewed provider login and authenticated operation survive client and service restarts on the same profile and handoff.
3. Demonstrate a second existing client, shared sessions, separate tabs, multiple profiles sharing a desktop, duplicate-free replay and peer-preserving closure.
4. Implement finite handoff-link retention and requester-selected TTL extensions; expose expiry and prove extension, link expiry and rejection of an expired handoff without changing browser or viewer lifetimes or adding a viewer-grant requirement.
5. Complete interrupted-operation recovery, desktop loss, operator stop, safe capacity growth/shrink and usable mobile viewing/input. Preserve normal Remote View authentication; extra Agent Browser viewer-grant revocation/expiry tests are superseded by the operator correction.
6. Qualify clean install, update, diagnostic doctor, first-install sudo exactly once, required platforms and changed-surface gates; integrate Agent Browser and Remote View through protected PRs.
Inherit cumulative effort and failures; stop and checkpoint at one million additional tokens from the current resumed goal baseline, superseding the prior 750000 cumulative ceiling. A successor grants no fresh allowance.

## Worker assignments
One primary owner; retain p220 and rv011 custody. Agent Browser owns browsers/sessions/profiles; Remote View owns viewer/provider repairs.
Serialize shared runtime effects. Production promotion, formal release and unrelated architecture are excluded; preserve profiles and peers.

## Evidence and exit
Record each installed action, observed effect, source/binary identity, receipt and remaining failure in RUNBOOK.
Reuse valid narrow passes; fixtures, builds, publication, CDP disconnect and idle cleanup cannot substitute for outstanding workflow outcomes.
After 30 minutes or two checkpoints without an outcome or verified blocker removal, change the failing tactic. Close only when every requirement above is proved and protected integration is complete.

## Revision 5 reconciliation
This revision removes stale authentication and consumer viewer-grant prerequisites, distinguishes the actual AuraCall default profile from the synthetic test profile, and records the empty-desktop defect without discarding valid narrower passes. Subsequent installed evidence accepted milestones 1, 2 and 3; RUNBOOK owns that current acceptance and the remaining work. Five acceptance milestones remain; delivery steps 4 and 5 both belong to recovery and capacity. It preserves the remaining delivery scope and inherited failures. The current ceiling follows the renewed user direction for one million additional tokens. The pre-reconciliation execution record is archived beside RUNBOOK.md.

Revision 5 preserves earlier scenario-specific passes and records the later cross-client recovery counterexample. Source tests do not establish installed recovery. Stale archive instructions cannot reintroduce consumer viewer grants or login prerequisites.
