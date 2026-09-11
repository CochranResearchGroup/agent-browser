# Plan 0167 Service State Lock Repair Evidence

Date: 2026-09-11

Status: ACCEPTED AND INTEGRATED

Plan: [Plan 0167](../plans/0167-2026-09-11-production-scale-service-state-lock-attribution-and-critical-section-repair.md)

Branch: `maintenance/plan-0167-service-state-lock`

Batch baseline: `7cfc1fd2c31a394eca5c378f6fa3b26d23b96b81`

Qualified source: `2135def23d4b820dbae4a5d88a38f96724db2592`

Integration receipt: PR 29, merge
`14fb3db060d8fb644665b0181ac8f2860b0af3f2`

## Scope and custody

P167 opened in the canonical worktree from the current `origin/main`. The
Plan 0165 and P0240 worktrees were clean and remained untouched. The paused
P157 branch has four commits not in `origin/main`; its only path overlap with
this implementation batch is
`cli/src/native/action_runtime/runtime/navigation.rs`, where P157 contains
format-only differences around unrelated retained-handoff code. P167 changed
only replay-safe cloning at its mutation closure in that file.

No Plan 0167 command installed a production runtime, mutated production Service
State, started or replaced a browser or profile, changed a provider or tenant,
terminated a process, or retried X or RuFresh. Other active lanes performed
production installs and browser work during the same wall-clock interval; those
effects were outside this branch's custody and are not claimed as Plan 0167
evidence.

## Frozen baseline

The first production-scale test generated a synthetic Service State with 200
jobs, 261 sessions, and 91 profiles and asserted a minimum serialized size of
8 MiB. It used two independent Rust test processes and the real
`LockedServiceStateRepository` mutation path. The child entered a deliberately
slow pure mutation phase while the parent attempted a competing mutation.

On the unoptimized baseline, the focused test failed for the intended reason:

```text
production-scale contender failed after 1001 ms:
service_state_lock_timeout: file lock; waited_ms=1001
```

This reproduced the fixed one-second file-lock deadline without using a raw
lock-only helper as the production-scale oracle.

## Repair selection

The baseline proved that the exclusive file lock spanned pure mutation and
transaction serialization. The candidate now loads a shared snapshot, applies
the pure transformation, and prepares the serialized transaction outside
exclusive ownership. It acquires the exclusive file lock only to reload the
current revision and commit the prepared transaction. One stale candidate is
replayed by the repository against a fresh snapshot. The trait requires an
`FnMut` closure, and affected callers retain cloned inputs so the compiler
rejects consuming transformations. External effects remain outside the
closure and are not replayed.

Top-level fields unknown to the current `ServiceState` model are retained in a
serde flatten map and round-trip through the same candidate transaction.

## Cross-process attribution

Every exclusive Service State file-lock holder writes a bounded JSON sidecar
beside the selected state file after acquisition. The sidecar contains only:

- schema and unique acquisition token;
- PID plus process-start identity;
- compile-time package and source generation;
- operation, mode, and current phase;
- acquisition wall time; and
- optional state bytes, revision, and coarse job, session, and profile counts.

The exact holder clears its token-matched sidecar before normal release. A
holder also writes the acquisition token into the already-exclusive lock file.
A contender reads the sidecar without acquiring the Service State lock and
attributes it only when the lock token, PID, and process-start identity all
match. At the deadline it re-probes the operating-system lock so an
acquisition-to-timeout release race is reported as `unknown_released_race`.
Missing, unreadable, corrupt, unknown-schema, token-mismatched, unobservable,
reused, and exited identities remain typed unknown or stale.
The record carries no URLs, profile paths, page content, capabilities,
credentials, request payloads, or tenant identifiers and grants no cleanup,
takeover, force-unlock, or retry authority.

An initial instrumentation attempt derived generation by hashing the full
debug test executable after lock acquisition. That exceeded the test helper's
10-second barrier and failed as
`production_scale_helper_release_timeout`. The candidate removed that work
from the critical section and uses constant-time compile metadata instead.

## Replay-safety audit

