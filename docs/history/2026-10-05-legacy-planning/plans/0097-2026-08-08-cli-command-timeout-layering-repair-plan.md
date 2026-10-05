# Plan 0097 | CLI Command Timeout Layering Repair

State: CLOSED
Roadmap: P97
Plan version: 4
Date: 2026-08-08

## Objective

Let an ordinary agent-browser CLI request carry a positive control-plane job
deadline so the daemon cancels a slow CDP operation before a caller-side
subprocess deadline expires. Prove the queue is released and the next command
is not forced to wait for the abandoned operation.

## Current State

- Global `--job-timeout-ms` now carries a positive top-level `jobTimeoutMs`
  into ordinary command JSON while preserving action-specific values.
- Timed-out and cancelled commands release the serialized queue without
  discarding a reachable BrowserManager. Only an observed process exit
  authorizes destructive browser cleanup.
- Runtime handoff preserves an explicit preferred target when responsive and
  falls back across retained targets under bounded Page/Runtime/Network domain
  initialization when a preferred or legacy target is frozen.
- Installed executable SHA-256 is
  `17f393c716f63de5008a25045f1ead0a4377efb7936300c8e1bcce2247d5995b`;
  runtime convergence and remote control are ready. The retained Last30Days
  browser remains PID 63205.

## Scope

- make `--job-timeout-ms <ms>` a global per-command CLI option;
- inject the positive value as top-level `jobTimeoutMs` for ordinary parsed
  commands while preserving an action-specific value when already present;
- preserve the existing control-plane cancellation and timed-out job record;
- persist the prepared active target when possible and make executable handoff
  resume fall back to another already-retained, CDP-responsive page when the
  preferred or legacy-selected target is frozen;
- document the timeout ordering contract on every required help surface;
- validate and install one local release-mode executable without opening or
  closing any retained browser.

## Non-Goals

- no daemon-wide default change, queue architecture rewrite, browser restart,
  profile migration, route change, retained-tab creation, or retained-tab
  cleanup;
- no claim that a timed-out command means authentication failure;
- no formal package release, tag, push, or public pull request.

## Acceptance Criteria

1. CLI parsing carries a positive global `--job-timeout-ms` into ordinary
   command JSON as `jobTimeoutMs` and removes the flag from positional args.
2. Existing action-specific `jobTimeoutMs` remains authoritative.
3. The worker timeout cancels the command, records `timed_out`, and releases the
   queue for a following command.
4. CLI help, README, Skill guidance, docs site, and inline comments explain
   that the inner deadline must be shorter than an outer caller deadline.
5. Focused tests, selected validation, canonical Rust tests, formatting,
   strict Clippy, docs build, patch checks, and installed-runtime convergence
   pass.
6. The retained Last30Days browser keeps the same PID and tab set throughout
   installation and proof.
7. Runtime handoff remains backward compatible with existing schema-v1
   descriptors, preserves the prepared active target when it is responsive,
   and skips a frozen retained target under a bounded initialization deadline.
8. Timing out or cancelling a live browser command releases the queue without
   discarding a reachable BrowserManager or launching a replacement browser;
   only an observed process exit authorizes destructive browser cleanup.

## Execution Bounds

- one red parser contract and one red end-to-end timeout ordering contract;
- one implementation pass and at most one focused rework;
- one build/install pass after source validation;
- one read-only retained-browser proof, with no browser or tab close action.

## Checkpoint C02 | Retained-target handoff recovery

- plan_version: 2
- state_transition: OPEN -> OPEN
- progress_classification: repair_scope_extended_by_installed_runtime_evidence
- evidence: browser-level CDP remained healthy on PID 96078; all seven retained
  pages attached; four answered `Page.enable` in 1-6 ms while three Facebook
  targets timed out at five seconds and also failed a bounded
  `Runtime.evaluate` probe.
- subagent_status: none
- authority_classification: inherited_authority
- review_disposition_summary: deterministic installed-runtime blocker;
  repair stays within retained-session handoff and does not create or close
  browsers or tabs.
- next_action_or_stop_reason: add red compatibility and candidate-order tests,
  implement bounded target selection, then retry release-mode convergence.

