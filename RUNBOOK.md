# Runbook

This file records dated execution turns for repo governance, planning, release,
and operational handoff work. Detailed command output belongs in validation
notes or artifacts, not in this log.

## Turn 19 | 2026-06-21

Scope: repair the Plan 0039 audit findings after closeout review.

Actions:

- Made `agent-browser remote-view open` accept the documented
  `--browser-build stealthcdp_chromium` and `--provider rdp_gateway` flags.
- Added post-launch failure cleanup to `remote_view_open`: tab open, focus, visible-window proof, or checkout failures now clean up before returning the typed error. New
  browser launches close the browser; reused retained browsers preserve the
  browser process and close only the opened tab when possible.
- Updated CLI help, README, docs command page, repo skill guidance, Plan 0039,
  and P16 roadmap text for the accepted flags and cleanup boundary.

Validation run:

- `cargo test --manifest-path cli/Cargo.toml test_remote_view_open_builds_route_bound_service_action -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml test_remote_view_open_cleanup_reports_new_browser_close_on_failure -- --test-threads=1`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml remote_view -- --test-threads=1`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml service_model -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_access_plan -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_health -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_contracts -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_config -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:service-client`
- `pnpm --dir docs build`
- `pnpm test:dashboard-view-streams`
- `pnpm test:dashboard-browser-row-actions-render`
- `pnpm test:dashboard-browser-table`
- `pnpm test:dashboard-workspace-navigator`
- `pnpm test:dashboard-inspector-actions`
- `pnpm build:dashboard`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `git diff --check`
- `python /home/ecochran76/workspace.local/agent-policies/repo-policy-selector/scripts/audit_planning_contract.py --repo-root /home/ecochran76/workspace.local/agent-browser --json`

Result:

- The focused Plan 0039 parser and cleanup tests passed, the non-live Rust,
  client, docs, and dashboard gates above passed, and the installed skill copy
  matches the repo skill.
- The direct documented dry-run command
  `agent-browser remote-view open --runtime-profile stealthcdp-default
  --browser-build stealthcdp_chromium --provider rdp_gateway --url
  https://www.linkedin.com/ --dry-run` returned `success=true` and
  `status=planned`.
- The repo-wide planning audit still reports older unrelated drift, but the
  Plan 0039 row remains clean: `state=CLOSED`, `current_state_ok=true`,
  `wired_in_roadmap=true`, and `wired_in_runbook=true`.

## Turn 18 | 2026-06-21

Scope: close Plan 0039 by making the route-specific `remote_view_open` lane the
documented default and proving it on the installed binary.

Actions:

- Added prelaunch route-display access repair to `remote_view_open`: it probes
  the selected route display, invokes the installed privileged helper when
  access is missing, and fails with typed display-access errors if access still
  cannot be proven.
- Fixed route binding selection so checked-out retained routes reuse their
  existing display allocation when no inline route material overrides them.
- Updated README, CLI help, docs site, service-request contract description,
  repo skill, installed skill, Plan 0039, ROADMAP, and downstream handoff note
  `docs/dev/notes/2026-06-21-remote-view-open-route-specific-handoff.md`.
- Rebuilt and installed binary SHA
  `54248451b6bea3ced7acb6df8dd3e0f7514c866e08584bb025569a2ec6ad28ad` into
  `~/.local/bin/agent-browser`, `bin/agent-browser-linux-x64`, and the pnpm
  package binary.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open -- --test-threads=1`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm --dir docs build`
- `pnpm test:service-client`
- `cargo test --manifest-path cli/Cargo.toml remote_view_doctor -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_contracts -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_model -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_access_plan -- --test-threads=1`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:dashboard-view-streams`
- `pnpm test:dashboard-inspector-actions`
- `pnpm build:dashboard`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `agent-browser install doctor --json`
- `agent-browser doctor remote-view --json`
- `pnpm test:remote-view-open-fixture-live`
- `pnpm test:rdp-guac-many-to-many-live`
- `git diff --check`
- `python /home/ecochran76/workspace.local/agent-policies/repo-policy-selector/scripts/audit_planning_contract.py --repo-root /home/ecochran76/workspace.local/agent-browser --json`

Result:

- `agent-browser install doctor --json` passed with no issues and aligned SHA
  `54248451b6bea3ced7acb6df8dd3e0f7514c866e08584bb025569a2ec6ad28ad`.
- `agent-browser doctor remote-view --json` reported `status=ready`,
  `remoteControl.status=ready`, `remoteControl.routeId=guacamole:3`,
  `remoteControl.displayName=:11`, and `manyToMany.status=ready`.
- `pnpm test:remote-view-open-fixture-live` passed with artifact directory
  `/tmp/agent-browser-remote-view-open-live-2026-06-21T01-24-32-095Z`.
- `pnpm test:rdp-guac-many-to-many-live` passed with artifact directory
  `/tmp/agent-browser-rdp-guac-many-to-many-2026-06-21T01-24-32-207Z`.
- `git diff --check` passed.
- The repo-wide planning audit still reports older unrelated planning-contract
  drift, but the Plan 0039 row is clean: `state=CLOSED`,
  `current_state_ok=true`, `wired_in_roadmap=true`, and
  `wired_in_runbook=true`.
- Plan 0039 and P16 are closed.

## Turn 17 | 2026-06-20

Scope: continue Plan 0039 remote-control ready command hardening after the
route-specific Guacamole/RDP lane exposed stale retained route state.

Actions:

- Repaired the retained service route pool from the current route-pool
  readiness report after backing up
  `~/.agent-browser/service/state.json.pre-route-pool-refresh-2026-06-21T00-56-42-211Z`.
- Changed `remote_view_open` route binding to prefer supplied/current
  route-pool identity over stale retained route id and display allocation
  state.
- Made requested route-pool entry id authoritative for allocation lookup and
  allowed top-level `readiness.state=ready` route-pool entries to be used even
  when informational nested components are not ready.
- Updated the remote-view open live smoke to use the selected route entry's
  display name and display isolation for CLI, HTTP, state, and X11 checks.
- Rebuilt and installed the local binary into `~/.local/bin/agent-browser`,
  `bin/agent-browser-linux-x64`, and the pnpm global package binary.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_dry_run_prefers_inline_route_pool_identity_over_stale_state -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml remote_view_doctor -- --test-threads=1`
