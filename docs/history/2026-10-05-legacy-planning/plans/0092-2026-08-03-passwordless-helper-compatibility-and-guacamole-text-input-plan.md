# Plan 0092: Passwordless Helper Compatibility and Guacamole Text Input Defaults

Date: 2026-08-03
Status: Complete
Lane: P92
Predecessor: P91
Source requirements: recurring installed workstation actions must remain
passwordless after bootstrap; Guacamole connections must default to text input

## Goal

Remove the false interactive-sudo gate caused by byte-only drift between a
compatible installed privileged helper and the newly bundled helper. Preserve
the narrow root-owned helper boundary and prove recurring workstation
reconciliation does not request credentials after the one-time bootstrap.

Make Guacamole text input the effective default for existing and new browser
origins through a versioned, one-time browser-local preference migration. A
user may still select another input method after that migration.

## Current Evidence

- The installed helper is root-owned, executable, allowed by the narrow
  passwordless sudoers rule, and successfully implements `check`,
  `status-json`, `ensure-rdp-route-user`, `restart-xrdp`, and
  `grant-display-access`.
- The installed helper reports its route desktop session, filesystem and
  abstract X11 socket support, and bounded xhost timeout ready.
- The privilege installer nevertheless requires exact helper bytes and the
  newer `verify-install` command. That byte-provenance check sends an otherwise
  ready repeat installation through `sudo -v`.
- Guacamole 1.5.5 stores the default input method in browser local storage
  under `GUAC_PREFERENCES`. Upstream initializes it to `none`, and stored
  preferences override that compiled default for every future connection from
  the same browser origin.
- Guacamole extensions may load JavaScript before Angular bootstraps. This
  provides a supported deployment surface for a one-time preference migration
  without rebuilding or mutating the pinned upstream image.

## Scope

- define compatibility from the installed helper's bounded command and status
  contract instead of exact bundled bytes;
- keep exact helper provenance visible as advisory drift;
- keep incompatible or missing helpers behind the one-time interactive
  bootstrap boundary;
- add a fixture proving a byte-different compatible helper is reused without
  `sudo -v` or privileged file installation;
- add a versioned Guacamole JavaScript extension that migrates the browser-local
  input method to `text` once per browser origin;
- package and mount the extension through workstation Guacamole assets;
- prove the extension preserves later user overrides;
- reconcile the live Guacamole service and prove the extension is loaded and
  text input is effective without changing connection identity or route state;
- update help, README, agent skill, docs site, roadmap, runbook, and a durable
  validation note.

## Non-Goals

- granting passwordless access to arbitrary shell commands, package managers,
  file installers, or user-selected payloads;
- adding a passwordless self-updater for root-owned code;
- changing Guacamole connection IDs, RDP route users, displays, credentials,
  or browser session ownership;
- forcing text input again after a user deliberately chooses another input
  method;
- formal release preparation or version advancement;
- touching the operator-owned untracked `--full-page` file.

## Execution

### Packet A: Public Regressions

- Extend the privilege clean fixture with a byte-different but
  contract-compatible installed helper.
- Require the repeat apply to perform only bounded `sudo -n` capability probes,
  with no `sudo -v`, no sudoers replacement, and no helper replacement.
- Extend the Guacamole asset test with extension manifest, JavaScript migration,
  compose mount, and generated bundle checks.
- Exercise the migration against empty, prior-default, and already-migrated
  browser-local preferences.

### Packet B: Minimal Implementation

- Add a shell compatibility probe that requires the helper's `check` and
  `status-json` contracts and the exact capabilities used by recurring runtime
  operations.
- Align install doctor and remote-view doctor with the same compatibility
  contract while retaining exact provenance as an advisory field.
- Add the Guacamole extension source and deterministic bundle generator, embed
  the resulting artifact in workstation payloads, and mount it through the
  Guacamole home template.

### Packet C: Validation and Live Reconciliation

- Run the two red tests green, the relevant Rust module tests, workstation
  install fixture, formatting, strict Clippy, and all checks selected from the
  whole slice base.
