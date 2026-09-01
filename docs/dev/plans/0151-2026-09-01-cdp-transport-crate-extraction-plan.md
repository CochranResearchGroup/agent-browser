# Plan 0151 | CDP Transport Crate Extraction

Date: 2026-09-01

Status: INTEGRATION_READY

Lane: P151

Branch: `architecture/cdp-transport-crate`

Target: `main`

Integration: `merge`

Source baseline: `09e1d6f69eecd0a5e2590a44f2ecba903e36214d`

Authority: SOURCE, TESTS, BUILD CONFIGURATION, CONTRIBUTOR DOCUMENTATION,
COMMIT, BRANCH PUSH, AND INTEGRATION TO THE PROJECT ORIGIN ARE IN SCOPE.
PRODUCTION INSTALLATION, RUNTIME STATE, BROWSER PROFILES, ACTIVE BROWSERS,
LEASE STATE, RELEASE PUBLICATION, AND PROVIDER MUTATION ARE OUT OF SCOPE.

## Objective

Establish the first real multi-crate architecture seam by extracting the CDP
protocol types, generated protocol bindings, and websocket transport into a
focused `agent-browser-cdp` library crate. Keep browser process launch,
`BrowserManager`, runtime orchestration, and product adapters in the existing
CLI crate.

## Architecture Decision

The extracted module is a deep transport module. Its interface owns CDP
connection setup, command lifecycle, typed command and event values, raw
message subscription, timeouts, protocol errors, and generated protocol
bindings. Chrome and Lightpanda are true external dependencies, while their
process lifecycle remains outside this seam.

The repository becomes a Cargo workspace containing the existing
`agent-browser` binary package and the new `agent-browser-cdp` library package.
Workspace build profiles remain behaviorally identical to the profiles
currently declared by the binary package.

The extraction does not retain `native::cdp::client` or
`native::cdp::types` as a transitional facade. Callers migrate to the library
crate directly so the dependency direction is enforceable and the completed
slice does not preserve two apparent owners.

## Owned Write Surfaces

- root and CLI Cargo manifests plus the shared Cargo lockfile;
- `crates/agent-browser-cdp/`;
- the CLI build script and CDP module declaration;
- Rust callers that import CDP transport or protocol types;
- focused architecture and validation-selection scripts;
- CI Rust quality and test entrypoints;
- contributor testing guidance in `AGENTS.md`;
- active-lane metadata and this plan.

## Non-Goals

- Do not move `chrome.rs`, `lightpanda.rs`, `discovery.rs`, or
  `browser.rs` into the library crate.
- Do not redesign the CDP interface or change command, timeout, event,
  keepalive, serialization, or error behavior.
- Do not create a generic shared-types crate.
- Do not install a production or development runtime candidate.
- Do not modify P150's worktree or uncommitted plan/catalog changes.

## Acceptance Criteria

1. Cargo metadata reports a workspace containing `agent-browser` and
   `agent-browser-cdp`.
2. CDP client, handwritten protocol types, generated protocol build logic, and
   protocol JSON sources are owned by `agent-browser-cdp`.
3. Chrome and Lightpanda process launch plus `BrowserManager` remain owned by
   `agent-browser`.
4. CLI source imports the new crate directly; no compatibility facade or old
   `native::cdp::client` and `native::cdp::types` implementation remains.
5. The existing transport lifecycle tests run as library-crate tests and prove
   timeout recovery, cancelled-command cleanup, and background-task shutdown.
6. The normal Rust test entrypoint includes library-crate tests, and strict
   formatting plus Clippy cover the whole workspace.
7. Validation selection recognizes root workspace and new-crate Rust changes.
8. Focused crate tests, the CLI test suite, strict workspace Clippy,
   formatting, architecture checks, and selected validation pass.
9. A measured focused-crate receipt demonstrates the resulting isolated
   development feedback loop; no claim about clean full-build improvement is
   made without a comparable benchmark.
