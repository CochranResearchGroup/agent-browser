# Plan 0128 Shared Stream Supervisor Advisory Hotfix

Date: 2026-08-23

Branch: `hotfix/supervisor-shared-stream-advisory`

State: `SOURCE_ACCEPTED_INSTALL_PENDING`

## Installed Evidence

Transaction `upgrade-1d41d42e-e3bd-4325-ba6b-1df703f195cf` accepted generation
`0.28.0-cf7527ab9003-9e38bacc997a` with binary SHA-256
`cf7527ab9003b8149b9c8761b62ce06a56b56c46e0e036a48dba30ec2d7e25dc`.
All seven workstation readiness axes are true. Runtime multiplicity is one
dashboard, one runtime host, one executable generation, and zero legacy
daemons.

Standalone doctor remained nonzero because the retained
`last30days-home-feed` supervisor manifest reports an inactive/dead unit with
no main PID while its matching published stream remains reachable through the
healthy shared runtime host.

## Repair

A matching reachable stream no longer makes an otherwise quiescent stopped
optional supervisor blocking. The stream can be served by the shared runtime
host after that historical lane has stopped. A reachable port whose published
metadata does not match the manifest remains a blocking port conflict.

The stopped result also remains blocking when the unit itself is active or
has a live main PID. Starting, unavailable, executable-drifted,
restart-exhausted, and port-conflicted supervisors remain blocking.

## Validation

- the exact shared-stream, inactive/dead, no-PID regression failed before the
  repair and passes afterward;
- all 12 session supervisor tests passed;
- canonical Rust formatting, strict Clippy, and diff hygiene passed.

## Runtime Boundary

No supervisor manifest, process, stream, browser, profile, provider, or
accounting state was changed by this source repair. The installed runtime
remains on the accepted generation above until this follow-up is integrated
and transactionally installed.
