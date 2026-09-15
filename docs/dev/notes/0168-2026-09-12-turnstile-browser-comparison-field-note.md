# Turnstile Browser Comparison Field Note

Date: 2026-09-12

## Purpose

Record what a controlled CAPTCHA fixture and one production Cloudflare
Turnstile comparison established about desktop input, browser selection, and
Agent Browser routing. This is an operational note, not a claim that Turnstile
can or should be bypassed.

The production target was the public Tennessee Department of State business
entity search:

`https://tncab.tnsos.gov/business-entity-search`

No credentials, private CRM records, search subjects, or returned business
records are retained in this note.

## Controlled fixture

The local `examples/captcha-lab` package provides separate Cloudflare
Turnstile and hCaptcha applications. It records callback state, supports
widget reset, and verifies test tokens through each provider's Siteverify
endpoint.

Cloudflare's documented forced-interaction sitekey was used for the Turnstile
fixture. hCaptcha's public integration-test key was used for its deterministic
pass path. The hCaptcha test key does not force an image-selection challenge;
that requires a non-production sitekey configured for Always Challenge.

Acceptance evidence:

- four provider-free HTTP and request-shape tests passed;
- both vendor widgets rendered at desktop and mobile viewport sizes;
- simulated pointer movement and one click reached each controlled widget;
- both resulting test tokens validated successfully;
- the callback readout remained legible after its initial inline-sizing defect
  was corrected.

The fixture proves perception, coordinate selection, pointer delivery,
callbacks, reset behavior, and test-token verification. It does not predict a
production anti-abuse decision because vendor test keys intentionally provide
controlled outcomes.

References:

- <https://developers.cloudflare.com/turnstile/troubleshooting/testing/>
- <https://docs.hcaptcha.com/#integration-testing-test-keys>

## Production attempt A: packaged Chrome for Testing

The first retained browser used Agent Browser's packaged Chrome for Testing
`153.0.8010.36`. It was already visible on an RDP-backed X11 desktop and was
preserved rather than relaunched.

Observed sequence:

1. The Tennessee page loaded and displayed a Turnstile checkbox.
2. The desktop capture established physical-pixel geometry of 1076 by 1874.
3. The checkbox center was visually located at approximately `(51, 747)`.
4. `xdotool` moved through a gradual multi-point path, hovered for 850 ms, and
   issued one left click.
5. The widget changed from `Verify you are human` to `Verification failed`.
6. No second click, reload loop, or automatic retry was performed.

Evidence hashes:

- pre-click capture:
  `714d40d35db66f249b0139ad1d73b0ec0b95e8972d1871e6494c297388176d30`
- post-click capture:
  `167effc860304c7e149fc222dc40c1ae24e99c76dd971930468a61de74b8759d`

The state transition proves that desktop targeting and click delivery worked.
It does not identify which browser, profile, network, or behavioral signal
caused the production rejection.

## Production attempt B: retained stable Google Chrome

The operator then authorized one comparison using stock Chrome. The usable
retained browser was stable Google Chrome `151.0.7922.108` at
`/opt/google/chrome/chrome`. Its existing profile and existing tabs were
preserved; one separate public-site tab was added.

Observed sequence:

1. The same Tennessee URL loaded in the retained stock browser.
2. No interactive Turnstile widget remained after page settlement.
3. Normal search controls were enabled.
4. One benign business-name query returned a normal registry results grid.
5. The results tab remained open and the pre-existing tabs were unchanged.

Evidence hashes:

- initial page capture:
  `a90562fec0f443bf78ae8dfaf2dd66f8f081661c40f78ed553a7f9f2bc5fd29f`
- settled search-control capture:
  `4e55e4f018c7078c22352e5000aaf999e082ecfb04ab953868df089d010d4e98`
- successful results-grid capture:
  `05a2f44bb7570bbe0e368fba63f602389b3066be7b698e33b1db6080e2c4e745`