10. The coherent change is committed, pushed, merged to `main`, and verified
    against the pushed integration commit without touching live runtime state.

## Execution Graph

| Slice | Depends on | Work | Exit condition |
|---|---|---|---|
| A | none | Record baseline and add one red architecture contract | The contract fails because the workspace crate is absent |
| B | A | Create workspace and move the transport module plus generator | Focused library tests pass |
| C | B | Migrate callers and remove the old module path | CLI compiles with one transport owner |
| D | C | Update CI, validation selection, and contributor guidance | Workspace coverage and architecture checks pass |
| E | D | Run focused and broad validation, measure the focused loop, and close the plan | Every acceptance criterion has current evidence |
| F | E | Commit, push, integrate, and verify the pushed main commit | Clean integrated custody and closed lane |

## Bounds And Stop Rules

- One implementation pass and at most one bounded repair pass after broad
  validation.
- One fresh architecture review pass only if current dependency evidence
  contradicts this plan.
- Stop and replan if transport extraction requires moving browser process
  ownership, Service State, runtime-host coordination, or lease authority.
- Do not use direct Cargo compilation on WSL; all compiling Cargo commands run
  through `scripts/ci/cargo-safe.sh`.
- Do not clean or delete the shared Cargo target directory.
- Pre-existing active-lane audit failures for P144 and P147 are recorded
  context, not evidence about this lane; do not rewrite their custody.

## Initial Evidence

- The source baseline contains one 316k-line Rust binary package and no Cargo
  workspace.
- `CdpClient` has 135 callers, but its implementation depends only on the CDP
  types module and transport libraries.
- `CdpCommand` has two direct callers inside the transport implementation.
- The three existing transport tests exercise observable lifecycle behavior
  through `CdpClient` against a local websocket adapter.
- Generated CDP types currently couple the CLI build script to protocol JSON
  files even though the generated output is consumed by the transport types
  module.
- Graphiti group `agent_browser_main` returned no prior crate-extraction
  decision. The relevant durable recalled invariant is that Agent Browser owns
  browser lifecycle and CDP connections; this extraction preserves that
  product ownership while creating an internal library seam.
- The default-branch lane audit currently reports pre-existing P144 and P147
  custody drift. P151 is isolated in its own worktree and does not alter those
  lanes.

## Checkpoint

State transition: `active -> integration_ready`

Acceptance state: criteria 1 through 9 are complete. Criterion 10 remains open
until the pushed branch is merged and the integrated `main` commit is verified.

Progress classification: `outcome_progress`

Evidence:

- Cargo metadata reports `agent-browser` and `agent-browser-cdp` in one
  workspace while preserving `cli/target` as the artifact directory.
- The architecture contract first failed against the single-crate baseline,
  then passed after protocol source, generation, handwritten types, transport,
  and lifecycle tests moved to the library crate.
- The focused crate's three transport lifecycle tests pass. A warm repeat
  completed Cargo work in 0.18 seconds and the tests in 0.07 seconds. The
  pre-extraction compile-and-test receipt was 141.42 seconds, while the first
  extracted-crate build was 33.68 seconds; those cold receipts are recorded as
  directional rather than equivalent benchmarks.
- `scripts/ci/rust-tests.sh` passed: 1,868 parallel CLI tests, all serial
  environment-sensitive partitions, three CDP crate tests, and the existing
  integration test binaries completed without failure.
- Strict workspace formatting and Clippy pass, as do version synchronization,
  validation selection, WSL Cargo safety, release-asset verification,
  remote-view documentation, targeted streaming tests, and `git diff --check`.
- `pnpm build:development-candidate` completed the optimized CI-profile build
  in three minutes; `cli/target/ci/agent-browser --version` reports 0.28.0.
- No runtime candidate was installed and no browser, profile, lease, provider,
  or production state was touched. The separate P150 worktree remains
  untouched.

Material blockers: none inside the owned scope.

Next action: commit and push the coherent branch, merge it to `main`, close the
lane, and verify the pushed integration commit.
