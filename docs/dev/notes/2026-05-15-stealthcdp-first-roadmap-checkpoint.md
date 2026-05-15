# Stealth CDP First Roadmap Checkpoint

Date: 2026-05-15

## Context

Earlier roadmap notes correctly identified that some sites react badly when a
Chrome DevTools Protocol endpoint is attached. That pushed the service roadmap
toward CDP-free operation for sites such as Canva and for Google first-login
profile seeding.

The promoted `chromium-stealthcdp` build changes the priority order. It keeps
the worker queue, CDP control plane, service-owned state, profile lease model,
health tracking, API, MCP, and generated client surfaces available while
reducing the browser-visible automation signals that made CDP-backed operation
fragile.

## Decision

`stealthcdp_chromium` is the preferred browser build for normal managed
automation when a validated build is available. New managed website and account
combinations should prefer the stealth build by default unless a site policy,
profile compatibility rule, or explicit operator choice selects a different
browser build.

CDP-free operation remains a first-class fallback posture, not the primary
implementation lane. It should be used when:

- a site policy says the existence of a DevTools endpoint is unsafe
- first-login seeding, sync setup, passkey setup, or browser extension setup
  must happen without CDP attachment
- an operator explicitly requests a CDP-free headed session
- future OS-level or remote-headed control work needs a no-DevTools browser
  process

`stock_chrome` remains a supported override for sites, identities, or operators
that need native Chrome behavior.

## Planning Impact

The next roadmap lane should harden stealth-CDP-first routing and validation
rather than expand CDP-free observability first.

Definition of done for the next bounded slice:

- access-plan clearly explains when `stealthcdp_chromium`, `stock_chrome`, or
  `cdp_free_headed` won browser-build selection
- the browser capability registry records enough validation evidence to trust a
  stealth build before it becomes the default for a site or identity
- profile selection keeps browser-build compatibility explicit so Chromium,
  Chrome, and future Windows or Chrome-compatible browsers do not silently share
  incompatible profiles
- docs and examples continue to present agent-browser as the owner of profile,
  browser, tab, queue, and request coordination
- CDP-free docs identify it as a fallback for hard CDP blocks and detached
  manual setup, not the normal solution for anti-bot hardening

## Next Recommended Slice

Implement or tighten the no-launch access-plan explanation for browser-build
selection. The plan should surface the winning build, the evidence source, the
profile compatibility reason, and any operator override so software clients and
agents can understand why a request will use stealth Chromium, stock Chrome, or
CDP-free headed mode before a browser is launched.
