# Interactive human assistance

State: ACTIVE
Consolidation: required
Product lane: PL-BUGFIX
Owner: ecochran76
Branch: fix/interactive-human-handoff
Target: main through PR
Issues: agent-browser#571, remote-view#817

Explicit browser_handoff defaults to interactive assistance on the same retained
browser/profile/task/tab. Passive interaction=view keeps its original link and
never pauses automation. A separate invitation coordinates cooperative pause,
Native authenticated recipient input, reconnect and explicit return. Native input is relinquished before the owner resumes automation.

Primary serializes the cross-repository contract and review. No delegation.
Write scope: workflow handoff, cooperative journal, Native private callback,
focused regressions and docs. No retirement/publication algorithms, shared
runtime installation, login, navigation or profile effects in this source packet.
Preserve the dirty canonical retirement-publication checkout and retained peers.
Admission receipt is private under Remote View diagnostics/interactive-handoff-817.

Source review base: origin/main82d166cd, reconciled with documentation-only PR574
at371acd19. Remote View companion PR818 merged as f39e4c4 and its compatible
consumer is installed. Hosted CI is skipped per operator cost policy.

## Source qualification

Rust source checkpoint5763c4233, with documentation-only canonical reconciliation
at5f45b27e9, passes35 focused tests through `scripts/ci/rust-tests.sh`:
`browser_host_control` (25), `mcp::workflow::context_tests` (5),
`native_mode_callback` (2), `native_browser_mode::control_target_tests` (3).
Strict `scripts/ci/cargo-safe.sh clippy --workspace --all-targets --locked --offline
-- -D warnings` and maintained fmt check are required before integration.

Real MCP with owned Chrome and the real authenticated private owner callback
passes `scripts/test-p237-native-selection.py --runtime-managed-session
--runtime-human-handoff` against the dev candidate. Its Native provider is
synthetic. Default assistance pauses/resumes the same retained task and draft;
passive links remain separate; unauthenticated inspection/return and stale
invitation return cannot resume the current human hold. Cleanup reports no
remaining private-root processes or unproven UI owners. Installed baseline
reproduces the missing default interaction intent. All failed samples are
retained privately under diagnostics/interactive-handoff-817.

User-facing docs build, documentation links, handoff docs and policy wiring pass.
Serial Standards review against82d166cd: no remaining blocking findings. Separate
Spec review of#571/#817: default assistance, same identity, cooperative pause,
pending uncertainty, explicit return and passive isolation are covered. The
legacy mode inspector's additive-intent comparison was corrected and its real
HTTP regression passes. Upstream PR574 changes documentation only and is retained.
The unchanged historical catalog tail has an existing YAML scalar parse defect;
the changed lane projection is valid. No catalog migration is included.

Hosted CI is skipped under operator policy. Sealed production packaging,
installation and actual authenticated Native input are separate pending gates.

The extra all-targets lint exposed pre-existing fixture-only warnings. The Native
integration fixture now uses equivalent range predicates and marks its included
partial HTTP module like the adjacent partial image module. Production code is
unchanged by this fixture repair. Frozen final strict lint verification follows.
