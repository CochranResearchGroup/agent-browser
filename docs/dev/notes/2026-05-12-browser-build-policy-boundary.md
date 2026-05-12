# Browser Build Policy Boundary

Date: 2026-05-12

## Context

The Chromium `chromium-stealthcdp` handoff gives agent-browser a narrow patched
Chromium option that keeps the normal CDP-backed operating model while
removing the explicit page-visible `navigator.webdriver` signal.

This note records the boundary for future anti-anti-bot hardening so
agent-browser does not overfit Chromium patches or try to spoof browser
internals from the service layer.

## Decision

Chromium-level patches should be rare and limited to browser-internal signals
that agent-browser cannot reliably manage from outside the browser process.

The current `navigator.webdriver` patch is in scope because it removes a
browser self-reporting signal that can be set by headless mode, CDP launch
posture, remote debugging pipe, `remote-debugging-port=0`, explicit automation
mode, or `Emulation.setAutomationOverride`.

Future Chromium patch requests should meet all of these criteria:

- The signal is directly page-observable or site-observable.
- The signal is caused by Chromium automation, CDP, or headless internals
  rather than by agent-browser behavior.
- The signal cannot be corrected coherently through launch flags, profile
  state, site policy, or service-level behavior.
- The patch can remain narrow enough that it does not create a larger
  fingerprint inconsistency.
- The patch can be validated with a focused smoke that proves the browser still
  works through agent-browser.

## Agent-Browser-Owned Hardening

Most hardening belongs in agent-browser because it depends on site, profile,
identity, task, timing, and operator policy.

agent-browser should own:

- site policy decisions such as CDP-free launch, headed versus headless,
  patched Chromium, local headed, Docker headed, cloud browser, viewport,
  locale, timezone, rate limits, jitter, challenge policy, and manual login
  preference
- profile policy decisions such as seeded login identity, extension posture,
  passkeys, stored credentials, keyring mode, cookies, profile freshness, and
  which services may share a profile
- behavior policy decisions such as real mouse and keyboard input, human-like
  pointer paths, scroll cadence, retries, cooldowns, batching, and stop points
  for operator approval
- service state decisions such as access-plan recommendations, monitor
  incidents, retained challenge records, profile readiness, and audit history

The design rule is:

```text
Chromium should remove impossible-to-hide self-reporting.
agent-browser should manage site-specific access posture and behavior.
```

## Browser Build Variants

Future service policy should gain a browser-build or engine-variant concept.
The initial vocabulary can stay descriptive until a public schema is added:

- `stock_chrome`: normal Chrome or Chromium, selected when CDP is acceptable
  and no stealth patch is required
- `stealthcdp_chromium`: patched Chromium with `navigator.webdriver`
  non-advertising, selected when normal CDP control is desirable but explicit
  automation self-reporting is risky
- `cdp_free_headed`: headed browser launched without a DevTools endpoint,
  selected when the presence of CDP itself is risky

This should initially be a policy and access-plan recommendation, not a new
browser lifecycle path. The existing executable-path surface is sufficient for
the patched Chromium binary until the service needs explicit build inventory,
capability reporting, or automatic binary selection.

## Validation Expectations

Before treating a Chromium patch as product-ready, agent-browser needs a live
smoke that proves:

- the patched executable launches through the existing executable-path surface
- CDP-backed navigation and snapshot or screenshot still work
- stream viewing does not change the page-visible result
- `navigator.webdriver` remains `false` in the targeted launch modes

For CDP-sensitive sites, this validation does not replace CDP-free launch
validation. A patched CDP browser and a CDP-free headed browser are different
access postures.

## Next Implementation Slice

Add no-launch policy vocabulary first:

- model an internal browser-build recommendation in access-plan decisions
- keep built-in site policy capable of preferring `stealthcdp_chromium` or
  `cdp_free_headed`
- expose the recommendation in service status, MCP resources, HTTP responses,
  and generated client helpers only after the shape is stable
- add the focused live `e2e_chromium_stealthcdp_navigator_webdriver_false`
  smoke when the patched Chromium binary is available
