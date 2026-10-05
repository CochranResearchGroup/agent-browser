# Plan 0079: Route-Specific Guacamole RDP Isolation Repair

Date: 2026-07-28
Status: Complete
Lane: P79
Predecessor: P78
Source incident: post-reboot agent-browser remote-view readiness failure

## Goal

Repair the supported two-route Guacamole/XRDP substrate after the installed
XRDP runtime collapsed two same-user connections onto one display. Migrate the
two managed routes in place to the existing route-specific Linux users, teach
the convergence interlock to select that repair, and prove one successful
manual convergence plus one recurring interlock pass without launching an
application browser or inspecting authentication state.

## Current Evidence And Causal Diagnosis

- Guacamole has exactly two managed RDP connections and the required read
  grants. Route A and route B currently use the same Linux user with configured
  Guacamole color depths 24 and 32.
- Route A authentication created XRDP display `:10`. Route B authentication
  succeeded, but XRDP logged it as a reconnection to the same user's display
  `:10`; no route B display `:11` socket exists.
- XRDP 0.9.24 is installed with sesman `Policy=Default`. The configured
  Guacamole color-depth difference did not survive negotiation as a distinct
  session-allocation key.
- Route readiness therefore reports `repair_rdp_route_display_session`, and
  the retained convergence receipt records display restoration status 1.
- The existing route-specific Linux users `agent-browser-rdp-a` and
  `agent-browser-rdp-b` and their guarded secret entries already exist.
- The route-specific setup command passes its read-only isolation gate, but it
  upserts different canonical connection names. Applying it without a
  migration would leave the two legacy managed rows and create two more.
- The persisted display-name configuration is `:10` and `:11`. Once
  route-specific sessions are created, actual allocation can differ, so live
  inspector evidence must take precedence over stale configured display names.

The root cause is a false same-user isolation assumption in the managed route
fixture, not a Guacamole authentication failure, repository synchronization
problem, application-browser failure, or source-account authentication issue.

## Considered Repairs

1. Keep one user and alter host-wide XRDP session policy.
   This changes the allocation contract for every XRDP consumer and requires
   privileged configuration plus a service restart.
2. Keep one user and seek another Guacamole parameter distinction.
   The installed runtime already disproved the supported color-depth
   distinction, and another metadata-only distinction would not provide an
   identity boundary.
3. Migrate the two managed rows to the existing route-specific users.
   This uses already-provisioned identities, changes only the owned Guacamole
   rows, needs no sudo or credential rotation, and gives XRDP an explicit
   allocation boundary.

Decision: use route-specific users and preserve the two existing connection
identities by migrating supported legacy rows in place.

## Scope

- add a guarded route-specific user sync command that migrates the two
  supported legacy managed rows in place;
- fail closed on duplicate, mixed canonical-plus-legacy, partial-schema, or
  missing-secret states;
- remove stale `color-depth` parameters from migrated managed routes because
  color depth is no longer an isolation mechanism;
- make convergence choose route-specific migration for proven same-user
  collapse and for empty fixtures when route-specific identities are ready;
- refresh doctors after migration and before opening displays;
- prefer live inferred route displays over stale configured display names;
- correct operator, CLI, docs-site, and agent-skill claims about isolation;
- run one controlled live repair attempt and one recurring interlock proof;
- preserve privacy-safe deterministic and live receipts.

## Non-Goals

- changing XRDP policy, restarting XRDP, creating Linux users, or rotating
  passwords;
- deleting unrelated Guacamole connections or rebuilding the database;
- launching an application browser, opening a source site, or inspecting
  cookies, credentials, CAPTCHA, checkpoints, or account authentication;
- solving the recurring Guacamole PostgreSQL reinitialization cause or adding
  backup infrastructure;
- formal release preparation.

## Owned Write Surfaces

