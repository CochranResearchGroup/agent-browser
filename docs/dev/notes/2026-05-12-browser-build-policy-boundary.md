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

TODO: expand this vocabulary into a broader browser capability registry. Future
support should cover Windows-hosted browsers and other Chrome-compatible
browsers such as Edge, Brave, and vendor-packaged Chromium builds. That work
should not change the current `stock_chrome`, `stealthcdp_chromium`, and
`cdp_free_headed` semantics until agent-browser has explicit capability
reporting for host OS, browser family, executable source, profile
compatibility, CDP support, extension support, keychain behavior, and
site-specific reliability. Operators should be able to make a
website and account identity primary on a non-default browser once that
inventory exists.

## Future Browser Capability Registry Shape

The future registry should describe concrete browser hosts and executables
before access policy assigns a site or account identity to them.

The first draft contract is
`docs/dev/contracts/service-browser-capability-registry.v1.schema.json`.
It is intentionally not wired into runtime state, HTTP, MCP, CLI, or generated
clients yet.

Suggested durable records:

- `BrowserHost`: machine or service that can own browser processes, with host
  ID, operating system, display support, remote-view support, service
  reachability, and lifecycle owner.
- `BrowserExecutable`: install or artifact that can be launched, with browser
  family, vendor, channel, version, build label, executable path, manifest
  source, patchset identity, and freshness metadata.
- `BrowserCapability`: normalized feature set for the executable on that host,
  including CDP support, CDP-free launch support, extension support, passkey
  support, profile lock behavior, keychain or password-store behavior, headed
  support, headless support, streaming support, and known platform limits.
- `ProfileCompatibility`: allowed profile pairings by browser family, vendor,
  major version, OS, keyring mode, and extension posture. This should prevent
  accidental Chrome-profile reuse from a Chromium, Edge, Brave, or Windows host
  unless an operator explicitly forces the mismatch.
- `BrowserPreferenceBinding`: policy record that marks a site, login identity,
  account ID, or service task as primary on a specific capability or executable.
  This is the future generalized form of routing a specific
  `OnlyWorksOnChrome.com/myuserID` identity to stock Chrome while the global
  default remains `stealthcdp_chromium`.
- `BrowserValidationEvidence`: retained smoke evidence for launch, CDP attach,
  CDP-free launch, extension availability, profile reuse, streaming, and
  site-specific reliability.

The registry should keep two concepts separate:

- build posture: what kind of browser behavior a request needs, such as
  `stealthcdp_chromium`, `stock_chrome`, or `cdp_free_headed`
- executable placement: which host and executable can satisfy that posture for
  this profile, site, and account identity

That separation lets agent-browser keep the current browser-build API stable
while later routing the same access-plan request to Linux Chrome, Windows
Chrome, Edge, Brave, a patched Chromium artifact, a Docker headed browser, or a
remote browser host.

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