- `node --check scripts/smoke-rdp-guac-route-pool-readiness.js`
- `node --check scripts/open-rdp-guac-route-displays.js`
- `node --check scripts/test-rdp-guac-many-to-many-live.js`
- `node --check scripts/smoke-remote-view-open-live.js`
- `pnpm test:remote-view-open-fixture-live`
- `pnpm test:rdp-guac-many-to-many-live`
- `agent-browser install doctor --json`
- `agent-browser doctor remote-view --json`
- `git diff --check`

Result:

- Route-specific `remote-view open` dry-run resolves `guacamole-rdp-a` to
  `guacamole:3`, display `:11`, and display allocation
  `remote-view-display:11`.
- `pnpm test:remote-view-open-fixture-live` passed with artifact directory
  `/tmp/agent-browser-remote-view-open-live-2026-06-21T01-05-37-262Z`.
- `pnpm test:rdp-guac-many-to-many-live` passed with artifact directory
  `/tmp/agent-browser-rdp-guac-many-to-many-2026-06-21T01-05-55-809Z`.
- `agent-browser doctor remote-view --json` reports `status=ready`,
  `remoteControl.status=ready`, and `manyToMany.status=ready`.
- Plan 0039 remains open only for Slice F documentation and downstream
  handoff closeout.

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

## Turn 3 | 2026-05-27

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

## Turn 4 | 2026-05-29

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

## Turn 5 | 2026-05-29

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

## Turn 6 | 2026-05-29

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

## Turn 7 | 2026-05-29

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

## Turn 8 | 2026-05-29

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

## Turn 9 | 2026-05-29

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

## Turn 10 | 2026-05-29

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

## Turn 11 | 2026-05-29

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

## Turn 12 | 2026-05-29

Scope: repair stale planning-audit residue after the `v0.27.0` release.

Actions:

- Normalized historical runbook headings to the deterministic
  `## Turn N | YYYY-MM-DD` format.
- Changed P02 plan state from `VALIDATED` to deterministic `CLOSED` while
  preserving `Outcome: VALIDATED`.
- Changed P03 plan state from `COMPLETE` to deterministic `CLOSED` while
  preserving `Outcome: COMPLETE`.
- Wired the existing P03 and P04 plan filenames into this runbook:
  `docs/dev/plans/0003-2026-05-28-guac-rdp-many-to-many-viewing-plan.md` and
  `docs/dev/plans/0004-2026-05-29-release-candidate-install-validation-plan.md`.

Validation run:

- `python /home/ecochran76/workspace.local/agent-policies/repo-policy-selector/scripts/audit_planning_contract.py --repo-root /home/ecochran76/workspace.local/agent-browser --json`
- `git diff --check`

Result:

- Both checks passed. The planning audit now reports `ok: true`, no problems,
  no open roadmap lanes, deterministic state for every plan, and runbook plus
  roadmap wiring for every plan.

## Turn 13 | 2026-05-30

Scope: open the CDP tab streaming lane for non-remote browsers.

Actions:

- Ran Graphiti discovery against `agent_browser_main` for prior CDP streaming
  context.
- Inspected the existing CDP stream server, stream WebSocket, service
  view-stream model, action-derived view streams, dashboard view-stream
  rendering, roadmap, and runbook surfaces.
- Added
  `docs/dev/plans/0008-2026-05-30-cdp-tab-streaming-for-non-remote-browsers-plan.md`.
- Added P08 to `ROADMAP.md`.

Validation run:

- `python /home/ecochran76/workspace.local/agent-policies/repo-policy-selector/scripts/audit_planning_contract.py --repo-root /home/ecochran76/workspace.local/agent-browser --json`
- `git diff --check`
- `pnpm validation:select -- --base HEAD`

Result:

- Planning audit passed with `ok: true`, no problems, and P08 wired through the
  roadmap, runbook, and open plan file.
- `git diff --check` passed.
- `pnpm validation:select -- --base HEAD` selected only `git diff --check` for
  the documentation-only planning slice.

## Turn 14 | 2026-06-04

Scope: open a resource monitor and garbage collector lane after live
agent-browser resource pressure cleanup.

Actions:

- Ran Graphiti discovery against `agent_browser_main` for prior resource
  cleanup and service lifecycle context.
- Confirmed the related retained orphan profile cleanup plan exists, but it
  covers service-state/profile metadata rather than live OS process pressure.
- Added
  `docs/dev/plans/0026-2026-06-04-resource-monitor-and-garbage-collector-plan.md`.
- Added P13 to `ROADMAP.md` with the dry-run-first resource monitor and GC
  recommendation.

Validation run:

- `git diff --check`
- `pnpm validation:select -- --base HEAD`
- `python /home/ecochran76/workspace.local/agent-policies/repo-policy-selector/scripts/audit_planning_contract.py --repo-root /home/ecochran76/workspace.local/agent-browser --json`

Result:

- P13 is open for Slice A and Slice B: read-only resource inventory plus
  conservative stale classification before any apply-mode garbage collection.
- `git diff --check` passed.
- `pnpm validation:select -- --base HEAD` included the pre-existing dirty
  dashboard files in its recommendation set, so it selected dashboard checks in
  addition to the documentation-only change.
- The planning audit still fails due to pre-existing roadmap/runbook drift for
  older plans, but the new P13 plan is wired in both `ROADMAP.md` and
  `RUNBOOK.md`.

