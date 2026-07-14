# Last30days Runtime Profile Routing Failure

Date: 2026-07-06

## Summary

A last30days X login check exposed a profile routing failure in the `open` path.
The operator asked whether X was still logged in after using the same runtime
profile as Facebook. The profile-level evidence showed that
`last30days-facebook` still had X auth cookies, but a live page probe that
explicitly requested that runtime profile attempted to launch
`stealthcdp-default` instead.

This note records the failure so it is not mistaken for an X auth failure.

## Context

The intended identity was:

```text
runtimeProfile=last30days-facebook
userDataDir=~/.agent-browser/runtime-profiles/last30days-facebook/user-data
```

The profile was used for Facebook remote-view work and then reused for X. The
question was whether X remained authenticated inside that same profile.

## Evidence

Direct cookie database inspection of the requested profile found X cookies in:

```text
~/.agent-browser/runtime-profiles/last30days-facebook/user-data/Default/Cookies
```

The check only reported names and flags. It did not print cookie values.

Observed cookie names included:

```text
auth_token
ct0
twid
guest_id
personalization_id
```

The important auth indicators were present:

```text
hasAuthToken=true
hasCt0=true
xCookieCount=19
```

`agent-browser --json --runtime-profile last30days-facebook runtime status`
reported no live reachable browser for that runtime profile:

```text
browserAlive=false
runtimeProfile=last30days-facebook
devtoolsPort=null
devtoolsReachable=false
```

No retained service browser row for `profileId=last30days-facebook` was found
in the service browser list during this check.

## Failed Live Probe

The attempted bounded live page check was:

```bash
agent-browser --json \
  --session x-login-check \
  --runtime-profile last30days-facebook \
  --browser-host remote_headed \
  --browser-build stealthcdp_chromium \
  open https://x.com/home
```

Expected behavior:

- launch or reuse the requested `last30days-facebook` runtime profile;
- navigate to `https://x.com/home`;
- verify page state against the X cookies already present in that profile.

Actual behavior:

```text
Auto-launch failed: Chrome profile ~/.agent-browser/runtime-profiles/stealthcdp-default/user-data is already in use by PID 25053
```

The diagnostic identified the owner as the `stealthcdp-default` profile:

```text
owner.activeSessionIds=["detected-profile-mirror-38305-2"]
owner.browserId=session:detected-profile-mirror-38305-2
owner.profileId=stealthcdp-default
owner.host=remote_headed
```

It also listed another retained service browser on the same default profile:

```text
browserId=session:tx-sos-google-chrome-b
profileId=stealthcdp-default
host=attached_existing
```

That means the probe did not test the requested `last30days-facebook` profile.
The later command output showing no X cookies came from the misrouted live
session, not from the requested profile database.

## Why This Matters

This is an identity-routing problem, not a target-site auth result.

If an operator passes `--runtime-profile last30days-facebook`, the `open` path
should not silently plan against `stealthcdp-default`. If the requested profile
cannot be launched or reused, the failure should name the requested profile and
explain that profile's conflict or readiness state.

The current behavior creates a misleading diagnostic loop:

1. direct profile inspection says the requested profile has X auth cookies;
2. live `open` attempts a different profile;
3. the different profile is locked;
4. any follow-up page or cookie evidence can be misread as proving the
   requested profile is logged out.

## Product Follow-Up

Recommended fixes:

- Ensure `open` honors `--runtime-profile` when combined with
  `--browser-host remote_headed` and `--browser-build stealthcdp_chromium`.
- If another selector overrides the runtime profile, surface that selector in
  the error before launch.
- When auto-launch fails due to profile lock, include both:
  - `requestedRuntimeProfile`
  - `plannedRuntimeProfile`
- Add a regression test for an `open` request that passes a non-default
  runtime profile while `stealthcdp-default` is locked by a retained service
  browser.
- Prefer a hard failure over falling back to `stealthcdp-default` for
  authenticated profile checks.

## Remediation Progress

Implemented under Plan 0069:

- Plain `open`, `goto`, and `navigate` now preserve explicit global launch
  routing flags on the `navigate` command payload:
  - `--runtime-profile`
  - `--browser-build`
  - `--browser-host`
  - `--view-stream-provider`
  - `--control-input-provider`
  - `--display-isolation`
- The no-launch regression
  `test_open_preserves_runtime_profile_when_default_profile_is_locked_shape`
  covers this note's failure class: an explicit `last30days-facebook` request
  must not be planned as `stealthcdp-default` when the default profile has a
  retained locked browser row.
- Plain navigation auto-launch now checks for a compatible retained browser
  already owning the requested profile. When that owner is live, matches the
  requested host/display posture, and has a CDP endpoint, agent-browser attaches
  to it and creates a fresh tab before navigation instead of launching a
  duplicate Chrome process.
- HTTP and MCP `service_request` `tab_new` now use the same access-plan
  shared-profile route-hint helper before relay. A compatible retained owner for
  the requested `runtimeProfile` receives the new tab request through synthesized
  top-level `browserId` and `sessionName` hints, while `params.sessionName`
  remains non-routing for ordinary non-focus requests.
