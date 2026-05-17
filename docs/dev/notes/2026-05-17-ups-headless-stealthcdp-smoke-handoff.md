# UPS Headless StealthCDP Smoke Handoff

Date: 2026-05-17

## Context

Odollo is using `agent-browser` for SoyLei Amazon APEX-1132 carrier status
polling. The operator asked for a live smoke of one shipped but undelivered UPS
package using the newly available `chromium-stealthcdp` build in headless mode.

The expectation was that headless `chromium-stealthcdp` should behave much more
like a fully headed, human-operated Chromium instance. The operator also noted
that the `agent-browser` skill should have steered the caller toward letting
agent-browser choose the profile and browser routing instead of manually
supplying a profile path.

## Smoke Target

- Source repo where smoke was run: `/home/ecochran76/workspace.local/odollo`
- Tenant flow: `soylei-prod / amazon-apex-1132`
- Amazon order: `111-1756199-8122618`
- UPS tracking number: `1Z035CX1YW53854301`
- Work-packet status before the smoke: not delivered, sheet shipping status
  `Unknown`, workflow state `amazon_confirmation_pending`

## Preflight Evidence

`agent-browser install doctor` passed and resolved the current launch
configuration to the Windows `chromium-stealthcdp` install:

```text
version: 0.26.1
path command: /home/ecochran76/.local/bin/agent-browser
launch config source: manifest
launch config ready: true
launch executable: /mnt/c/Users/ecoch/AppData/Local/chromium-stealthcdp/current/chrome.exe
No install drift detected.
```

The browser capability preflight command was also run:

```bash
agent-browser service browser-capability preflight \
  --browser-build stealthcdp_chromium \
  --service-name Odollo \
  --agent-name codex \
  --task-name upsCarrierSmoke \
  --target-service-id ups \
  --headless
```

It returned:

```text
Browser capability preflight: apply=no reason=explicit_executable_path build=stealthcdp_chromium profile=none headless=yes cdp_free=no
```

That output is important: this smoke still forced an explicit executable path
later, so the more complete service-owned access-plan/profile routing path was
not actually exercised.

## Commands Tried

First Odollo carrier smoke, forcing headless patched Chromium and a throwaway
profile path:

```bash
scripts/odollo-with-profile.sh soylei-prod \
  sync fulfillment preview-carrier-tracking-browser-evidence \
  --flow amazon-apex-1132 \
  --carrier UPS \
  --tracking-code 1Z035CX1YW53854301 \
  --amazon-order-number 111-1756199-8122618 \
  --headless \
  --browser-profile /tmp/odollo-stealthcdp-ups-smoke-profile \
  --executable-path /mnt/c/Users/ecoch/AppData/Local/chromium-stealthcdp/current/chrome.exe \
  --wait-ms 12000 \
  --timeout-seconds 90 \
  --screenshot-output "$HOME/.odollo/tenants/soylei-prod/artifacts/fulfillment-carrier-status/stealthcdp-headless-smoke-111-1756199-ups.png" \
  --json-output "$HOME/.odollo/tenants/soylei-prod/artifacts/fulfillment-carrier-status/stealthcdp-headless-smoke-111-1756199-ups.json"
```

Result:

```json
{
  "status": "lookup_failed",
  "delivered": false,
  "errors": ["Navigation failed: net::ERR_HTTP2_PROTOCOL_ERROR"]
}
```

Second Odollo carrier smoke, attempting an HTTP/2 workaround:

```bash
AGENT_BROWSER_ARGS='--disable-http2' scripts/odollo-with-profile.sh soylei-prod \
  sync fulfillment preview-carrier-tracking-browser-evidence \
  --flow amazon-apex-1132 \
  --carrier UPS \
  --tracking-code 1Z035CX1YW53854301 \
  --amazon-order-number 111-1756199-8122618 \
  --headless \
  --browser-profile /tmp/odollo-stealthcdp-ups-smoke-profile-http1 \
  --executable-path /mnt/c/Users/ecoch/AppData/Local/chromium-stealthcdp/current/chrome.exe \
  --wait-ms 12000 \
  --timeout-seconds 90 \
  --screenshot-output "$HOME/.odollo/tenants/soylei-prod/artifacts/fulfillment-carrier-status/stealthcdp-headless-http1-smoke-111-1756199-ups.png" \
  --json-output "$HOME/.odollo/tenants/soylei-prod/artifacts/fulfillment-carrier-status/stealthcdp-headless-http1-smoke-111-1756199-ups.json"
```

Result:

```json
{
  "status": "lookup_failed",
  "delivered": false,
  "errors": ["CDP command timed out: Page.navigate"]
}
```

Control smoke proving the same patched Chromium install could launch headless:

