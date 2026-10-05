# Plan 0078: Guacamole Route Fixture Recovery Interlock

Date: 2026-07-27
State: BLOCKED
Lane: P78
Source incident: last30days Plan 0012 post-reboot route preflight

## Current State

The deterministic controller repair is retained, but the replacement live
attempt collapsed both same-user XRDP routes onto display `:10`. Issue #85 owns
selection and proof of a distinct route-isolation mechanism. No route sync,
display restoration, or provider retry is authorized by this reconciliation.

## Goal

Make the recurring local-runtime convergence interlock recover the supported
two-route Guacamole/XRDP topology after an empty or reinitialized Guacamole
PostgreSQL data directory, then prove that route fixtures, live displays,
permissions, retained service state, and remote control converge without
touching browser authentication or launching an application browser.

## Current Evidence And Diagnosis

- At 2026-07-27 11:46:23 UTC, the bound Guacamole PostgreSQL data directory
  ran `initdb`; its current database contains zero
  `guacamole_connection` rows and zero connection permissions.
- The Guacamole web, PostgreSQL, guacd, and host XRDP services are reachable,
  but `pnpm --silent test:rdp-guac-route-pool-readiness -- --report-only`
  reports `provision_second_guacamole_rdp_connection` and an empty route pool.
- `agent-browser-runtime-interlock.timer` is enabled and active, while its
  latest convergence receipt is unsuccessful and contains no selected
  remedies.
- `scripts/converge-local-runtime.js` always ensures the Guacamole schema, but
  `routeDisplayRecoveryRequired()` only recognizes display-session recovery
  actions. It does not recognize the fixture-provisioning action and therefore
  never invokes the existing guarded
  `sync:rdp-guac-existing-user-route-pool` command.
- The remote-view acquisition preflight is behaving correctly: with no
  explicit display allocation, browser-owned display, or available route-pool
  entry, it fails before browser creation.

This evidence proves a convergence-controller coverage defect. It does not
prove why the persistent directory became empty, and it is not evidence of X,
Facebook, LinkedIn, profile, cookie, or account-authentication failure.

## Scope

- add a typed convergence remedy for the exact supported empty-route-fixture
  state;
- reuse the guarded existing-user route-pool sync command rather than
  duplicating SQL or credential handling;
- re-read doctors after fixture repair before opening route displays;
- retain the existing display restoration, access grant, service reconcile,
  and final doctor sequence;
- add deterministic regression coverage for empty route fixtures and the
  recovery order;
- run one authorized live recovery attempt and preserve a privacy-safe
  convergence receipt;
- investigate the data-directory reset far enough to record whether a current
  recoverable backup exists and what remains unexplained.

## Non-Goals

- a second last30days route-open attempt or any application-browser launch;
- DOM inspection, login, checkpoint, CAPTCHA, cookie, credential, or source
  acquisition work;
- replacing Guacamole, PostgreSQL, XRDP, Docker Compose, or the static
  two-route design;
- creating new XRDP users or rotating existing credentials;
- broad database restore, destructive reset, or deletion of current runtime
  state;
- claiming a root cause for the PostgreSQL reset without evidence.

## Owned Write Surfaces

- `scripts/converge-local-runtime.js`;
- `scripts/test-local-runtime-convergence.js` and one focused fixture-backed
  regression harness if source-shape coverage cannot prove the behavior;
- `package.json` only if a new focused validation command is needed;
- the narrow runtime/operator documentation that states the automatic
  interlock contract;
- this plan, `ROADMAP.md`, `RUNBOOK.md`, and a privacy-safe validation note;
- live Guacamole route records, permissions, route desktops, and retained
  agent-browser route state only during the separately authorized live gate.

## Design

### Typed fixture recovery

Add a narrow predicate for the doctor action
`provision_second_guacamole_rdp_connection`. In apply mode, that predicate
runs:

```text
pnpm sync:rdp-guac-existing-user-route-pool
```

The existing script remains the only owner of route-row SQL and existing XRDP
credential lookup. That command applies by default and accepts `--dry-run` as
its only mode flag. The convergence controller must not print, copy, or retain
the password.

After the sync, the controller must rerun both doctors. It may proceed to
`open:rdp-route-displays` only when route-pool readiness now exposes the
supported two-entry topology. Partial schema, ambiguous topology, missing
secret material, or a failed sync remains fail-closed.

### Ordered convergence

The apply sequence is:

```text
ensure Guacamole schema
  -> inspect route-pool readiness
  -> reconcile retained service state
  -> provision missing supported route fixtures when explicitly requested
  -> rerun doctors
  -> restore missing route displays when explicitly requested
  -> rerun doctors
  -> grant display access when explicitly requested
  -> final doctors and retained receipt
```

Dry-run mode remains read-only. No generic doctor string may be converted into
an arbitrary shell command.

### Recovery durability

Record the current backup/restore truth for the Guacamole PostgreSQL bind
mount. If no usable backup exists, document that limitation and a separate
recommended durability packet; do not expand this repair into an unbounded
backup-system redesign.

## Execution Packets

### Packet A | Deterministic controller regression

Status: completed

- add a failing regression proving that the provisioning next action selects
  the existing guarded sync command;
- prove the controller reruns doctors before display restoration;
- prove unrelated next actions do not provision fixtures;
- prove default/dry-run execution does not mutate runtime state.

Terminal condition: focused regression passes and the diff is limited to the
owned source/test surfaces.

### Packet B | Contract and operator documentation

Status: completed