## Checkpoint C03 | Interrupted-job browser ownership

- plan_version: 3
- state_transition: OPEN -> OPEN
- progress_classification: repair_scope_extended_by_live_timeout_proof
- evidence: the one-second never-resolving eval released the queue, but the
  timeout branch unconditionally called `cleanup_exited_browser` against a
  still-reachable attached browser. The next command launched default-profile
  Chrome PID 97130 while retained PID 96078 and all seven tabs remained live.
- subagent_status: none
- authority_classification: inherited_authority
- review_disposition_summary: blocking fail-open replacement launch; exact
  default-browser ownership is proven by daemon TCP connections and process
  start time, and cleanup is bounded to that repair-created PID.
- next_action_or_stop_reason: add the interruption cleanup policy regression,
  preserve live browser state on timeout/cancel, remove only PID 97130, resume
  PID 96078, rebuild, and repeat the timeout proof.

## Checkpoint C04 | Installed closeout

- plan_version: 3
- state_transition: OPEN -> CLOSED
- progress_classification: outcome_complete
- evidence: the canonical Rust suite passed 1,789 tests with 57 ignored;
  formatting, strict Clippy, focused cancellation/handoff regressions, docs
  build, patch checks, publisher convergence, install doctor, remote-view
  doctor, and installed skill sync passed. A one-second never-resolving eval
  terminated in 1.486 seconds, the next tab-list command completed in 466 ms,
  retained PID 96078 and its seven-tab/active-index state were unchanged, and
  no default-profile Chrome remained.
- subagent_status: none
- authority_classification: inherited_authority
- review_disposition_summary: the first proof exposed the destructive cleanup
  branch and created default-profile PID 97130; exact daemon ownership was
  proved, only that repair-created process was closed, and the final proof
  passed with the retained browser untouched.
- next_action_or_stop_reason: stop; Last30Days may consume the installed global
  command deadline and retained-target recovery contracts.

## Checkpoint C05 | Post-closeout renderer termination correction

- plan_version: 4
- state_transition: CLOSED → CLOSED
- progress_classification: corrective_adoption_from_downstream_failure
- evidence: three distinct Last30Days candidates showed that cancelling the
  worker future did not stop renderer JavaScript and that navigation readback
  depended on Runtime. Direct CDP proved Chromium's renderer timeout interrupts
  an infinite loop and leaves the same target usable. A system-call trace also
  measured 5.75 seconds spent hashing the 289 MB debug executable before an
  ordinary command.
- implementation: Runtime evaluations now carry a renderer deadline;
  navigation uses `Target.getTargetInfo`; evaluation uses cached target
  metadata; worker responses precede follow-up health probes; Linux daemon
  identity uses a same-inode fast path with SHA-256 fallback.
- validation: focused browser, action, connection, control-plane, CDP stream,
  and route-confusion gates passed. Required formatting and production Clippy
  passed. The canonical partition passed 1,220 parallel-safe tests, all 35
  connection tests, and all 260 action tests before stopping at the unrelated
  stale `DISPLAY=:9` expectation in an untouched Chrome test.
- installed_proof: installed SHA-256
  `17f393c716f63de5008a25045f1ead0a4377efb7936300c8e1bcce2247d5995b`;
  baseline evaluation returned in 455 ms, the infinite-loop evaluation returned
  a typed CDP failure in 4.546 seconds under a 6-second outer guard, and the
  immediate recovery evaluation returned `2` in 1.482 seconds. The disposable
  browser was closed. Install doctor and remote-view doctor are ready with zero
  issues; retained Facebook PID 63205 was not restarted or closed.
- authority_classification: inherited corrective authority; no fourth
  Last30Days provider attempt was consumed.
- next_action_or_stop_reason: source and installed repair are complete; require
  fresh operator authority before a downstream Facebook provider proof.

## Done Definition

- all acceptance criteria have current evidence;
- installed executable hash matches the validated candidate;
- ROADMAP, RUNBOOK, this plan, required docs, and runtime readbacks agree;
- Last30Days can consume the global flag with an outer timeout grace window.