## Turn 15 | 2026-06-05

Scope: open and start the minimal runtime-profile reuse lane after Plan 0026
closed the resource-monitor and GC cleanup surface.

Actions:

- Ran Graphiti discovery against `agent_browser_main` for profile reuse,
  service queue, lease, and access-plan context.
- Added
  `docs/dev/plans/0027-2026-06-05-minimal-runtime-profile-reuse-plan.md`.
- Updated P13 in `ROADMAP.md` so Plan 0026 is the closed cleanup surface and
  Plan 0027 is the prevention surface.

Current target:

- Plan 0027 Slice A: add a read-only access-plan `profileReuse` advisory that
  recommends `reuse_existing_browser`, `wait_for_profile_lease`, or
  `launch_new_browser` before any launch mutates runtime state.

## Turn 16 | 2026-06-13

Scope: write an implementation handoff note for AuraCall-driven browser
service feature requests.

Actions:

- Ran Graphiti discovery against `agent_browser_main` and verified the local
  Graphiti runtime was healthy.
- Reviewed the existing access-plan service-request handoff note and the
  service request/client contract surfaces.
- Added
  `docs/dev/notes/2026-06-13-auracall-cdp-feature-requests.md`.
- Patched the note so AuraCall source paths are explicitly relative to the
  sibling `../auracall` repository.

Validation run:

- `git diff --check`
- Verified the listed agent-browser source surfaces exist in this repository.
- Verified the listed AuraCall source surfaces exist under the sibling
  `../auracall` repository.
- Ran Graphiti discovery against `agent_browser_main` for AuraCall CDP
  migration, BYOP, controlled CDP attach, bounded evaluate, and service tab
  handle context.

Result:

- The handoff note requests profile-origin and BYOP registration, a
  lease-backed service tab handle, controlled CDP attach, bounded evaluate
  jobs, readiness and identity probe recipes, tab reuse repair, diagnostic
  evidence bundles, and service-client ergonomics.
- The note keeps provider-specific ChatGPT, Gemini, Grok, and AuraCall
  semantics out of agent-browser and frames the work as service-owned browser
  primitives for a future implementation agent.

## Turn 17 | 2026-06-13

Scope: open a high-level upgrade plan suitable for subagents and goal-driven
execution.

Actions:

- Added
  `docs/dev/plans/0033-2026-06-13-auracall-service-cdp-upgrade-plan.md`.
- Added P14 to `ROADMAP.md`.
- Structured the plan as a parent goal with slice-level subagent prompts,
  acceptance criteria, coordination rules, validation matrix, and open
  questions.

Validation run:

- `git diff --check`
- `python /home/ecochran76/workspace.local/agent-policies/repo-policy-selector/scripts/audit_planning_contract.py --repo-root /home/ecochran76/workspace.local/agent-browser --json`
- `pnpm validation:select -- --base HEAD`

Result:

- P14 is open for profile origin/BYOP, lease-backed service tab handles,
  controlled CDP attach, bounded evaluate, diagnostics/readiness evidence, and
  client ergonomics.
- The first recommended implementation slice is P14 Slice A: profile-origin
  schema plus explicit BYOP registration/readback.

## Turn 18 | 2026-06-13

Scope: implement P14 Slice A profile-origin and BYOP registration/readback.

Actions:

- Added durable service profile origin values:
  `agent_browser_owned`, `external_byop`, and `external_observed`.
- Added external profile registration metadata and browser compatibility
  evidence to service profile records.
- Added `registerExternalProfile()` to
  `@agent-browser/client/service-observability` for explicit BYOP or observed
  external profile registration.
- Exposed `profileOrigin` through service profile allocation readback and
  access-plan selected profiles.
- Hardened retained-state orphan profile pruning so `external_byop` and
  `external_observed` profiles are never pruned as owned profile data.
- Preserved profile origin and external metadata through the dashboard profile
  config save path.
- Updated service schemas, generated client types, README, docs site, and the
  installed agent-browser skill.

Validation run:

- `git diff --check`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml service_model -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_access_plan -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_profiles -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml test_prune_retained_service_state_removes_orphaned_custom_profiles -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `pnpm test:service-client`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:dashboard-profile-allocation`
- `pnpm --dir docs build`
- `pnpm build:dashboard`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`

Result:

- Slice A is implemented as a no-launch contract slice.
- Access-plan and profile readback can distinguish owned, BYOP, and observed
  external profile lanes.
- Explicit external profile registration records caller identity, target
  identities, account ids, user-data directory, and browser compatibility
  evidence.
- The next recommended P14 slice is Slice B: lease-backed service tab handles.

## Turn 19 | 2026-06-13

Scope: implement P14 Slice B lease-backed service tab handles.

Actions:

- Added `ServiceTabHandle` and `ServiceTabHandleTraceFilter` to the service
  model.
- Derived tab handles from service state for `service tabs`, grouped browser
  `tabHandles`, and tab lifecycle trace event details.
- Extended direct `tab_new` responses with CDP target/session IDs and a
  conservative immediate `serviceTabHandle`.
- Added `getServiceTabHandle()` and `requireServiceTabHandle()` to
  `@agent-browser/client/service-request`.
- Updated service tab/browser schemas, generated client declarations, README,
  docs site, and the installed agent-browser skill.
- Added no-launch Rust and service-client fixtures for valid handles, binding
  fields, and stale-handle rejection.

Validation run:

- `git diff --check`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml service_model -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_health -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `pnpm test:service-client`
- `pnpm test:service-api-mcp-parity`
- `pnpm --dir docs build`
- `pnpm validation:select -- --base HEAD`
- `node scripts/dev/select-validation.js --base HEAD --json`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`

Result:

- Slice B is implemented as a no-launch contract slice.
- Software clients can use the returned service tab handle instead of
  rediscovering browser, session, profile, tab, target, lease, or trace
  identity.
- Stale handles fail closed through the client helper and expose explicit
  stale reasons in service readbacks.
- The selector recommended `pnpm test:service-cdp-tab-streaming-live` because
  browser/tab surfaces changed; that live smoke was deferred to Slice C unless
  live proof is requested before controlled CDP attach work starts.
- The next recommended P14 slice is Slice C: controlled CDP attach for leased
  service tab handles.

## Turn 20 | 2026-06-13

Scope: implement P14 Slice C controlled CDP attach for leased service tab
handles.

Actions:

- Added `cdp_attach` and `cdp_detach` to the service request action metadata,
  HTTP relay, MCP service request surface, Rust daemon dispatcher, JSON schema,
  generated client types, and `@agent-browser/client/service-request` helpers.
- Gated attach on a valid `serviceTabHandle`, `cdpAttachmentAllowed: true`,
  non-CDP-free posture, matching service session, handle freshness, and target
  identity.
- Returned a service-owned attach descriptor with browser, session, tab,
  target, profile, lease, cleanup, trace, websocket, and detach metadata.
- Made detach preserve the browser process by default and return explicit
  detach metadata.
- Updated README, docs site, repo skill, and installed agent-browser skill for
  the new attach/detach helper path.
- Updated P14 plan and ROADMAP so Slice D bounded evaluate is the next
  implementation target.

Validation run:

- `git diff --check`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml service_request_command -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml service_contracts -- --test-threads=1`
- `pnpm test:service-client`
- `pnpm test:service-api-mcp-parity`
- `pnpm --dir docs build`
- `pnpm validation:select -- --base HEAD`
- `node scripts/dev/select-validation.js --base HEAD --json`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `pnpm test:service-cdp-tab-streaming-live`

Result:

- Slice C is implemented with no-launch policy and stale-handle coverage.
- Live CDP tab-streaming smoke passed for `session:cdp-tab-stream-98925`,
  stream `37669`.
- A dedicated attach-read-detach live smoke remains as the validation gap before
  treating controlled attach as AuraCall migration proof.
- The next recommended P14 slice is Slice D: bounded evaluate against leased
  service tab handles.

## Turn 21 | 2026-06-13

Scope: implement P14 Slice D bounded evaluate against leased service tab
handles.

Actions:

- Added `evaluate` to the service request action metadata, HTTP relay, MCP
  service request surface, JSON schema, generated client types, and
  `@agent-browser/client/service-request` helpers.
- Required `serviceTabHandle`, `script` or `expression`, positive `timeoutMs`,
  and positive `maxReturnBytes` for service-owned evaluate requests.
- Made service-bound evaluate skip browser auto-launch, switch to the handle's
  CDP target, execute with a daemon-side timeout, cap serialized return data,
  and return URL/title plus truncation metadata.
- Added no-launch HTTP, MCP, and service-client coverage for missing handles,
  missing caps, stale handles, and helper request shape.
- Updated README, docs site, repo skill, installed agent-browser skill, P14
  plan, and ROADMAP for the new bounded evaluate helper path.

Validation run:

