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
  `1b67077ccdb5e80d8667d3bcc8327e9c2a1a8521417c25280f71d059bc3b1694`.
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
  readiness, helper, sudoers, group, privilege readiness, and version state.
  A later P06 slice should decide whether service readiness belongs directly in
  install doctor or remains composed through `agent-browser doctor remote-view`.
