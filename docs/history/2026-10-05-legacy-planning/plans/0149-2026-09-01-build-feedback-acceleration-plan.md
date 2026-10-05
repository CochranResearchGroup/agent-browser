# Plan 0149 | Build Feedback Acceleration

Status: COMPLETE

## Objective

Shorten ordinary Rust development and installed-candidate feedback without
weakening the final production release build, WSL resource admission, or
cross-platform behavior.

## Authority And Scope

The operator authorized planning and execution of priorities 1 through 4:

1. use the existing `ci` Cargo profile for development candidates;
2. reserve the full release profile for final production or release-candidate
   installation;
3. add compiler-cache and fast-linker support, retaining each only after a
   measured local proof;
4. benchmark four, six, and eight Cargo jobs under the existing WSL cgroup and
   host-reserve contract, then select the fastest safe default.

Owned write surfaces are `cli/Cargo.toml`, `package.json`, the development
runtime publisher, Cargo wrapper and benchmark scripts, focused tests,
`AGENTS.md`, `README.md`, `skills/agent-browser/SKILL.md`, the documentation
site, and this plan. Production runtime state, browser profiles, lease state,
release version metadata, and unrelated active runtime-host lanes are out of
scope.

## Acceptance Criteria

- `pnpm build:development-candidate` produces
  `cli/target/ci/agent-browser` through `scripts/ci/cargo-safe.sh`.
- `pnpm development-runtime:install` defaults to that CI-profile artifact;
  `--binary` remains an explicit override.
- The full `build:native` release path remains full LTO with one codegen unit
  and is documented as the final production gate, not the iteration loop.
- The Cargo wrapper reports and uses `sccache` and a Linux fast linker only
  when their exact executables are available, with deterministic environment
  opt-outs and without weakening WSL admission or cgroup limits.
- A checked-in benchmark command records wall time, peak RSS, job count,
  toolchain acceleration state, and Cargo timings for isolated four-, six-,
  and eight-job `cargo check` runs. It uses disposable target directories and
  never cleans the shared target directory.
- One bounded benchmark pass selects a default only when all three runs
  complete and the winner remains inside the existing memory contract.
- Focused wrapper, publisher, entrypoint, documentation, formatting, and
  strict Clippy checks pass. A final development-candidate build and isolated
  development doctor prove the new default artifact is usable.

## Execution Graph

| Slice | Depends on | Work | Exit condition |
|---|---|---|---|
| A | none | Add red contract tests for CI-profile publication and acceleration discovery | Tests fail for the missing behavior |
| B | A | Implement build commands, publisher default, wrapper acceleration, and benchmark harness | Focused tests pass |
| C | B | Install or discover local acceleration tools and run the bounded 4/6/8 benchmark | Complete comparable receipts exist |
| D | C | Select the safe default, update all operator/agent documentation, build and publish one development candidate | Candidate doctor is green |
| E | D | Validate, reconcile remote main, commit, merge, and push | Clean pushed main and closed plan |

## Bounds And Gates

- Benchmark attempts: one complete 4/6/8 pass, with at most one rerun for a
  typed infrastructure failure.
- Tool trials: one cache trial and one linker trial. A missing package or
  unavailable sudo boundary does not weaken the wrapper; it leaves the tool
  optional and records the remaining installation gate.
- No `cargo clean`, shared-target deletion, production install, release, or
  browser launch is authorized by this plan.
- A job-count default may increase only when the measured run succeeds within
  the existing per-scope and aggregate memory limits.

## Initial Evidence

- The repository is one 315,893-line Rust binary crate with 328 locked
  packages.
- `cli/target` occupies 34 GiB, including 21 GiB of incremental artifacts.
- The wrapper defaults to four jobs on a 20-CPU host and already preserves a
  16 GiB host reserve plus per-build and aggregate cgroup bounds.
- Neither `sccache`, `mold`, nor `lld` is currently available on `PATH`.
- The full release profile uses full LTO and one codegen unit. The existing
  `ci` profile retains release optimization while using thin LTO and sixteen
  codegen units.

## Completion Evidence

- `pnpm benchmark:cargo-build-jobs` completed one isolated, cache-disabled
  pass and retained its local report under
  `cli/target/build-benchmarks/2026-09-01T213618-658Z/`. The shared target
  directory was preserved and all disposable benchmark targets were removed.
- Four jobs completed in 65.639 seconds with maximum RSS of 3,715,476 KiB.
  Six jobs completed in 63.115 seconds with maximum RSS of 3,715,364 KiB.
  Eight jobs completed in 62.737 seconds with maximum RSS of 3,712,164 KiB.
  Each run produced a Cargo timing report, exited zero, and remained below the
  existing 24 GiB per-scope limit. Eight jobs is therefore the selected
  default on this 20-CPU workstation.
- Ubuntu packages `sccache` 0.7.7 and `mold` 2.30.0 were installed. A cold
  accelerated candidate populated the cache without cache errors. A separate
  disposable-target proof increased cache hits from zero to 236, including
  220 Rust hits and 16 C or C++ hits, again with zero cache errors.
- `pnpm build:development-candidate` built
  `cli/target/ci/agent-browser` through the admitted eight-job wrapper with
  `sccache` and `mold`. The cold optimized build completed in 4 minutes 59
  seconds and reported `agent-browser 0.28.0`; the prior full release reference
  was approximately 9 minutes 19 seconds.
- Development generation `0.28.0-ed3b3598cdfa` was installed without changing
  production. `pnpm development-runtime:doctor` passed every development,
  lease-authority, provider-isolation, executable, port, and skill check.
- Focused publisher, wrapper, benchmark, WSL entrypoint, remote-view
  documentation, release-asset fixture, documentation build, formatting, and
  strict Clippy checks passed. Production installation and release publication
  were not performed.
