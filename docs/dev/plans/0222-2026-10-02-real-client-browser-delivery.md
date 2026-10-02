# Plan 0222 | Deliver Usable Browser Workflows Through Existing Clients

Date: 2026-10-02

Plan version: 1

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

RUNBOOK Turn 437 binds source 80f9316e, installed candidate identity and partial
workflow evidence. Requested stock and Stealth executable selection, navigation,
clicking, JavaScript, tab creation, shared sessions and final browser cleanup
were demonstrated through the installed CLI. Actual AuraCall acceptance still
fails because operator viewing remains converging. Full authenticated-client,
reconnect, recovery, capacity and installation acceptance are incomplete.

## Consolidated batch

The first batch has one outcome: AuraCall opens the requested Stealth build on
the intended development profile, returns its durable authenticated handoff,
the operator sees and controls the addressed tab, automation continues, and
closing work releases its owned browser resources. Fix the owning grant,
readback, readiness and client integration code together when they share this
failure. Captured-time validation and provider-scoped versus local grant-key
comparison are diagnoses to verify, not accepted fixes.

Preserve the existing division: Agent Browser owns profiles, browsers and tabs;
Remote View owns desktops and viewing. Reuse or simplify existing machinery
according to the observed failure. Do not create another contract framework,
simulator or independent test project to stand in for the client result.

## Delivery sequence and budget

| Milestone | Demonstrated user outcome | Exit evidence |
| --- | --- | --- |
| 1. First usable client | AuraCall launches requested Stealth, navigates, returns a durable authenticated handoff, operator sees and controls the addressed tab, automation continues and close works. | Actual client success plus visible/control observation, executable identity and fresh owned-resource inventory. |
| 2. Persistent authentication | Reviewed manual login remains usable on subsequent automation and client/service restart; the same handoff reconnects. | Authenticated page operation before and after restart without exposing credentials or private page contents in tracked evidence. |
| 3. Shared operation | A second existing client and AuraCall operate separate tabs; multiple profile browsers share a desktop; one session closing preserves peers; final close releases resources. | Real client requests and responses, tab/browser identity, stable-key replay without duplicate creation and fresh cleanup census. |
| 4. Recovery and capacity | Browser, provider and viewer interruption restore the workflow or produce an actionable bounded failure; grant expiry/revocation and operator stop behave correctly; demand grows and empty capacity shrinks. | Separate observed disruptions, preserved logical identities and durable URL, no duplicate effects or unexplained retained resources. Desktop/mobile viewing and focus are included. |
| 5. Reproducible installation | Clean development install and update run the same client workflows; doctor diagnoses broken dependencies; required supported-platform and changed-surface gates pass. | Source-bound artifacts, first-install sudo exactly once, update/doctor observations and protected integration readiness. Formal release remains separately directed. |

Finish milestone 1 before broadening implementation. Derive only the next
bounded packet from a real failure: outcome, owner, exact write surface, inputs,
focused checks and installed demonstration. Commit a coherent working increment
before moving on. Record unavailable inputs and continue independent work within
scope; do not fabricate profile authentication or operator visibility.

The operator authorized execution on 2026-10-02 with a stop and checkpoint
before the active goal meter reaches 750000 tokens. This resumes execution;
750000 is a ceiling, not a spending target. Any subsequently authorized execution inherits cumulative effort,
accepted findings and no-progress history; successors do not reset them. Stop
when the intended product is delivered even if budget remains.

During execution, use focused checks for observed failures and consequential
invariants. Reuse passed gates with unchanged inputs; repeat installed acceptance
only after relevant changes or unresolved failures. Required repository gates
apply at coherent source/integration boundaries. Before an expensive cycle,
state the user-visible result it should unlock. If consecutive cycles deliver no
working increment or verified blocker removal, reassess and simplify before
another cycle. Do not turn reassessment into a planning campaign.

## Worker assignments

One primary owner retains the existing Agent Browser worktree and coordinates
necessary fixes in the existing Remote View custody. The client-to-viewing path
is serialized. Documentation and installation work may become independent only
after the actual interface works; no parallel agents or extra worktrees are
required. Declare any shared-source writer and dependency before overlap.
Use the least expensive capable validation route for the affected surface.

## Evidence and exit

RUNBOOK owns one current milestone results table, with PASS, FAIL or NOT RUN,
source/binary identity, actual client and observation, evidence locator, and the
next exact blocker. Plans and test counts are not product progress. Each
checkpoint explains what the user can now do and which code enabled it.

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
