# Release Candidate Install Validation Plan

Date: 2026-05-29
State: CLOSED
Lane: P04
Outcome: PASSED
Current state: P03 closed the Guacamole/RDP many-to-many viewing lane with a
passing OCR-backed live gate, a ready `agent-browser doctor remote-view`
surface, and a one-time `agent-browser` group plus privileged helper path for
recurring desktop setup. The remaining risk is release packaging and installed
candidate behavior: the validated workflow must work from a built release
binary, not only from the repo checkout.

Closed state: The installed 0.26.1 release candidate was validated from
`/home/ecochran76/.local/bin/agent-browser` with checksum
`9c7ebcab0c2437841268e610c2042cafdf2aa675906b9f9df42f92117b5f96a2`.
The previous user-scoped binary was preserved at
`/home/ecochran76/.local/bin/agent-browser.bak-20260529T031016Z`.
`agent-browser install doctor --json` and
`agent-browser doctor remote-view --json` both passed from the installed
candidate. The installed remote-view doctor reported the helper ready at
`/usr/local/libexec/agent-browser/agent-browser-privileged-helper`, the current
user in the `agent-browser` group, route display access ready for `:12` and
`:11`, and `requiresInteractiveSudo=false` for the next many-to-many command.
The OCR-backed many-to-many live gate passed with artifacts at
`/tmp/agent-browser-rdp-guac-many-to-many-2026-05-29T03-17-31-591Z`.

## Purpose

Agent Browser needs a release-candidate checkpoint that proves the install and
remote-view hardening work is usable from the normal operator path. This lane
turns the P03 result into a releaseable state by validating:

- the packaged binary exposes the new installer and doctor commands
- the Linux installer can install browser dependencies and remote-view
  privileges through one intentional authorization
- the installed helper path does not depend on mutable repo files
- `agent-browser install doctor` and `agent-browser doctor remote-view` agree
  from the installed candidate
- the many-to-many Guacamole/RDP live gate passes from the installed candidate
  environment
- release docs, help output, skill guidance, and validation notes tell
  operators exactly what to run

## Non-Goals

- Do not reopen P03 feature work unless the installed candidate fails a P03
  invariant.
- Do not add new remote-view backends such as CDP streaming, VNC, or noVNC.
- Do not broaden the privileged helper beyond route-user setup, XRDP restart,
  route-display access, and readiness checks.
- Do not require passwordless broad sudo for repo-local scripts.
- Do not create extra RDP users or Guacamole connections unless the doctor
  reports drift that requires repair.

## Product Invariants

This lane is not complete until these invariants hold:

- A release candidate binary can run `agent-browser install
  --with-deps --with-remote-view-privileges` on Linux.
- The installed privileged helper is root-owned and lives outside the mutable
  checkout at `/usr/local/libexec/agent-browser/agent-browser-privileged-helper`.
- The installed sudoers rule grants only the narrow helper command to members
  of the `agent-browser` group.
- The current operator user is in the `agent-browser` group before recurring
  route maintenance is considered ready.
- `agent-browser doctor remote-view` reports `requires interactive sudo:
  false` on a ready host.
- Packaged help output, README, docs site, and `skills/agent-browser/SKILL.md`
  document the same command names and operator sequence.
- The many-to-many live gate proves two simultaneous Guacamole/RDP browser
  routes from the installed candidate, including route release after one
  browser closes.

## Slices

### Slice A | Candidate Packaging Audit

Audit the build and package surfaces that determine what lands in a release
candidate.

Tasks:

- Verify `package.json` includes every script and helper needed for the
  remote-view setup path.
- Verify `scripts/libexec/agent-browser-privileged-helper` is packaged for
  source and binary-adjacent installs where relevant.
- Verify the Rust installer embeds the helper setup and does not rely on
  relative checkout paths at runtime.
- Verify `agent-browser install --help` documents
  `--with-remote-view-privileges`.
