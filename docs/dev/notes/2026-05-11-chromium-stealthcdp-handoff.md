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

## 2026-05-12 Headed Canva Managed-Profile Probe

After the CI fix for the `browserBuild` output test, ran a headed Canva probe
against the existing managed `canva-preview` profile with:

```bash
AGENT_BROWSER_EXECUTABLE_PATH=/home/ecochran76/workspace.local/chromium/src/out/Default/chrome
AGENT_BROWSER_SOCKET_DIR=/tmp/agent-browser-canva-stealth-smoke-1778607622
AGENT_BROWSER_SESSION=canva-stealth-smoke
cargo run --manifest-path cli/Cargo.toml -- \
  --runtime-profile canva-preview --headed open https://www.canva.com
```

Result:

- The patched Chromium executable reported `Chromium 150.0.7835.0`.
- The managed `canva-preview` profile uses
  `/home/ecochran76/.agent-browser/runtime-profiles/canva-preview/user-data`.
- The isolated headed launch initially loaded Canva with title
  `Canva: Visual Suite for Everyone` at `https://www.canva.com/`.
- The follow-up CDP read failed with connection refused on the recorded
  DevTools port.
- Process inspection showed the browser PID was already gone.
- A subsequent `runtime status` reported `Browser alive: false` with no
  targets.

Interpretation:

- Headed stealth CDP with the existing Canva managed profile is not yet stable
  enough to call Canva-ready.
- The result strengthens the need for browser-health reconciliation to classify
  immediate post-navigation process exit as a browser crash or degraded browser
  condition, rather than leaving operators to infer it from a refused CDP port.
- Canva should continue to prefer `cdp_free_headed` until a CDP-backed headed
  smoke can both load the page and survive queued follow-up reads.

### Post-Reachability-Gate Rerun

After commit `321c13b` added `runtime status` `DevTools reachable` reporting
and gated managed-runtime auto-attach on reachable DevTools, reran the same
headed smoke with an isolated socket:

```bash
AGENT_BROWSER_EXECUTABLE_PATH=/home/ecochran76/workspace.local/chromium/src/out/Default/chrome
AGENT_BROWSER_SOCKET_DIR=/tmp/agent-browser-canva-stealth-smoke-1778610500
AGENT_BROWSER_SESSION=canva-stealth-smoke
cargo run --manifest-path cli/Cargo.toml -- \
  --runtime-profile canva-preview --headed open https://www.canva.com
```

Result:

- `open` again loaded `Canva: Visual Suite for Everyone` at
  `https://www.canva.com/`.
- Immediate `runtime status` showed `Browser alive: true` and
  `DevTools reachable: true`.
- The follow-up `eval` still failed because the browser exited between the
  status probe and the queued read.
- A later `runtime status` showed `Browser alive: false`.
- `service browsers` recorded `session:canva-stealth-smoke` as
  `health=process_exited`, `failure=browser_process_exited`, and
  `exit_cause=unexpected_process_exit` for PID `427283`.
- `service events --kind browser_health_changed` recorded the transition from
  `Ready` to `ProcessExited`.

Interpretation:

- The durable service health surface now classifies the Canva headed stealth
  crash instead of leaving only a refused CDP port for operators to interpret.
- The direct command error is still low-level CDP discovery text. A future UX
  improvement should translate this race into an immediate browser
  process-exited error when the daemon can observe the PID exit during command
  dispatch.

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

## 2026-05-12 Crash Trace Handoff

This section is for the `chromium-stealthcdp` patch agent. The observed symptom
is a headed Canva smoke where the page initially loads, then a follow-up CDP
read fails with connection refused on the recorded DevTools port. The first run
looked like a browser exit:

- Session: `canva-stealth-smoke`
- Runtime profile: `canva-preview`
- Profile directory:
  `/home/ecochran76/.agent-browser/runtime-profiles/canva-preview/user-data`
- Browser binary:
  `/home/ecochran76/workspace.local/chromium/src/out/Default/chrome`
- Browser version: `Chromium 150.0.7835.0`
- Chromium source branch: `ec/chromium-stealthcdp`
- Chromium patch commit:
  `24ecda02e9 Make navigator.webdriver non-advertising`

