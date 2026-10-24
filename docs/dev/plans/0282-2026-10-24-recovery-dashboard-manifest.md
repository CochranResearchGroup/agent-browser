# P282: Recovery dashboard identity across controller builds

Status: IN_PROGRESS
Product lane: PL-BUGFIX
Work item: CochranResearchGroup/agent-browser#405
Branch: fix/recovery-dashboard-manifest-405
Baseline: 07fb4c9cb020bb6349d3faf1efcd152cc34fd9ee

Complete the authorized combined deployment by repairing read-only selected-dashboard identity proof. The recovery controller currently synthesizes older dashboard assets from its own embedded bundle. Reuse the existing observed-backend proof bound to the sealed selected executable path and SHA. Keep registered manifest, generation, presentation, host, socket and checkpoint requirements.

Reuse the clean dashboard primary. Own current_selection.rs and remove its now-unused synthesis helper in dashboard_ingress.rs; P280 owns static delivery. Preserve dirty authentication, enrollment candidates, profile-proof, mode401 and operator-led-upgrade checkouts. No shared installer orchestration edits or provider/ingress changes. Source PR must merge before publishing a repaired controller. Qualification: differing-assets red/green, executable and registered-digest refusals, all current-selection tests, format, strict Clippy and installer fixture. Existing 07fb candidate remains pinned and is not relabeled. A second production-shaped build is justified only by this reproduced recovery blocker.

## Source qualification

Differing-assets regression reproduced the production error before repair; all five current-selection cases pass afterward. Strict workspace Clippy passes. See the [evidence note](../notes/0282-2026-10-24-recovery-dashboard-manifest.md). Installed acceptance remains pending.
