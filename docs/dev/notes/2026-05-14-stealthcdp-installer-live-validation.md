# Chromium Stealth CDP Installer Live Validation

Date: 2026-05-14

## Summary

The repo binary installed the public `chromium-stealthcdp` Windows release asset
to:

```text
/mnt/c/Users/ecoch/AppData/Local/chromium-stealthcdp/current
```

The generated stable manifest is:

```text
/mnt/c/Users/ecoch/AppData/Local/chromium-stealthcdp/current/manifest.json
```

The manifest points at `chrome.exe` and uses the release-packaged
`smoke-win.json` evidence. The installed `chrome.exe` SHA256 matched the
handoff value:

```text
5d4cb9a996df941885cf29beefc53e154a46750eed37b3d83b25e6c423c70f2c
```

## Findings

- The public ZIP has a top-level `chromium-stealthcdp/` directory, not a
  `chrome-*` directory, so the installer now flattens that release layout into
  the stable `current/chrome.exe` contract.
- The public ZIP ships `smoke-win.json`; the installer now preserves and points
  the generated manifest at that smoke evidence instead of creating a placeholder.
- With an empty config, `install doctor` auto-discovers the stable installed
  manifest and reports `stealthCdpChromiumReady: true`.
- Launching the Windows `chrome.exe` from WSL needs mounted Windows paths passed
  to Chrome in Windows form, for example `C:\Users\...`, not `/mnt/c/...`.
- This WSL-launched Windows executable mode also needs `--no-sandbox`; without
  it, Chromium exits before DevTools with `Sandbox cannot access executable`.

## Validation

Passed:

```bash
cargo run --manifest-path cli/Cargo.toml -- install stealthcdp-chromium --force
```

Passed no-launch resolver check with an empty config:

```bash
cargo run --manifest-path cli/Cargo.toml -- --config "$tmp" install doctor --json
```

Passed live smoke:

```bash
TMPDIR=/mnt/c/Users/ecoch/AppData/Local/Temp \
AGENT_BROWSER_EXECUTABLE_PATH=/mnt/c/Users/ecoch/AppData/Local/chromium-stealthcdp/current/chrome.exe \
AGENT_BROWSER_HOME="$(mktemp -d)" \
cargo test --manifest-path cli/Cargo.toml \
  e2e_chromium_stealthcdp_navigator_webdriver_false \
  -- --ignored --test-threads=1 --nocapture
```

The live smoke proved that the installed Windows build can launch through
agent-browser, expose CDP, support a snapshot, and evaluate
`navigator.webdriver` as `false`.

## Remaining Risk

The successful WSL live smoke used `TMPDIR` under the mounted Windows temp
directory so the e2e profile was Windows-accessible. Service-owned Windows
browser profiles should likewise live on a mounted Windows path or another
Windows-accessible path when the executable is Windows-native.