- update the local-runtime convergence contract and post-reboot recovery note;
- distinguish deterministic fixture recreation from database backup/restore;
- record the reset evidence and unresolved causal question without private
  values.

Terminal condition: docs match the tested sequence and name all fail-closed
gates.

### Packet C | Authorized live recovery

Status: blocked after the authorized replacement attempt

- capture pre-mutation counts and current doctor/convergence receipts;
- run one convergence apply attempt;
- verify exactly two supported RDP connection records, distinct route targets,
  required read permissions, and two live route-display sockets;
- verify service reconciliation exposes available route-pool entries and
  remote-view doctor reports ready remote control;
- do not launch an application browser.

Terminal condition: the full route substrate is ready, or the first typed
failure is retained and live mutation stops.

The 2026-07-27 authorized attempt stopped before route mutation because the
plan supplied `--apply` to an existing-user sync command that applies by
default and accepts only `--dry-run`. The retained receipt records status 2
for `provision_rdp_guac_route_fixtures`; the database still has zero route
connections. The controller, fixture test, and command example are corrected,
but the hard bound prohibits a second live attempt without new operator
authorization.

The operator authorized one replacement attempt on 2026-07-27. Fixture
provisioning then succeeded and created exactly two authorized Guacamole RDP
connections with distinct configured target identities. Display restoration
failed because XRDP attached both same-user connections to display `:10`
instead of allocating display `:11`. The retained receipt records
`restore_rdp_route_displays` status 1, and report-only readiness identifies
`repair_rdp_route_display_session` for route B.

### Packet D | Installed interlock and closeout

Status: blocked on a successful Packet C retry

- verify the installed timer uses the repaired source/runtime;
- run one report-only readiness check after the recurring interlock interval;
- preserve the final convergence receipt and current Git/installed/runtime
  identities separately;
- update Plan 0012 with the exact resume gate.

Terminal condition: the recurring interlock remains successful with no
duplicate routes, or the packet closes blocked with a typed residual defect.

## Hard Bounds And Stop Conditions

- one implementation attempt and one review/rework cycle per packet;
- one live fixture-recovery apply attempt;
- no second database reset, route sync, display-open, or route-open retry;
- stop on partial Guacamole schema, more than the expected route topology,
  missing XRDP secret material, ambiguous existing route ownership, or any
  command that would create users or rotate credentials;
- stop before application-browser launch, authentication inspection, or
  last30days canary work;
- no subagent delegation: this repair crosses one live shared route substrate
  and requires a single critical-path owner.

## Validation

Required deterministic checks:

```text
pnpm test:local-runtime-convergence
pnpm test:rdp-guac-postgres-hardening
pnpm test:rdp-guac-route-pool-readiness -- --report-only
node --check scripts/converge-local-runtime.js
git diff --check
```

Add and run a fixture-backed behavior test if the existing source-contract
test cannot prove command selection, doctor re-read order, and fail-closed
behavior.

Required live checks, only after explicit Packet C authorization:

- pre/post Guacamole connection and permission counts;
- report-only route-pool readiness;
- route-display inspection;
- `agent-browser service reconcile --json`;
- `agent-browser doctor remote-view --json`;
- `agent-browser install doctor --json`;
- retained `~/.agent-browser/convergence/local-runtime-latest.json`.

## Acceptance Criteria

- the provisioning next action invokes exactly one guarded existing-user
  route-fixture sync in apply mode and none in dry-run mode;
- doctor evidence is refreshed after fixture provisioning before display
  restoration is considered;
- an empty supported Guacamole database converges to exactly two distinct
  authorized RDP routes without creating users or exposing credentials;
- the two route desktops and X11 sockets are live, service route entries are
  available, and remote control is ready;
- the recurring interlock produces a successful retained receipt on the
  recovered topology without duplicating routes;
- no application browser, source authentication, or last30days acquisition is
  touched;
- the PostgreSQL reset cause and backup availability are reported truthfully
  as proved, disproved, or unresolved.

## Current Blocker And Resume Gate

- No usable PostgreSQL dump, archive, snapshot unit, or documented restore
  workflow was found for the Guacamole bind mount.
- The packaged initialization SQL can recreate schema only; it cannot restore
  route rows or permissions.
- Container history shows fresh initialization events on July 4, 15, 21, 26,
  and 27. No repo-owned script was found that removes the bind mount, so the
  recurring external cause remains unresolved.
- Route fixture recovery now has exactly two Guacamole RDP connections, the
  required read grants, and two distinct configured target identities.
- XRDP `Policy=Default` allocates by user and negotiated bit depth. Both
  existing-user Guacamole connections were treated as the same session:
  XRDP created display `:10` for route A, then logged route B as a reconnection
  to display `:10`. Route B therefore has no `:11` X11 socket.
- The configured Guacamole color depths of 24 and 32 did not create distinct
  XRDP session-allocation keys on this installed XRDP 0.9.24 runtime.
- The replacement live attempt is consumed. Do not rerun route sync or display
  restoration under P78.
- Resume only through a new bounded plan that chooses and validates an explicit
  isolation mechanism, such as route-specific users or a reviewed XRDP session
  policy change, before any further live mutation.

Recommended durability follow-up: open a separate bounded packet for scheduled
`pg_dump`, retention, and restore validation of the Guacamole database after
route recovery. That work is intentionally outside P78.

## Definition Of Done

The supported Guacamole route substrate self-recovers from the observed empty
fixture state through the installed recurring interlock, with deterministic
tests and one bounded live receipt, or the first residual typed blocker is
preserved without broad retry. Only then may last30days Plan 0012 request new
authorization for its route-open and serialized canary sequence.
