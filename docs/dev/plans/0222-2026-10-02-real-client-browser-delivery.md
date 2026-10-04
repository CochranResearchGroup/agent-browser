# Plan 0222 | Deliver Usable Browser Workflows Through Existing Clients

Date: 2026-10-02

Plan version: 4

State: OPEN

Consolidation: required

Product lane: PL-PLATFORM

Lane: P222

Predecessor: P221, CLOSED by supersession while incomplete

Work item: [CochranResearchGroup/agent-browser#202](https://github.com/CochranResearchGroup/agent-browser/issues/202)

Branch: platform/p220-remote-view-consumer (retain existing source custody)

Target: main

Owner: primary browser integration owner

## Objective and scope

Deliver usable browser automation and operator viewing through existing clients,
starting with AuraCall using Chromium Stealth. Then deliver persistent login,
shared sessions, recovery and reproducible installation. Preserve P221's full
many-to-many browser and desktop objective. Each milestone must enable a user
workflow demonstrated through its normal installed interface.

Production promotion, formal release, unrelated architecture, IAM redesign and
speculative detection-countermeasure research are outside this delivery scope.
Browser executable selection and navigator.webdriver=false do not establish
site acceptance or detection bypass.

## Current State

P221 was closed by supersession while incomplete. Every unproved requirement
remains in this plan. None of the five milestones is fully accepted.
[RUNBOOK](../../../RUNBOOK.md) owns current results, evidence, installed identity,
remaining failures and spending authority. Experimental validation targets p221.
Historical failures and passing observations retain their original scope.

## Consolidated batch

Finish the complete AuraCall workflow: launch the requested Stealth browser on
the intended profile, navigate, return the authenticated durable handoff, use
Remote View's full supported controls on the addressed tab, continue automation,
and explicitly finish work through the real client. Confirm that closure releases
owned resources and preserves any peers with a fresh process and resource census.

The next action is to reproduce the reported missing Remote View controls through
an Agent Browser handoff, identify the supported controls absent from that path,
and restore them. Verify each affected control in the installed p221 handoff and
finish the AuraCall continuation and explicit close. Basic pixels and keyboard
input alone do not establish the full controls outcome.

Agent Browser owns profiles, browsers and tabs; Remote View owns desktops and
viewing. Change the existing owning path only as needed to deliver this workflow.

## Delivery sequence and budget

| Milestone | Demonstrated user outcome | Exit evidence |
| --- | --- | --- |
| 1. First usable client | AuraCall launches requested Stealth, navigates, returns a durable authenticated handoff, operator uses the full supported Remote View controls on the addressed tab, automation continues and explicit end-of-work close releases owned resources. | Complete normal-client workflow, each supported handoff control, executable identity and fresh process/resource census after explicit close. |
| 2. Persistent authentication | Reviewed manual login remains usable on subsequent automation and client/service restart; the same handoff reconnects. | Authenticated page operation before and after restart without exposing credentials or private page contents in tracked evidence. |
| 3. Shared operation | A second existing client and AuraCall operate separate tabs; multiple profile browsers share a desktop; one session closing preserves peers; final close releases resources. | Real client requests and responses, tab/browser identity, stable-key replay without duplicate creation and fresh cleanup census. |
| 4. Recovery and capacity | Browser, provider and viewer interruption restore the workflow or produce an actionable bounded failure; finite handoff retention and requester-selected TTL extensions work; grant expiry/revocation and operator stop behave correctly; demand grows and empty capacity shrinks. | Separate observed disruptions, preserved logical identities and durable URL, no duplicate effects or unexplained retained resources. Desktop/mobile viewing and focus are included. |
| 5. Reproducible installation | Clean development install and update run the same client workflows; doctor diagnoses broken dependencies; required supported-platform and changed-surface gates pass. | Source-bound artifacts, first-install sudo exactly once, update/doctor observations and protected integration readiness. Formal release remains separately directed. |

Finish milestone 1 before starting later milestone implementation. Each next
packet names the user action that fails, expected behavior, existing owner,
smallest repair and installed observation that will decide whether it works.
Missing inputs remain explicit; continue only work that does not depend on them.

RUNBOOK owns the current spending allowance and stop instructions. Preserve
cumulative effort and no-progress history across revisions and successors.
Stop when all required outcomes are delivered, even if allowance remains.

A build, publication, green test, commit, document or planning checkpoint cannot
advance a user-workflow milestone by itself. A verified blocker removal is an
intermediate result; acceptance requires the corresponding installed workflow.
Before another expensive cycle, identify the unresolved failure it will resolve
and the observation that will distinguish success from failure. If the previous
cycle established neither working behavior nor a causal blocker, change the
approach before repeating it. Reuse valid evidence for unchanged behavior and
run the required checks for affected surfaces.

## Recovery and lifetime requirements

Reopening the same retained handoff restores the intended profile and recorded
tabs after positively established browser loss. Logical browser, session, tab
and handoff identities survive; replacement process and transport identities may
change. Preserve history and prevent duplicate launches. Healthy or occupied
browsers are not relaunched, and explicitly closed targets remain closed.
Acceptance includes same-link viewing, control and subsequent real-client use.

The existing five-minute idle rule releases the physical browser without erasing
an eligible retained handoff. Active peers or outstanding operations prevent
premature closure. Explicit end-of-work closure is a separate required outcome;
automatic idle cleanup does not substitute for it.

Implement and demonstrate finite handoff retention and requester-selected TTL
extensions through the normal interface. State the effective expiry to the
requester and demonstrate extension, expiry and rejection of further access.
Browser idle lifetime, viewer-grant lifetime and handoff retention must each
behave as requested without silently extending another lifetime. Renewal and
recovery observations do not accept indefinite retention.

Recovery acceptance also covers interrupted operations, service/provider restart,
desktop loss, revocation and operator stop. Show preserved peer work, bounded
failure where recovery is impossible, and no duplicate effects or unexplained
resources. Demonstrate demand-driven capacity growth, safe shrink after demand
ends, and usable desktop and mobile controls.

## Worker assignments

One primary owner retains the existing Agent Browser worktree and coordinates
necessary fixes in the existing Remote View custody. The client-to-viewing path
is serialized. Documentation and installation work may become independent only
after the actual interface works; no parallel agents or extra worktrees are
required. Declare any shared-source writer and dependency before overlap.
Use focused checks for the affected behavior and complete required integration
gates before claiming delivery readiness.

## Evidence and exit

RUNBOOK owns one current milestone results table, with PASS, PARTIAL, FAIL or NOT RUN,
source/binary identity, actual client and observation, evidence locator, and the
next exact blocker. Each
checkpoint states what the user can now do, the actual installed observation,
and the remaining acceptance gap. Preserve failures alongside subsequent passes.
Implementation status and acceptance status remain separate.

Completion requires all five milestones, carrying forward every unproved P221
acceptance axis. Partial CLI success does not complete a real-client milestone.
No hidden resource residue, silent profile substitution, counterfeit readiness
or weakened pass conditions are allowed. Installation qualification and protected
integration must be complete before declaring the candidate ready; publication
or formal release still requires its standing separate direction.

Existing reviewed isolated development-runtime and installed-effect boundaries
from [P220](0220-2026-09-28-remote-view-consumer-integration.md#separately-gated-installed-acceptance-packet)
carry forward. Preserve pending effect keys and attributable evidence. No
production runtime, profile, provider or ingress replacement is authorized by
this plan.
