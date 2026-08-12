# Review 0109 | Runtime Dependability Handoffs

Date: 2026-08-11

Reviewed inputs:

- `docs/dev/notes/2026-08-11-im-receipts-google-messages-rdp-handoff.md`
- `docs/dev/notes/2026-08-11-facebook-authenticated-search-blink-crash-handoff.md`

Repository state reviewed at commit `1bb6ef2f`. The Google Messages note was
untracked and was treated as operator-authored evidence, not as authorization
to edit, commit, restart, close, install, or retry anything.

## One-Sentence Assessment

Four agent-browser changes are recommended: make global close unambiguous,
productize named fixed-port daemon supervision, separate requested-route doctor
readiness from unrelated global drift, and propagate renderer crashes into
typed command, tab, incident, and diagnostic state. Existing helper
compatibility, opaque handoff, profile-mismatch, process-identity, and
no-launch collection contracts should be extended with regressions rather than
redesigned.

## Repository Profile And Retrieval

- Path type: stable local git repository.
- Task class: mixed code and documentation architecture review with change
  impact analysis.
- Retrieval posture: hybrid. CodeGraph supplied current symbol and call-flow
  evidence; literal search and direct reads were limited to known notes,
  policies, plans, event names, and exact source ranges.
- CodeGraph state: healthy, 535 indexed files, 18,425 nodes, and 63,691 edges.
- Graphiti group: `agent_browser_main`. The focused search returned eight
  facts, five nodes, and five episode previews, but no current source-backed
  answer for these two handoffs. Current repository artifacts remained
  authoritative.
- Persistence: this note and Plan 0109 in the stable repository. No bundle or
  snapshot is needed.

## Finding Ledger

### R0109-01 | `close --all` plus explicit `--session`

Disposition: `accepted_blocking`

Observed source:

- `cli/src/main.rs::run_close_all` inventories every daemon PID and closes
  every reachable session.
- command dispatch selects `run_close_all` whenever `close`, `quit`, or `exit`
  contains `--all`.
- the selected `Flags.session` value does not scope that loop.
- an unreachable daemon is force-killed from the PID file and its session
  files are removed after a signal-zero liveness check. This path does not
  bind the signal to a recorded daemon process-start identity.

Consequence: an operator can reasonably read `--session <name> close --all` as
scoped while the implementation performs a host-wide destructive operation.
This explains the five-session disruption in the Google Messages handoff.

Recommendation: reject explicit `--session` combined with `--all` before any
inventory, command, signal, or cleanup. Direct the operator to the existing
single-session close spelling. Keep `close --all` global and automation-safe;
do not add an interactive confirmation that unattended callers can bypass
accidentally. Separately remove PID-file-only forced termination: an
unreachable daemon without exact process identity must remain a reported
failure, not become a blind `SIGKILL` or `TerminateProcess` target.

### R0109-02 | Named daemon supervision on a fixed loopback port

Disposition: `accepted_blocking`

Observed source:

- `cli/src/native/daemon.rs` honors `AGENT_BROWSER_STREAM_PORT` and publishes
  the actual port in `<session>.stream`.
- current workstation units supervise the dashboard and global reconciliation,
  not arbitrary named daemon sessions.
- the Google Messages integration therefore owns a custom user unit whose
  oneshot initialization cannot independently recover a later daemon crash.

Consequence: a downstream integration must duplicate lifecycle policy to keep
a durable HTTP and WebSocket endpoint alive. Crash recovery, executable drift,
port ownership, and shutdown semantics vary by consumer.

Recommendation: add a first-class named-session supervisor contract backed by
an instance user unit and a validated, private session manifest. The
supervisor owns the daemon only. It must not implicitly launch a browser,
navigate, authenticate, or repair unrelated sessions. After a daemon crash it
may reacquire only a process-identity-proven browser already owned by the same
session and profile; otherwise it reports degraded instead of launching a
duplicate lane.