- Verify `agent-browser doctor remote-view --json` exposes machine-readable
  fields for helper readiness, group membership, display access, and next
  command.

Exit criteria:

- A repo-local candidate build exposes the expected help, doctor, and installer
  flags.
- Any missing package files or stale docs are patched before live validation.

### Slice B | Installed Candidate Swap

Install or stage a local release candidate without relying on the active repo
checkout as the executable on `PATH`.

Tasks:

- Build the candidate binary with the normal release build path.
- Preserve the previous user-scoped binary or package location for rollback.
- Install the candidate into the user-scoped command path used by operators.
- Run `agent-browser install doctor` and confirm the command on `PATH`,
  running executable, package binary, and checkout binary status are understood.
- Record the candidate binary path, version, checksum when practical, and
  rollback path in a dated validation note.

Exit criteria:

- `agent-browser install doctor` exits cleanly or reports only explained,
  non-blocking drift.
- The installed `agent-browser` command is the candidate being validated.

### Slice C | One-Time Privilege Install Gate

Prove the installer-owned privilege path works from the installed candidate.

Tasks:

- Run the remote-view privilege installer path from the candidate:
  `agent-browser install --with-deps --with-remote-view-privileges`.
- Confirm the helper path, owner, mode, sudoers file, group, and current-user
  membership.
- Open a new shell or use `newgrp agent-browser` when group membership changed
  during the run.
- Run `agent-browser doctor remote-view` and confirm `privileged helper:
  ready=true`, `userInGroup=true`, and `requires interactive sudo: false`.
- Run the route-pool setup and display-access commands in dry-run mode to prove
  they prefer non-interactive helper execution on a ready host.

Exit criteria:

- Recurring desktop setup no longer needs interactive sudo on the ready host.
- The doctor recommends the many-to-many live gate rather than privilege
  installation or display-access repair.

### Slice D | Installed Candidate Remote-View Live Gate

Run the P03 live gate against the installed candidate and record evidence.

Tasks:

- Confirm route pool readiness with the existing non-secret readiness smoke.
- Confirm route displays and display access through the doctor.
- Run `pnpm test:rdp-guac-many-to-many-live` with the installed candidate on
  `PATH`.
- Confirm the gate opens two browser workspaces, binds Browser A and Browser B
  to separate Guacamole/RDP routes, refreshes viewers, closes Browser A,
  keeps Browser B visible, and releases Browser A's route-pool entry.
- Record the artifact directory and relevant command output in a dated
  validation note.

Exit criteria:

- The many-to-many live gate passes from the installed candidate environment.
- Any failure produces an artifact with a clear next action and does not leave
  stale checked-out route-pool entries.

### Slice E | Release Surface Closeout

Close the release candidate lane only after docs, validation, and handoff are
consistent.

Tasks:

- Update `CHANGELOG.md` and `docs/src/app/changelog/page.mdx` only when this
  lane becomes part of an actual release preparation slice.
- Update `README.md`, docs site, CLI help, and skill guidance for any command
  or behavior changes discovered during validation.
- Add a dated note under `docs/dev/notes/` with candidate version, installed
  path, doctor output summary, remote-view doctor summary, live gate artifact
  path, and residual risk.
- Update `ROADMAP.md` when P04 closes.
- Run the selected validation set from `pnpm validation:select -- --base
  <ref>` before handoff.

Exit criteria:

- The plan, roadmap, and validation note agree on candidate status.
- The handoff contains concrete pass or fail evidence and one recommended next
  action.

## Validation Matrix

