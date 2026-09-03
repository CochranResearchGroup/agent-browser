# Plan 0158 W2 Campaign Controller

Date: 2026-09-02

Plan: `docs/dev/plans/0158-2026-09-02-frozen-candidate-historical-failure-stress-campaign.md`

State: COMPLETE

## Outcome

W2 adds a provider-free campaign kernel at
`scripts/lib/p158-campaign-controller.js`. It owns the monotonic controller
state, deterministic dependency-aware schedule, seeded attempts, immutable
freeze inputs, append-only evidence ledger, atomic exclusive artifact writes,
environment-scoped safety stops, exact terminal outcomes, scheduled teardown,
evidence sealing, analysis terminality, and SHA-256 integrity readback.

The controller does not execute browser, service, provider, dashboard, fault,
or cleanup commands. Later work injects those actions through the already
frozen schedule. The controller records their declared attempts and evidence;
it cannot invent a retry or mutate a terminal result.

## Contracts

Two JSON Schema 2020-12 authorities define the persisted surfaces:

- `docs/dev/contracts/p158-campaign-manifest.v1.schema.json` freezes candidate
  identity, all 54 allowed case IDs, schedule sequence and attempt identity,
  dependency edges, no-repair policy, safety policy, and evidence policy; and
- `docs/dev/contracts/p158-campaign-result.v1.schema.json` defines the
  hash-chained ledger envelope and typed controller transition, attempt
  terminal, artifact, safety observation, teardown, evidence seal, and final
  analysis records.

Every ledger record binds to the immutable campaign-manifest digest and its
predecessor digest. Artifact records carry byte count, digest, capture state,
capture gap when applicable, redaction receipts, and parent artifact digests.
The execution-terminal transition persists exact counts for all seven result
states before hashing; the in-memory view is not permitted to diverge from the
on-disk record.

Ajv and `ajv-formats` are direct workspace development dependencies so schema
validation is portable and does not depend on a transitive pnpm-store path.

## Behavioral Guarantees

The controller enforces:

- one claimed run root with exclusive first write;
- deterministic integer seeds and stable topological case ordering;
- immutable registry, candidate, schedule, teardown, and configuration inputs
  after freeze;
- the exact `prepared -> frozen -> executing -> execution_terminal ->
  evidence_sealed -> analyzed` progression;
- one terminal result per scheduled attempt with no overwrite, unscheduled
  attempt, or opportunistic retry;
- explicit prerequisite-loss semantics: a reproduced or new failure does not
  automatically block useful downstream work;
- recursive `skipped_blocked` propagation only from an exact declared lost
  prerequisite, with its terminal record as evidence;
- rejection of terminal evidence that names an artifact not already present in
  the append-only store;
- two-sample numeric resource stops and immediate isolation, leakage, or
  evidence-corruption stops, scoped to the affected environment;
- scheduled teardown only after all ordinary attempts are terminal, with
  teardown failure preserved as a result rather than repaired; and
- sealing only after exact terminal-count closure.

## Integration Corrections

The first independent drafts exposed a useful integration failure: the
controller initially emitted a compact event shape that did not validate
against the fuller manifest and result schemas. W2 was held open until the
controller emitted schema-shaped manifests and ledger records and two
independent validations accepted actual lifecycle output.

Primary-agent review then corrected two additional issues:

1. execution result counts had been attached to the in-memory transition after
   the corresponding file was written and hashed; they are now part of the
   persisted schema-valid payload before hashing; and
2. every non-pass result had defaulted to `blocksDependents=true`; the default
   is now nonblocking, while exact prerequisite loss and safety stops propagate
   explicitly.

The review also made unknown terminal evidence artifact IDs fail closed and
aligned the safety sample field with the frozen `campaignProcessCount` metric.

## Validation

`pnpm test:p158-campaign` passes. The campaign-controller portion contains 11
adversarial tests covering:

- existing-root overwrite rejection;
- reproducible ordering and seeds;
- terminal overwrite and opportunistic retry rejection;
- exact blocked propagation with independent continuation;
- continued dependent execution after a nonblocking observed failure;
- non-monotonic transition rejection;
- post-freeze input mutation rejection;
- atomic artifact write, path overwrite rejection, no temporary residue,
  unknown evidence reference rejection, sealing, and digest verification;
- environment-scoped consecutive safety violations;
- teardown ordering and failure preservation; and
- exact terminal-count closure with equality between persisted and in-memory
  result counts.

The same test compiles both schemas with strict Ajv 2020 plus format checking,
validates the persisted campaign manifest and every ledger record, and
independently recomputes contiguous sequence numbers and predecessor hashes.
The schema lane separately validated a disposable full lifecycle containing
one manifest and nine ledger records.

No installed runtime, browser, service, provider, route, display, Profile, or
production state was touched.

## Remaining Gate

W3 is next. It must build the cross-surface causal logging auditor and
synthetic sensitive-value scanner against deliberately missing, duplicate,
conflicting, reordered, null, and leaking fixtures. The controller can preserve
those findings now, but it does not itself adjudicate response, job, event,
trace, incident, or dashboard completeness.
