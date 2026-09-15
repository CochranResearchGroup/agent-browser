# Runbook

Current execution and stop-state index. Detailed checkpoints through Turn 312
are preserved in [the September 13 archive](RUNBOOK-history-2026-09-13-through-turn312.md).
Keep this file at or below 200 lines under policy 0043.

## Turn 319 | 2026-09-14

[Plan 0187](docs/dev/plans/0187-2026-09-14-challenge-countermeasure-control-plane-blueprint.md)
is PLANNED under PL-CHALLENGE and P169. It recovers the provider-neutral
challenge control-plane trunk from the current Turnstile and hCaptcha leaves,
defines the shared desktop transaction module and product branches, and divides
delivery into workfronts W0 through W8. Its first consolidated batch is W0
through W4.

The immediate gate is W0 custody normalization: reconcile current main,
renumber branch-local plan collisions, create a parent work item, assign one
active implementation plan, and record its branch and baseline. Plan 0187 is
planning-only authority. It does not authorize source implementation, browser
or provider mutation, desktop input, a live challenge attempt, hCaptcha retry,
production mutation, or release.

## Turn 318 | 2026-09-14

[Plan 0189](docs/dev/plans/0189-2026-09-13-hcaptcha-fixture-checkbox-acceptance.md)
is BLOCKED under P169 and [issue #66](https://github.com/CochranResearchGroup/agent-browser/issues/66).
The provider-free hCaptcha locator and interaction work, plus the exact
`desktop_interaction_authority_required` no-effect recourse repair, are
checkpointed at `0def316d207efb8c5c5b6c82c44ec50eac4b6ed9` with selected tests,
formatting, strict Clippy, docs build, and documentation checks passing.

Installed development generation `0.28.0-9587f109293e` opened the fixture in
job `r538730`; route, display, browser, and operator presentation were ready.
Observation `r196319` matched exactly one checkbox, and the renewed controller
lease used the same `codex-p181-hcaptcha` identity as the action. The sole
interaction job `r163653` acknowledged nine pointer-motion events and then
stopped at `desktop_interaction_stale_observation` before `LeftDown`. Terminal
observation `r7758` still matched the visible checkbox at the same geometry.

This is an Agent Browser interaction-timing defect rather than a fixture
defect. No click, retry, reset, challenge-solving action, provider apply,
production mutation, or release occurred. Resumption requires a bounded
freshness-semantics repair and a new explicit live interaction budget.

## Turn 317 | 2026-09-13

[Plan 0188](docs/dev/plans/0188-2026-09-13-captcha-guard-contract-and-threat-model.md)
is CLOSED under P169 and issue #66 at published checkpoint
`c6f0b447a03a586991561824ad0e744974267e81`. The guard request, capability,
receipt, threat model, dependency policy, and deterministic fixtures are frozen.
All selected provider-free validation passed.

This packet freezes guard requests, capabilities, receipts, threat boundaries,
redaction, replay, and future crate dependency direction. It does not authorize
installation, browser or profile mutation, desktop input, a live challenge
attempt, or retry. [Plan 0169](docs/dev/plans/0169-2026-09-11-cloudflare-turnstile-desktop-challenge-plan.md)
remains blocked on its distinct live acceptance gate.

## Turn 316 | 2026-09-13

[Plan 0178](docs/dev/plans/0178-2026-09-13-browserless-runtime-lane-quiescence.md)
is source-integrated and BLOCKED on separately authorized installed acceptance.
PR #83 merged browserless lane quiescence as `ae426642`; all fast checks and
comprehensive Rust passed. PR #92 merged exact runtime-admission claims for the
old-runtime status and close commands as `7e59ae35`. Its focused tests, format,
Rust Quality, Dashboard, Service Client, and Version Sync passed; Workstation
Fixtures and comprehensive Rust were still running at the one lazy readback and
were not actively watched.

Both source tips are ancestors of `origin/main`. Their clean worktree and local
and remote refs are retired, so P178 leaves active Git custody. Issue #84 remains
open and BLOCKED behind production maintenance quarantine issue #76. No
installation, restart, handoff, browser closure, or Service State mutation
occurred in closeout.

Provider-free issue #78 is also closed. PR #93 merged the development status
port repair as `4190fdf1`; the fixture, read-only current-runtime JSON and text
readback, documentation checks, and docs build passed. It performed no runtime
mutation. Installed shared-skill parity remains separately owned by issue #79.

## Turn 315 | 2026-09-13

[Plan 0179](docs/dev/plans/0179-2026-09-13-policy-selector-v0-1-26-integration.md)
is CLOSED through [issue #89](https://github.com/CochranResearchGroup/agent-browser/issues/89).
PR #90 merged the v0.1.26 selector rollout as `3f842f59` after policy wiring,
122 selector tests with three source-checkout-only skips, active planning and
goal audits, remote-view documentation checks, and patch hygiene passed.
Canonical `main` is clean and synchronized. Exact ancestry was verified before
the clean topic worktree and local and remote refs were retired. Selected CI
was queued at the one lazy readback and was not actively watched.

No installed runtime, browser, profile, provider, tenant, service, or
supervisor state changed. Turn 316 supersedes the then-current Plan 0178 and
issue #78 next actions with their integrated source outcomes.

## Turn 314 | 2026-09-13

Plan 0177 is CLOSED after [PR #86](https://github.com/CochranResearchGroup/agent-browser/pull/86)
integrated the governance and repository-readiness campaign as
`2d71134a55fc2f919daaf8cb7595efd3c7ebef79`. All selected local checks passed.
The PR's Dashboard, Service Client, Version Sync, Rust Quality, and Workstation
Fixtures jobs passed. Its unselected full Rust lane failed only in a pre-existing
workstation process-exit fixture now separately owned by issue #84 and PR #83;
Plan 0177 did not retry or absorb that source lane.

P177 leaves the active-lane catalog. P169 remains a published clean
`PAUSED_REF` at `32e7ec83`; P178 remains a separate active worktree and PR at
`f6b263f0`. Production maintenance and provider-backed acceptance remain
quarantined under issue #76. The canonical checkout is fast-forwarded only
after this closeout receipt integrates; no runtime mutation is part of closeout.

## Turn 313 | 2026-09-13

[Plan 0177](docs/dev/plans/0177-2026-09-13-development-governance-and-repository-readiness.md)
is OPEN through [issue #65](https://github.com/CochranResearchGroup/agent-browser/issues/65).
Policies 0047 through 0049, selector v0.1.25, the owned-fork registry, issue
forms, labels, and issues #65 through #85 are published on
`platform/plan-0177-execution`. The formerly untracked profile-selection note
is preserved and split into issues #67, #81, and #66.

Canonical `main` is clean and equal to `origin/main`. The prior no-op local and
remote branch is retired after PR63 custody. A concurrent installer repair is
tracked by [Plan 0178](docs/dev/plans/0178-2026-09-13-browserless-runtime-lane-quiescence.md),
issue #84, and PR83; it is not Plan 0177 implementation. Turnstile is
published at `ba90ef38`, includes current main, has one P169 identity, and is
paused behind issue #66 because live acceptance remains unproven.

Production is singular and not mid-install, but maintenance and provider-backed
acceptance are quarantined by issue #76 after two current-scale Service State
monitor lock timeouts. Issue #77 owns unknown stale process roots. Development
core is isolated; current-source provenance, status ports, and presentation
provider readiness remain issues #79, #78, and #80. No runtime mutation occurred.

### Active Planning Ledger

- [Plan 0012](docs/dev/plans/0012-2026-05-31-workspace-inspection-pane-app-intelligence-roadmap.md)
  is OPEN under issue #68; Plans 0018 and 0021 are superseded into it.
- [Plan 0078](docs/dev/plans/0078-2026-07-27-guacamole-route-fixture-recovery-interlock-plan.md)
  is BLOCKED under issue #85 on distinct XRDP route-display allocation.
- [Plan 0111](docs/dev/plans/0111-2026-08-13-multi-agent-shared-browser-profile-authority-plan.md)
  is OPEN under issue #69 for atomic shared-browser owner authority.
- [Plan 0116](docs/dev/plans/0116-2026-08-15-runtime-adoption-and-transactional-upgrade-plan.md)
  is OPEN under issue #70 for cooperative surrender and singular convergence.
- [Plan 0144](docs/dev/plans/0144-2026-08-31-lease-authority-coordination-and-revocation-plan.md)
  is OPEN under issue #71 for its remaining public, effect, and installed gates.
- [Plan 0158](docs/dev/plans/0158-2026-09-02-frozen-candidate-historical-failure-stress-campaign.md)
  is BLOCKED under issue #72 pending explicit protected-campaign reopening.
- [Plan 0160](docs/dev/plans/0160-2026-09-06-production-profile-identity-and-operational-readiness.md)
  remains OPEN as the umbrella production-readiness authority.
- [Plan 0162](docs/dev/plans/0162-2026-09-10-production-desktop-slot-and-jit-viewer-allocation.md)
  is OPEN under issue #73 for source work and separately gated installed proof.
- [Plan 0163](docs/dev/plans/0163-2026-09-10-profile-data-reset-backup-and-restore.md)
  is OPEN under issue #74 for provider-free source work; destructive use is gated.
- [Plan 0165](docs/dev/plans/0165-2026-09-11-bill-identifier-form-drift-repair.md)
  is BLOCKED under issue #75 on sealed authentication and read-only acceptance.

Closed or superseded during reconciliation: Plans 0018, 0021, 0037, 0045,
0069, 0091, 0114, 0123, and 0137. ROADMAP lanes P13, P14, P44, P69, and P91
are closed. Retained implementation and evidence remain in Git and their plans.

## Turn 312 | 2026-09-13

Plan 0177 was authored and merged through PR64 as `17ae56b0`. That merge is
source custody only because GitHub allowed it while Rust and Workstation
Fixtures were still running. The execution campaign revalidated its own changed
surfaces and preserves final CI as a separate closeout gate.
