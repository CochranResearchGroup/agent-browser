# P04 Release Candidate Install Validation

Date: 2026-05-29
Status: Passed

## Candidate

- Installed command: `/home/ecochran76/.local/bin/agent-browser`
- Version: `agent-browser 0.26.1`
- Candidate checksum:
  `9c7ebcab0c2437841268e610c2042cafdf2aa675906b9f9df42f92117b5f96a2`
- Rollback binary:
  `/home/ecochran76/.local/bin/agent-browser.bak-20260529T031016Z`

The workspace binary and pnpm package binary were synced to the same checksum
as the installed command so `agent-browser install doctor` could validate the
operator path rather than an old package copy.

## Installed Doctor

`agent-browser install doctor --json` passed from the installed command with no
issues. It confirmed the current executable, workspace binary, and pnpm package
binary all point at the validated candidate checksum.

## Remote View Doctor

`agent-browser doctor remote-view --json` passed from the installed command.
The doctor reported:

- route pool ready
- route displays ready with route A on `:12` and route B on `:11`
- route display access ready
- simultaneous viewing ready
- privileged helper ready at
  `/usr/local/libexec/agent-browser/agent-browser-privileged-helper`
- `agent-browser` group present with the current user as a member
- next command `pnpm test:rdp-guac-many-to-many-live`
- `requiresInteractiveSudo=false`

The remaining drift is expected for the current many-to-many implementation:
route-specific RDP users exist while the single-user route is still retained.

## Live Gate

The installed candidate passed the OCR-backed many-to-many live gate:

```bash
pnpm test:rdp-guac-many-to-many-live
```

Artifact directory:

```text
/tmp/agent-browser-rdp-guac-many-to-many-2026-05-29T03-17-31-591Z
```

The gate used Google Chrome and Brave as distinct viewing clients, bound route
A to display `:12`, bound route B to display `:11`, proved both browser
markers through Guacamole/RDP, closed Browser A, kept Browser B visible, and
released Browser A's route-pool entry.

## Regression Fixes Proven

- Default-profile lock regression: `agent-browser --json get title` now
  attaches to the live default runtime profile instead of launching a second
  Chrome against the locked default profile.
- Route display binding: the many-to-many smoke now overlays
  `AGENT_BROWSER_RDP_ROUTE_A_DISPLAY_NAME` and
  `AGENT_BROWSER_RDP_ROUTE_B_DISPLAY_NAME` onto route-pool JSON entries when
  the JSON source lacks `target.displayName`.

## Validation Commands

```bash
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
cargo test --manifest-path cli/Cargo.toml runtime_profile_name_for_launch -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml daemon_profile_for_launch -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml install_doctor -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml remote_view_doctor -- --test-threads=1
bash -n scripts/libexec/agent-browser-privileged-helper scripts/install-agent-browser-privileges.sh scripts/setup-rdp-guac-route-pool.sh scripts/grant-rdp-route-display-access.sh
agent-browser install doctor --json
agent-browser doctor remote-view --json
pnpm test:rdp-guac-many-to-many-live
git diff --check -- cli/src/main.rs scripts/test-rdp-guac-many-to-many-live.js docs/dev/plans/0004-2026-05-29-release-candidate-install-validation-plan.md ROADMAP.md
```