### Original Service Evidence

Persisted service events for `session:canva-stealth-smoke` show this sequence:

- `2026-05-12T18:28:32.856991586Z`: health changed from `not_started` to
  `ready`.
- `2026-05-12T18:28:32.857086485Z`: launch metadata recorded PID `427283`,
  host `local_headed`, profile `canva-preview`, CDP endpoint
  `ws://127.0.0.1:37961/devtools/browser/e7383fb7-eecc-4033-83aa-fba13b59ac97`.
- `2026-05-12T18:28:54.972217296Z`: Canva page target opened with title
  `Canva: Visual Suite for Everyone` at `https://www.canva.com/`.
- Immediate `runtime status` reported `Browser alive: true` and
  `DevTools reachable: true`.
- The next `eval` failed with connection refused against port `37961`.
- `2026-05-12T18:29:28.402645664Z`: health changed from `ready` to
  `process_exited` with `failureClass=browser_process_exited`,
  `processExitCause=unexpected_process_exit`, and error
  `Recorded browser PID 427283 is no longer running`.

Crash artifacts were sparse. The profile contained
`CrashpadMetrics-active.pma`, but no `*.dmp` file or `chrome_debug.log` was
found under the profile during this investigation.

### Direct Chrome Controls

Two direct controls were run with the same patched Chromium binary, same
`canva-preview` profile, same Canva URL, `--remote-debugging-port=0`,
`--password-store=basic`, and `--use-mock-keychain`.

The first direct control had no display and is not the target crash. It exited
after one second with status `1` and:

```text
ERROR:ui/ozone/platform/x11/ozone_platform_x11.cc:257] Missing X server or $DISPLAY
ERROR:ui/aura/env.cc:246] The platform failed to initialize.  Exiting.
```

Artifacts:

```text
/tmp/chromium-stealthcdp-canva-crash-1778611853/
```

The second direct control used `xvfb-run -a -s '-screen 0 1280x800x24'`. It
stayed alive for 75 seconds, served `/json/version`, and exposed a Canva page
target plus extension and service-worker targets. It was terminated by the test
harness, not by Chromium:

```text
DevTools port: 37939
Browser: Chrome/150.0.7835.0
Canva target: Canva: Visual Suite for Everyone, https://www.canva.com/
Exit status: 143 after harness SIGTERM
```

Artifacts:

```text
/tmp/chromium-stealthcdp-canva-xvfb-1778611966/
```

The third direct control used `DISPLAY=:0.0`, matching agent-browser's headed
fallback when no `DISPLAY` is set in the agent shell. It stayed alive for 20
seconds, served `/json/version`, exposed the Canva target, and exited cleanly
after harness termination:

```text
DevTools port: 36785
Browser: Chrome/150.0.7835.0
Canva target: Canva: Visual Suite for Everyone, https://www.canva.com/
Exit status: 0 after polite harness termination
```

Artifacts:

```text
/tmp/chromium-stealthcdp-canva-display0-1778612141/
```

Important privacy note: the verbose Chromium logs can include request headers
and cookies. Do not paste raw `stdout.log` contents into tickets or memory.
Use local inspection or sanitized excerpts only.

### Agent-Browser Repro Retry

A fresh agent-browser service-path retry used:

```bash
AGENT_BROWSER_EXECUTABLE_PATH=/home/ecochran76/workspace.local/chromium/src/out/Default/chrome
AGENT_BROWSER_SOCKET_DIR=/tmp/agent-browser-canva-repro-1778612199/socket
AGENT_BROWSER_SESSION=canva-stealth-smoke-repro
cargo run --manifest-path cli/Cargo.toml -- \
  --runtime-profile canva-preview --headed open https://www.canva.com/
```

Result:

- `open` loaded `Canva: Visual Suite for Everyone`.
- Immediate `runtime status` showed PID `525847`, port `37027`,
  `Browser alive: true`, and `DevTools reachable: true`.
- After 20 seconds, `runtime status` still showed PID `525847`, port `37027`,
  `Browser alive: true`, and `DevTools reachable: true`.
- The follow-up `eval navigator.webdriver` failed with connection refused on
  `/json/version`, `/json/list`, and the browser WebSocket URL for port
  `37027`.
