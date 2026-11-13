# Managed MCP browser default, issue676

Product lane: PL-BUGFIX
Owner: ecochran76
State: OPEN
Branch: fix/mcp-stealthcdp-default
Base: origin/main37974eff22e40c2547fddda57e28add998160620

## Objective and current state

Honor the configured StealthCDP executable/default when the managed browser
host starts, and reject unavailable required StealthCDP before generic Chrome
discovery. Diagnosis reproduced a disposable MCP launch choosing testing
Chrome despite a valid service manifest. Implementation has not started.

## Seams and acceptance

- Managed host command/open with isolated config, SQLite and an external
  executable recorder: catches host wiring to actual launch selection; does
  not qualify Chrome rendering, Native presentation or installed production.
- Managed host startup with unavailable required manifest: catches unintended
  fallback before any process launch; does not establish artifact integrity.
- Existing flags/manifest tests: preserve precedence and existing readiness
  semantics; do not alone qualify managed-host wiring.

One RED test, one minimal implementation, then the next slice. Reuse the
existing Rust fixture/test runner. Preserve explicit profile pins and operator
executable overrides; do not migrate profiles or install browsers. Production
installation, peer termination and service restarts are outside this packet.
Primary owns the serial startup/test changes; recovery placement PR675 owns a
different section of the host file and will be preserved at integration.

## Bounds and custody

At most two attempts per failing validation unit, one consolidated repair
review. Overall ceiling two hours, reassess at30minutes without outcome
progress. Full local Rustfmt/Clippy and selected checks run at final batch.
Hosted CI is excluded under operator instruction. Source validation is distinct
from installed workflow acceptance. Next custody review:2026-11-14.

## Preflight

Governance MCP could not load its configured manifest from this client, so
canonical policy files were used. Worktree admission retained five existing
clean checkouts and recorded one new operator-task primary. Discovery used the
canonical checkout's graph for unchanged startup/flags symbols, verified by
current source reads; no index was created. Graphiti recall was advisory and
provided no qualified cause beyond the current diagnosis.

Build storage exception: reuse the existing59GiB Cargo target for focused
tests and required lint with Cargo's target locking; no cleanup and no installed
artifact replacement. Two jobs and an8GiB memory claim plus12GiB reserve are
recorded in the private build manifest. Compiler-cache version probing stalled
before compilation; the exact probe was terminated and its failed receipt is
retained. Retry uses the documented cache-off switch. No production runtime
or browser profile was changed.

## TDD cycle1 RED

The existing-host regression failed because all three process attempts reported
TESTING_CHROME_SELECTED instead of STEALTHCDP_SELECTED. No implementation was
changed before that failure. Exact command: RUST_MIN_STACK=16777216 followed by
the compiled CLI unit-test binary with filter
managed_disposable_launch_honors_required_stealthcdp_manifest and
--nocapture --test-threads=1. The test ran42.83seconds; provider/network/browser
rendering was absent. The initial unrestricted Cargo target selection separately
failed compiling the unrelated remote_view_native_control integration target
with missing native_login_ocr/native_login_perception imports; that is not the
regression's RED. Subsequent cycles select only --bin agent-browser.

## TDD cycle1 GREEN and cycle2

The same managed disposable launch regression passed after startup reused the
existing configuration/manifest source resolver. Cargo selected --bin
agent-browser and one test; zero compiler warnings/errors, exit0,260.702s
including compilation. Cycle2 adds only the required-missing-manifest startup
refusal test before implementation of that guard. The competing cache remains
present in both fixture scenarios.

## TDD cycle2 RED

The missing-required-manifest test failed with the intended expectation:
required StealthCDP must not permit generic Chrome discovery. Exit101, one
failed test, zero compiler errors/warnings. Exact Cargo command selected
--bin agent-browser managed_host_refuses_missing_required_stealthcdp_instead_of_testing_cache
-- --nocapture --test-threads=1 through cargo-safe with the same cache-off,
16MiB test stack and resource settings as cycle1. Implementation now checks
required default readiness before constructing the managed runtime; explicit
executable precedence and healthy-manifest profile pins remain unchanged.

## Qualified source checkpoint

Both regressions pass together after cycle2 (2/2,44.25seconds test time;
402.178seconds including admission and compilation). The original disposable
launch behavior is included in this rerun. Focused existing flags and host
checks also pass. Documentation build, links, policy wiring, handoff guidance,
and five source/host-provisioning fixture checks pass. Rustfmt check passes.
Strict workspace Clippy and composed-current-main qualification remain pending;
this checkpoint is not deployment or merge readiness.

Concurrent integration moved origin/main through PR675 and PR678 while this
packet tested. Rebase this clean custody checkpoint onto the freshly fetched
main, preserve those placement and first-run changes, and qualify that exact
composition before opening the reviewable PR. Installed shared skill comparison
shows drift; user-scoped publication is intentionally not included in this
source-only repair. No production runtime or retained profile was modified.
