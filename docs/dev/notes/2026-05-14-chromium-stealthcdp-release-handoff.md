# Chromium Stealth CDP Release Handoff

Date: 2026-05-14

## Context

`chromium-stealthcdp` now has a public GitHub prerelease with a Windows
Chromium binary attached.

- Patchset repo:
  `https://github.com/CochranResearchGroup/chromium-stealthcdp`
- Release:
  `https://github.com/CochranResearchGroup/chromium-stealthcdp/releases/tag/v150.0.7835.0-stealthcdp.6b6558b55a1d`
- Release asset:
  `chromium-stealthcdp_150.0.7835.0+stealthcdp.6b6558b55a1d_win64.zip`
- Release asset SHA256:
  `1e3878f270b383acdc99d0e6b82c687f1ffc4f8d27b86cc4bba1dd334946fae3`
- Chromium version: `150.0.7835.0`
- Chromium source SHA:
  `24ecda02e97db6fa730a7ccf8747776a4d21e4b9`
- Chromium upstream revision:
  `d421c3af8268e2e6227b7fe4461183e69b64bc61`
- Patchset commit:
  `fcf2d964f9070cc9acf7aabbfb2c576f36107bbe`
- Patch queue SHA256:
  `6b6558b55a1d3b0dc081871e2d76cd6dc74665d28d0e5789846b456388afd3cf`
- `chrome.exe` SHA256:
  `5d4cb9a996df941885cf29beefc53e154a46750eed37b3d83b25e6c423c70f2c`

The release is intentionally marked prerelease until agent-browser consumes the
artifact end to end.

## Windows Install Boundary

Runnable Windows browser executables should be installed on the Windows
filesystem, not under a WSL checkout or WSL artifact directory.

For the current WSL tenant owner, the verified user-scoped install path is:

```text
/mnt/c/Users/ecoch/AppData/Local/chromium-stealthcdp/current/chrome.exe
```

The patchset repo has an installer for this:

```bash
cd /home/ecochran76/workspace.local/chromium/chromium-stealthcdp
scripts/install-windows-user.sh \
  --artifact ../artifacts/chromium-stealthcdp/current \
  --force
```

That script installs into `%LOCALAPPDATA%\chromium-stealthcdp`, creates a
Windows directory junction named `current`, and applies the app-container ACLs
needed by Chromium's Windows sandbox.

## Current Local State

On this workstation the Windows user install was verified as:

```text
C:\Users\ecoch\AppData\Local\chromium-stealthcdp\current
```

The junction target was:

```text
C:\Users\ecoch\AppData\Local\chromium-stealthcdp\150.0.7835.0+stealthcdp.6b6558b55a1d
```

The installed-path smoke launched the LocalAppData executable in place and
reported:

```json
{
  "platform": "win",
  "success": true,
  "chromeVersion": "Chromium 150.0.7835.0",
  "checks": {
    "versionRuns": true,
    "cdpReachable": true,
    "navigatorWebdriver": "false",
    "navigatorWebdriverExpected": "false"
  }
}
```

## Agent-Browser Action Items

Add a resolver or installer path that can consume this GitHub release asset
without requiring a local Chromium build tree.

The first implementation can be narrow:

1. Download the Windows zip from the release URL.
2. Verify the zip SHA256 before extraction.
3. Extract into a temporary directory.
4. Install or copy the runtime under:

   ```text
   /mnt/c/Users/<windows-user>/AppData/Local/chromium-stealthcdp/
   ```

5. Expose the stable executable path through:

   ```text
   /mnt/c/Users/<windows-user>/AppData/Local/chromium-stealthcdp/current/chrome.exe
   ```

6. Record the source as a manifest-backed `stealthcdp_chromium` executable,
   not as an ad hoc custom Chrome path.

Do not make agent-browser depend on:

- `/home/ecochran76/workspace.local/chromium/src/out/...`
- `/home/ecochran76/workspace.local/chromium/artifacts/...`
- WSL UNC execution of `chrome.exe`

Those paths are build or packaging surfaces. The Windows runtime contract is
LocalAppData.

## Required Agent-Browser Tests

Before treating the release as ready for managed browser operation, run a live
smoke from `/home/ecochran76/workspace.local/agent-browser`.

Suggested environment:

```bash
export AGENT_BROWSER_EXECUTABLE_PATH=/mnt/c/Users/ecoch/AppData/Local/chromium-stealthcdp/current/chrome.exe
export AGENT_BROWSER_HOME="$(mktemp -d)"
```

Focused privacy smoke:

```bash
cargo test --manifest-path cli/Cargo.toml \
  e2e_chromium_stealthcdp_navigator_webdriver_false \
  -- --ignored --test-threads=1 --nocapture
```

Streaming and headless CDP checks:

```bash
cargo test --manifest-path cli/Cargo.toml \
  e2e_runtime_stream_enable_before_launch_attaches_and_disables \
  -- --ignored --test-threads=1

cargo test --manifest-path cli/Cargo.toml \
  e2e_stream_frame_metadata_respects_custom_viewport \
  -- --ignored --test-threads=1
```

Full native E2E check, when time allows:

```bash
cargo test --manifest-path cli/Cargo.toml e2e -- --ignored --test-threads=1
```

Expected proof:

- agent-browser launches the LocalAppData Windows executable through the
  existing executable-path surface.
- CDP-backed navigation works.
- Snapshot or screenshot works.
- Streaming still works.
- Page JavaScript sees `navigator.webdriver === false`.
- The browser process and runtime profile are cleaned up after the test.

## Product Boundary

This release only removes the explicit Chromium self-reporting signal exposed
through `navigator.webdriver`.

It does not prove that all site challenge systems are solved. Keep
site-specific access posture in agent-browser site policy, profile management,
launch mode, account reputation handling, and input behavior. Continue to treat
`cdp_free_headed` as a distinct site-policy posture when the presence of CDP is
itself the problem.
