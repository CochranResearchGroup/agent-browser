# Plan 0091: Systemd Interlock Self-Quiesce Repair

Date: 2026-08-03
State: CLOSED
Lane: P91
Predecessor: P90
Source incident: external dashboard HTTP 502 after a timer-triggered workstation reconcile

## Current State

The self-quiesce repair and compatible-helper installation were completed by
later runtime work. The current production monitor lock-timeout regression is
a distinct defect under issue #76 and does not keep this historical repair open.

## Goal

Prevent `agent-browser install workstation reconcile --json` from terminating
its own running systemd service while preserving the existing dashboard and
timer quiesce contract. Replace the installed runtime with the reviewed fix,
then prove one successful timer-triggered pass without losing the dashboard,
Guacamole route visibility, or PostgreSQL backups.

## Current Evidence

- Guacamole connection `1` remained present and mapped `wsl-chrome-3` to
  display `:10`, but the `admin` entity lacked `READ` on the two managed
  connections. The guarded route sync restored three of three grants on each
  connection and exact API reads now return HTTP 200.
- Starting the installed runtime interlock produced
  `Result=signal`, `ExecMainStatus=15`, and left
  `agent-browser-dashboard.service` inactive. The public dashboard route then
  returned HTTP 502.
- The dashboard was restarted directly and both the local dashboard and public
  route returned HTTP 200.
- CodeGraph traces `agent-browser install workstation reconcile --json` through
  `run_workstation_reconcile`, `reconcile_workstation`, and
  `reconcile_workstation_locked` to `quiesce_existing_user_units`.
- `quiesce_existing_user_units` includes
  `agent-browser-runtime-interlock.service` in the same stop request issued by
  that service. Systemd therefore terminates the reconciler before it can
  reactivate the dashboard and timers.

## Scope

- add one regression for the reconcile quiesce set;
- remove the running interlock service from that set while retaining the
  dashboard, interlock timer, and backup timer;
- keep the public workstation reconcile interface unchanged;
- run focused Rust, source-free workstation, formatting, Clippy, and selected
  validation;
- build and install the corrected runtime as an ordinary workspace checkpoint,
  not a formal release;
- re-enable both timers and require one successful interlock pass plus exact
  dashboard, Guacamole, route, doctor, and backup readbacks;
- update roadmap, runbook, and one validation note with durable evidence.

## Non-Goals

- changing Guacamole connection identities, RDP users, displays, or browser
  sessions;
- formal release preparation or version advancement;
- running the many-to-many browser live gate;
- touching the operator-owned untracked `--full-page` file.

## Execution

### Packet A: Regression and Minimal Repair

- Extract the managed reconcile quiesce set into one source-local constant.
- Add one test proving the running interlock service is excluded while the
  dashboard and both timer units remain included.
- Record the failing test before removing the service from the set.
- Apply the one-line behavioral repair and rerun the focused test.

### Packet B: Deterministic Validation

- Run the workstation install Rust module tests.
- Run `pnpm test:workstation-install-fixture`.
- Run Rust formatting and strict Clippy because Rust source changed.
- Run `pnpm validation:select -- --base 7dd12436` and every additional focused
  check selected for the touched surface.

### Packet C: Installed Runtime Gate

- Build the corrected binary without changing release version metadata.
- Install the reviewed runtime through the repo-supported workstation path.
- Verify installed binary and unit provenance.
- Enable the interlock and backup timers.
- Require one completed interlock service result with exit status 0.
- Require dashboard local and public HTTP 200, Guacamole ids `1` and `2`
  visible to current users, `wsl-chrome-3` still bound to route
  `guacamole:1` and display `:10`, and both doctors successful.

## Hard Stops

- Stop on a second unexpected live interlock failure.
- Stop before closing, relaunching, or navigating `wsl-chrome-3`.
- Stop on installed binary or unit provenance mismatch.
- Stop if the dashboard, Guacamole route, or backup timer does not recover
  exactly after the single installed pass.

## Rollback

Keep the interlock timer disabled if the corrected installed pass fails. The
dashboard may be started directly, and the daily backup timer may remain
enabled independently. The pre-repair Guacamole PostgreSQL dump at
`~/.agent-browser/backups/guacamole-postgres/guacamole-postgres-20260803T162312-171716168Z.dump`
is the recovery anchor for the earlier permission repair.

## Acceptance Criteria

- the red regression fails for the self-quiescing set and passes after the
  minimal repair;
- focused and selected validation is green;
- the installed interlock completes once with result `success` and exit status
  0;
- dashboard local and public routes return HTTP 200 after the pass;
- the Guacamole connection and `wsl-chrome-3` route/display mapping remain
  unchanged and ready;
- the daily backup timer is enabled and active;
- source, installed runtime, and live readbacks are recorded separately.

## Outcome

The source repair is implemented and deterministic validation is green. The
regression first failed with the interlock service present in the quiesce set,
then passed after the service was removed while the dashboard and both timers
remained covered.

The corrected `0.28.0` candidate is installed with SHA-256
`23e71f0ffd8e75355719896a71d09849f57bf6c7e5c417eaf366e8489405d684`.
The installed payload reports ready and its manifest binds that exact binary
and all five user units. The workstation apply stopped at the existing
interactive sudo gate because the root-owned privilege helper differs from the
bundled helper. Payload materialization had already replaced the user binary,
so twenty active daemons now truthfully report the prior executable and cannot
be converged without coordinated session restarts.

The installed runtime gate therefore stopped before closing, relaunching, or
navigating any browser session. The interlock timer remains disabled. The
dashboard was restored directly and returns HTTP 200 locally and through the
public HTTPS route. The daily PostgreSQL backup timer is enabled and active.
Read-only route readiness is `ready`: connections `1` and `2` each retain
three of three READ grants, Route A remains `guacamole:1` on display `:10`,
and Route B remains `guacamole:2` on display `:11`.

After the initial stop, the operator reported that `wsl-chrome-3` was no
longer reachable. Its retained session had no browser and its prior display
allocation was orphaned. One bounded Route A recovery relaunched the durable
profile on `:10`; a canonical-allocation reattachment then reused its restored
ChatGPT target and persisted browser, display, route, and pool state as ready.
This user-directed recovery reduced the stale daemon count from twenty to
nineteen without touching the remaining owners.

The remaining gate is one operator-coordinated maintenance window that can
refresh the root-owned helper interactively, hand off all nineteen other
active daemon sessions, run one installed interlock pass to exit status 0, and
only then re-enable its recurring timer. Plan 0091 remains blocked until that
exact gate is completed.
