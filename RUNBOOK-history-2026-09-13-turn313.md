# Runbook History | Turn 313

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