The candidate changes the durable repository mutator contract from `FnOnce` to
`FnMut` because one stale prepared candidate may be replayed against a fresh
revision. A whole-source closure audit found no direct filesystem, process,
network, sleep, environment, provider-census, or boot-epoch observation inside
production repository mutation closures after these corrections:

- producer census and its timestamp are captured before legacy connection
  reconciliation;
- provider inventory overlay is performed by each fresh JSON snapshot load,
  not from inside remote-view recovery mutation;
- boot epoch reads are captured before affected control-plane, remote-view,
  health, lifecycle, and profile-recovery mutations; and
- completion adapters recreate their state transition from already-completed,
  cloned receipts and perform no external effect in the closure.

The production-scale helper reports `helper_outcome=ok:2`, proving exactly one
stale-candidate replay while each logical writer effect remains present once.

## Candidate evidence

These focused provider-free tests pass on the same source candidate:

```text
production_scale_independent_mutations_do_not_timeout_behind_slow_preparation
prepared_transactions_serialize_competing_writers_before_commit
independent_process_file_lock_timeout_is_classified_before_mutation
durable_file_lock_holder_attribution_fails_closed_for_untrusted_sidecars
```

The production-scale candidate completes with two competing writer processes,
two additional snapshot-reader processes, both logical writer effects,
monotonic revision 2, valid JSON, the synthetic unknown field, and clean child
exit. Six consecutive measurements on the qualified debug test binary used a
9,642,672-byte fixture. Contender elapsed time was 709 to 739 ms with p50 722
ms and p95 739 ms. Exclusive prepared-commit wait was 0 ms in every sample;
hold time was 358 to 380 ms with p50 365 ms and p95 380 ms. The committed test
requires contender completion below 900 ms and exclusive commit hold below 500
ms, preserving at least 10 percent deadline headroom while leaving the ordinary
one-second timeout unchanged. The frozen baseline sample was timeout-censored
at 1,001 ms, so it cannot truthfully supply an uncensored hold percentile.

The full 37-test `service_store` family, including crash residue recovery,
passed. Focused `service_model` (37), `service_health` (98), `service_config`
(29), and `service_monitors` (18) families also passed. API and MCP parity,
generated service client contracts, client type coverage, remote-view docs,
the docs production build, and all six selector-requested workstation fixture
checks passed on the reconciled candidate. Format, strict workspace Clippy, and
diff hygiene passed. The comprehensive Rust suite passed with
`nativeLane=0`, `supportLane=0`, and 540 seconds elapsed using one Cargo build
job and no sccache after a default-pressure attempt exhausted host threads.
The same run includes the selector-requested workstation Rust compartment and
the CLI integration compartment. A fresh post-test readback found no Plan 0167
production-scale helper process, holder sidecar, or temporary fixture file.

Two qualification interruptions were preserved rather than hidden. First, the
full suite exposed a pre-existing stale workstation test left by `e4ca545b`:
runtime policy made `GenerationCommitted` forward-only while the test still
expected post-commit rollback. The test-only repair now proves rollback is
rejected and committed generation and Service State remain preserved. Second,
after merging current `origin/main`, the default-pressure integration link hit
`os error 11` because the host had 900 processes and 7,972 threads. The exact
integration compartment and the complete suite both passed with one Cargo job
and cache disabled; no foreign process was terminated.

The repository skill intentionally differs from the shared installed skill by
the new holder-attribution guidance. Publishing it would mutate a production
user-scoped surface and remains an explicit unexecuted gate. An isolated
development candidate was not applicable because the repaired persistence
boundary, diagnostics, and public projections are fully exercised by the
provider-free cross-process, contract, and documentation gates.

One attempted measurement command selected the ordinary debug CLI instead of
the emitted test harness. Five invocations performed only startup reads of the
default Service State, reported the retained `job_terminal` enum as unknown,
and rejected `--exact`. They did not mutate Service State, start a browser, or
perform a retry. Subsequent measurements used the exact Cargo-emitted test
binary `agent_browser-0bbcc91e17ee36cb`.

## Remaining gates

Source integration into protected `main` completed through PR 29. Production
installation, shared skill publication, installed doctor, and any consumer X
evaluation are separate, unauthorized gates. A later X result cannot change
this source acceptance, and this acceptance does not predict that X will
succeed.
