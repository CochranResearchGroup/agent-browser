# P117 Slice D Source Acceptance

Date: 2026-08-20

Plan: `docs/dev/plans/0117-2026-08-19-runtime-lifecycle-authority-and-convergence-plan.md`

## Accepted Boundary

Slice D is accepted at the source and isolated-fixture boundary. This slice did
not install a binary, apply cleanup to the workstation, mutate authenticated
profiles, or reconcile live daemons.

## Retention Authority

`cli/src/runtime_retention.rs` is the shared policy authority for profile and
generation retention. It classifies package-owned profile evidence as
protected, reviewable, or automatically reclaimable after joining profile
class, terminal age, durable references, process observation, filesystem
inventory, and projected bytes.

The accepted defaults are now executable policy:

- unreferenced ephemeral profiles require 24 terminal hours;
- failed or quarantined profiles require seven days and explicit review; and
- persistent profiles are never automatically age-deleted.

Present eligible directories pass exact-root and symlink checks, move through
a deterministic quarantine manifest, and are removed before their Service
State record is discarded. Restored references reject apply. Completed replay
is idempotent, and a later apply resumes an exact prepared quarantine left by
an interrupted process.

## Generation Convergence

Generation GC now distinguishes durable transaction metadata from live
rollback protection. Healthy accepted transactions retain old and candidate
references for 24 hours, then become automatically finalizable during locked
apply. Their transaction files remain durable. Ordinary policy protects the
selected generation, the immediately previous healthy rollback generation,
live process executables, supervisor manifests, active transactions, and open
rollback references.

The live-shaped unit corpus contains 49 durable transactions across 21
generation identities. Its retention plan protects only the current and
previous healthy generations without deleting or rewriting the 49 metadata
records. A separate fixture proves that apply advances an eligible accepted
transaction to `old_generation_retirable`.

## Validation

- `scripts/ci/cargo-safe.sh fmt --manifest-path cli/Cargo.toml -- --check`
- `scripts/ci/cargo-safe.sh clippy --manifest-path cli/Cargo.toml -- -D warnings`
- focused retention authority: 7 passed
- focused profile filesystem reclamation: 1 passed
- focused workstation install module: 62 passed
- serial Rust suite: 2,197 passed, 57 ignored; integration suites 4, 1, and 1 passed
- source-free workstation install failure matrix: passed
- workstation host provision fixture: passed
- fresh workstation VM harness contract: passed
- workstation Guacamole asset validation: passed
- Guacamole PostgreSQL durability contract: passed
- route-specific user sync contract: passed
- docs build: all 35 pages generated
- remote-view handoff documentation checks: passed
- repository and installed Agent Browser skills: byte-identical
- patch whitespace check: passed

The first parallel Rust run exposed two shared-state test collisions. Both
passed alone, and the complete serial run passed. That parallel run also left
one disposable headless Chrome test group holding the Cargo lock descriptor.
The exact test-only process group and `/tmp/agent-browser-managed-runtime-*`
profile identity were verified before termination. No live runtime browser was
targeted.

## Next Boundary

Execute Slice E by introducing one user-scoped runtime host with logical named
lanes and a forwarding-only compatibility adapter for legacy session sockets.
