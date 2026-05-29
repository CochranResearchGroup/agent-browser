# Runbook

This file records dated execution turns for repo governance, planning, release,
and operational handoff work. Detailed command output belongs in validation
notes or artifacts, not in this log.

## Turn 1 | 2026-05-26

Scope: repair the planning contract after adopting Graphiti and CodeGraph
policy modules.

Actions:

- Added top-level `ROADMAP.md` as the planning index.
- Added top-level `RUNBOOK.md` as the dated execution log.
- Wired `docs/dev/plans/0001-2026-05-26-rdp-guac-hardening-test-plan.md`
  into both planning authorities.
- Changed plan 0001's deterministic plan state to `CLOSED` while preserving
  its `VALIDATED` outcome.

Validation run:

- `python /home/ecochran76/workspace.local/agent-policies/repo-policy-selector/scripts/audit_planning_contract.py --repo-root /home/ecochran76/workspace.local/agent-browser --json`
- `git diff --check`

Result:

- Both checks passed for the planning-contract repair.

## Turn 2 | 2026-05-27

Scope: create the Guacamole remote-view routing hardening lane after roadmap
alignment review.

Actions:

- Added `docs/dev/plans/0002-2026-05-27-guac-remote-view-routing-hardening-plan.md`.
- Added P02 to `ROADMAP.md`.
- Kept P01 closed and made the hardcoded Guacamole route, metadata-only
  `view_takeover`, and external-open race the explicit P02 scope.

Validation run:

- `python /home/ecochran76/workspace.local/agent-policies/repo-policy-selector/scripts/audit_planning_contract.py --repo-root /home/ecochran76/workspace.local/agent-browser --json`
- `git diff --check`

Result:

- Both checks passed for the P02 planning turn.

## 2026-05-27 Turn 3 | P02 Route Authority And Takeover Events

Scope: implement the first Guacamole route hardening slices.

Actions:

- Added `docs/dev/notes/2026-05-27-guac-route-authority-audit.md`.
- Added service-owned `ViewStream` route metadata: `frameUrl`,
  `externalUrl`, `routeId`, `connectionId`, `connectionName`, and
  `routeSource`.
- Removed production Guacamole client-hash repair from Rust service status
  handling and the dashboard workspace viewport.
- Changed dashboard external open to await `view_takeover` acceptance before
  opening `externalUrl`.
- Changed `view_takeover` to return typed acceptance metadata and persist a
  `viewer_takeover_requested` service event with `viewerLeaseId` and route
  details.
- Updated README, CLI help, docs site pages, service contracts, generated
  observability client, harness artifacts, and the repo plus installed
  `agent-browser` skill.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml remote_headed_view_stream -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml guacamole -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml view_takeover -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_events -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml apply_remote_headed_launch_env_hints -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml apply_daemon_env_forwards_keychain_settings -- --test-threads=1`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm test:dashboard-view-streams`
- `pnpm test:dashboard-workspace-nodes`
- `pnpm test:dashboard-workspace-navigator`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:service-client-contract`
- `pnpm test:service-client-types`
- `pnpm --dir docs build`
- `pnpm build:dashboard`
- `pnpm test:dashboard-browser-table`
- `pnpm test:dashboard-browser-row-actions-render`
- `pnpm test:dashboard-launcher-eligibility`
- `pnpm test:dashboard-inspector-actions`
- `node --check scripts/smoke-remote-headed-utils.js`
- `node --check scripts/test-rdp-guac-browser-switch-live.js`
- `node --check scripts/test-rdp-guac-viewer-transfer-live.js`
- `pnpm test:rdp-gateway-readiness-live -- --require-html5-client`
- `AGENT_BROWSER_RDP_TEST_CLIENT_A_EXECUTABLE=/usr/bin/google-chrome AGENT_BROWSER_RDP_TEST_CLIENT_B_EXECUTABLE=/usr/bin/brave-browser AGENT_BROWSER_REMOTE_HEADED_DISPLAY=:0 pnpm test:rdp-guac-viewer-transfer-live`
- `AGENT_BROWSER_RDP_TEST_CLIENT_A_EXECUTABLE=/usr/bin/google-chrome AGENT_BROWSER_RDP_TEST_CLIENT_B_EXECUTABLE=/usr/bin/brave-browser AGENT_BROWSER_REMOTE_HEADED_DISPLAY=:0 pnpm test:rdp-guac-browser-switch-live`
- `git diff --check`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`