Minimum validation for implementation changes in this lane:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml install_doctor -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml remote_view_doctor -- --test-threads=1`
- `bash -n scripts/libexec/agent-browser-privileged-helper
  scripts/install-agent-browser-privileges.sh
  scripts/setup-rdp-guac-route-pool.sh
  scripts/grant-rdp-route-display-access.sh`
- `agent-browser install doctor`
- `agent-browser doctor remote-view`
- `pnpm test:rdp-guac-many-to-many-live`

Release-candidate validation should additionally run:

- `pnpm validation:select -- --base <ref>`
- every command selected by that script for changed files
- `agent-browser install --help` from the installed candidate
- `agent-browser doctor remote-view --json` from the installed candidate

## Closeout Evidence

- Built optimized release candidate with
  `cargo build --release --manifest-path cli/Cargo.toml`.
- Replaced `/home/ecochran76/.local/bin/agent-browser` and synced the matching
  workspace and pnpm package binaries so `agent-browser install doctor` could
  verify the installed command, workspace binary, and pnpm package binary all
  share checksum
  `9c7ebcab0c2437841268e610c2042cafdf2aa675906b9f9df42f92117b5f96a2`.
- Confirmed `agent-browser --version` reports `agent-browser 0.26.1`.
- Confirmed `agent-browser install --help` exposes
  `--with-remote-view-privileges`.
- Confirmed `agent-browser install doctor --json` exits successfully with no
  issues from the installed command.
- Confirmed `agent-browser doctor remote-view --json` exits successfully with
  route pool, route displays, display access, privileged helper, group
  membership, and many-to-many readiness all ready.
- Confirmed recurring route maintenance no longer needs broad interactive sudo:
  the remote-view doctor reports `requiresInteractiveSudo=false`, helper
  readiness succeeds through `sudo -n`, and dry-run route setup/display-access
  commands select the installed privileged helper path.
- Confirmed the profile-lock regression with the default runtime profile is
  fixed: `agent-browser --json get title` attaches to the live runtime profile
  instead of trying to launch another Chrome against the locked default profile.
- Confirmed the many-to-many live gate passes from the installed candidate
  environment with Google Chrome and Brave as separate viewing clients, route A
  bound to `:12`, route B bound to `:11`, and artifact directory
  `/tmp/agent-browser-rdp-guac-many-to-many-2026-05-29T03-17-31-591Z`.
- Patched the many-to-many smoke so route display names discovered by
  `scripts/inspect-rdp-route-displays.js` override route-pool JSON entries
  when the JSON source does not include `target.displayName`.

Validation commands run:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml runtime_profile_name_for_launch -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml daemon_profile_for_launch -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml install_doctor -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml remote_view_doctor -- --test-threads=1`
- `bash -n scripts/libexec/agent-browser-privileged-helper scripts/install-agent-browser-privileges.sh scripts/setup-rdp-guac-route-pool.sh scripts/grant-rdp-route-display-access.sh`
- `agent-browser install doctor --json`
- `agent-browser doctor remote-view --json`
- `pnpm test:rdp-guac-many-to-many-live`
- `git diff --check -- cli/src/main.rs scripts/test-rdp-guac-many-to-many-live.js docs/dev/plans/0004-2026-05-29-release-candidate-install-validation-plan.md ROADMAP.md`

## Rollback

If the candidate breaks install, doctor, or remote-view routing:

- Restore the previous user-scoped binary or package path.
- Keep the root-owned privileged helper only if the installed sudoers policy
  still points to the reviewed helper path and `agent-browser doctor
  remote-view` reports it ready.
- If the helper itself is faulty, remove or replace only
  `/usr/local/libexec/agent-browser/agent-browser-privileged-helper` and the
  matching `/etc/sudoers.d/agent-browser` file through an explicit privileged
  operator action.
- Run `agent-browser doctor remote-view` after rollback to confirm route-pool
  and display state did not drift.

## Open Questions

- Should the release candidate installer run the remote-view privilege setup
  by default on Linux when Guacamole/RDP config is detected, or should it remain
  explicit through `--with-remote-view-privileges`?
- Should `agent-browser install doctor` include a compact remote-view
  privilege summary, or is `agent-browser doctor remote-view` the only
  authority for that host state?
- Should release validation require a fresh user shell after group membership
  changes, or is `newgrp agent-browser` acceptable for the documented local
  gate?