```bash
agent-browser \
  --session stealth-smoke-direct \
  --executable-path /mnt/c/Users/ecoch/AppData/Local/chromium-stealthcdp/current/chrome.exe \
  --profile /tmp/odollo-stealthcdp-direct-smoke \
  batch --bail "open https://example.com" "get title" "close"
```

Result:

```text
Example Domain
https://example.com/
Example Domain
Browser closed
```

## Outcome

- `chromium-stealthcdp` launched successfully in headless mode for a simple
  page.
- UPS did not load successfully in headless mode for this tracking URL.
- No screenshot was captured in either UPS attempt because navigation failed
  before page content was available.
- The Odollo carrier artifact shape handled the failure cleanly by writing a
  read-only `odollo.fulfillment.carrier_tracking_browser_evidence.v1` artifact.

Artifact paths:

```text
~/.odollo/tenants/soylei-prod/artifacts/fulfillment-carrier-status/stealthcdp-headless-smoke-111-1756199-ups.json
~/.odollo/tenants/soylei-prod/artifacts/fulfillment-carrier-status/stealthcdp-headless-http1-smoke-111-1756199-ups.json
```

## Interpretation

This is not evidence that `chromium-stealthcdp` is generally broken. It is
evidence that the current headless UPS path is still not equivalent to a
fully-headed human-operated browser for this site and this environment.

The failed first run suggests a network/protocol or site compatibility problem
at navigation time. The `--disable-http2` retry changed the symptom from an
HTTP/2 protocol error to a `Page.navigate` timeout, but did not produce usable
tracking evidence.

## Agent Workflow Mistake

The smoke manually supplied both:

```text
--executable-path /mnt/c/Users/ecoch/AppData/Local/chromium-stealthcdp/current/chrome.exe
--browser-profile /tmp/odollo-stealthcdp-ups-smoke-profile
```

That was not the ideal agent-browser workflow. For a new site or operational
service, the better path is:

1. Ask agent-browser for an access plan with service, agent, task, target
   service, URL, and preferred browser build.
2. Let agent-browser select or create the compatible profile and browser build.
3. Request the browser tab through the service control plane.
4. Inspect trace, browser capability evidence, and profile compatibility
   records if the route fails.

The skill text already says not to create a new runtime profile merely because
another automation might be active, and to prefer the service/access-plan
control plane. However, the Odollo carrier command currently exposes low-level
`--browser-profile` and `--executable-path` knobs and defaults to stock Chrome,
so a downstream caller can still bypass the intended routing model.

## Product Gaps

- `agent-browser` should make it harder for agents to skip access-plan/profile
  routing when the task is site automation rather than a low-level browser
  binary smoke.
- The service/access-plan path should have a concise copyable carrier-site
  recipe, for example a UPS target service that can recommend
  `stealthcdp_chromium`, headed versus headless, profile compatibility, and
  fallback posture before launch.
- The browser capability preflight output was terse. For this case, it surfaced
  `reason=explicit_executable_path`, but the operator-facing implication should
  be clearer: explicit executable/profile overrides mean the caller is bypassing
  normal brokered browser-build and profile routing.
- Odollo should stop treating browser build and profile choice as its own
  low-level concern. It should call a service-owned agent-browser request or an
  equivalent access-plan-guided command so profile selection, browser build,
  lease handling, and site policy stay centralized in agent-browser.

## Recommended Next Steps

1. Reproduce the same UPS tracking lookup using agent-browser service/access-plan
   routing, not explicit `--profile` and `--executable-path` flags.
2. Run a paired comparison on the same tracking URL:
   - headless `stealthcdp_chromium`
   - headed `stealthcdp_chromium`
   - stock Chrome headed if needed
3. Capture service trace and browser stderr paths for the failed UPS headless
   navigation so the difference between protocol failure, site challenge,
   Windows-host networking, and headless rendering can be separated.
4. If headed succeeds and headless fails, add or update a UPS site policy so
   carrier tracking recommends headed stealth Chromium or another known-good
   posture instead of assuming headless stealth is sufficient.
5. Update the agent-browser skill with a stronger warning: for site smokes,
   use access-plan/service request first; explicit executable/profile flags are
   only for binary validation or operator-directed override.

## Validation

Read-only validation was run only. No repo code was changed by the smoke.

Passed:

```bash
agent-browser install doctor
```

Passed for basic headless launch:

```bash
agent-browser --session stealth-smoke-direct \
  --executable-path /mnt/c/Users/ecoch/AppData/Local/chromium-stealthcdp/current/chrome.exe \
  --profile /tmp/odollo-stealthcdp-direct-smoke \
  batch --bail "open https://example.com" "get title" "close"
```

Failed for UPS headless navigation as described above.
