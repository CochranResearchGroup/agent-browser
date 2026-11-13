# Managed MCP browser default, issue676

Product lane: PL-BUGFIX
Owner: ecochran76
State: OPEN
Branch: fix/mcp-stealthcdp-default
Base: origin/main 37974eff22e40c2547fddda57e28add998160620

## Objective

Honor the configured StealthCDP executable/default when the managed browser
host starts, and reject unavailable required StealthCDP before generic Chrome
discovery.

## Current State

The source repair is implemented and both real-host regressions demonstrated
RED then GREEN. Source `39736fd14116a35f361b6eaa5518211c3e19498a` is composed
with main `bd3d9212230aec7812421900446862824967fefa` and passes all 118 selected
Rust tests, strict workspace Clippy, formatting and documentation gates.
PR #679 contains the repair; merge and installed adoption remain outside this
source-only packet. The plan remains OPEN for that integration boundary.
The initial unrelated standalone integration-target compile failure remains
unqualified, with no full-suite validation claim.

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
review. Overall ceiling two hours, reassess at 30 minutes without outcome
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

Build storage exception: reuse the existing59 GiB Cargo target for focused
tests and required lint with Cargo's target locking; no cleanup and no installed
artifact replacement. Two jobs and an8 GiB memory claim plus12 GiB reserve are
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
--nocapture --test-threads=1. The test ran42.83 seconds; provider/network/browser
rendering was absent. The initial unrestricted Cargo target selection separately
failed compiling the unrelated remote_view_native_control integration target
with missing native_login_ocr/native_login_perception imports; that is not the
regression's RED. Subsequent cycles select only --bin agent-browser.

## TDD cycle1 GREEN and cycle2

The same managed disposable launch regression passed after startup reused the
existing configuration/manifest source resolver. Cargo selected --bin
agent-browser and one test; zero compiler warnings/errors, exit 0,260.702 s
including compilation. Cycle2 adds only the required-missing-manifest startup
refusal test before implementation of that guard. The competing cache remains
present in both fixture scenarios.

## TDD cycle2 RED

The missing-required-manifest test failed with the intended expectation:
required StealthCDP must not permit generic Chrome discovery. Exit 101, one
failed test, zero compiler errors/warnings. Exact Cargo command selected
--bin agent-browser managed_host_refuses_missing_required_stealthcdp_instead_of_testing_cache
-- --nocapture --test-threads=1 through cargo-safe with the same cache-off,
16 MiB test stack and resource settings as cycle1. Implementation now checks
required default readiness before constructing the managed runtime; explicit
executable precedence and healthy-manifest profile pins remain unchanged.

## Qualified source checkpoint

Both regressions pass together after cycle2 (2/2,44.25 seconds test time;
402.178 seconds including admission and compilation). The original disposable
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

## Final acceptance evidence

| Requirement | Evidence | Scope and limit |
| --- | --- | --- |
| Honor manifest in managed launch | Original regression RED with TESTING_CHROME_SELECTED, then GREEN with STEALTHCDP_SELECTED | Real host/config/process boundary; executable recorders, no rendering |
| Required missing StealthCDP refuses fallback | New startup test RED before guard, GREEN after guard | Competing testing cache remains present |
| Current-main composition | 2 regressions + 100 flags + 16 host tests, 118 passed, exit 0 | Focused Linux/WSL; no comprehensive or target-platform matrix claim |
| Rust quality | cargo-safe fmt --all --manifest-path Cargo.toml -- --check; clippy --workspace --manifest-path Cargo.toml -- -D warnings, both pass | Source `39736fd14`; no compiler warnings |
| Docs and wiring | pnpm test:policy-wiring, test:documentation-links, test:remote-view-handoff-docs and pnpm --dir docs build pass | Docs/site and policy, repeated on rebased source where changed |
| Workstation static fixtures | test:workstation-host-provision, test:fresh-workstation-vm-harness, test:workstation-guacamole-assets, test:guacamole-postgres-durability and test:rdp-guac-route-specific-user-sync pass | Synthetic/static harnesses; not installed runtime qualification |
| Cleanup | Fresh /proc check finds no managed-build fixture browser processes | All temporary fixture roots are test-owned; no peer close |
| Shared custody | PR #679, branch fix/mcp-stealthcdp-default, remote source checkpoint39736fd14 verified | No merge or production deployment |

Final behavioral commands use the existing runner:
`scripts/ci/rust-tests.sh --focused default_build_tests`,
`--focused flags::tests`, and
`--focused native::browser_session_host::tests::`.
The private build manifest retains exact Cargo target, two jobs, cache-off,
8 GiB memory claim,12 GiB reserve and16 MiB test stack. Cargo Signal receipts retain
both RED/GREEN cycles, the cache-probe failure and final strict/behavior results.
Final source-file hashes and fresh process evidence are retained in the user's
local-state validation receipt. A documentation-only closeout does not alter
those qualified Rust bytes.

The initially unrestricted Cargo invocation compiled a standalone integration
target before the source repair and failed on missing native_login_ocr and
native_login_perception imports. That historical failure is retained, not
presented as passed or fixed here. The selector's broad source-free installer
fixture is also unexecuted: no installer/payload behavior changed, and no
production-shaped artifact was assembled. Installed shared-skill drift remains
explicit; publishing the skill and runtime is a separate effect boundary.
Hosted CI was not dispatched. Local evidence is Linux/WSL with isolated native
recorders, not the hosted OS matrix. No Chrome for Testing was downloaded or
compiled by these tests. Dependency setup downloaded an upstream Agent Browser
native CLI locally; it was removed and never installed or used for acceptance.

Next action: review PR #679 and address integration/production adoption under
separate authority. Retain the clean primary for that review and acceptance
handoff; owner ecochran76, next review 2026-11-14. No auxiliary checkouts exist.

Memory disposition: unavailable. The receipted Graphiti remember attempt was
rejected before queue acceptance because another Graphiti operation held the
read/queue boundary. No job ID or accepted write was returned; safe_to_retry
was false and no retry was attempted. Repository evidence remains authoritative.
