# P03 Doctor-First Refactor

Date: 2026-05-28

## Decision

P03 should stop treating route provisioning as the first response to a
Guacamole/RDP viewing failure. The next implementation slice is a unified
doctor/setup discovery surface that records the current install, runtime,
network, Guacamole, RDP host, route-pool, and route-display state before any
new users, Guacamole records, env files, or provider settings are added.

## Rationale

The workstation already has the reusable `agent-browser-rdp` account and a
user-scoped Guacamole secret file. Adding route-specific users would create
unnecessary host state unless the existing-user path is proven insufficient.
The no-sudo existing-user Guacamole sync created managed connections 2 and 3
with color depths 24 and 32, and route-pool readiness now selects those as
ready route candidates. The remaining proof is route-display and visual
target binding, not more untracked setup.

## Doctor Requirements

- Compose install doctor, runtime status, service status, RDP/Guacamole
  readiness, route-pool inventory, route-display inspection, Docker/network
  checks, and secret key presence.
- Report source ownership for each fact: repo docs, user config, service
  state, Guacamole DB, Docker runtime, host OS, or live browser runtime.
- Redact all secrets and avoid printing raw auth state.
- Prefer `agent-browser-rdp` reuse while current XRDP policy supports it.
- Recommend route-specific Linux users only after doctor evidence proves the
  existing-user topology cannot produce distinct displays.
- Identify managed Guacamole route-pool entries separately from the legacy
  shared fallback.
- Emit JSON and concise human output with one primary next action.

## Next Slice

The doctor surface is now implemented as `agent-browser doctor remote-view`.
The observed current result is:

- install/runtime/service/network: inspectable
- Guacamole route pool: ready with managed connections 2 and 3
- legacy shared route: labeled fallback
- route displays: blocked after both RDP route clients were opened because the
  existing `agent-browser-rdp` topology produced only one active display
- next action: use a route-specific user or XRDP policy isolation fallback from
  an interactive privileged shell, then rerun the doctor and live gate

The route-specific user setup command now checks the display inspector before
requesting sudo. It allows the fallback in the current state because the
existing-user topology has already collapsed to one display, and otherwise
refuses unless an operator supplies `--force`.

Current XRDP logs confirm why this fallback is required: two Guacamole RDP
connections authenticated at the same time, but both reported login success on
display 10 and connected to the same Xorg PID. The no-sudo color-depth split
therefore does not create independent route displays on this host.

After the privileged route-user setup, route A and route B produced distinct
XRDP displays `:12` and `:11`, but launching Chrome onto them failed until the
agent user is granted local X access to those route-user sessions. The
`grant:rdp-route-display-access` helper now reports and applies the narrow
`xhost +SI:localuser:<operator>` grants for the active route displays.

Follow-up refactor: route maintenance now has a one-time authorization path.
`pnpm install:privileges -- --apply` installs the root-owned helper under
`/usr/local/libexec/agent-browser`, creates the `agent-browser` group, adds the
operator user, and writes sudoers limited to that helper. The route-pool setup
and display-access scripts prefer that installed helper with non-interactive
sudo, falling back to interactive sudo only when the helper is unavailable.

2026-05-29 live update: `agent-browser doctor remote-view` now reports the
privileged helper state separately from route-display access. On this host,
route pool, route displays, and route display access reported ready while the
privileged helper still reported not installed. The full OCR-backed
many-to-many gate passed with local Guacamole frame URLs and artifacts at
`/tmp/agent-browser-rdp-guac-many-to-many-2026-05-29T01-34-49-701Z`.
The gate also exposed and then covered the service close-path fix that returns
route-pool entries to available when a browser-owned display allocation is
released.

2026-05-29 completion update: the privileged helper is installed and the
operator shell is in the `agent-browser` group. `agent-browser doctor
remote-view` reports `privileged helper: ready=true`, `userInGroup=true`,
`simultaneous viewing ready: true`, and `requires interactive sudo: false`.
The CLI installer now exposes
`agent-browser install --with-deps --with-remote-view-privileges`, which embeds
the same helper setup so release binaries can install the desktop maintenance
privileges without relying on repo-local scripts.
