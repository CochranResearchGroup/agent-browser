# Stock popup blocking for managed launches

Product lane: PL-BUGFIX
Owner: ecochran76
State: OPEN
Consolidation: required
Branch: fix/stock-popup-default
Work item: CochranResearchGroup/agent-browser#693
Base: e16ae2ef0a5138c791304dae8c2fe69e429fe545

## Current State

Minimal repair removes the unconditional popup-blocking disable. The actual
headed browser regression demonstrated RED then GREEN; it now checks default,
user-activation and explicit opt-out postures under distinct session identities.
All93 selected tests, strict workspace Clippy, Rustfmt and affected documentation
checks pass. PR/source integration and exact installed MCP adoption are pending.

## Consolidated batch

Restore stock popup blocking in ordinary browser launches; preserve explicit
custom arguments and user-initiated popup workflows. Add one real headed-browser
regression, then the minimal implementation and affected guidance. H264/AAC
configuration qualification belongs to the parent Chromium task and does not
expand this source repair into a codec build or identity spoofing feature.

## Delivery sequence and budget

Use the existing isolated Rust e2e seam and pinned StealthCDP executable, local
HTTP fixture, disposable HOME/profile, serial test execution and exact cleanup.
The test asserts page-observed unsolicited-popup blocking after ordinary launch;
it does not alone prove installed MCP behavior. Original experiment is the
installed RED baseline. New regression must fail before changing implementation.
Preserve explicit existing argument opt-in through the same public seam.

Bounds: two attempts per failing validation unit, one serial review/remediation
pass; three hours overall including compilation/adoption, with reassessment
at30 minutes without acceptance progress. Use Cargo Signal CLI fallback and
cargo-safe with existing cached target, cache off, two jobs,8 GiB memory claim
and12 GiB reserve. Run strict Rust/docs gates once at the final batch boundary;
retain all first failures. No hosted CI or Chrome for Testing.

## Worker assignments

Primary is sole source/test writer. Reuse clean operator primary mcp-stealthcdp676
on a new branch from fresh main; no additional checkout. Preserve the prior
branch, sealed candidate files and immutable source objects. Full population and
remote-divergence admission readback is retained privately. No peer worktree,
retained profile, global compiler cache or viewer configuration is changed.

## Evidence and exit

Require actual browser regression RED/GREEN, explicit opt-in coverage, relevant
existing launch checks, Rustfmt, strict workspace Clippy and affected docs gates.
Self-review published diff against frozen head. Normal source integration and
any authorized preserving adoption must use a sealed merged production candidate
and fresh exact-task MCP popup check; no local-only candidate deployment.
Use one evidence table for source, installed acceptance and remaining codec lane.
Keep Source versus Installed qualification distinct. Source-only or blocked gates
remain explicit; do not label old-binary evidence as final adoption.

## TDD RED

The new headed-browser regression failed on the exact intended assertion:
unsolicited opened=true with userActivation=false, while an actual click opened
a popup with userActivation=true. navigator.webdriver remained false in both.
The owned browser was closed and its PID verified absent before the assertion.
Exit101, one failed test, zero compiler errors/warnings,6.16 seconds test time
and314.64 seconds including compilation. Exact cargo-safe selection:
`test --manifest-path cli/Cargo.toml --bin agent-browser e2e_stock_popup_default_blocks_unsolicited_but_allows_user_click -- --ignored --test-threads=1 --nocapture`.
Private run manifest pins isolated user state, executable, resource settings and
retained Cargo Signal receipts. Minimal implementation removes only the
unconditional popup-blocking disable; first GREEN compilation is in progress.

## Final source evidence

One public e2e regression exercises two actual headed StealthCDP launches:
ordinary launch blocks the no-activation popup while an actual click opens it;
an explicit custom-argument opt-out permits the unsolicited popup. Its owned
process is closed before verdict assertions. Together with92 existing Chrome
launch tests,93 pass. Strict workspace Clippy and Rustfmt pass with zero compiler
warnings/errors; combined Cargo Signal duration188.115 seconds. Local docs
build,11 policy-wiring cases, documentation links and handoff docs pass. No
hosted CI or comprehensive/platform-matrix claim.

The first paired fixture failed before the second PID readback. The explicit
opt-out passed alone in1.524 seconds. Consolidation into one HOME still exposed
existing_session_profile_identity_unproven: the fixture reused one session for
differing profiles. The earlier cached-HOME explanation was only a hypothesis.
Distinct default/opt-out subjects resolved the exact binding refusal without
clearing owner state. Those failed receipts remain; all product behavior changes
are confined to removing the one default switch. No extra implementation fix.

Final commands: cargo-safe test --bin agent-browser e2e_stock_popup_ with
--ignored --test-threads=1 --nocapture, existing rust-tests.sh --focused
native::cdp::chrome::tests, cargo-safe fmt --all --manifest-path Cargo.toml
-- --check and cargo-safe clippy --workspace --manifest-path Cargo.toml
-- -D warnings. Each uses the same isolated/cached run-manifest inputs.