- Run the source privilege installer against the provisioned host and require
  a successful repeat apply without `sudo -v` or privileged writes.
- Back up current Guacamole state, reconcile only the Guacamole web container,
  and prove extension load, local and public HTTP readiness, unchanged route
  rows, and effective text input in the operator browser.

## Hard Stops

- Stop if compatibility can be satisfied without the bounded helper command
  set or required X11 and route desktop capabilities.
- Stop before widening sudoers beyond the fixed root-owned helper path.
- Stop before any helper update mechanism can install user-controlled bytes as
  root.
- Stop if Guacamole reconciliation would delete or recreate its PostgreSQL
  volume or managed connection rows.
- Stop before closing, relaunching, or navigating an unrelated browser session.

## Rollback

The privilege change is source-only until a reviewed candidate is installed;
the existing root helper and sudoers file remain untouched. The Guacamole
extension can be removed from the template mount and the web container
recreated without changing PostgreSQL data. Preserve a pre-change PostgreSQL
backup and exact route readback before live reconciliation.

## Acceptance Criteria

- a compatible byte-different helper causes no interactive sudo boundary and
  no privileged installation on repeat apply;
- a missing or incompatible helper still requires the one-time bootstrap;
- both doctor surfaces report compatible helper readiness and distinguish it
  from exact bundled provenance;
- sudoers remains restricted to the fixed root-owned helper executable;
- Guacamole loads the agent-browser defaults extension;
- empty and prior-default browser origins migrate once to text input;
- a user override after migration survives later Guacamole loads;
- live Guacamole connection identities, permissions, route users, and display
  bindings remain unchanged;
- focused and selected validation is green;
- source, installed runtime, and live activation state are reported separately.

## Outcome

The privilege installer now decides recurring readiness from the installed
helper's root ownership, fixed path, passwordless `check` and `status-json`
commands, and the exact route-desktop and display-access capabilities used by
the workstation runtime. Exact bundled-byte provenance remains visible but is
advisory when that runtime contract passes. Route-user setup and display-access
granting no longer fall back to direct sudo commands. The AppArmor readiness
gate is also kernel-aware: an AppArmor-enabled host still requires the bounded
managed Chrome policy, while this WSL2 kernel reports AppArmor disabled and no
longer triggers an impossible repeat bootstrap.

The host-provision regression first crossed a second `sudo -v` after a
compatible local policy annotation, then passed after capability-based policy
validation. It additionally proves an AppArmor-disabled WSL-like rerun needs
only the two noninteractive helper probes. A live source-installer apply with
workstation dependencies returned `No privileged changes were needed` without
prompting. The installed 0.28.0 doctor independently reports the v2 helper
ready and `requiresInteractiveSudo: false`; the installed binary itself remains
the prior reviewed candidate because replacing it would make the active daemon
owners stale outside this slice.

The Guacamole defaults extension is packaged by the source workstation
installer and mounted through `GUACAMOLE_HOME`. A prechange PostgreSQL backup
was published at
`~/.agent-browser/backups/guacamole-postgres/guacamole-postgres-20260803T172928-655720657Z.dump`
with SHA-256
`84543945df97b756b2cee9ed529622fd5f5e7755a5ee419e4db6cd978f06c4b7`.
Only the Guacamole web container was recreated; PostgreSQL and guacd retained
their container identities. Guacamole logged the extension as loaded, its
served application JavaScript contains the migration, and a disposable fresh
origin read back `inputMethod: text` with migration version `1`. Connections
`1` and `2`, 22 parameters, and 6 permissions remained unchanged.

During final verification, the durable `wsl-chrome-3` profile had a dead saved
DevTools port and no browser process. The operator-directed Route A recovery
reused its restored ChatGPT target and returned browser, display `:10`, route
`guacamole:1`, stream, and operator access all ready. The public Guacamole path
returns its expected authentication redirect followed by an HTTP 200 login
page. Source and live activation are complete; installing the new source
candidate and handing off the remaining stale daemons stays in Plan 0091's
coordinated maintenance window.
