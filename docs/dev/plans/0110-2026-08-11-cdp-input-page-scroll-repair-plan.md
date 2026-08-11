# Plan 0110 | CDP Input Page Scroll Repair

State: OPEN
Roadmap: P110
Plan version: 1
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

## Done Definition

- all criteria have source, commit, installed-runtime, and downstream evidence;
- Plan 0110/P110 and the runbook agree on the terminal state;
- retained browser, profile, route, and schedule identity remain unchanged.