Result:

- Local source and contract validation passed.
- Live readiness, viewer-transfer, and browser-switch validation passed for
  the configured shared Guacamole route.
- Viewer-transfer artifacts:
  `/tmp/agent-browser-rdp-guac-hardening-2026-05-27T19-40-36-319Z`
- Browser-switch artifacts:
  `/tmp/agent-browser-rdp-guac-browser-switch-2026-05-27T19-41-29-855Z`

## 2026-05-29 Turn 4 | P05 Runtime Checkpoint And P06 Plan

Scope: refactor the P05 handoff after maintainer clarification that the
Guacamole/RDP campaign is not ready for a formal release.

Actions:

- Reframed P05 as a validated installed-runtime checkpoint instead of a release
  preparation lane.
- Replaced the P05 plan with
  `docs/dev/plans/0005-2026-05-29-runtime-checkpoint-and-no-release-handoff-plan.md`.
- Added P06 in
  `docs/dev/plans/0006-2026-05-29-guac-rdp-productization-hardening-plan.md`.
- Removed the public docs changelog `v0.27.0` entry and kept current work under
  `## Unreleased` in `CHANGELOG.md`.
- Kept `CHANGELOG.md` release markers around the latest published `0.26.1`
  release entry.
- Changed `.github/workflows/release.yml` to manual dispatch only so ordinary
  pushes to `main` cannot publish checkpoint work as a GitHub release.
- Updated `AGENTS.md` and `ROADMAP.md` with the formal release boundary:
  release only after the hardened many-to-many Guacamole/RDP operational
  milestone, including one-time-sudo install and fully diagnostic doctors.

Validation run:

- `git diff --check`
- `pnpm --dir docs build`
- `pnpm validation:select -- --base HEAD`
- `pnpm version:sync`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `agent-browser --version`
- `agent-browser install doctor --json`
- `agent-browser doctor remote-view --json`

Result:

- Checks passed. The installed runtime reports `agent-browser 0.27.0`, install
  doctor is successful with matching installed, workspace, and pnpm package
  binary checksum
  `e99093bb46891983afe71c2bf992a5f5c1ded16ecbbd29504a3e9e55a16be33f`, and
  remote-view doctor reports route pool, route displays, display access,
  privileged helper, and simultaneous viewing readiness with
  `requiresInteractiveSudo=false`.

## 2026-05-29 Turn 5 | P06 Doctor And Live-Gate Productization

Scope: execute and refactor the first P06 slice after auditing the installed
checkpoint against the productization issues from P05.

Actions:

- Added install-doctor remote-view privilege readiness fields for helper,
  sudoers, group, membership, helper check, nested issues, and
  `requiresInteractiveSudo`.
- Added remote-view doctor top-level issue codes, remediations, viewer browser
  and OCR prerequisites, install drift propagation, sudoers readiness, and
  many-to-many prerequisite status.
- Changed the many-to-many live harness to prefer installed `agent-browser`,
  hydrate route-pool and route-display environment from remote-view doctor
  output, auto-discover common viewer browsers, and fail public Guacamole route
  URLs with `non_embeddable_guacamole_url`.
- Updated README, CLI help, docs site pages, the repo skill guidance, the P06
  plan, the roadmap, and the P06 validation note.
