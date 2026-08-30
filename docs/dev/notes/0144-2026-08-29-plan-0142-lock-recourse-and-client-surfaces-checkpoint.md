# Plan 0142 Lock Recourse And Client Surfaces Checkpoint

Date: 2026-08-29

Plan: 0142

State: CHECKPOINT COMPLETE

## Integrated evidence

- `edbfb8a0` moves stable snapshot reads off the process mutation mutex, uses a
  shared file lock for stable reads, and acquires the cross-process file lock
  before the process mutex for mutations and transaction recovery.
- `a8968c56` adds phase-aware file-lock recourse, bounded `waitMs`, safe
  `holderOperation` metadata, the versioned JSON schema, generated client
  fields, help, README, docs-site, and repository skill guidance.
- Failed viewport jobs preserve the original error and project
  `effect_uncertain`, `inspect_before_retry`, and `reuseAllowed=false` to the
  response and retained job.
- The dashboard selected-job inspector now renders recourse without creating a
  retry or reuse action.

## Green validation

- all 18 focused `service_store` tests, including four-file crash-boundary
  atomicity;
- all 5 `service_failure` tests;
- service API and MCP parity;
- service contracts no-launch smoke;
- the full no-launch generated service-client suite;
- dashboard inspector action contract smoke;
- Rust format and clippy with warnings denied.

## Remaining gate

Plan 0142 remains open. Next is explicit durable revision and stale prepared
transaction rejection at the commit boundary, followed by deterministic burst
measurement, lifecycle recovery matrix integration, docs build, and isolated
development-runtime acceptance. No production runtime was touched.
