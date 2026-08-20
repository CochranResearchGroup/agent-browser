# Plan 0117 Slice A Source Acceptance

Date: 2026-08-20

Status: SOURCE ACCEPTED

Plan: `docs/dev/plans/0117-2026-08-19-runtime-lifecycle-authority-and-convergence-plan.md`

Source baseline: `93503441a735accab2d2414c170f84ff4bab22b7`

## Outcome

Slice A adds evidence and schema without changing lifecycle effects. The
existing runtime-owner registry now contains a backward-compatible lifecycle
ledger. Registries written before Plan 0117 load with an empty ledger, which is
conservative and does not create cleanup eligibility.

`agent-browser install doctor --json` now includes
`runtimeMultiplicity`. The report counts the dashboard process, runtime hosts,
legacy per-session daemons, executable generations, and the current transaction
window independently from executable-version convergence.

## Frozen Gap Evidence

`docs/dev/fixtures/runtime-lifecycle/confirmed-gaps.v1.json` contains six
sanitized deterministic fixtures:

1. rewritten single-field Chrome command bytes lose supplementary profile and
   CDP flag evidence;
2. GC records a process group but its live terminator targets the root PID;
3. owner transfer does not yet carry durable cleanup accountability;
4. retained-state profile pruning does not reclaim the profile directory;
5. accepted historical transactions still pin old and candidate generations;
   and
6. named sessions still resolve to distinct daemon sockets.

The fixtures contain synthetic PIDs, paths, generations, start tokens, and
process trees. They contain no browser auth state or private command data.

## Validation

- RED then GREEN:
  `legacy_owner_registry_loads_with_conservative_lifecycle_defaults`.
- RED then GREEN:
  `legacy_session_daemons_are_reported_as_multiplicity_drift`.
- `p117` focused Rust filter: 4 passed.
- `red_` Rust filter: 58 passed, including every source-bound Plan 0117 gap
  proof.
- Runtime multiplicity tests: 3 passed.
- Lifecycle schema and legacy-load tests: 2 passed.
- Rust formatting check: passed after canonical formatting.
- Strict Clippy with `-D warnings`: passed.
- Full parallel Rust suite exposed five concurrency-sensitive baseline
  failures. The canonical serial rerun passed: 2,174 passed, 57 ignored, plus
  all integration test binaries.
- Documentation production build: passed with all 35 pages generated.
- Repo and installed shared `agent-browser` skill parity: passed.
- `git diff --check`: passed.

## No-Launch Runtime Readback

The source-built install doctor performed no lifecycle effects and reported:

- dashboard processes: 1;
- runtime hosts: 0;
- legacy daemons: 7;
- executable generations: 1;
- multiplicity state: `drift`; and
- issues: `runtime_host_count_not_one` and
  `legacy_session_daemons_present`.

This corrects the prior semantic ambiguity where all daemons could match the
current executable and the runtime could still be summarized as converged.

## Scope Boundary

No process was signaled. No browser, profile, transaction, generation,
supervisor, dashboard route, or live Service State record was changed. The
installed runtime remains on the validated 0.28.0 generation. Slice B begins
the deep lifecycle-owner migration through the existing registry seam.