- `scripts/sync-rdp-guac-route-specific-user-pool.sh`;
- `scripts/converge-local-runtime.js`;
- route-pool readiness display-selection code;
- focused deterministic tests and `package.json` commands;
- the narrow README, CLI help, agent skill, docs-site, and inline comments that
  describe route isolation and convergence;
- this plan, `ROADMAP.md`, `RUNBOOK.md`, and one validation note;
- during the live gate only, the two managed Guacamole connection rows and
  their parameters/permissions, the two Guacamole viewer sessions, retained
  convergence state, and the user-scoped interlock unit.

## Repair Design

### Guarded in-place migration

The new sync command applies by default and supports only `--dry-run`. It must:

- verify the Guacamole PostgreSQL container and required schema;
- require both route-specific Linux users and both route-specific username and
  password secret pairs, with distinct usernames;
- never print secret values;
- identify only the exact canonical names `Agent Browser RDP Route A/B` and
  legacy names `Agent Browser RDP Existing User Route A/B`;
- fail if a route has both canonical and legacy rows or more than one exact
  match;
- update a single legacy row in place and rename it, update a single canonical
  row, or insert a missing canonical row;
- set hostname, port, route-specific username, password, security,
  ignore-certificate, and resize settings;
- delete `color-depth` from the two managed routes;
- grant required read permissions without altering unrelated rows;
- use a transaction, PostgreSQL error-stop behavior, post-write invariants,
  commit, and checkpoint.

The postcondition is exactly one canonical A row and one canonical B row, no
legacy managed rows, distinct usernames, and unchanged unrelated connections.

### Typed convergence selection

The convergence controller selects the route-specific sync when:

- readiness requests fixture provisioning and route-specific identities are
  ready; or
- readiness requests display repair, both selected routes use the same target
  user, and route-specific identities are ready.

It retains existing-user fixture sync only as a compatibility fallback when
route-specific identities are unavailable and no same-user collapse has been
proved. After either sync, convergence reruns both doctors before considering
display restoration.

### Live display authority

Route readiness must prefer the display names inferred from current Xorg
processes and X11 sockets over configured display-name hints. Configuration is
only a fallback when live inference is unavailable.

## Execution Packets

### Packet A | Plan and failing behavior tests

Status: completed

- capture the diagnosis and repair invariants in this plan;
- add one failing convergence fixture for proven same-user collapse;
- add one failing sync-command fixture for guarded in-place migration;
- add one failing display-selection regression for live evidence precedence.

Terminal condition: each behavior fails for the intended missing capability.

### Packet B | Implementation and deterministic validation

Status: completed

- implement the migration command, typed convergence selection, and display
  precedence;
- update all required user-facing documentation and inline comments;
- run focused tests, PostgreSQL hardening, syntax checks, selected validation,
  and one bounded review/rework cycle.

Terminal condition: deterministic gates pass and no live state has changed.

### Packet C | One controlled live repair

Status: completed

- capture pre-mutation route rows, permissions, doctors, displays, and retained
  receipt;
- close only the two failed Guacamole viewer sessions;
- run one convergence apply with publication skipped;
- verify exactly two canonical managed routes, distinct route users, required
  permissions, two distinct route-specific Xorg displays and sockets,
  reconciled available routes, remote-view readiness, and install doctor;
- stop at the first typed failure and do not retry.

Terminal condition: the route substrate is ready or one bounded failure receipt
is retained.

### Packet D | Recurring interlock and closeout

Status: completed

- verify the installed unit invokes the repaired convergence surface;
- run one interlock service pass and restore the enabled/active timer;
- verify it succeeds without duplicate rows or display collapse;
- update plan, roadmap, runbook, validation note, Git state, and advisory
  Graphiti memory as applicable.

Terminal condition: source, installed unit, runtime, and retained receipt are
reported separately and the recurring timer is healthy.

## Hard Bounds And Stops

