# Plan 0110 | CDP Input Page Scroll Repair

State: CLOSED
Roadmap: P110
Plan version: 2
Date: 2026-08-11
Predecessor: Plan 0109 version 2/checkpoint C02

## Objective

Move selectorless page scrolling off renderer JavaScript and onto Chromium's
CDP input-wheel primitive so a responsive retained Facebook browser can scroll
without waiting on a stalled `Runtime.evaluate` response.

## Current State

- the exact installed Plan 0109 candidate fixed evaluation deadline
  propagation and preserved Facebook browser PID 13177 and all retained tabs;
- the one downstream tick successfully navigated and evaluated, then
  operation `r958354` failed selectorless scroll after 28.49 seconds with
  `CDP command timed out: Runtime.evaluate`;
- current selectorless `interaction::scroll` implements
  `window.scrollBy(dx, dy)` through the fixed-timeout typed
  `Runtime.evaluate` path, while later browser-level tab inventory remains
  responsive;
- selector-targeted element scrolling is a separate contract and remains out
  of scope.

## Authority And Safety Boundary

The active Last30Days `repair the tick` goal and successor Plan 0044 authorize
this bounded dependency repair, one browser-preserving installed candidate,
and at most one later Facebook-only acceptance tick. Work continues in the
isolated `repair/eval-budget-propagation` worktree from exact installed commit
`1c1331efefbb41d7c5ba2384089eb2bfbd358f81`.

This plan does not authorize a formal release, public push, browser restart or
close, tab close, profile mutation, login attempt, route change, schedule
change, or more than one downstream Facebook tick.

## Scope

- add a fake-CDP regression proving selectorless scroll emits a wheel input
  command with the requested horizontal and vertical deltas;
- replace only the selectorless JavaScript scroll path with the browser-level
  CDP input-wheel operation;
- preserve selector-targeted element scrolling and existing response shape;
- run focused and required repository validation, commit one successor
  candidate, and install it through the supported browser-preserving handoff;
- hand the one downstream acceptance attempt to Last30Days Plan 0044.

## Non-Goals

- no generalized interaction refactor, timeout increase, Facebook selector or
  extraction change, browser/profile cleanup, credential change, or retry of
  the failed Plan 0109 tick;
- no public push or immutable release.

## Acceptance Criteria

1. The new fake-CDP regression is red against the current JavaScript scroll
   path and observes `Input.dispatchMouseEvent` with type `mouseWheel` after the
   repair.
2. Horizontal and vertical deltas, session routing, and existing command
   response shape are preserved.
3. Selector-targeted scrolling remains unchanged and focused interaction tests
   pass twice.
4. Formatting, strict production Clippy, selected validation, and the canonical
   touched-surface Rust gates pass.
5. One exact committed candidate installs while retaining Facebook browser PID,
   CDP endpoint, and tab inventory.
6. No more than the one downstream Facebook tick authorized by Last30Days Plan
   0044 is consumed.

## Execution Bounds

- one red regression, one implementation pass, and at most one focused rework;
- one candidate commit/build/install and one downstream Facebook tick;
- no subagents under the current orchestration restriction;
- hard stop on browser/profile identity drift, auth/challenge/rate-limit
  evidence, failed source gates, terminal downstream failure, or any effect
  outside this plan.

## Checkpoint C01 | Distinct Scroll Owner Identified

- plan_version: 1
- state_transition: downstream_scroll_failure -> OPEN
- progress_classification: blocker_reduction
- evidence: operation `r958354` is an exact selectorless `scroll` failure at
  `Runtime.evaluate`; structural source inspection binds that command to
  `window.scrollBy`, while a later tab inventory proves the browser-level CDP
  channel remained responsive.
- subagent_status: none; prohibited by current orchestration policy
- authority_classification: inherited_goal_authority
- next_action_or_stop_reason: add the fake-CDP wheel regression red, implement
  the minimal selectorless path change, and stop before installation unless all
  source gates pass.

## Checkpoint C02 | Focused Browser Green