- Rebuilt and installed the checkpoint binary to the local command, workspace
  binary, and pnpm package binary.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml install_doctor -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml remote_view_doctor -- --test-threads=1`
- `node --check scripts/test-rdp-guac-many-to-many-live.js`
- `node --check scripts/smoke-utils.js`
- `pnpm --dir docs build`
- `agent-browser install doctor --json`
- `agent-browser doctor remote-view --json`
- `AGENT_BROWSER_RDP_TEST_USE_INSTALLED=1 node scripts/test-rdp-guac-many-to-many-live.js`
- `AGENT_BROWSER_REMOTE_VIEW_URL=http://127.0.0.1:8092/guacamole/ AGENT_BROWSER_RDP_TEST_USE_INSTALLED=1 node scripts/test-rdp-guac-many-to-many-live.js`

Result:

- Installed doctor and remote-view doctor passed with no issues. The installed
  runtime checksum is
  `1b67077ccdb5e80d8667d3bcc8327e9c2a1a8521417c25280f71d059bc3b1694`.
- The public Guacamole URL invocation failed fast with the intended
  `non_embeddable_guacamole_url` precondition diagnostic.
- The local embeddable Guacamole many-to-many gate passed from the installed
  command with artifacts at
  `/tmp/agent-browser-rdp-guac-many-to-many-2026-05-29T14-06-07-291Z`.
- P06 remains open for clean-machine first-install sudo proof and the
  install-doctor service-readiness ownership decision.

## 2026-05-29 Turn 6 | P06 Install Doctor Service Probe And Idempotence

Scope: continue P06 by resolving the remaining install-doctor service
ownership decision and strengthening the already-provisioned privilege
installer re-run contract.

Actions:

- Added `data.service` to `agent-browser install doctor --json` using an
  isolated no-launch service-status probe.
- Made install doctor fail with `service_status_not_ready` when the no-launch
  service probe does not report ready.
- Changed `scripts/install-agent-browser-privileges.sh --apply` to exit before
  privileged changes when the helper source matches the installed helper, the
  sudoers file exists, the operator is in the `agent-browser` group, and
  `sudo -n <helper> check` succeeds.
- Updated CLI help, README, docs site installation/service-mode pages, skill
  guidance, the P06 plan, roadmap, and validation note.

Validation run:

- `cargo run --quiet --manifest-path cli/Cargo.toml -- install doctor --json`
- `cargo test --manifest-path cli/Cargo.toml install_doctor -- --test-threads=1`
- `bash -n scripts/install-agent-browser-privileges.sh`
- `AGENT_BROWSER_PRIVILEGED_HELPER_SOURCE=scripts/libexec/agent-browser-privileged-helper bash scripts/install-agent-browser-privileges.sh --dry-run`
- `AGENT_BROWSER_PRIVILEGED_HELPER_SOURCE=scripts/libexec/agent-browser-privileged-helper bash scripts/install-agent-browser-privileges.sh --apply`
- `pnpm build:native`
- `agent-browser install doctor --json`
- `agent-browser doctor remote-view --json`

Result:

- The source-build install doctor showed the new service probe as ready and
  no-launch, while still correctly reporting source/install binary drift.
- The already-provisioned helper installer re-run exited with "already ready"
  and made no privileged changes.
- The rebuilt installed runtime checksum is
  `1ec7a0528944fad76fc4b3c2539b57b15944a503126038e47fb9d8727bdfa53a`.
- Installed doctor and remote-view doctor passed with no issues, and install
  doctor reports `data.service.ready=true` plus `data.service.noLaunch=true`.
- P06 remains open for clean-host or equivalent reset-fixture proof that first
  install uses one clear sudo authorization boundary.

## 2026-05-29 Turn 7 | P06 Closeout

Scope: finish P06 by proving the first-install sudo boundary with an equivalent
clean reset fixture, validating route-pool restart durability, and running the
final installed gates.

Actions:

- Added `pnpm test:install-privileges-clean-fixture`, which runs the privilege
  installer against fake `sudo`, `getent`, `id`, `groupadd`, `usermod`, and
  `visudo` under a temp install root.
- Reordered the Linux install path so
  `agent-browser install --with-deps --with-remote-view-privileges` runs
  remote-view privilege setup before dependency installation.
- Added a Rust guard that keeps remote-view privilege setup before Linux
  dependency installation.
- Updated README, docs site installation guidance, skill guidance, P06 plan,
  roadmap, and P06 validation note.
- Rebuilt and installed the checkpoint binary to the local command, workspace
  binary, and pnpm package binary.

Validation run:

- `pnpm test:install-privileges-clean-fixture`
- `cargo test --manifest-path cli/Cargo.toml install_orders_remote_view_privileges_before_linux_deps -- --test-threads=1`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm --dir docs build`
- `pnpm build:native`
- `agent-browser install doctor --json`
- `agent-browser doctor remote-view --json`
- `node scripts/smoke-rdp-guac-route-pool-readiness.js --report-only`
- `docker restart agent-browser-guacamole agent-browser-guacd && node scripts/smoke-rdp-guac-route-pool-readiness.js --report-only`
- `pnpm sync:rdp-guac-existing-user-route-pool`
- `pnpm grant:rdp-route-display-access -- --apply`
- `agent-browser --json get title`
- `AGENT_BROWSER_REMOTE_VIEW_URL=http://127.0.0.1:8092/guacamole/ AGENT_BROWSER_RDP_TEST_USE_INSTALLED=1 node scripts/test-rdp-guac-many-to-many-live.js`
- `AGENT_BROWSER_RDP_TEST_USE_INSTALLED=1 node scripts/test-rdp-guac-many-to-many-live.js`

Result:

- The clean-fixture smoke proved first apply uses exactly one explicit
  `sudo -v` boundary and second apply does not add another prompt boundary or
  repeat privileged install commands.
- Installed doctor and remote-view doctor passed with no issues. The final
  P06 installed runtime checksum is
  `cb9f81a245464c516d313aee875fa076049cdc5559e9342250c9680463faa9e4`.
- Route-pool readiness survived Guacamole web and guacd restarts.
- Route sync and route-display access grant reruns passed without interactive
  sudo.
- Default command attach passed.
- The local embeddable Guacamole many-to-many gate passed with artifacts at
  `/tmp/agent-browser-rdp-guac-many-to-many-2026-05-29T14-39-55-085Z`.
- The public Guacamole URL invocation failed fast with the intended
  `non_embeddable_guacamole_url` diagnostic and artifacts at
  `/tmp/agent-browser-rdp-guac-many-to-many-2026-05-29T14-40-34-292Z`.
- P06 is closed. Formal release work remains a separate lane.

## 2026-05-29 Turn 8 | P07 v0.27.0 Formal Release Prep

Scope: open the formal release lane now that P06 closed the Guacamole/RDP
productization blocker.

Actions:

- Created
  `docs/dev/plans/0007-2026-05-29-v0-27-0-formal-release-plan.md`.
- Moved `CHANGELOG.md` release extraction markers from `0.26.1` to `0.27.0`.
- Added the public docs changelog entry for `v0.27.0` dated May 29, 2026.
- Added P07 to `ROADMAP.md`.
- Added release-preparation validation note
  `docs/dev/notes/2026-05-29-p07-v0-27-0-release-prep-validation.md`.

Validation run:

- `git log v0.26.1..HEAD --format='%an <%ae>' | sort -u`
- `pnpm version:sync`
- `git diff --check`
- `pnpm validation:select -- --base HEAD`
- `pnpm --dir docs build`
- `agent-browser install doctor --json`
- `agent-browser doctor remote-view --json`

Result:

- Local release-preparation validation passed. The installed runtime checksum
  remains
  `cb9f81a245464c516d313aee875fa076049cdc5559e9342250c9680463faa9e4`.