- one implementation cycle and one review/rework cycle;
- one live migration/convergence attempt and no automatic retry;
- no XRDP policy mutation, XRDP restart, user creation, password rotation,
  database reset, or unrelated connection deletion;
- stop on ambiguous managed rows, partial schema, missing route-specific user
  or secret material, non-distinct usernames, failed postcondition, or any
  unexpected viewer/session ownership;
- stop before application-browser launch or source-authentication work;
- no subagent delegation receipt: `not_spawned`, because all mutations cross
  one shared Guacamole/XRDP route substrate and require one critical-path
  owner.

## Validation

Required deterministic checks:

```text
pnpm test:local-runtime-convergence
pnpm test:rdp-guac-route-specific-user-sync
pnpm test:rdp-guac-route-pool-readiness
pnpm test:rdp-guac-postgres-hardening
pnpm validation:select -- --base 87cc5dfdab8765cc99ad4750b31095284f3f9eb3
node --check scripts/converge-local-runtime.js
git diff --check
```

Required live checks after Packet B:

- exact managed connection names, ids, protocols, and usernames with no secret
  output;
- exact read-permission counts;
- route-display process and X11 socket inspection;
- `pnpm --silent test:rdp-guac-route-pool-readiness -- --report-only`;
- `agent-browser service reconcile --json`;
- `agent-browser doctor remote-view --json`;
- `agent-browser install doctor --json`;
- retained `~/.agent-browser/convergence/local-runtime-latest.json`;
- one `agent-browser-runtime-interlock.service` result and active timer readback.

## Rollback

Before mutation, retain the two managed connection ids, names, usernames, and
non-secret parameter keys. If the SQL transaction fails, it rolls back. If the
database migration commits but later display restoration fails, stop with the
route-specific canonical rows intact because they are the selected repair;
do not rotate credentials or recreate legacy same-user rows. Viewer sessions
may be closed without deleting their XRDP desktops. A later rollback requires a
new explicit plan because restoring the disproved same-user topology would
reintroduce the incident.

## Acceptance Criteria

- deterministic tests prove migration selection, SQL guards/postconditions,
  doctor refresh ordering, and live display precedence;
- exactly two canonical managed Guacamole RDP routes remain, with distinct
  route-specific users and required read grants;
- two distinct route-specific Xorg displays and X11 sockets are live;
- route readiness and remote-view doctor are ready;
- one manual convergence and one recurring interlock pass succeed without
  duplicate routes;
- no application browser or authentication surface is touched;
- the PostgreSQL reset cause and backup gap remain explicitly separate and
  unresolved.

## Closeout

- The guarded migration preserved connection ids `1` and `2`, renamed them to
  the canonical route A/B names, assigned `agent-browser-rdp-a` and
  `agent-browser-rdp-b`, removed `color-depth`, and retained two read grants
  per route.
- Guacamole opened route-specific Xorg displays `:11` and `:12`; the inspector,
  route-pool readiness, single-route remote control, and many-to-many
  prerequisites all report ready.
- The first final doctor exposed a cwd-dependent inspector path: the doctor
  ran the readiness helper from `scripts/`, while the helper tried to open
  `scripts/inspect-rdp-route-displays.js` relative to that directory. A
  red-green follow-up resolves the inspector from the module URL. No second
  database migration or display restoration was run.
- A read-only convergence pass then succeeded. The installed recurring
  interlock service completed with result `success`, exit status 0, and no
  route fixture, display restoration, or access-grant steps. Its timer is
  enabled, active, and waiting.
- Source commits `2dcac761` and `641f45ae` are pushed to `origin/main`.
- The installed binary was not replaced. `agent-browser install doctor`
  reports zero issues and seven converged runtimes; the installed interlock
  executes the repaired repo scripts through its repo working directory.
- No application browser or source-authentication surface was opened.
- The recurring Guacamole PostgreSQL reinitialization cause and lack of a
  validated backup remain a separate unresolved durability issue.
