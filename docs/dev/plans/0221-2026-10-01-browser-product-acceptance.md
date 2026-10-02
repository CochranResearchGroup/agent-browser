# Plan 0221 | Browser Product Acceptance

Date: 2026-10-01

Plan version: 1

State: PLANNED

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

## Current State

Source checkpoint `eeb0703d` connects ordinary requests, managed tab creation,
profile selection and durable handoffs. Synthetic composition and model checks
pass. No known-client authenticated profile load or installed Agent Browser
acceptance has been proved. The earlier development binary predates that source.
The previous implementation goal is paused at 952,452 tokens. This planning
change does not resume it, reset its meter or authorize installed effects.

## Consolidated batch

Run the eight scenarios below and fix whatever prevents the expected product
result. Some behavior still needs implementation. Keep the existing ownership
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
| 1. Known-client profiles | Capture the real request shapes of at least two existing clients. Replay them through their normal CLI/HTTP/MCP entry points, first with synthetic inputs, then reviewed development copies of the intended profiles. Navigate, repeat and restart. | Correct profile and login state; same healthy browser reused; no silent disposable substitution; headers, wait options and client response/handle contracts work. |
| 2. Sessions and tabs | Open Alice and Bob in one profile, create another tab, then open a second profile's browser on the same desktop. Repeat one request with its stable key. | Separate sessions/tabs/handoffs; no duplicate creation; service tab handles remain usable; multiple browsers can share one desktop. |
| 3. Viewing and focus | Open each handoff from desktop and mobile viewers. Switch between tabs and browsers; change operator focus; also interrupt presentation while CDP stays healthy. | The addressed tab is visible and controllable; focus does not fight the operator; passive viewing does not keep a session alive; viewing failure does not stop healthy automation. |
| 4. Reconnect and expiry | Reopen the same bookmark after viewer reconnect, service restart, grant expiry and revocation. | Current presentation is renewed when appropriate; logical session/tab/handoff identity survives; unavailable presentation is reported accurately. |
| 5. Deliberate close | Close one tab/session, then deliberately stop its browser. Open the old handoff; explicitly reopen afterward. | Peers remain usable; closed work stays closed; operator stop is respected until explicit reopen. |
| 6. Interrupted operations and recovery | Lose a reply or restart during allocation, launch and tab creation. Separately lose a browser and make its desktop unavailable under active demand. | Exact readback resolves pending work; no duplicate effect or permanent stale lock; one bounded recovery/relocation preserves logical identities and durable URL intent. |
| 7. Capacity and cleanup | Add demand beyond current capacity, end a non-final session, then end all work and shrink eligible capacity. Compare fresh before/after process, listener, desktop, assignment and viewer inventories. | Capacity grows only when needed; peers survive; final browser close and assignment return are exact; only empty clean desktops shrink; no unexplained owned resources remain. |
| 8. Candidate installation | Freeze source and artifact identity; run changed-surface/platform gates, first install, update and doctor; complete protected integration. | Reproducible candidate; sudo exactly once on first install; diagnostic doctor identifies broken dependencies; supported many-to-many operation is proved and every release blocker is named. |

## Delivery sequence and budget

Start with test 1's real client request and expected response. Run the cheapest
check that reproduces its failure, fix the owning code, then rerun that scenario
and affected regressions. Work through the list; group failures sharing a cause.
Do not create another planning backlog or count extra tests as product progress.

Keep one current results table in RUNBOOK. Reuse valid checks when their inputs
have not changed. Mark unavailable runs NOT RUN with the exact missing input;
never weaken a pass condition to fit the implementation. Freeze the candidate
only after known blocking fixes are included. A successor does not reset prior
cumulative effort or the user's stop instruction.

Existing reviewed development-runtime and installed-effect boundaries carry
forward from [P220](0220-2026-09-28-remote-view-consumer-integration.md#separately-gated-installed-acceptance-packet).
Use exact isolated binaries, reviewed targets and disposable/development profile
roots. Ordinary source repairs remain in scope. Production promotion and formal
release remain separately directed actions.

## Worker assignments

One primary owner drives the checklist and repairs the owning implementation.
Reuse the current branch/worktree; no parallel workers or new infrastructure
tracks are required. Change only the product surfaces needed for a failing row.

## Evidence and exit

Complete when all eight scenarios pass with source-bound installed evidence,
known clients work through their normal interfaces, no unexplained owned
resources remain, and an installable candidate passes its required gates.
Report unresolved failures plainly. P220's unproved requirements carry forward
through these scenarios and are not declared complete by supersession.
