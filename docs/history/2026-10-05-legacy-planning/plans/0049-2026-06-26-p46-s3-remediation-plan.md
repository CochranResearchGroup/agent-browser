# P46 S3 Remediation Plan

Date: 2026-06-26
State: SUPERSEDED BY P50 DAEMON-AUTHORITY LOCK
Lane: P49
Depends On:
- `docs/dev/plans/0046-2026-06-24-remote-view-stress-hardening-plan.md`
- `docs/dev/plans/0050-2026-06-26-s3-binary-authority-visible-window-plan.md`
- `/tmp/agent-browser-p46-s3-2026-06-26T12-59-10-995Z`
- `/tmp/agent-browser-p46-s3-2026-06-26T13-03-59-219Z`

## Purpose

Repair S3 enough to produce trustworthy evidence for "default profile,
multiple operators, different tabs" without masking route, display, or tab
ownership failures. The prior S3 lock remains valid until this plan closes
with a clean S3 pass or a new source-backed blocker.

## Failure Summary

S3 attempt 1 proved the route-bound browser and dashboard viewer path could
work, but the harness selected tab targets from positional tab-list rows and
treated an existing default-profile tab as test-owned evidence.

S3 attempt 2 exposed a real route display proof failure:
`browser_window_not_visible: route 'guacamole:3' display ':13' state is
'non_browser_windows'`. The harness then continued after the failed
`remote-view open`, created new tabs against a fallback default session, and
made the artifact unsuitable for product conclusions beyond the failed open.

## Non-Negotiable Rules

- Do not run S4 or later until S3 passes from a clean reset.
- If `remote-view open` fails, S3 must stop immediately after writing status,
  incident, display, and route-pool artifacts.
- Do not infer S3 tab ownership from positional fallback rows when command
  results fail to provide stable service tab handles.
- The post-remediation S3 retry keeps the P46 rule: one failure may be repaired
  agentically; two failures after remediation lock execution for maintainer
  planning.

## Goal 1: Harden S3 Harness Gating

`/goal execute P49 goal 1: make the S3 runner fail closed after failed route-bound open and require stable command-returned tab handles before viewer clients or tab controls run`

Work:

- Add an S3 open success assertion immediately after `remote-view open`.
- Capture service status, incident summary, display content, route-pool
  readiness, and route-bound finalization evidence before throwing.
- Make S3 tab A and tab B derive from `remote-view open` and `tab new`
  service tab handles.
- Fail before launching external dashboard clients if either tab handle is
  missing or duplicated.

Evidence:

- no-live harness check covers S3 scenario support and fail-fast source
  patterns;
- failed-open artifacts contain the required audit inputs without dashboard
  viewer contamination.

## Goal 2: Add Bounded Visible-Window Proof Wait

`/goal execute P49 goal 2: make route-bound open wait briefly for a browser window to become visible before failing non_browser_windows display proof`

Work:

- Replace the single display-proof read in `remote_view_open_visible_window_proof`
  with a bounded polling loop.
- Keep terminal states as hard failures. Only retry transient
  `non_browser_windows`, `empty_display`, and unavailable display probe states.
- Include the final display state in the existing failure message.

Evidence:

- focused Rust test or existing remote-view proof test updated for retry
  behavior where practical;
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`;
- a focused no-live test that validates the S3 runner still writes the
  expected audit shape.

## Goal 3: Reopen P46 S3 And Execute One Clean Retry

`/goal execute P49 goal 3: reopen P46 at S3, run S3 from reset-before and reset-after, and record pass or blocker evidence`

Work:

- Update P46 to point at this remediation plan and mark S3 execution as
  eligible for one post-remediation retry.
- Run:

```bash
pnpm test:p46-stress-scenario -- --scenario s3 --reset-before --reset-after
```

Pass criteria:

- `remote-view open` reports `operatorVisible.state=ready`;
- two distinct command-returned service tab handles are recorded;
- both external dashboard viewer clients show the expected browser/session/tab
  selection and functional refresh controls;
- route display screenshot exists and is browser-window visible;
- route-bound finalization evidence is coherent;
- reset-after returns to zero sessions, zero browsers, and zero active
  incidents.

Failure handling:

- On the first post-remediation S3 failure, inspect artifacts and repair only
  if the defect is source-backed and agentically bounded.
- On a second post-remediation S3 failure, update P46 and this plan to locked
  state with artifact paths and stop execution for maintainer planning.

## Closeout Criteria

- P46 and this plan both reflect the final S3 state.
- Runtime cleanup is verified with `agent-browser --json service status`.
- No-live checks and relevant formatting checks pass.
- Live S3 either passes, or the plan is locked with an artifact-backed blocker
  and no active runtime incidents remain.

## Execution Log

### 2026-06-26

Implemented Goal 1 and Goal 2:

- `scripts/run-p46-stress-scenario.js` now stops S3 immediately after a failed
  `remote-view open`, captures display, incident, service-status,
  route-pool-readiness, and route-bound-finalization artifacts, and avoids
  dashboard viewer launch or tab controls when the open is not ready.
- S3 now refuses to launch dashboard viewers if tab A and tab B do not have two
  distinct stable service tab handles.
- `cli/src/native/actions.rs` now retries transient visible-window proof states
  for a bounded window while keeping terminal states hard failures.

Validation passed:

- `node scripts/test-p47-scenario-harness.js`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml test_remote_view_visible_window_proof_retryable_states_are_transient_only`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `git diff --check -- scripts/run-p46-stress-scenario.js scripts/test-p47-scenario-harness.js cli/src/native/actions.rs docs/dev/plans/0049-2026-06-26-p46-s3-remediation-plan.md`

Post-remediation S3 attempt 1:

- Artifact: `/tmp/agent-browser-p46-s3-2026-06-26T21-08-54-578Z`
- Result: failed from a harness evaluator null-access bug after the new
  failed-open path correctly avoided tab and dashboard follow-on actions.
- Runtime cleanup: resolved stale `session:default` health incident after
  confirming zero sessions, zero browsers, zero tabs, and available route-pool
  entries.

Post-remediation S3 attempt 2:

- Artifact: `/tmp/agent-browser-p46-s3-2026-06-26T21-11-13-480Z`
- Result: failed closed at `remote-view open` with
  `browser_window_not_visible: route 'guacamole:3' display ':13' state is
  'non_browser_windows'`.
- Display evidence: route A `:13` and route B `:14` both showed only
  `Openbox`, no browser window.
- Route-pool evidence: `scripts/smoke-rdp-guac-route-pool-readiness.js
  --report-only` reported ready Guacamole, permissions, backend TCP, and route
  display sockets.
- Harness behavior: no tab creation, dashboard viewer launch, or tab control
  action ran after the failed open.
- Runtime cleanup: after explicit resolution of stale `session:default`
  health evidence, final status was zero sessions, zero browsers, zero tabs,
  zero active incidents, and both route-pool entries available.

Important caveat:

- The live S3 retries used the installed `agent-browser` command selected by
  the harness default. That means the repository Rust change to bounded
  visible-window proof was compiled and tested, but not exercised by the live
  installed binary during these retries. This is an execution defect in this
  remediation run and must be corrected in the next planning pass by either
  installing the candidate binary or setting `AGENT_BROWSER_COMMAND` to the
  freshly built repo binary before live stress execution.

Lock decision:

- P49 and P46 are locked for maintainer planning. Do not run another S3 retry
  until the next plan decides whether to first validate the rebuilt local
  binary path, harden browser-window realization on the RDP display, or both.

## P50 Supersession

P50 validated the concern in this plan's caveat: command authority must include
the long-lived daemon, not only the CLI path. The narrow `s3-open` proof used
the explicit repo CLI but found multiple default-socket daemon listeners,
including the installed binary and older repo debug processes. P46 remains
locked until daemon listener ownership is singular and source-backed.
