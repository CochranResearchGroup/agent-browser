# Plan 0142 Revision CAS Checkpoint

Date: 2026-08-29

Plan: 0142

State: CHECKPOINT COMPLETE

## Decision and evidence

Commit `452f0230` adds a monotonic `stateRevision` to the additive v2 Service
State envelope. JSON-backed mutations now load revision N through a stable
snapshot, apply the in-memory mutation and prepare the complete four-file
transaction before exclusive commit, then re-read durable state under the
file lock. A revision mismatch fails before any file replacement with
`service_state_stale_revision`.

The shared failure classifier projects a stale revision as `no_effect`,
`inspect_before_retry`, and `reload_state_and_replan`. It does not grant
browser reuse or blind retry.

## Green validation

- 20 focused Service State tests, including deterministic stale prepared-write
  rejection and every four-file interruption boundary;
- 20 migration and install rollback tests, including mixed-version and unknown
  field preservation;
- focused stale-revision recourse test;
- Rust format and clippy with warnings denied.

## Remaining gate

Plan 0142 remains open. The next work is the realistic 2.9 MiB contention
fixture and lifecycle reuse, recovery, and hard-block matrix, followed by full
public parity, docs build, and isolated development-runtime acceptance. No
production runtime was touched.