- plan_version: 1
- state_transition: OPEN -> OPEN
- progress_classification: blocker_reduction
- evidence: the fake-CDP regression failed against the original source because
  it observed `Runtime.evaluate` instead of `Input.dispatchMouseEvent`. The
  repaired path emits type `mouseWheel`, the exact session, and both requested
  deltas. Its first `(0,0)` input position passed the protocol regression but
  failed the existing real-Chrome scroll e2e, so the one allowed focused rework
  moved the event inside the viewport and added a 50 ms compositor-application
  boundary. The fake-CDP test, all 11 interaction tests, and the ignored
  real-Chrome `e2e_hover_scroll_press` test then passed twice.
- validation: patch check, formatting, strict production Clippy, and the
  repository selector bound to base `1c1331ef` pass; selector-targeted element
  scrolling remains unchanged.
- subagent_status: none; prohibited by current orchestration policy
- authority_classification: inherited_goal_authority
- next_action_or_stop_reason: run the canonical Rust gate, commit the exact
  candidate, and do not install unless it remains green.

## Checkpoint C03 | Candidate Source Gates Green

- plan_version: 1
- state_transition: OPEN -> OPEN
- progress_classification: blocker_reduction
- evidence: the canonical Rust runner passed 1,043 parallel-safe tests with 57
  ignored and every serialized partition; formatting, strict production
  Clippy, patch checks, selected validation, the interaction partition, and the
  real-Chrome scroll e2e also pass.
- subagent_status: none; prohibited by current orchestration policy
- authority_classification: inherited_goal_authority
- next_action_or_stop_reason: commit this exact candidate and perform the one
  supported browser-preserving local install; stop before the downstream tick
  unless PID, endpoint, target inventory, and runtime provenance converge.

## Checkpoint C04 | Installed Runtime Converged

- plan_version: 1
- state_transition: OPEN -> OPEN
- progress_classification: blocker_reduction
- evidence: exact commit `a954bc95023b16e2bee5c9d6dfe369915e748f0c`
  installed as executable SHA-256
  `76b2779ffc65d85f22817c698732e387dffe9cd4f8225f9aaf6b65bba467d3d1`.
  The supported publisher handed Facebook daemon 95356 to daemon 99264 while
  retaining browser PID 13177, exact endpoint
  `ws://127.0.0.1:38770/devtools/browser/00317084-6844-44c8-b1a3-c63555867ced`,
  and four attached targets. Source-free workstation provenance now matches the
  same digest, runtime convergence is `converged`, and dashboard plus runtime
  interlock timer are active.
- residual: install doctor remains false only for the pre-existing visible
  duplicate-profile-pressure warning; service resources report zero candidates
  and zero readiness-impacting candidates, so this warning is nonblocking and
  no unrelated browser is closed to silence it.
- subagent_status: none; prohibited by current orchestration policy
- authority_classification: inherited_goal_authority
- next_action_or_stop_reason: hand control to Last30Days Plan 0044 for fresh
  service/database/schedule/profile preflight and its one allowed Facebook tick.

## Checkpoint C05 | Downstream Target Stall

- plan_version: 2
- state_transition: OPEN -> CLOSED
- progress_classification: blocker_reduction_then_distinct_upstream_failure
- evidence: the one downstream tick
  `tick-877ca3d32b5e6c335d60b585fc631985` reached the installed wheel path. Exact
  job `r213109` sent `Input.dispatchMouseEvent` but the Facebook target did not
  acknowledge it for 28.37 seconds; subsequent direct page evaluation also
  timed out while browser-level tab inventory remained responsive.
- adjudication: the installed selectorless wheel implementation remains proven
  by fake-CDP and real-Chrome e2e tests. Last30Days diagnosis found that its
  replacement-target query capture extracts immediately after navigation, then
  sleeps four seconds but reuses the stale empty capture instead of performing
  the intended post-wait extraction. That upstream stale-read forces the
  unnecessary scroll and now owns the remaining acceptance failure.
- subagent_status: none; prohibited by current orchestration policy
- authority_classification: inherited_goal_authority
- next_action_or_stop_reason: close P110 without another browser transport
  attempt; Last30Days successor Plan 0045 owns stale prepared-extraction refresh.

## Done Definition

- all criteria have source, commit, installed-runtime, and downstream evidence;
- Plan 0110/P110 and the runbook agree on the terminal state;
- retained browser, profile, route, and schedule identity remain unchanged.