- MCP `service_request` now accepts and forwards `runtimeProfile`, `profileId`,
  `profile`, `browserHost`, `viewStreamProvider`, `controlInputProvider`, and
  `displayIsolation` so the tool schema and command path can express the same
  profile-sharing selector as the HTTP service request path.
- Access-plan planned tab requests now preserve `profileClass` through the
  service-request JSON schema, generated client, HTTP adapter, and MCP adapter.
  The live service-request smoke caught this as a contract drift before the
  same-profile tab proof could run.
- `pnpm test:service-request-live` passed using
  `AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD=./cli/target/debug/agent-browser`. The
  smoke opened two tabs through one retained same-profile browser, released one
  tab, physically closed that target, preserved the browser/session route, and
  successfully evaluated the surviving tab handle.
- Slice C now has a named `routeBoundHandoff` proof record for remote-view
  plans and successful opens. The record centralizes the authoritative profile,
  browser, session, tab, route, display, and operator-visible proof facts so a
  client does not need to infer them from neighboring response fields.
- Remote-view planned and opened response assembly now lives behind the
  `remote_view_handoff` module, so the command dispatcher does not duplicate
  the authoritative profile, browser, session, route, display, tab, proof, and
  verification response shape.
- Operator-visible proof failures now include a focused `routeBoundHandoff`
  failure diagnostic for the failing route binding. Post-checkout proof
  failures keep pre-checkout evidence separately labeled as
  `preCheckoutOperatorVisible`.
- Dashboard workspace inventory now carries `profileActionability` so live
  service-owned retained browser rows recommend `openSharedProfileTab` when
  the profile is already in use by a compatible retained owner. Profile-only
  conflict rows instead recommend waiting for or inspecting the holder, so the
  dashboard no longer has to treat every in-use profile as unavailable.
- The dashboard `add-tab` action for those service-owned retained browser rows
  now posts HTTP `service_request` `tab_new` with the owner `browserId`,
  `sessionName`, runtime profile, and actionability evidence, then refreshes
  service state and selects the returned browser/tab identity.
- Dashboard workspace inventory now also projects viewer-control and route
  attachment cases through the same actionability interface. Rows with an
  active viewer controller lease recommend `takeOverViewer`, and rows whose
  stream attachability recommends a route switch expose `routeSwitch` with the
  attachability reason instead of enabling the wrong `add-tab` operation.
- Software clients now have
  `summarizeServiceSharedProfileAcquisition()` in
  `@agent-browser/client/service-request`. The helper accepts either an
  access-plan response or a tab response and returns compact requested/planned
  profile, retained browser/session route hints, tab handle, acquisition mode,
  and duplicate-process policy fields so clients do not parse raw service
  state to understand profile sharing.
- The broader `remote_view_open_` Rust filter caught and now covers a matching
  flag-preservation issue for `remote-view open`: global
  `--browser-build stealthcdp_chromium` is preserved into the command payload
  along with runtime profile, host, view-stream, control-input, and display
  posture.

Plan 0069 live-proof closeout:

- Slice C is closed at the intended module-depth boundary for this plan:
  handoff-owned response/proof/failure vocabulary is extracted, and remaining
  dispatcher code is limited to command dispatch, live browser side effects,
  timestamp supply, and repository/service plumbing.
- Live proof found and fixed one remaining plain `open` sharing gap:
  `shared_profile_attach_target_for_auto_launch` now accepts `open` in addition
  to `navigate` and `tab_new`, and it can reuse the current session's live
  service browser record when the profile is already owned by that same
  session.
- Live proof also found and fixed the route repeat-open bug: same-owner
  checked-out routes, including reconciliation-stale `orphaned` route records
  whose browser/session/display still agree, are reusable by repeat
  `remote-view open` instead of failing with
  `route_pool_entry_unavailable`.
- `AGENT_BROWSER_RDP_ROUTE_A_DISPLAY_NAME=:10
  AGENT_BROWSER_RDP_ROUTE_B_DISPLAY_NAME=:11
  AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD=./cli/target/debug/agent-browser
  pnpm test:remote-view-open-fixture-live` passed with artifact
  `/tmp/agent-browser-remote-view-open-live-2026-07-06T22-14-26-356Z`.
  The proof reported route `guacamole:4`, display allocation
  `remote-view-display:10`, display `:10`, one active intended target,
  `route_bound_ready`, `browser_window_visible`, and OCR text containing
  `REMOTE VIEW OPEN FIXTURE 55948`.

## Safe Reproduction Shape

Use a profile with harmless non-secret cookies or a temp profile marker. Do not
copy real X cookie values into fixtures.

1. Create or select a non-default runtime profile.
2. Ensure `stealthcdp-default` is locked by a retained browser.
3. Run:

```bash
agent-browser --json \
  --session profile-routing-repro \
  --runtime-profile <non-default-profile> \
  --browser-host remote_headed \
  --browser-build stealthcdp_chromium \
  open https://example.com/
```

4. Assert the planned user-data directory and any lock diagnostic reference the
   requested non-default profile, not `stealthcdp-default`.

## Current Operator Guidance

Until this is fixed, do not use a failed live `open` probe as proof that a
non-default runtime profile is logged out. Check the requested profile identity
first with `runtime status`, service browser rows, or a direct profile database
inspection that redacts values. If the live command routes to another profile,
treat the page result as invalid for the requested identity.
