# Plan 0109 | Evaluation Deadline Propagation Repair

State: OPEN
Roadmap: P109
Plan version: 1
Date: 2026-08-11

## Objective

Carry an explicit positive evaluation deadline through both Chromium's
renderer timeout and the CDP response transport so a caller-selected 45-second
job is not truncated by older fixed 25-second and 30-second limits.

## Authority And Safety Boundary

The active Last30Days `repair the tick` goal and its Plan 0043 authorize this
bounded dependency repair and one local installed candidate. Source work uses
the isolated `repair/eval-budget-propagation` worktree at exact architecture
checkpoint `baaed508a7b884ec6382816eddf27c47589e56ab`.

This plan does not authorize a formal package release, public push, browser
restart, browser close, tab close, profile mutation, login attempt, route
change, or more than one downstream Facebook tick. The retained browser and
profile must survive the executable handoff unchanged.

## Confirmed Defect

`BrowserManager::evaluate_with_timeout` accepted the caller deadline but
capped Chromium's renderer timeout at 24.75 seconds and used
`CdpClient::send_command`, whose transport timeout was fixed at 30 seconds.
A Last30Days request with a 45-second job deadline therefore returned an
agent-browser error after 31.810 seconds even though its caller still had
budget.

The red regression sent a response after 31 virtual seconds and failed with
`CDP command timed out: Runtime.evaluate`. The socket-backed paused-time
harness was unsuitable as a permanent test because Tokio virtual time and
kernel socket scheduling were nondeterministic. Its red receipt is preserved;
the durable regression instead binds both deadlines in the pure request
contract consumed by the production call.

## Scope

- remove the legacy 25-second renderer cap while preserving 250 milliseconds
  for response delivery;
- make the evaluation request own both renderer parameters and its caller-sized
  CDP transport timeout;
- send that request through `send_command_with_timeout`;
- add deterministic contract and immediate fake-CDP integration coverage;
- validate, commit, build, and install one local release-mode candidate;
- hand downstream acceptance back to Last30Days Plan 0043.

## Acceptance Criteria

1. A 45-second evaluation request carries a 44.75-second renderer timeout and
   a 45-second CDP transport timeout.
2. A short request still reserves the same response grace.
3. The production evaluation path consumes the tested request contract and
   the fake-CDP integration sees the renderer deadline.
4. The browser partition passes twice, formatting and strict Clippy pass, and
   the repository-selected validation is green or any unrelated baseline
   failure is exactly classified.
5. The local release candidate is tied to an exact commit and installed with
   the supported browser-preserving handoff.
6. Install doctor and runtime readbacks remain ready with the retained browser
   process and Facebook tab inventory unchanged.
7. No more than the one downstream Facebook tick authorized by Last30Days
   Plan 0043 is consumed.

## Execution Bounds

- one captured red regression and one deterministic replacement seam;
- one implementation pass and at most one focused rework;
- one candidate commit, build, and installed handoff;
- one downstream Facebook tick, owned by Last30Days Plan 0043;
- no subagent evaluation under the current orchestration restriction.

## Checkpoint C01 | Deterministic Green Slice

- plan_version: 1
- state_transition: OPEN -> OPEN
- progress_classification: confirmed_root_cause_and_focused_repair
- evidence: the captured delayed-response regression failed at the fixed
  30-second CDP timeout; the request-contract regression now proves 44.75 and
  45-second renderer and transport deadlines; the 35-test browser partition
  passed twice with one thread. Repository-selected patch, formatting, strict
  Clippy, and focused gates pass; the canonical runner passed 1,042
  parallel-safe tests with 57 ignored plus every serialized partition.
- subagent_status: none; prohibited by current orchestration policy
- authority_classification: inherited_goal_authority
- next_action_or_stop_reason: run repository validation, build the exact local
  candidate, and use the supported browser-preserving executable handoff.

## Done Definition

- all acceptance criteria have source, artifact, installed-runtime, and
  downstream evidence;
- the exact candidate commit and installed executable digest are recorded;
- P109, this plan, the runbook, and the execution receipt agree;
- no retained browser, profile, route, or schedule authority drift occurred.
