# Chromium Stealth CDP Handoff

Date: 2026-05-11

## Context

The Chromium checkout at `/home/ecochran76/workspace.local/chromium` now has a
small companion patchset repository:

- Patchset repo: `/home/ecochran76/workspace.local/chromium/chromium-stealthcdp`
- Chromium source repo: `/home/ecochran76/workspace.local/chromium/src`
- Chromium source branch: `ec/chromium-stealthcdp`
- Base revision: `d421c3af8268e2e6227b7fe4461183e69b64bc61`
- Active Chromium patch commit: `24ecda02e9 Make navigator.webdriver non-advertising`
- Patchset repo commit: `4add95c Drop browser UI patch from website privacy queue`

The current exported patch queue contains one patch:

```text
/home/ecochran76/workspace.local/chromium/chromium-stealthcdp/patches/0001-Make-navigator.webdriver-non-advertising.patch
```

## Decision

`chromium-stealthcdp` is intended to let agent-browser keep its normal
headless plus CDP operating model while removing the most explicit
website-visible automation signal.

The patch changes `Navigator::webdriver()` so websites see:

```js
navigator.webdriver === false
```

This covers launch modes where upstream Chromium would otherwise set the
Blink `AutomationControlled` feature, including headless mode, remote debugging
pipe, `remote-debugging-port=0`, explicit automation mode, and
`Emulation.setAutomationOverride`.

The browser automation infobar patch was intentionally dropped. That infobar is
browser UI visible to a human observer, but it does not expose automation status
to an external website through JavaScript or normal web APIs. Human observers
are acceptable for this objective.

## Agent-Browser Integration Posture

Use the patched Chromium executable through the existing custom executable
surface:

```bash
AGENT_BROWSER_EXECUTABLE_PATH=/home/ecochran76/workspace.local/chromium/src/out/Default/chrome agent-browser open https://example.com
```

or:

```bash
agent-browser --executable-path /home/ecochran76/workspace.local/chromium/src/out/Default/chrome open https://example.com
```

Do not add a separate browser lifecycle path just for this patchset. Agent
browser should continue to own browser process lifecycle, CDP connections,
streaming, and service-owned serialized control. The patched Chromium binary is
only an engine choice.

## What This Does Not Prove

This patch does not make headless CDP behavior indistinguishable from a human
using a headed no-CDP browser. It only removes the explicit
`navigator.webdriver` website-visible signal.

Remaining differences may include headless rendering details, GPU behavior,
fonts, permissions, focus, media devices, download behavior, timing, synthetic
input patterns, viewport choices, network reputation, account reputation, and
site-specific challenge behavior.

## Chromium-Side Validation

From `/home/ecochran76/workspace.local/chromium/src`, verify that the source
branch still contains only the website-privacy patch:

```bash
git status --short --branch --untracked-files=no
git log --oneline --decorate -3
git apply --check --reverse ../chromium-stealthcdp/patches/0001-Make-navigator.webdriver-non-advertising.patch
```

Compile the touched Blink object before using the binary:

```bash
/home/ecochran76/workspace.local/depot_tools/autoninja -C out/Default obj/third_party/blink/renderer/core/core/navigator.o
```

When a runnable browser binary is needed, build:

```bash
/home/ecochran76/workspace.local/depot_tools/autoninja -C out/Default chrome
```

## Agent-Browser Validation Matrix

Run live tests with an isolated home and profile so the patched Chromium smoke
does not mutate the default runtime profile:

```bash
export AGENT_BROWSER_EXECUTABLE_PATH=/home/ecochran76/workspace.local/chromium/src/out/Default/chrome
export AGENT_BROWSER_HOME="$(mktemp -d)"
```

Baseline stream and headless CDP operation:

```bash
cd /home/ecochran76/workspace.local/agent-browser
cargo test --manifest-path cli/Cargo.toml e2e_runtime_stream_enable_before_launch_attaches_and_disables -- --ignored --test-threads=1
cargo test --manifest-path cli/Cargo.toml e2e_stream_frame_metadata_respects_custom_viewport -- --ignored --test-threads=1
```

Full native E2E check, when time allows:

```bash
cd /home/ecochran76/workspace.local/agent-browser
cargo test --manifest-path cli/Cargo.toml e2e -- --ignored --test-threads=1
```

Add a focused live privacy smoke before treating the patch as product-ready:

1. Launch patched Chromium through agent-browser in headless CDP mode.
2. Navigate to a simple controlled page.
3. Evaluate `navigator.webdriver`.
4. Assert the result is `false`.
5. Repeat while stream viewing is connected.
6. Repeat after `Emulation.setAutomationOverride({ enabled: true })` if the
   test harness has a CDP helper for raw protocol calls.

Suggested test name:

```text
e2e_chromium_stealthcdp_navigator_webdriver_false
```

Expected proof:

- agent-browser launches the patched Chromium binary through the existing
  executable-path surface.
- CDP-backed navigation, screenshot or snapshot, and stream frame delivery still
  work.
- Human stream connection does not change the page-visible
  `navigator.webdriver` result.
- `navigator.webdriver` remains `false` in headless CDP mode.

## 2026-05-12 Live Validation

The patched Chromium binary was available at:

```text
/home/ecochran76/workspace.local/chromium/src/out/Default/chrome
```

Added and ran the focused ignored E2E smoke:

```bash
AGENT_BROWSER_EXECUTABLE_PATH=/home/ecochran76/workspace.local/chromium/src/out/Default/chrome \
cargo test --manifest-path cli/Cargo.toml \
  e2e_chromium_stealthcdp_navigator_webdriver_false \
  -- --ignored --test-threads=1 --nocapture
```

Result:

- passed
- launched the patched executable through agent-browser in headless CDP mode
- evaluated `navigator.webdriver` as `false`
- confirmed CDP-backed page reads still work by taking a snapshot from the
  same session

An isolated fresh-profile Canva probe with the same patched executable also
confirmed `navigator.webdriver === false`, but the page still returned Canva's
`Just a moment...` challenge. That means the Chromium patch works for the
explicit webdriver signal, but it is not enough evidence to make Canva prefer
CDP-backed stealth mode over `cdp_free_headed`.

Current posture:

- `stealthcdp_chromium` is validated for the explicit `navigator.webdriver`
  signal.
- `cdp_free_headed` should remain the built-in Canva recommendation until a
  headed or authenticated-profile Canva smoke proves CDP-backed stealth mode
  loads reliably.
- Do not treat this patch as proof that all Canva-class anti-bot checks are
  solved.

## Service-Mode Notes

This patchset should not change the service policy semantics for sites that
explicitly require CDP-free operation. `requiresCdpFree` remains a site-policy
decision for cases where the presence of CDP itself breaks a target before page
code runs. The patched Chromium binary is useful for the normal headless CDP
path, not a replacement for every detached headed no-CDP seeding flow.

If service policy behavior changes in the same slice, also run:

```bash
pnpm test:service-access-plan-no-launch
pnpm test:service-site-policy-sources-no-launch
cargo test --manifest-path cli/Cargo.toml service_access -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml service_model -- --test-threads=1
```