### R0109-03 | Requested-route doctor scope

Disposition: `accepted_blocking`

Observed source:

- `remote_view_doctor_report` always composes install doctor, the full runtime
  inventory, all route-pool rows, all displays, XRDP users, and global issue
  selection.
- its only current scope option is `--allow-shared-target`.
- the output lifts global `runtimeInventory` and `runtimeConvergence` directly
  from install doctor.

Consequence: a healthy requested route can be reported unusable because an
unrelated session is stale. Operators cannot distinguish subject readiness
from host advisories without reconstructing the report.

Recommendation: add an explicit requested subject with session, runtime
profile, and route selectors. Preserve the global inventory as advisories, but
compute a separate `requestedScope.status`, issues, and next action. Never hide
unrelated drift; never let it falsify the requested subject result.

### R0109-04 | Privileged-helper verification mismatch

Disposition: `regression_only`

Observed source:

- Plan 0092 is complete and defines compatibility from the bounded helper
  command and status contract instead of exact helper bytes.
- current `scripts/libexec/agent-browser-privileged-helper` implements
  `verify-install`.
- current documentation says compatible byte drift is advisory and does not
  force `sudo -v`.

Consequence: the note likely describes installed-runtime drift or an older
helper, not a missing current-source command. Re-implementing helper
compatibility would duplicate completed work.

Recommendation: add one installed-helper contract regression proving that
doctor accepts a compatible helper without requiring the optional
`verify-install` probe. Expose command-set and contract-version evidence so a
stale installed helper is diagnosed precisely. Do not weaken root ownership,
fixed path, sudoers, or capability checks.

### R0109-05 | Runtime interlock activation

Disposition: `deferred_live_gate`

Observed evidence:

- Plan 0091 remains blocked at its installed runtime gate after a prior
  self-quiesce failure and stale-daemon maintenance boundary.
- the recurring timer was recently stopped because it repeatedly attempted
  global convergence while unrelated active sessions remained stale.
- commit `1bb6ef2f` adds read-only dashboard drift detection immediately after
  login and every ten seconds without restarting sessions.

Recommendation: keep the global recurring interlock disabled. Named-session
uptime belongs to the new supervisor, while drift notification remains
read-only. Reconsider global repair only after R0109-01 through R0109-03 are
complete and one separately authorized maintenance-window canary proves
subject scoping, browser preservation, and exact rollback.

### R0109-06 | Opaque remote-view URLs

Disposition: `already_satisfied`

Observed source and plan:

- Plan 0096 is closed and makes `/remote-view/<handoff-id>` the durable,
  authenticated public contract.
- raw Guacamole URLs remain provider evidence and are not durable identity.

Recommendation: add a structural compatibility assertion to the new
supervisor and doctor tests. No new public URL design is required.

### R0109-07 | Blink renderer crash propagation

Disposition: `accepted_blocking`

Observed source:

- CDP draining handles `Target.targetDestroyed` and
  `Target.detachedFromTarget`.
- destroyed targets are removed from the in-memory page list.
- no current event path records `Inspector.targetCrashed`.
- removal does not distinguish explicit close, renderer crash, process swap,
  or normal target teardown and does not update retained service tab state.

Consequence: the Facebook renderer aborted, but the retained tab remained
`ready`, the command lacked a typed crash failure, and no incident tied the
failure to its target, process, profile, request, or stderr.

Recommendation: introduce a typed crash observation at the CDP boundary,
correlate it with the active command and service tab, persist
`TabLifecycle::Crashed`, emit one incident, and return a typed command failure.
`Target.targetDestroyed` alone must not imply a crash.

### R0109-08 | Profile routing, inventory effects, and attribution

Disposition: `partially_satisfied_with_regressions`

Observed source:

- `active_browser_profile_mismatch` rejects a request whose selected runtime
  profile or user-data path differs from the active browser.
