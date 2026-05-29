# P06 Installer And Doctor Productization

Date: 2026-05-29
Plan: `docs/dev/plans/0006-2026-05-29-guac-rdp-productization-hardening-plan.md`
Status: Validated slice handoff

## Slice A Audit

Current installed checkpoint state before source edits:

- `agent-browser install doctor --json` passed and showed matching installed,
  workspace, and pnpm package binary checksums.
- `agent-browser doctor remote-view --json` passed with route pool ready,
  route displays ready, route display access ready, privileged helper ready,
  and `requiresInteractiveSudo=false`.
- Manual P05 preconditions still existed outside the productized contract:
  route-pool/display variables had to be copied into the many-to-many harness,
  viewer executable variables had to be supplied or guessed by the operator,
  and the public Guacamole URL failed the live gate without an explicit local
  URL override.

Ownership decision:

- Installer-owned: root-owned privileged helper, sudoers rule, `agent-browser`
  group, operator membership, and helper check readiness.
- Doctor-owned: install drift, helper and sudoers readiness, group membership,
  Guacamole route pool, route displays, route display access, viewer browser
  and OCR tooling prerequisites, issue codes, remediations, and next command.
- Live-test-owned: rendering proof, OCR proof, browser checkout and release,
  and classification of non-embeddable Guacamole URLs.

## Implemented Changes

- `agent-browser install doctor --json` now includes
  `data.remoteViewPrivileges` with helper, sudoers, group, membership,
  `requiresInteractiveSudo`, helper check, and nested issue fields.
- `agent-browser doctor remote-view --json` now includes stable top-level
  `data.issues` entries for install drift, route-pool blockers, missing route
  displays, display-access gaps, privilege-helper gaps, sudoers gaps, and
  viewer prerequisite gaps.
- `agent-browser doctor remote-view --json` now reports
  `data.viewerPrerequisites`, including viewer browser executable discovery
  and `identify`, `convert`, and `tesseract` readiness for OCR-backed gates.
- `scripts/test-rdp-guac-many-to-many-live.js` now prefers the installed
  `agent-browser` command, hydrates route-pool and route-display environment
  from `agent-browser doctor remote-view --json` when explicit route variables
  are absent, auto-discovers common local viewer browsers, and fails public
  Guacamole route URLs with a `non_embeddable_guacamole_url` diagnostic unless
  explicitly allowed for a reviewed public-ingress diagnostic.
- Documentation and skill guidance were updated for the new doctor fields and
  many-to-many gate preflight behavior.

## Validation

Passed before installing the rebuilt checkpoint:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml install_doctor -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml remote_view_doctor -- --test-threads=1`
- `node --check scripts/test-rdp-guac-many-to-many-live.js`
- `node --check scripts/smoke-utils.js`
- `pnpm --dir docs build`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm build:native`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `git diff --check`

Source-build diagnostic checks:

- `cargo run --quiet --manifest-path cli/Cargo.toml -- install doctor --json`
  reported the expected `current_executable_path_command_mismatch` because the
  source debug binary did not match the installed command, while also proving
  `remoteViewPrivileges.ready=true` and `requiresInteractiveSudo=false`.
- `cargo run --quiet --manifest-path cli/Cargo.toml -- doctor remote-view --json`
  surfaced the install drift as
  `install_current_executable_path_command_mismatch`, reported
  `viewerPrerequisites.ready=true`, and pointed `nextAction` at
  `repair_install_drift`.
- `AGENT_BROWSER_RDP_TEST_USE_INSTALLED=1 node scripts/test-rdp-guac-many-to-many-live.js`
  failed fast with `non_embeddable_guacamole_url` against the public
  Guacamole URL, proving the new precondition classifier before any browser
  launch.

Pending before closing this slice:

- None for this slice.

Installed checkpoint validation after rebuild:

- `pnpm build:native` passed and produced the updated Linux binary.
- The rebuilt binary was installed to `/home/ecochran76/.local/bin/agent-browser`,
  `bin/agent-browser-linux-x64`, and the pnpm global package binary.
- All three installed/runtime binary locations now share checksum
  `1ec7a0528944fad76fc4b3c2539b57b15944a503126038e47fb9d8727bdfa53a`.
- `agent-browser install doctor --json` passed with no issues and
  `data.remoteViewPrivileges.ready=true`.
- `agent-browser doctor remote-view --json` passed with no issues,
  `data.viewerPrerequisites.ready=true`, and
  `data.manyToMany.simultaneousViewingReady=true`.
- `AGENT_BROWSER_REMOTE_VIEW_URL=http://127.0.0.1:8092/guacamole/ AGENT_BROWSER_RDP_TEST_USE_INSTALLED=1 node scripts/test-rdp-guac-many-to-many-live.js`
  passed from the installed command with artifacts at
  `/tmp/agent-browser-rdp-guac-many-to-many-2026-05-29T14-06-07-291Z`.

Final validation:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml install_doctor -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml remote_view_doctor -- --test-threads=1`
- `node --check scripts/test-rdp-guac-many-to-many-live.js`
- `node --check scripts/smoke-utils.js`
- `pnpm --dir docs build`

Residual P06 scope:

- This slice proves the current already-provisioned machine does not require
  interactive sudo for install doctor, remote-view doctor, or the live gate.
  It does not freshly prove the first-install "sudo exactly once" path on a
  clean host.
- `agent-browser install doctor --json` now reports binary drift, browser-build
  readiness, helper, sudoers, group, privilege readiness, service readiness,
  and version state.

## Follow-Up Slice B/C Work

Additional changes made after the first validation slice:

- `scripts/install-agent-browser-privileges.sh --apply` now exits before any
  privileged changes when the helper source matches the installed helper, the
  sudoers file exists, the operator is in the `agent-browser` group, and
  `sudo -n <helper> check` succeeds.
- The idempotence probe intentionally uses only the narrow installed helper
  for the non-interactive sudo check. It does not require broad sudo access to
  inspect `/etc/sudoers.d/agent-browser`.
- `agent-browser install doctor --json` now includes a `data.service` object
  from a no-launch `agent-browser --json --session install-doctor-service-probe
  service status` probe under an isolated temporary `AGENT_BROWSER_HOME`.

Validation added for this follow-up:

- `AGENT_BROWSER_PRIVILEGED_HELPER_SOURCE=scripts/libexec/agent-browser-privileged-helper bash scripts/install-agent-browser-privileges.sh --apply`
  passed with "already ready" and no privileged changes.
- `cargo run --quiet --manifest-path cli/Cargo.toml -- install doctor --json`
  showed the new service probe as `ready=true` and `noLaunch=true`; the command
  still exited nonzero because the source debug binary intentionally differs
  from the installed checkpoint binary.
- `pnpm build:native` passed after the follow-up source changes.
- The rebuilt installed command, workspace binary, and pnpm package binary
  were synchronized at checksum
  `1ec7a0528944fad76fc4b3c2539b57b15944a503126038e47fb9d8727bdfa53a`.
- Installed `agent-browser install doctor --json` passed with no issues and
  reported `data.service.ready=true` plus `data.service.noLaunch=true`.
- Installed `agent-browser doctor remote-view --json` passed with no issues
  and included the install doctor `service` object in its composed install
  state.
- `git diff --check` passed.
- `node scripts/dev/select-validation.js --base HEAD --json` passed and wrote
  `/tmp/agent-browser-p06-validation-select-turn6.json`.

Remaining P06 scope:

- A clean-host or equivalent reset-fixture validation is still needed before
  claiming the first-install path asks for sudo exactly once.