- `git diff --check`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml service_request_command -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml service_contracts -- --test-threads=1`
- `pnpm test:service-client`
- `pnpm test:service-api-mcp-parity`
- `pnpm --dir docs build`
- `pnpm validation:select -- --base HEAD`
- `node scripts/dev/select-validation.js --base HEAD --json`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `pnpm test:service-cdp-tab-streaming-live`

Result:

- Slice D is implemented with no-launch contract coverage.
- Live CDP tab-streaming smoke passed for `session:cdp-tab-stream-73918`,
  stream `37595`.
- A dedicated live bounded-evaluate smoke remains as the validation gap before
  treating bounded evaluate as AuraCall migration proof.
- Screenshot-on-failure capture is deferred to Slice E diagnostic bundles so
  screenshot storage, caps, and trace links are implemented in one evidence
  surface.
- The next recommended P14 slice is Slice E: diagnostics and readiness
  evidence.

## Turn 22 | 2026-06-13

Scope: implement the P14 Slice E diagnostic bundle sub-slice for leased service
tab handles.

Actions:

- Added `diagnostics` to service request action metadata, HTTP relay, MCP
  service request validation, Rust daemon dispatch, JSON schema, generated
  client types, and `@agent-browser/client/service-request` helpers.
- Required a valid `serviceTabHandle` and reused the service-owned queue and
  handle validation path rather than adding a caller-owned browser path.
- Returned a compact evidence bundle with URL/title, browser/session/tab
  identity, profile readiness, route/view metadata, browser health, console
  entries, page errors, recent request summaries, snapshot summary, caller
  context, trace filter, and optional screenshot path.
- Added no-launch client helper coverage for request shape, stale handles, and
  evidence count caps.
- Updated README, docs site, repo skill, P14 plan, and ROADMAP for the new
  diagnostic helper path.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml service_request_command -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_contracts -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `pnpm generate:service-client`
- `pnpm test:service-client`
- `pnpm test:service-api-mcp-parity`
- `pnpm --dir docs build`
- `pnpm validation:select -- --base HEAD`
- `node scripts/dev/select-validation.js --base HEAD --json`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `pnpm test:service-cdp-tab-streaming-live`

Result:

- Slice E diagnostic bundles are implemented with no-launch contract coverage.
- Live CDP tab-streaming smoke passed for `session:cdp-tab-stream-95746`,
  stream `36831`.
- Slice E remains open for readiness/freshness lifecycle gating and any focused
  live diagnostics smoke requested before AuraCall migration proof.

## Turn 23 | 2026-06-20

Scope: open the corrective planning lane for recurring Guacamole/RDP
false-ready states after the live LinkedIn manual-auth route repair.

Actions:

- Added
  `docs/dev/plans/0039-2026-06-20-remote-control-ready-command-plan.md`.
- Added P16 to `ROADMAP.md`.
- Made the combined readiness invariant explicit: a remote-control browser is
  ready only when the selected browser window is loaded, visible, and
  controllable through the selected external Guacamole/RDP route.
- Captured the two recurring failure classes as plan gates:
  - Guacamole unhappy document or internal error caused by schema, route, URL,
    or permission drift.
  - Terminal-only remote desktop caused by browser/display mismatch.
- Scoped the next fix as a generic one-command/API path,
  `agent-browser remote-view open` and service action `remote_view_open`,
  rather than a LinkedIn-specific or AuraCall-specific repair.

Validation run:

- `python /home/ecochran76/workspace.local/agent-policies/repo-policy-selector/scripts/audit_planning_contract.py --repo-root /home/ecochran76/workspace.local/agent-browser --json`
- `git diff --check`

Result:

- Focused Plan 0039 validation passed. The broad planning audit remains red
  from pre-existing historical plan drift, but it reports no Plan 0039
  problems.
- Implementation remains open under Plan 0039. Slice A and Slice B are the
  recommended parallel starting points.

## Turn 24 | 2026-06-22

Scope: open the runtime convergence lane after remote-view and dashboard
binary harmonization exposed remaining runtime identity confusion.

Actions:

- Added `docs/dev/plans/0042-2026-06-22-runtime-convergence-plan.md`.
- Added P42 to `ROADMAP.md`.
- Captured the missing invariant: the dashboard runtime manifest proves only
  the dashboard service identity, not every active daemon session, stream
  backend, route helper, retained browser row, or foreign CDP browser.
- Scoped executable slices for active runtime inventory, daemon executable
  SHA-256 convergence, actionable doctor remedies, idempotent remote-view
  bootstrap, live rail boundaries, and one-command local convergence.
- Kept P41 foreign CDP discovery as a separate dependency so non-owned browser
  addressability is not confused with lifecycle ownership.

Validation run:

- `git diff --check`

Result:

- P42 is active and not complete. Slice D is already in progress through the
  Guacamole Postgres/schema bootstrap guard. The next implementation slice is
  daemon executable SHA convergence and active runtime inventory.

## Turn 25 | 2026-06-22

Scope: execute P42 Slice B daemon executable SHA convergence.

Actions:

- Added daemon executable SHA metadata next to the existing daemon version
  metadata.
- Made daemon reuse compare the invoking executable SHA-256 against the daemon
  SHA metadata when the invoking executable can be hashed.
- Treated missing daemon SHA metadata as stale by default, with
  `AGENT_BROWSER_ALLOW_LEGACY_DAEMON_SHA_REUSE=1` as an explicit reviewed
  compatibility escape hatch.
- Extended stale daemon cleanup to remove `<session>.sha256`.
- Updated P42 Slice B completion notes.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml daemon_executable_sha -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml cleanup_stale_files_removes_version_and_executable_sha -- --nocapture`

Result:

- Focused daemon SHA convergence tests passed. P42 remains open for active
  runtime inventory, doctor remedies, live rail convergence boundaries, and
  one-command local convergence.

## Turn 26 | 2026-06-22

Scope: execute P42 Slice A active runtime inventory in doctor output.

Actions:

- Added `runtimeInventory` to `agent-browser install doctor --json`.
- The inventory scans the daemon socket metadata directory without launching
  Chrome and reports daemon session PID, PID liveness, package version match,
  executable SHA-256 match, stream port, and metadata presence.
- Added `active_runtime_stale_executable` install doctor issues for active
  daemon sessions whose metadata is stale or incomplete.
- Lifted the install doctor's runtime inventory into
  `agent-browser doctor remote-view --json` as top-level `runtimeInventory`.
