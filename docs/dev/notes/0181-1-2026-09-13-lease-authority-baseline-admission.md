# P181 Baseline Admission Receipt

Date: 2026-09-13

Product lane: PL-PLATFORM

Disposition: active-input

Owning plan or work item: Plan 0181; CochranResearchGroup/agent-browser#99

Related lanes: P144

## Frozen Baseline

- Source SHA: `16d4fb22dfd8cf96cd7e65edfd0b66fe54953945`
- Source condition: Lease Authority remains in the CLI binary compilation unit.
- Cargo jobs: 8 through the repository default.
- Cache: off.
- Fast linker: off.
- Focused preparation command:
  `scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml service_lease_authority -- --test-threads=1`
- Isolated target: `/tmp/agent-browser-p181-baseline-target`.
- Measurement wrapper: `/usr/bin/time -v`.

The baseline and candidate must use identical test selections from the frozen
invariant ledger. The immutable SHA remains the source authority even if the
preparation command is executed after source movement in another worktree.

## Admission Attempt

The first preparation attempt waited 2 minutes 22.08 seconds. The Cargo wrapper
reported `memory_pressure` with zero active Cargo claims and did not admit the
command. At inspection, the host reported 70 GiB total memory, 19 GiB available
memory, and all 32 GiB of swap in use. The owned waiter was interrupted with
exit status 130. `/usr/bin/time -v` reported 4,000 KiB maximum RSS and no Cargo
scope or rustc process started.

This is a preserved infrastructure deferral, not a product failure and not a
benchmark sample. It consumes zero of the 23 admitted compiling benchmark
invocations. No unrelated browser, service, or user process was stopped to
manufacture capacity.

## Remaining Gate

Retry the same baseline preparation from the exact frozen SHA only when
`cargo-safe.sh` admits it. It must pass before any candidate measurement. Do not
change job count, cache posture, linker posture, test selection, or resource
limits to obtain admission.