- P07 remains open for release PR merge, release workflow dry run, real
  release workflow run, and GitHub release asset verification.

## 2026-05-29 Turn 9 | P07 Release Dry Run Cross-Target Fix

Scope: respond to the first manual `Release` workflow dry-run failure.

Actions:

- Ran the `Release` workflow with `dry_run=true` on `main`.
- Confirmed release-state precheck passed.
- Diagnosed the platform build failures as a Rust cfg leak in
  `cli/src/native/cdp/chrome.rs`.
- Kept the private remote-headed virtual-display fallback inside the Linux cfg
  block so non-Linux targets do not reference Linux-only helpers.
- Added
  `docs/dev/notes/2026-05-29-p07-release-dry-run-cross-target-fix.md`.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml private_remote_display -- --test-threads=1`
- `cargo check --manifest-path cli/Cargo.toml --target x86_64-pc-windows-gnu`

Result:

- Format, clippy, and the focused private remote-display unit test passed.
- The local Windows cross-target check advanced past the previous missing
  symbols, then stopped because this workstation lacks
  `x86_64-w64-mingw32-gcc` for the `ring` build script.
- The release workflow dry run must be retried after this fix lands on `main`.

## 2026-05-29 Turn 10 | P07 Linux Release Link Fix

Scope: respond to the second manual `Release` workflow dry-run failure.

Actions:

- Reran the `Release` workflow with `dry_run=true` on `main`.
- Confirmed Windows x64, macOS x64, and macOS ARM64 passed after the cfg fix.
- Diagnosed Linux target failures as release-time `-lX11` linking from the
  browser-focus helper.
- Changed the Linux X11 focus helper to load `libX11` dynamically with
  `dlopen` and `dlsym` at runtime instead of statically linking X11.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml browser -- --test-threads=1`
- `git diff --check`
- `cargo build --release --manifest-path cli/Cargo.toml`
- `rg -n "#\\[link\\(name = \\\"X11\\\"\\)|-lX11" cli/src`

Result:

- Local validation passed.
- No static X11 link remains in `cli/src`.
- The local machine does not have `cargo-zigbuild`, so the release workflow
  dry run must be retried after this fix lands on `main`.

## 2026-05-29 Turn 11 | P07 v0.27.0 Release Publication

Scope: publish and verify the formal `v0.27.0` GitHub release.

Actions:

- Reran the manual `Release` workflow with `dry_run=true`.
- Ran the manual `Release` workflow with `dry_run=false` after the dry run
  passed.
- Verified the public GitHub release and asset list.
- Closed P07 in the roadmap and plan surfaces.

Validation run:

- `gh run view 26648621169 --json conclusion,url,headSha`
- `gh run view 26649196974 --json conclusion,url,headSha`
- `gh release view v0.27.0 --json tagName,name,url,isDraft,isPrerelease,assets,targetCommitish`
- `git fetch --tags origin`
- `git rev-list -n1 v0.27.0`
- `git rev-parse origin/main`

Result:

- Dry run succeeded:
  `https://github.com/CochranResearchGroup/agent-browser/actions/runs/26648621169`
- Real release run succeeded:
  `https://github.com/CochranResearchGroup/agent-browser/actions/runs/26649196974`
- Release URL:
  `https://github.com/CochranResearchGroup/agent-browser/releases/tag/v0.27.0`
- Release commit and `origin/main` both resolve to
  `17a284f8624e6108473970e2ec2b380debf9f7ac`.
- The release is not a draft, is not a prerelease, and has seven assets:
  `agent-browser-darwin-arm64`, `agent-browser-darwin-x64`,
  `agent-browser-linux-arm64`, `agent-browser-linux-musl-arm64`,
  `agent-browser-linux-musl-x64`, `agent-browser-linux-x64`, and
  `agent-browser-win32-x64.exe`.
