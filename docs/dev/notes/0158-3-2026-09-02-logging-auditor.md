# Plan 0158 W3 Logging Completeness Auditor

Date: 2026-09-02

Plan: `docs/dev/plans/0158-2026-09-02-frozen-candidate-historical-failure-stress-campaign.md`

State: COMPLETE

## Outcome

W3 adds a provider-free cross-surface logging auditor at
`scripts/lib/p158-logging-auditor.js`. It reconstructs immutable causal
envelopes from ingress requests, immediate responses, durable jobs, terminal
events, traces, incidents, dashboard projections, artifacts, and redaction
receipts. It reports exact expected, observed, missing, duplicate, conflicting,
and leaking evidence without parsing error prose as a substitute for missing
structure and without mutating or repairing its input.

The auditor is deliberately independent of the campaign controller. W2
preserves append-only evidence; W3 adjudicates whether that evidence contains a
complete and internally consistent Service causal chain.

## Frozen Defect Corpus

`docs/dev/fixtures/p158/logging-causal-envelopes.v1.json` contains 13 fully
materialized, synthetic, production-shaped envelopes:

- one complete causal chain;
- one order-shuffled copy that must remain clean;
- missing trace;
- duplicate terminal record;
- conflicting structured projection;
- causal timestamp inversion;
- null structured failure;
- null immutable provenance;
- one-transport-only terminal visibility;
- broken parent;
- effect-uncertain outcome paired with unsafe same-request retry;
- synthetic sensitive canary leakage; and
- partial capture with an explicit gap.

The null fixtures model the production defect on the terminal durable job
itself, not only on downstream event or trace projections. This protects the
39 failed or timed-out retained rows found in W1.

The fixture and report authorities are:

- `docs/dev/contracts/p158-logging-causal-fixtures.v1.schema.json`; and
- `docs/dev/contracts/p158-logging-audit-report.v1.schema.json`.

Every report envelope retains its fixture correlation, request identity,
expected and observed surface roles, source-record count, completeness state,
and finding IDs. Every finding has deterministic identity, defect code,
severity, affected surface and records, expected and observed evidence, and
`repairAttempted=false`.

## Detection Contract

The auditor detects exactly 11 frozen classes:

1. `missing_record`
2. `duplicate_terminal`
3. `conflicting_projection`
4. `timestamp_inversion`
5. `null_failure`
6. `null_provenance`
7. `one_transport_only`
8. `broken_parent`
9. `effect_retry_conflict`
10. `capture_gap`
11. `sensitive_value_leak`

Grouping uses causal identifiers, not caller labels. Failure and provenance
values are compared by stable canonical hashes across terminal projections.
Parent references are resolved explicitly and checked against clock-offset
normalized timestamps. Effect and retry values are reconciled, including an
explicit rejection of `effect_uncertain` with `retry_same_request`.

The sensitive scanner matches synthetic canary values and forbidden field
classes across records and artifact metadata. Findings retain only the canary
identifier and value hash, never the canary value. Redacted and excluded
sentinels are accepted; unreceipted partial, missing, or redacted capture is a
gap.

## Integration Corrections

Parallel integration was held until the auditor emitted the committed report
schema and expanded the fixture-set wrapper. A later schema check added
required nullable `fixtureId` correlation so a combined audit could be traced
back to each source seed.

Primary-agent review then broadened terminal projection inspection from events
and traces to terminal immediate responses, jobs, events, traces, and dashboard
projections. The null corpus now marks its durable jobs terminal and failed,
and a dedicated assertion proves both historical null defects are reported on
the `durable_job` surface.

## Validation

`pnpm test:p158-logging-auditor` passes. It:

- compiles both schemas with strict Ajv 2020 and format checking;
- validates the 13-fixture corpus and emitted report;
- proves deterministic output and no input mutation;
- produces exactly one finding and one matching summary count for every frozen
  defect class;
- keeps both the complete and reordered envelopes clean;
- reconciles exact expected and observed record totals;
- preserves per-envelope finding joins and state classification; and
- detects null failure and null provenance on terminal durable jobs.

No installed runtime, browser, service, provider, route, display, Profile, or
production state was touched.

## Remaining Gate

W4 is next. It must build the external-ingress runner and durable-handoff
oracle. Provider-free fixtures must first prove a good public HTTPS path and
reject loopback, private, link-local, `.local`, raw provider, wrong-browser,
and duplicate-launch paths. Actual E2 ingress remains a W6 preparation and W8
execution effect.
