# Plan 0080: Guacamole PostgreSQL Durability Remediation

Date: 2026-07-28
Status: Complete
Lane: P80
Predecessors: P78 and P79

## Goal

Remove the Docker Desktop WSL bind-mount failure mode from Guacamole
PostgreSQL, preserve the current two-route database, add fail-closed cluster
continuity checks, and prove scheduled backup plus isolated restore viability.

## Causal Diagnosis

- The PostgreSQL container has run `initdb` on June 23 and July 4, 15, 21, 26,
  and 27.
- The July 26 and July 27 events align with WSL boot times. The July 26 event
  followed a healthy checkpoint without a PostgreSQL shutdown record.
- Docker declares a bind source at
  `~/.agent-browser/guacamole/data/postgres`, but the running container sees
  that mount as `tmpfs` with device and inode `32:26`.
- The host path and a newly created probe container see the same ext4 directory
  with device and inode `2128:38068951`.
- The host directory stopped changing on July 21 while the running container
  created a new cluster on July 27. The current running cluster identifier is
  `7667172823649325092`; the host-visible retained cluster identifier is
  `7640969869501161507`.
- No repo-owned removal command, user timer, cron entry, or tmpfiles rule was
  found for the data path.

The direct cause is a stale Docker Desktop WSL bind attachment retained by the
long-running container across WSL restarts. PostgreSQL then starts on a fresh
empty `tmpfs` attachment and initializes a new cluster. The existing schema
guard masks the discontinuity by importing schema into any wholly empty
database.

## Scope

- add one durability operator that reports mount and identity continuity,
  creates atomic checksummed custom-format dumps, enforces retention, records
  the current identity, and runs an isolated restore drill;
- make schema assurance fail closed when a recorded cluster identity changes
  or a previously initialized database becomes wholly empty;
- migrate the live database from the WSL bind mount to a Docker named volume
  after a verified pre-migration backup;
- install a user-scoped daily backup timer;
- update operator docs, CLI help, agent skill, roadmap, runbook, and a
  validation note;
- preserve the current two route rows, parameters, permissions, users, and
  live route displays.

## Non-Goals

- restoring the older host-visible July 26 cluster over the current database;
- deleting the old bind directory or any backup;
- changing Guacamole, XRDP, route, browser, or authentication semantics;
- formal release preparation.

## Execution Packets

### Packet A | Red-capable durability contract

Status: completed

Add fixture-backed behavior coverage for stale `tmpfs` detection, continuity
failure, atomic backup metadata, restore-drill cleanup, and named-volume
readiness. The initial run must fail because the durability operator is absent.

### Packet B | Source implementation

Status: completed

Implement the operator, harden schema assurance, install the recurring timer,
and update every operator-facing documentation surface.

### Packet C | Controlled live migration

Status: completed

1. Capture the current cluster identifier and exact Guacamole row counts.
2. Create and validate a current logical backup.
3. Stop Guacamole web access, change only the PostgreSQL data mount to the
   named volume, recreate PostgreSQL, and restore the dump.
4. Require exact route, permission, and required-schema invariants before
   restarting Guacamole.
5. Record the new cluster identity, run an isolated restore drill, and retain
   both the old bind directory and backup.

### Packet D | Recurrence and closeout

Status: completed

Install and run the backup service once, keep its timer enabled, verify remote
control and many-to-many readiness, then commit and push the bounded slice.

## Hard Stops

- no migration without a custom-format dump whose SHA-256 and `pg_restore`
  catalog both validate;
- stop on a source cluster identifier or pre-state count mismatch;
- stop before Guacamole restart if restored schema, route, permission, or
  parameter invariants differ;
- never delete the old bind directory or the pre-migration backup;
- no production restore replacement command is exposed in this packet;
- delegation receipt: `spawned` for the required read-only independent review
  after the single-owner live migration; handle
  `/root/p80_independent_review`, with no reviewer mutation.

## Acceptance Criteria

- PostgreSQL uses a Docker named volume and the running mount is not `tmpfs`;
- the two canonical routes and four read grants survive unchanged;
- schema assurance refuses a recorded identity discontinuity or absent schema;
- a checksummed atomic backup exists outside the database mount;
- an isolated restore drill recreates all required tables and expected route
  counts, then removes only its temporary database;
- the daily backup unit succeeds and its timer is enabled and active;
- remote-control, many-to-many, install, Git, and recurring-runtime readbacks
  remain healthy and are reported separately.

## Completion Evidence

- Pre-migration backup
  `guacamole-postgres-20260728T164852Z.dump` is mode 0600, has SHA-256
  `dd881cb6ca8e936a59f1a105ee0fedfa86942339471c6f53be45a83656e04a72`,
  has a 193-line restore catalog, and has a paired `.keep` retention marker.
- Pre-migration and post-migration restore drills recreated five required
  tables, two routes, and four connection permissions in a temporary database,
  then removed that database.
- PostgreSQL now uses named volume
  `agent-browser-guacamole-postgres-data`; the running data mount is ext4.
- The restored database has cluster identifier `7667622067821133864`, two
  canonical routes, 22 route parameters, and four permissions. The identity
  file records the same identifier.
- The installed backup service completed successfully and its enabled active
  timer is scheduled daily.
- The installed convergence pass completed successfully, route readiness is
  ready, remote control is ready, and many-to-many readiness is ready.
- The old bind directory and all pre-migration backups remain retained.

## Independent Review

The first read-only review found fail-open restore cleanup, identity-rebind,
backup-concurrency, retention, fixture-coverage, and scheduled-runtime gaps.
One bounded remediation pass added strict cleanup verification, guarded
identity establishment, backup continuity and locking, protected retention,
complete-pair selection, behavior fixtures, readiness wait, bounded retry, and
a source-checkout-independent installed helper.

The independent re-review returned `PASS` with no residual blocking findings.
It confirmed live continuity, timer success, installed-helper parity, and zero
temporary restore databases. Git closeout remained with the primary agent.