- service profile, browser, session, and tab collection handlers are read-only
  projections over supplied service state.
- Plan 0108 completed process-identity ownership and PID-reuse safety.
- caller labels remain optional and missing labels are warnings.

Recommendation: freeze the current profile and no-launch behavior through
real ingress tests covering CLI, HTTP, and MCP. For effect-capable service
requests, add typed attribution that is always available: explicit service,
agent, and task labels when supplied, otherwise an authenticated or local CLI
principal plus request ID. Reject effectful requests only when no accountable
principal can be derived. Read-only collection requests remain label-optional
and side-effect-free.

### R0109-09 | Chromium upstream repair

Disposition: `external_follow_up`

The Blink `LineBreaker` repair belongs in the separately governed Chromium
repository. Agent-browser should consume a reviewed artifact and validate
crash propagation against an injected CDP event before any authenticated
Facebook retry. This plan does not authorize editing Chromium, copying a
profile, launching a browser, or consuming a Last30Days attempt.

## Recommended Ordering

1. Land the close-scope rejection first because it is small and prevents
   repeat cross-session damage.
2. Land renderer-crash lifecycle and attribution before another browser
   workload can reproduce an invisible failure.
3. Productize named-session supervision without enabling global reconciliation.
4. Add requested-subject doctor scoping and helper contract evidence.
5. Run source-only validation and independent review.
6. Use a separately authorized installed canary for one disposable supervised
   session. Do not use Google Messages or Facebook as the first canary.

## Uncertainty

- The Google Messages note records a successful runtime snapshot, not current
  live state. No PID, route, profile, or authentication claim was refreshed in
  this review.
- Native Windows and macOS supervisor behavior needs platform-specific unit
  generation or an explicit Linux-only first release.
- The exact upstream Chromium change list remains unidentified.
- Existing downstream custom units need an adoption and rollback story, but
  this review does not authorize editing the im-receipts installation.
- The installed planning auditor currently recognizes only two-digit `P##`
  lane IDs. It cannot parse `P109`; its forced run also reports unrelated
  historical roadmap debt. Plan 0109 was therefore checked manually for lane,
  current state, roadmap, runbook, scope, validation, rollback, and acceptance
  wiring.

## State Location

- Review: this file.
- Implementation authority: `docs/dev/plans/0109-2026-08-11-runtime-dependability-handoff-remediation-plan.md`.
- No persistence bundle was created because the repository path is stable.

## Implementation Disposition | 2026-08-11

Reviewed source head: `c00c9655`.

- `R0109-01` complete: explicit-session global close rejects before discovery,
  and ordinary close requires bound daemon process identity.
- `R0109-02` source complete: named Linux daemon sessions have a validated,
  fixed-port, no-browser supervisor and typed status. Installed supervision was
  not exercised.
- `R0109-03` complete: requested session, profile, and route status is distinct
  from preserved global advisories, with ambiguity failing closed.
- `R0109-04` complete as a regression: compatible helper capability remains
  ready without requiring the optional verification probe.
- `R0109-05` unchanged by design: the recurring global runtime interlock
  remains disabled and no live repair was attempted.
- `R0109-06` preserved: public remote-view identity remains the opaque
  `/remote-view/<handoff-id>` contract.
- `R0109-07` complete: `Inspector.targetCrashed` is the typed crash authority,
  while ordinary detach, destroy, replacement, and explicit close remain
  non-crash paths.
- `R0109-08` complete: shared normalization derives accountable request
  identity before effects, and CLI, HTTP, MCP, generated-client, and dashboard
  collection paths retain no-launch coverage.
- `R0109-09` untouched: the Chromium fix remains an external repository task.

Fresh-context review found and corrected two source-structure regressions and
one ambient-state test fixture. Closed-world verification found no remaining
source blocker. The separately authorized installed canary remains open and
must use a disposable session and profile rather than Google Messages or
Facebook.
