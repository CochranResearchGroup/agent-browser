# Plan 0223 | Complete Dependable Browser Workflows

Date: 2026-10-04
Updated: 2026-10-06
Plan version: 6
State: COMPLETED
Execution: COMPLETED on 2026-10-06 within the explicit resumed bound
Product lane: PL-PLATFORM
Lane: P222
Work item: [agent-browser#202](https://github.com/CochranResearchGroup/agent-browser/issues/202)
Implementation branch: platform/p220-remote-view-consumer
Target: main
Owner: @ecochran76

## Objective

Complete real browser work with less client code, less agent context and fewer failures: launch, automate, hand the same browser to a person through native Remote View, continue automation, and clean up predictably. [The product contract](../policies/0054-browser-product-contract.md) governs this plan. [RUNBOOK](../../../RUNBOOK.md) owns execution status and evidence.

Use the profile appropriate to the task: an existing client profile, a managed persistent profile or a disposable profile. Existing client profiles are not a universal prerequisite. Preserve authentication when the workflow needs it; disposable work need not acquire or retain a login.

## Workflow and ownership

Agent Browser owns browser operation, profile/session continuity and the consumer integration. Remote View owns desktop operation and the native viewer. Give the operator a direct desktop handoff through the provider's existing authentication, including production Authelia. Human interaction and automation continue in the same browser. Prefer a chooser showing Agent Browser-owned desktops.

No copied Remote View service, duplicate viewer, dashboard detour, extra Open desktop step or separate Agent Browser viewer-grant ceremony. Essential implementation constraints must produce clear, actionable failures; they must not become permanent product rituals. Reuse Remote View qualification within its proven scope.

AuraCall's default profile and unchanged Odollo adapter remain useful existing-client scenarios. They do not require client migration or make those profiles mandatory for all browser work.

## Acceptance

| Milestone | Concrete outcome |
| --- | --- |
| M1. Complete browser work | Normal interfaces launch the appropriate browser, perform useful navigation/interaction, open a responsive native desktop, return to automation and close task-owned resources without disrupting peers. |
| M2. Useful continuity | A persistent-profile workflow retains the required authentication and usable browser through the supported restart/reconnect path. Disposable work closes predictably. Reuse qualified profile scenarios; add a new case only for a material uncovered behavior. |
| M3. Existing clients and sharing | Unchanged clients complete their workflows. Separate tasks/tabs coexist; retries do not create duplicate work; closing one task preserves others. |
| M4. Workflow recovery | The retained handoff restores the intended browser after the already-observed interruption. Supported reconnect/restart and explicit operator stop produce continued work or an actionable result, with peer-preserving cleanup. |
| M5. Reproducible use | A clean supported installation/update can run that workflow and report actionable readiness/failures. First-install privilege setup is clear and bounded; routine browser work does not demand repeated setup. Qualify touched surfaces and land the implementation through protected PRs. |

Preserve prior M1–M3 acceptance only within its recorded scenarios. A contract rewrite adds no installed or platform proof. Keep failures and counterexamples attributable. Do not replace real browser, viewer and cleanup observations with fixture, build or publication success. Avoid exhaustive combinations when qualified provider or client evidence already establishes the relevant behavior.

## Executed bounded packet

Use the existing isolated p221 acceptance environment; production publication is excluded. Resolve the retained original-handoff recovery blocker using the already-prepared repair. Verify the original link shows the intended browser, controls respond, normal automation continues, and task cleanup preserves peers. Match the installed source and binary to that observation. Reuse unaffected passes; do not replay the entire acceptance program.

Then address only demonstrated M4/M5 gaps. Each packet names one concrete workflow failure, its expected improvement and the cheapest meaningful checks. An unsuccessful packet records the counterexample and a narrower next action rather than starting another broad redesign or acceptance loop.

## Acceptance readback

Revision 6 is complete within its bounded recorded scenarios. [RUNBOOK](../../../RUNBOOK.md) retains the source, installed evidence and counterexamples. Consumer implementation entered canonical main through [PR 212](https://github.com/CochranResearchGroup/agent-browser/pull/212), merge `85c6fa22e67f9767578110b83584f5573b3c4cfb`; the provider dependency entered through [Remote View PR 310](https://github.com/CochranResearchGroup/remote-view/pull/310).

M1–M3 retain their previously qualified existing-client scenarios. M4 adds valid retained-link recovery, ordinary automation after natural physical idle cleanup, visible native keyboard input, continued automation, peer-preserving task stop and actionable expired/closed-link results. The original link expired during the pause and its restoration packet remains failed. A normal post-expiry handoff reused the original logical browser, session and tab. One first ordinary recovery attempt refused unproven absence without launching; fresh positive evidence preceded its successful retry, and a second natural idle cycle recovered on its first ordinary command. Its unproven transient cause remains a limit on reliability claims.

M5 qualifies isolated p221 update, selected-install diagnostics, disposable launch/read/close/residue cycles and canonical source integration. The installed binary's 864 recorded compile inputs match the merged consumer tree. Canonical CI and Lease Authority workflows are disabled; no GitHub required main-branch checks are configured. Relevant local gates passed, with corrected fixture passes and original failures retained separately.

Production promotion, a formal release and a broad platform matrix remain outside this acceptance. Current application authentication is unverified at its interstitial; this closeout preserves earlier M2 qualification within its recorded scope and adds no new application-authentication claim. It does not resume unrelated paused plans or authorize cleanup of retained dirty checkouts.

## Follow-ups outside this finish line

Finite handoff TTL/extension already has narrow evidence; it is not a new viewer-access requirement. Elastic desktop growth/shrink, a comprehensive mobile-input matrix, every provider control combination and a broad platform/release campaign are separate follow-ups when an actual supported workflow needs them. This revision removes them as unconditional gates for this bounded browser-workflow outcome. Existing evidence and implementation remain preserved.

Production promotion, a formal release, unrelated architecture and changes to client/provider authentication are excluded. Existing browser/profile data, peer work and provider authentication must remain intact.

## Pause and effort

Re-anchoring alone grants no execution authority. The operator explicitly resumed execution on 2026-10-05 with a checkpoint before 1.5 million tokens in the resumed thread. Preserve the prior reached ceiling, cumulative effort and failures. This renewed bound comes from that direction, not from revision 6.

## Revision provenance

[Revision 5](../../history/2026-10-05-legacy-planning/p223-plan-v5.md) is preserved verbatim as historical evidence. Revision 6 replaces its broad mechanism-driven finish line with concrete product outcomes, retains qualified narrow acceptance and the original pause, and records removed gates as follow-ups. Historical instructions cannot reintroduce viewer grants, universal existing-profile requirements, or authority to restart work.
