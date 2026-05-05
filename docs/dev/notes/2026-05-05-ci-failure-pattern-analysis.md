# CI Failure Pattern Analysis

## Summary

On 2026-05-05, repeated ordinary `CI` failures on `main` were investigated as
an explicit CI evaluation task. The failures were not caused by GitHub Actions
instability. They were caused by local validation gaps before pushing.

The final confirming run was:

- commit: `5e085b1`
- workflow: `CI`
- run: `25376114145`
- result: success

## Failure Pattern

The recent failure cluster from `55f7aa9` through `9b6d4b1` all failed in
`Rust Quality`. The failing step was the same clippy command used locally in
the validation section below.

The failing diagnostic was:

```text
unused imports: `ProfileReadinessState` and `ProfileTargetReadiness`
src/native/service_access.rs:12
```

The root cause was that `cli/src/native/service_access.rs` imported test-only
types at production scope. The local closeout checks for the follow-up commits
were docs and service-client checks, so they did not exercise the Rust Quality
gate even though the slice history had touched Rust.

Older sampled failures showed the same class of mistake:

- `25345112292`, `25340459519`, and `25314997228` failed in `Rust Quality`
  because clippy rejected a manual `Default` implementation for
  `ProfileReadinessState`.
- `25355772439` and `25356652670` failed in `Rust` because service model
  contract tests expected newly added job fields such as `targetServiceId`.
- `25351909313` failed in `Rust` because output formatter tests were not
  updated after readiness output changed.

## Lessons

Local validation must track the touched surface across the whole work slice,
not only the apparent surface of the last small commit. A docs-only follow-up
can still leave CI red if an earlier Rust commit in the same slice introduced a
clippy or unit-test failure.

For this repo, any slice that has touched `cli/src/` should run:

```bash
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
```

If the slice changes service schemas, service model records, output formatters,
contract metadata, HTTP/MCP service resources, or generated client surfaces, add
the focused Rust or pnpm contract tests for that surface before pushing.

## Current Status

The repeated clippy failure was fixed by commit `5e085b1`, which moved
`ProfileReadinessState` and `ProfileTargetReadiness` imports into the
`#[cfg(test)]` module where they are used.

Local validation before that push passed:

```bash
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
cargo test --manifest-path cli/Cargo.toml service_access -- --test-threads=1
```

The pushed `CI` run passed, including Version Sync Check, Dashboard, Service
Client, Rust Quality, Rust tests, and no-launch service smokes.