This establishes that the retained stock-browser path completed the protected
workflow. It does not isolate stock Chrome as the causal factor because the
browser version, executable, profile history, launch flags, and retained
state all differed from attempt A.

## Agent Browser findings

### Native retained-session control remained inconsistent

Service status classified the first retained browser as viable and healthy,
but a native URL read against its session failed
`existing_session_profile_identity_unproven`. Desktop capture and direct X11
observation still reached the exact retained window. A viable service browser
should not expose contradictory native-session custody without a precise
reconciliation action.

### Admission drain interrupted observation

A workstation upgrade entered runtime-admission drain after Tennessee
navigation and before a requested desktop capture. The capture returned
`runtime_admission_draining` with an uncertain terminal outcome and a
no-blind-retry interlock. Direct read-only X11 capture was used to inspect the
already-rendered page. The upgrade later reached terminal `accepted` state.

The interlock correctly prevented an unsafe retry, but desktop observation of
an already-owned retained window should be evaluated separately from browser
launch or navigation admission where that can be proven safe.

### Explicit stock selection was overridden

An access-plan request explicitly supplied `browserBuild=stock_chrome` with
the existing `Default` profile. The returned plan instead selected
`stealthcdp_chromium` from the global preference binding and reported
`operatorOverride=false`.

An explicit valid browser-build request should either win or fail with a typed
compatibility reason. It should not silently become a different build.

### Stock preflight and profile compatibility diverged

The exact stock preflight for `Default` returned
`profile_compatibility_missing_or_blocked`, even though the capability
registry contained passed stock-Chrome installation evidence. This may be a
correct profile-compatibility stop, but the access plan should preserve the
requested build and present that stop rather than substituting the global
default.

### Planner generated incomplete route hints

A registered durable profile whose default build was `stock_chrome` produced
an access plan with:

- `browserBuild=stock_chrome`;
- no live occupancy or duplicate pressure;
- `launch_new_browser` as the profile-reuse action;
- an eligible terminal lifecycle replacement; and
- a copyable `tab_new` request.

Submitting that exact generated request failed before launch with
`service_access_plan_incomplete_route_hints`. The copied request contained a
session hint without the matching browser hint required by the executor.
Profile diagnosis then reported `runtime_browser_record_missing` and
`plan_preserving_profile_repair`, despite the access plan having advertised an
eligible replacement.

The planner and executor must agree on one of two outcomes:

1. emit a complete browser/session replacement pair that the executor accepts;
2. withhold the executable request and return the required preserving-profile
   repair as the blocking next action.

## Recommended product behavior

1. Treat controlled CAPTCHA fixtures as input and state-machine acceptance,
   never as production-pass acceptance.
2. Detect at least `checkbox_present`, `verification_in_progress`,
   `verification_succeeded`, `verification_failed`, `expired`, and
   `widget_absent_with_protected_action_available` states.
3. Bind every production interaction to a fresh frame receipt, geometry epoch,
   coordinate space, browser identity, and one explicit operation ID.
4. Default production challenges to one attempt. Require a separate operator
   decision before changing browser capability or retrying.
5. Record a passed non-interactive challenge only after a protected downstream
   action succeeds; widget absence alone is insufficient.
6. Preserve browser build and profile as separate evidence dimensions. Do not
   infer that one caused acceptance when both changed.
7. Make explicit browser-build requests authoritative or return a typed
   rejection before launch planning.
8. Validate generated route hints before presenting a service request as
   copyable or executable.
9. Keep browser/profile reuse as the default. Do not create a temporary or
   duplicate profile merely to escape an identity or lifecycle error.

## Current state

- The local CAPTCHA lab is installed, tested, committed, and pushed on branch
  `reconcile/canonical-dirty-20260912` at commit `43a5668c`.
- The stock-Chrome Tennessee results tab remains open in the retained browser.
- The packaged Chrome-for-Testing Tennessee tab remains in its failed
  Turnstile state for comparison.
- No browser process or profile was deleted, reset, or replaced during this
  fieldwork.
