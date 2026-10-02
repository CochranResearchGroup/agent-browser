# Plan 0221 | Deliver Working Browser Workflows

Date: 2026-10-01

Plan version: 3

State: OPEN

Consolidation: required

Product lane: PL-PLATFORM

Lane: P221

Predecessor: P220, superseded while incomplete

Work item: [CochranResearchGroup/agent-browser#202](https://github.com/CochranResearchGroup/agent-browser/issues/202)

Branch: platform/p220-remote-view-consumer (retain existing source custody)

Target: main

Owner: primary browser integration owner

## Objective

Deliver a browser experience we can demonstrate using our existing clients:
load the right profile, operate its tabs, view the right tab, reconnect, recover
and close cleanly. Finish with an installable, reproducible release candidate.

The goal is working product behavior. Plans, contracts, tests and reports are
tools for developing and verifying that behavior, not delivery outcomes. A
passing synthetic suite or completed planning packet does not establish product
progress without a demonstrated working workflow or removal of its observed
blocker.

This revision makes the operator's delivery priority explicit: implement missing
product behavior and demonstrate it through existing clients. Do not spend the
delivery budget completing plans, aligning internal layers or expanding tests
as independent objectives. Compilation establishes a buildable artifact; it
does not establish a usable development candidate or make testing the only
remaining work.

## Current State

Source checkpoint `14f06754` repairs interrupted placement, owned-tab command
routing, ordinary read reuse and browser identity across restart. Installed
partial evidence proves Chrome launch on the development profile, navigation to
Example Domain, URL/title reads and explicit managed close. The actual AuraCall
workflow still fails at operator presentation. No authenticated profile,
operator-visible viewing or full acceptance scenario is proved. RUNBOOK binds
the current installed candidate and retained resources.
The earlier implementation stopped at 952,452 tokens; the subsequent
continuation crossed the cumulative one-million-token checkpoint and was
paused. RUNBOOK owns the current execution record. The operator subsequently resumed implementation with a new 500,000-token
checkpoint instruction. RUNBOOK records that resumption and its candidate
identity; prior effort remains historical.

## Consolidated batch

Build and demonstrate one real existing-client workflow first, then extend
that working product to the remaining scenarios below. Some behavior still
needs implementation. P220 implementation and architecture are candidates for
reuse, not requirements to preserve. Simplify, replace or remove machinery
that obstructs the product result; preserve relevant evidence and safety
boundaries rather than perpetuating an implementation because it has tests.
Keep the existing ownership
split: Agent Browser owns browsers/profiles/tabs; Remote View owns desktops and
viewing. Preserve P219 custody and P220 evidence. No unrelated architecture,
IAM, multi-tenant redesign or retired XRDP/Guacamole repair enters this batch.

## Acceptance checklist

For each scenario, record only setup, action, expected result, current result
and evidence. Use PASS, FAIL or NOT RUN; identify the exact source/binary tested.
Synthetic results prepare the real run and never substitute for installed proof.
All eight installed scenarios are currently NOT RUN.

| Test | Run | Expected product result |
| --- | --- | --- |
| 1. Known-client profiles | Capture the real request shapes of at least two existing clients. Replay them through their normal CLI/HTTP/MCP entry points, using reviewed development copies of the intended profiles. Navigate, repeat and restart. | Correct profile and login state; same healthy browser reused; no silent disposable substitution; headers, wait options and client response/handle contracts work. |
| 2. Sessions and tabs | Open Alice and Bob in one profile, create another tab, then open a second profile's browser on the same desktop. Repeat one request with its stable key. | Separate sessions/tabs/handoffs; no duplicate creation; service tab handles remain usable; multiple browsers can share one desktop. |
| 3. Viewing and focus | Open each handoff from desktop and mobile viewers. Switch between tabs and browsers; change operator focus; also interrupt presentation while CDP stays healthy. | The addressed tab is visible and controllable; focus does not fight the operator; passive viewing does not keep a session alive; viewing failure does not stop healthy automation. |
| 4. Reconnect and expiry | Reopen the same bookmark after viewer reconnect, service restart, grant expiry and revocation. | Current presentation is renewed when appropriate; logical session/tab/handoff identity survives; unavailable presentation is reported accurately. |
| 5. Deliberate close | Close one tab/session, then deliberately stop its browser. Open the old handoff; explicitly reopen afterward. | Peers remain usable; closed work stays closed; operator stop is respected until explicit reopen. |
| 6. Interrupted operations and recovery | Lose a reply or restart during allocation, launch and tab creation. Separately lose a browser and make its desktop unavailable under active demand. | Exact readback resolves pending work; no duplicate effect or permanent stale lock; one bounded recovery/relocation preserves logical identities and durable URL intent. |
| 7. Capacity and cleanup | Add demand beyond current capacity, end a non-final session, then end all work and shrink eligible capacity. Compare fresh before/after process, listener, desktop, assignment and viewer inventories. | Capacity grows only when needed; peers survive; final browser close and assignment return are exact; only empty clean desktops shrink; no unexplained owned resources remain. |
| 8. Candidate installation | Freeze source and artifact identity; run changed-surface/platform gates, first install, update and doctor; complete protected integration. | Reproducible candidate; sudo exactly once on first install; diagnostic doctor identifies broken dependencies; supported many-to-many operation is proved and every release blocker is named. |

## Delivery sequence and budget

The first deliverable is an installed demonstration through one existing
client's normal interface: load its intended development profile with the
expected login state, navigate, read or act on the page, repeat a request using
the same healthy browser, open the handoff and see/control the addressed tab,
then close the work cleanly. Record the exact client, source and installed
binary identity and the observed result. This first demonstration advances the
full objective; it does not replace the other acceptance requirements.

On implementation resumption:

1. Capture one existing client's actual request and expected response. Select
   the reviewed isolated development target and profile needed to run it.
2. Build and install a current development candidate, then run that workflow.
   Use the earliest real failure to identify the missing product behavior.
3. Implement the simplest complete repair in the owning code. Remove or bypass
   unnecessary internal machinery when that better delivers the required
   behavior while preserving applicable safety and ownership boundaries.
4. Run focused regression checks and rebuild only affected artifacts, then
   repeat the installed workflow. Do not replace the demonstration with a
   synthetic analogue. If an input is unavailable, name that exact blocker and
   advance independent product implementation rather than inventing scaffolding.
5. Demonstrate a second existing client, then extend the working path through
   sessions/tabs, viewing, reconnect, close, recovery and capacity. Group
   observed failures sharing a cause. Qualify the final installable candidate
   after the necessary product fixes are included.

Tests should reproduce observed failures or protect consequential behavior.
Do not expand models, contract layers, review rounds or test matrices without
a concrete product need. Required repository gates still apply at their proper
boundaries; they do not replace installed evidence. Before another expensive
build or validation cycle, identify which missing user-visible result it will
resolve. If repeated cycles yield only more scaffolding or test coverage, stop
that approach and simplify the implementation instead of opening another plan.

Keep one current results table in RUNBOOK. Reuse valid checks when their inputs
have not changed. Mark unavailable runs NOT RUN with the exact missing input;
never weaken a pass condition to fit the implementation. Report what a user can
now do, what was actually demonstrated and what still fails. A successor does
not reset prior cumulative effort or the user's stop instruction.

Existing reviewed development-runtime and installed-effect boundaries carry
forward from [P220](0220-2026-09-28-remote-view-consumer-integration.md#separately-gated-installed-acceptance-packet).
Use exact isolated binaries, reviewed targets and disposable/development profile
roots. Ordinary source repairs remain in scope. Production promotion and formal
release remain separately directed actions.

## Worker assignments

One primary owner drives the checklist and repairs the owning implementation.
Reuse the current branch/worktree; no parallel workers or new infrastructure
tracks are required. The primary owns implementation, the installed demonstration and any
simplification needed to deliver it. There is no separate contract-completion
or test-count target.

## Evidence and exit

For each delivery checkpoint, name the code change, the user behavior it enables,
and the actual installed observation. Distinguish an implemented but unproved
change from a demonstrated capability. Test counts, alignment work and document
completion are not substitutes for that account. A failed real workflow directs
the next implementation step; it must not trigger an unbounded testing project.

Complete when all eight scenarios pass with source-bound installed evidence,
known clients work through their normal interfaces, no unexplained owned
resources remain, and an installable candidate passes its required gates.
Report unresolved failures plainly. P220's unproved requirements carry forward
through these scenarios and are not declared complete by supersession.