- Updated P42 Slice A completion notes.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml install_doctor -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml runtime_inventory_from_install -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml daemon_executable_sha -- --nocapture`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `./cli/target/debug/agent-browser install doctor --json`
- `./cli/target/debug/agent-browser doctor remote-view --json`

Result:

- Focused tests and clippy passed.
- The rebuilt debug-binary install doctor reported
  `runtimeInventory.status=stale`, `runtimeCount=4`, and `staleCount=4`.
- The rebuilt debug-binary remote-view doctor lifted the same inventory and
  reported `runtimeInventory.status=stale`. This intentionally made the
  debug-binary readback not remote-control ready against the installed runtime,
  proving stale active runtimes are no longer omitted from readiness.

## Turn 27 | 2026-06-22

Scope: execute the first P42 Slice C convergence doctor remedy.

Actions:

- Added session-scoped remedy metadata to `active_runtime_stale_executable`
  install doctor issues.
- Each stale daemon issue now carries `session`,
  `nextAction=restart_stale_daemon_session`, and an argv-safe remedy for
  `agent-browser close --session <session>`.
- Made remote-view doctor prefer
  `restart_stale_daemon_sessions_then_rerun_doctor` when install readiness is
  blocked by stale active daemon sessions.
- Updated P42 Slice C progress notes.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml install_doctor -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml recommend_next -- --nocapture`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo build --manifest-path cli/Cargo.toml`
- `./cli/target/debug/agent-browser install doctor --json`
- `./cli/target/debug/agent-browser doctor remote-view --json`

Result:

- Focused tests, clippy, and debug CLI build passed.
- The rebuilt debug-binary install doctor reported four
  `active_runtime_stale_executable` issues; the first issue carried
  `session=default`, `nextAction=restart_stale_daemon_session`, and
  `remedy.argv=["agent-browser","close","--session","default"]`.
- The rebuilt debug-binary remote-view doctor reported
  `nextAction=restart_stale_daemon_sessions_then_rerun_doctor` and a
  next-command explanation that points operators back to each issue's
  session-scoped `remedy.argv`.

## Turn 28 | 2026-06-22

Scope: execute P42 local binary/runtime convergence after publishing the
structured commits.

Actions:

- Extended `pnpm publish:local-dashboard` so it synchronizes the user-scoped
  install binary, ignored workspace package binary, and user pnpm package
  binary to the same freshly built executable by default.
- Added `--skip-reference-sync` for operator cases that intentionally do not
  want reference binaries changed.
- Published the current debug build to the local dashboard runtime and restarted
  `agent-browser-dashboard.service`.
- Applied the stale daemon restart path by invoking the three
  session-scoped remedies reported by install doctor. Those commands returned
  nonzero because `close --session` still routes through daemon restart, but
  the restart path did replace the stale daemon metadata and all active daemon
  rows converged.
- Reran publish after adding reference-binary sync so install doctor no longer
  failed on pnpm/workspace binary drift.

Validation run:

- `pnpm publish:local-dashboard -- --skip-browser --json`
- `agent-browser install doctor --json`
- `agent-browser doctor remote-view --json`

Result:

- The publish report synchronized
  `/home/ecochran76/.local/bin/agent-browser`,
  `bin/agent-browser-linux-x64`, and the user pnpm global package binary to
  `94d1d022b4f1315b2f3eb9ff08fdc3faa816d77960500c6b6854cab98161cfa8`.
- Installed `agent-browser install doctor --json` reported `success=true`,
  `runtimeInventory.status=converged`, `staleCount=0`, no issue codes, and
  matching PATH, pnpm, and workspace binary SHA-256 values.
- Installed `agent-browser doctor remote-view --json` reported `success=true`,
  `status=ready`, `remoteControl.ready=true`,
  `runtimeInventory.status=converged`, and
  `nextAction=run_many_to_many_live_gate`.
- Follow-up required: make stale daemon close remedies return success without
  depending on a daemon restart side effect.

## Turn 29 | 2026-06-22

Scope: finish P42 close/remedy and install-doctor probe convergence discovered
during local execution.

Actions:

- Added a `close --session` prestart path that targets an existing daemon
  before daemon convergence startup.
- Added explicit-session stale metadata cleanup for unauthorized or non-ready
  daemon close attempts, returning success with a warning instead of trying to
  start a replacement daemon.
- Classified running PID metadata without an addressable socket, stream, or
  port as `diagnostic` instead of stale active runtime inventory.
- Changed `service status` to execute locally before daemon startup.
- Changed install doctor service-status probing to use a unique owned session,
  terminate the owned probe daemon after reading status, and treat the isolated
  no-state probe as no-launch ready.
- Ran service GC apply for the orphaned Xvfb candidate that was blocking local
  install doctor readiness.
- Published the final local runtime and synchronized reference binaries.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml force_close_session_from_metadata -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml close_targets_existing_daemon_before_prestart -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml service_status_locally_before_daemon -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml install_doctor -- --nocapture`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm publish:local-dashboard -- --skip-browser --json`
- `agent-browser install doctor --json`
- `agent-browser doctor remote-view --json`

Result:

- Final local publish succeeded and restarted
  `agent-browser-dashboard.service`.
- Final installed executable SHA-256:
  `19ba0d616388e1eb84241eea5ddcffa56a1803831c5085acc25abb01277b78e6`.
- Reference binaries in `~/.local/bin`, ignored workspace `bin/`, and user
  pnpm global package path matched the installed executable SHA-256.
- Final installed `agent-browser install doctor --json` reported
  `success=true`, no issue codes, `runtimeInventory.status=none`,
  `runtimeCount=0`, and `staleCount=0`.
- Final installed `agent-browser doctor remote-view --json` reported
  `success=true`, `status=ready`, `remoteControl.ready=true`,
  `runtimeInventory.status=none`, `staleCount=0`, and
  `nextAction=run_many_to_many_live_gate`.

## Turn 30 | 2026-06-22

Scope: finish P42 live-rail and one-command local runtime convergence.

Actions:

- Added `pnpm converge:local-runtime` as a dry-run by default local operator
  convergence command.
- In apply mode, the command runs local dashboard publication, applies only
  doctor-reported `agent-browser close --session <name>` stale-daemon
  remedies, runs the Guacamole Postgres schema ensure, runs route-pool
  readiness, applies route display-access grants only when remote-view doctor
  asks for them, and reruns final doctors.
- Added `pnpm test:local-runtime-convergence` to lock the command contract,
  foreign-process refusal boundary, display-grant sequencing, and retained
  evidence behavior.
- Marked P42 Slice E done from the dashboard live-rail contract tests and
  Slice F done from command validation.

Validation run:

- `node --check scripts/converge-local-runtime.js`
- `node --check scripts/test-local-runtime-convergence.js`
- `pnpm test:local-runtime-convergence`
- `pnpm --silent converge:local-runtime -- --json`
- `pnpm --silent converge:local-runtime -- --apply --json --evidence-path /tmp/agent-browser-converge-local-runtime-evidence.json`
- `pnpm test:dashboard-workspace-navigator`
- `pnpm test:dashboard-workspace-nodes`

Result:

- Dry-run convergence returned `success=true`, final install doctor ready,
  final remote-view ready, zero safe stale remedies, and zero skipped remedies.
- Apply convergence returned `success=true`, wrote
  `/tmp/agent-browser-converge-local-runtime-evidence.json`, final install
  doctor ready, final remote-view ready, and zero skipped remedies.
- Dashboard workspace tests passed, proving the live rail keeps retained and
  no-action attention rows out of the live control surface and groups
  reachable non-owned CDP browsers separately.
- P42 remains open for Slice C stale dashboard/stream classifications and
  Slice D bootstrap hardening.

## Turn 31 | 2026-06-22

Scope: continue P42 Slice C by classifying stale or unreadable live dashboard
runtime manifests.

Actions:

- Added an install-doctor live dashboard manifest probe for the local
  `/api/runtime/manifest` endpoint.
- Kept dashboard-not-running as non-drift, but classified a running dashboard
  that serves no readable manifest or a mismatched executable SHA-256 as
  `dashboard_runtime_stale_or_unreadable`.
- Added a bounded remedy pointing to
  `pnpm converge:local-runtime -- --apply --json`.
- Updated remote-view doctor so that dashboard runtime drift recommends
  `converge_local_runtime_then_rerun_doctor` before generic install drift.
- Updated `pnpm converge:local-runtime -- --apply --json` so initial nonzero
  doctor JSON is treated as repairable input in apply mode instead of aborting
  before local publish.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml install_doctor -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml recommend_next -- --nocapture`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo build --manifest-path cli/Cargo.toml`
- `./cli/target/debug/agent-browser install doctor --json`
- `pnpm --silent converge:local-runtime -- --apply --json --evidence-path /tmp/agent-browser-converge-local-runtime-turn31-final.json`
- `agent-browser install doctor --json`

Result:

- Format, focused Rust tests, clippy, and debug CLI build passed.
- The rebuilt debug install doctor reported
  `dashboard_runtime_stale_or_unreadable` with `state=stale_executable` when
  the running dashboard manifest executable SHA-256 did not match the debug
  executable.
- Convergence apply started with initial install issue
  `dashboard_runtime_stale_or_unreadable`, published the new local runtime, and
  ended with final install doctor ready, final remote-view ready, zero skipped
  remedies, and retained evidence at
  `/tmp/agent-browser-converge-local-runtime-turn31-final.json`.
- Direct installed `agent-browser install doctor --json` then reported
  `success=true`, no issue codes, `liveDashboardRuntime.ready=true`,
  `liveDashboardRuntime.state=ready`, and `runtimeInventory.status=none`.
- P42 Slice C still has remaining stale stream-backend classification work.

## Turn 32 | 2026-06-22

Scope: continue P42 Slice C by adding explicit runtime convergence summary
states.

Actions:

- Added install-doctor `runtimeConvergence` with schema
  `agent-browser.runtime-convergence.v1`.
- Derived summary status from runtime inventory plus live dashboard manifest
  state, using `converged`, `partial`, `stale`, and
  `manual_review_required`.
- Lifted the summary into remote-view doctor and printed it in text output
  separately from raw runtime inventory status.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml runtime_convergence -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml install_doctor -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml recommend_next -- --nocapture`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm --silent converge:local-runtime -- --apply --json --evidence-path /tmp/agent-browser-converge-local-runtime-turn32.json`
- `agent-browser install doctor --json`

Result:

- Format, focused Rust tests, and clippy passed.
- Unit coverage now locks the `converged`, `partial`, `stale`, and
  `manual_review_required` summary statuses plus remote-view summary lifting.
- Convergence apply published the summary-state build and ended with final
  install doctor ready, final remote-view ready, zero skipped remedies, and
  retained evidence at `/tmp/agent-browser-converge-local-runtime-turn32.json`.
- Direct installed `agent-browser install doctor --json` reported
  `success=true`, no issue codes, `runtimeConvergence.status=converged`,
  `liveDashboardRuntime.state=ready`, and `runtimeInventory.status=none`.
- P42 Slice C still has remaining stale stream-backend and diagnostic
  retained-row classification work.

## Turn 33 | 2026-06-22

Scope: finish P42 Slice C stale stream-backend classification.

Actions:

- Extended runtime inventory to probe advertised daemon stream ports.
- Added runtime row `streamReachable` and `driftReasons` evidence.
- Classified live daemon sessions with unreachable or invalid stream metadata
  as stale instead of converged.
- Added install-doctor issue code `active_runtime_stale_stream_backend` with
  the bounded `agent-browser close --session <session>` remedy.
- Updated remote-view doctor to treat stale stream backends as a
  session-scoped daemon restart prerequisite before generic install drift.
- Marked P42 Slice C done.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml stream_backend -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml install_doctor -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml recommend_next -- --nocapture`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm --silent converge:local-runtime -- --apply --json --evidence-path /tmp/agent-browser-converge-local-runtime-turn33.json`
- `agent-browser install doctor --json`

Result:

- Format, focused Rust tests, and clippy passed.
- Unit coverage now proves unreachable stream metadata produces a stale runtime
  inventory row, install doctor emits
  `active_runtime_stale_stream_backend`, and remote-view doctor recommends the
  same session-scoped restart prerequisite.
- Convergence apply published the stream-backend build and ended with final
  install doctor ready, final remote-view ready, zero skipped remedies, and
  retained evidence at `/tmp/agent-browser-converge-local-runtime-turn33.json`.
- Direct installed `agent-browser install doctor --json` reported
  `success=true`, no issue codes, `runtimeConvergence.status=converged`,
  `staleRuntimeCount=0`, and `runtimeInventory.status=none`.

## Turn 34 | 2026-06-22

Scope: close P42 by auditing and validating Slice D idempotent remote-view
bootstrap.

Actions:

- Verified `pnpm ensure:rdp-guac-postgres -- --apply` exists and is invoked by
  local convergence.
- Verified route-pool setup, existing-user route sync, and legacy autologin
  setup call the shared Guacamole Postgres schema guard before writing records.
- Verified the schema guard refuses partial `guacamole_*` relation state,
  imports only absent schema state, waits for Postgres readiness, and
  checkpoints after ready/imported states.
- Verified the live Guacamole compose file keeps explicit Postgres durability
  settings for WSL hard-stop resilience.
- Marked P42 `State: CLOSED`.

Validation run:

- `bash scripts/ensure-rdp-guac-postgres.sh --dry-run`
- `pnpm --silent test:rdp-guac-route-pool-readiness -- --report-only`
- `agent-browser doctor remote-view --json`

Result:

- Schema guard dry-run reported `Guacamole Postgres schema is ready.`
- Route-pool readiness reported `success=true`; Postgres, schema, Guacamole
  web/login, guacd, RDP connections, connection permissions, distinct targets,
  and both RDP backend TCP checks were ready.
- Direct installed remote-view doctor reported `success=true`, `status=ready`,
  `remoteControl.ready=true`, `runtimeConvergence.status=converged`,
  `runtimeInventory.status=none`, and
  `nextAction=run_many_to_many_live_gate`.

## Turn 35 | 2026-06-22

Scope: investigate the `last30days` Facebook remote-view friction and open the
next route-handoff audit lane.

Actions:

- Read the incident note at
  `docs/dev/notes/2026-06-22-facebook-remote-view-open-friction.md`.
- Used Graphiti discovery for advisory prior context and CodeGraph for the
  route-binding and dashboard stream helper joins.
- Captured live readbacks from `agent-browser doctor remote-view --json`,
  `agent-browser service browsers --json`, and
  `agent-browser service tabs --json`.
- Added P43 in
  `docs/dev/plans/0043-2026-06-22-route-handoff-confusion-audit-plan.md`.
- Updated `ROADMAP.md` with the open P43 lane.

Findings:

- P42 binary/runtime convergence remains green. The failure sits above that
  layer.
- `session:default` owns the `last30days-facebook` browser on display `:11`
  with Facebook tabs and a generic Guacamole stream.
- `session:litscout-ai-smoke-clean` is a separate browser on display `:93`
  with several `127.0.0.1` tabs and its own generic Guacamole stream.
- The dashboard has stream metadata that can embed Guacamole, but it does not
  yet require row-bound proof that the stream is showing the intended browser
  instead of a terminal.

Validation run:

- `git diff --check`
- `pnpm validation:select -- --base HEAD`
- `python /home/ecochran76/workspace.local/agent-policies/repo-policy-selector/scripts/audit_planning_contract.py --repo-root /home/ecochran76/workspace.local/agent-browser --json`

Result:

- `git diff --check` passed.
- `pnpm validation:select -- --base HEAD` selected only `git diff --check` for
  the docs-only change set.
- The planning-contract audit still fails on pre-existing older plan wiring and
  deterministic-state debt. The new P43 plan itself is reported with
  `filename_ok=true`, `lane_ok=true`, `state_ok=true`,
  `wired_in_roadmap=true`, and `wired_in_runbook=true`.

## Turn 36 | 2026-06-22

Scope: execute P43 Slice A with a read-only route-handoff audit surface.

Actions:

- Added `scripts/audit-route-handoff.js`.
- Added package command `pnpm audit:route-handoff`.
- Added no-launch fixture coverage in `scripts/test-route-handoff-audit.js`.
- Added package command `pnpm test:route-handoff-audit`.
- Documented the audit command in `README.md`.
- Marked P43 Slice A done and updated `ROADMAP.md` next recommendation.

Validation run:

- `node --check scripts/audit-route-handoff.js`
- `node --check scripts/test-route-handoff-audit.js`
- `pnpm test:route-handoff-audit`
- `pnpm --silent audit:route-handoff -- --json --skip-doctor`
- `pnpm --silent audit:route-handoff -- --json`
- `git diff --check`
- `pnpm validation:select -- --base HEAD`
- `node scripts/dev/select-validation.js --base HEAD --json`
- `pnpm test:route-handoff-audit && pnpm --silent audit:route-handoff -- --json --skip-doctor | jq -e '.success == true and .data.summary.route_bound_ready == 2 and .data.summary.direct_remote_headed == 11'`

Result:

- Syntax checks passed.
- The fixture test passed and covers `route_bound_ready`,
  `route_bound_proof_missing`, `route_bound_terminal_only`,
  `direct_remote_headed`, `foreign_cdp`, and `stale_or_retained`
  classifications.
- The live read-only audit with `--skip-doctor` returned `success=true`,
  `collections.browsers=2`, `collections.tabs=13`, and summary
  `route_bound_ready=2`, `direct_remote_headed=11`.
- The full live audit also returned `success=true`, no collection errors,
  `runtime.convergenceStatus=converged`,
  `runtime.inventoryStatus=converged`, `runtime.runtimeCount=1`, and
  `runtime.remoteControlStatus=ready`.
- `git diff --check` passed.
- `pnpm validation:select -- --base HEAD` recommended `git diff --check` and
  `node scripts/dev/select-validation.js --base HEAD --json`; both passed.
- The combined fixture plus live summary assertion passed.

## Turn 37 | 2026-06-22

Scope: execute P43 Slice B one-line CLI contract and help.

Actions:

- Added command-specific `remote-view` help covering
  `agent-browser remote-view open`.
- Added the Facebook-style one-liner and flag placement guidance to CLI help.
- Changed `parse_remote_view_open` to copy global `--session-name` into the
  `remote_view_open` request.
- Added parser tests for post-subcommand `--runtime-profile`, `--session`,
  `--session-name`, `--browser-build`, and `--provider` placement.
- Updated `README.md`, `docs/src/app/commands/page.mdx`, and
  `skills/agent-browser/SKILL.md`.
- Marked P43 Slice B done and updated `ROADMAP.md` next recommendation.

Validation run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open -- --nocapture`
- `cargo run --quiet --manifest-path cli/Cargo.toml -- remote-view open --help | rg -n "Facebook|Global placement|--session selects|last30days-facebook"`
- `pnpm --dir docs build`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm validation:select -- --base HEAD`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`

Result:

- Rust format passed after applying `cargo fmt`.
- Focused Rust tests passed: 10 passed, 0 failed.
- Help output includes the global placement section, Facebook examples, and
  the `--session` versus `--session-name` distinction.
- Docs build passed.
- Clippy passed with `-D warnings`.
- Validation selector required the Rust format, focused Rust test, clippy,
  docs build, diff hygiene, and repo-installed skill sync checks.
- The repo and installed `agent-browser` skill copies now match.