- `service browsers` still showed
  `session:canva-stealth-smoke-repro health=ready profile=canva-preview
  pid=525847`.
- The explicit `close` command then moved the session to `not_started` as
  `operator_requested_close`.

Artifacts:

```text
/tmp/agent-browser-canva-repro-1778612199/agent-browser.log
```

This retry did not reproduce the same durable `process_exited` health
transition before close. It did reproduce the user-visible CDP connection
refusal after a previously reachable status probe.

### Service-State Oddity

During the repro, persisted service state also had:

```text
session:default health=ready host=local_headless profile=canva-preview pid=302687
```

But process inspection of PID `302687` showed it was actually launched with:

```text
--user-data-dir=/home/ecochran76/.agent-browser/runtime-profiles/default/user-data
```

That means at least one service-state record had stale or incorrect
`profileId` metadata. This is probably an agent-browser state reconciliation
bug rather than a Chromium crash, but it matters because the original crash
event also reported `profileLeaseConflictSessionIds=["default"]`.

### Current Interpretation

The `navigator.webdriver` patch alone is unlikely to be the direct crash cause.
The same patched browser and same profile can load Canva and stay alive under
both `xvfb-run` and `DISPLAY=:0.0` direct controls. The failure currently looks
like one of these:

- An agent-browser service or runtime-state race where a follow-up command reads
  a stale DevTools port after the browser or daemon state changed.
- A stale profile-lock or stale `DevToolsActivePort` interaction around repeated
  headed launches. The probes removed stale `Singleton*` files and
  `DevToolsActivePort` when the recorded PID was dead.
- A service-state reconciliation bug, proven by `session:default` claiming
  `profile=canva-preview` while its live process used the default profile.
- A site or extension induced browser exit that is intermittent, since the
  direct controls loaded Canva and the service retry survived at least
  20 seconds before the `eval` connection refusal.

### Requested Chromium-Patch-Agent Work

Please trace from the Chromium side with symbols and process diagnostics, but
start by trying to reproduce the service-path failure rather than assuming the
patch is crashing Blink:

1. Re-run the agent-browser service repro with Chromium logging enabled in the
   launched browser process. The current agent-browser automation path pipes
   Chrome stderr but does not persist it to a stable artifact for this handoff.
2. Compare the same flags and profile under direct `DISPLAY=:0.0` and
   agent-browser launch. If direct stays alive while agent-browser loses CDP,
   trace process lifetime and DevToolsActivePort writes around the agent-browser
   daemon.
3. Capture minidumps by configuring Crashpad or a local dump directory. Current
   evidence found only `CrashpadMetrics-active.pma`, not an actionable dump.
4. Test with extensions disabled on a copied `canva-preview` profile. The live
   targets include several extension service workers, including
   `fpeoodllldobpkbkabpblcfaogecpndd`,
   `hdokiejnpimakedhajhdlcegeplioahd`, and
   `lcfdefmogcogicollfebhgjiiakbjdje`.
5. Test with stock Chrome or unpatched Chromium using the same profile and
   flags. If stock also loses the DevTools port, this is outside the
   `navigator.webdriver` patch.
6. Inspect why the build prints `WebKit-Version` with an all-zero revision in
   `/json/version`. This may be harmless local-build metadata, but it is worth
   ruling out because the patch build is intended to be the preferred
   agent-browser engine.

Recommended next step: instrument agent-browser's Chrome launch path to write a
per-launch browser stderr log and record child exit status as soon as the child
can be reaped. Without that, Chromium-side tracing has to reconstruct too much
from service-state symptoms and transient DevTools refusal.

## 2026-05-12 Agent-Browser Diagnostic Follow-Up

agent-browser now drains locally owned Chrome stderr into per-launch logs under
`~/.agent-browser/tmp/chrome-launches/`. Launch failures include the log path in
the error text, process-exit health events include `browserStderrLogPath` when
available, and CDP-disconnect health events include the same path for locally
owned Chrome processes.

This does not prove the stealth Chromium crash root cause. It gives the
Chromium patch agent a stable first artifact to request from future service-path
repros before moving to minidumps or symbolized Chromium traces.
