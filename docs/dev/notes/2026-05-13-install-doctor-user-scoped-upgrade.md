# Install Doctor And User-Scoped Upgrade

Date: 2026-05-13

## Context

The user-scoped `agent-browser` command is intentionally isolated from the active
repo workspace while service-roadmap development continues. The durable install
pattern remains a local release tarball under `~/.agent-browser/releases/`,
followed by an explicit user-scoped binary update when the operator wants active
agents to use the new release.

## Finding

The 0.26.1 manifest-resolver package had been installed into the pnpm global
package store, but `/home/ecochran76/.local/bin/agent-browser` was still an
older standalone ELF from April 30. `agent-browser --version` alone was not
sufficient evidence that the command, pnpm package binary, checkout binary, and
browser-build manifest readiness all matched.

The new `agent-browser install doctor` command was added to make this drift
detectable without launching Chrome. It checks:

- the running executable
- the `agent-browser` command found on `PATH`
- the pnpm global package native binary when pnpm is available
- the current checkout binary when run from a repo checkout
- the no-launch `launchConfig` readiness view, including manifest-backed
  `stealthcdp_chromium` readiness

## Current User-Scoped State

The active user-scoped binary was replaced with the rebuilt 0.26.1 native binary.

Relevant artifacts:

- install-doctor tarball:
  `/home/ecochran76/.agent-browser/releases/agent-browser-0.26.1-install-doctor-20260513065112.tgz`
- prior user-scoped binary backup:
  `/home/ecochran76/.local/bin/agent-browser.pre-install-doctor-20260513065113`
- previous manifest-resolver tarball:
  `/home/ecochran76/.agent-browser/releases/agent-browser-0.26.1-manifest-resolver-20260513063331.tgz`

The current user config no longer uses top-level `executablePath`. It resolves
the preferred patched Chromium through:

```json
{
  "service": {
    "defaultBrowserBuild": "stealthcdp_chromium",
    "browserBuildManifests": {
      "stealthcdp_chromium": {
        "manifestPath": "/home/ecochran76/workspace.local/chromium/artifacts/chromium-stealthcdp/current/manifest.json"
      }
    }
  }
}
```

## Verification

Post-install doctor output showed no drift:

```bash
agent-browser install doctor
```

Result:

- version: `0.26.1`
- path command: `/home/ecochran76/.local/bin/agent-browser`
- pnpm package binary:
  `/home/ecochran76/.local/share/pnpm/global/5/node_modules/agent-browser/bin/agent-browser-linux-x64`
- workspace binary:
  `/home/ecochran76/workspace.local/agent-browser/bin/agent-browser-linux-x64`
- launch config source: `manifest`
- launch config ready: `true`

The JSON smoke also showed `success: true`, no issue codes, identical hashes for
PATH, pnpm, and workspace binaries, and manifest-backed launch readiness.

Focused validation for the committed command slice:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml install_doctor -- --nocapture`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm --dir docs build`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`

## Operator Rule

After any user-scoped upgrade, tarball install, direct binary replacement, or
custom browser manifest change, run:

```bash
agent-browser install doctor
```

Treat a nonzero result as an install or browser-build readiness problem before
debugging service behavior.
