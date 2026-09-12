# Retained Tab Session Identity Conflict And Profile Path Launch Failure

## Summary

Fresh Roof state-registry fieldwork exposed two related control-plane defects
while attempting to reuse the durable `default` profile.

1. A live retained browser was advertised under one browser/session alias while
   its tab handles named a different owner session. Navigation rejected both
   identities with `service_navigation_tab_identity_conflict`, leaving no
   executable route to a tab that Service inventory presented as ready.
2. After the original process exited and the access plan correctly authorized
   terminal-owner replacement, passing the plan's exact profile path to the
   native CLI generated an invalid synthetic runtime-profile name containing a
   colon. The launch failed before a browser became observable, but was
   classified as `effect_uncertain`.

The durable profile data was preserved. A bounded retry using the named profile
`default` successfully launched chromium-stealthcdp and opened the South
Carolina Secretary of State search page.

## User Impact

The first defect makes a retained browser appear reusable while every effect
route is rejected. Clients cannot determine which advertised identity is
authoritative, and repeated attempts risk uncertain duplicate effects.

The second defect means a caller cannot safely copy a profile path from an
access-plan request into the native CLI. The resulting failure is especially
confusing because the validation error is deterministic and prelaunch, yet the
terminal outcome says the effect is uncertain.

Together these failures encourage exactly the wrong operational workarounds:
creating another profile, launching a duplicate browser, or discarding retained
identity. Those workarounds would fragment user state and weaken profile reuse.

## Environment

- Observed: September 11 and 12, 2026
- Installed browser build: `stealthcdp_chromium`
- Selected managed profile: `default`
- Browser executable selected by capability preflight:
  `/home/ecochran76/workspace.local/chromium/artifacts/chromium-stealthcdp/153.0.8003.0+stealthcdp.86aec912997e/chrome-linux/chrome`
- Target site:
  `https://businessfilings.sc.gov/BusinessFiling/Entity/Search`
- Service: `Odollo`
- Task: `freshroof-sc-registry-enrichment`

No new profile was created and no existing profile was deleted or reset.

## Defect 1: Retained Tab Identity Conflict

### Observed state

Service inventory advertised the retained Ohio browser as:

- browser ID: `session:handoff-eb77590fba7e3b76`
- profile: `default`
- Ohio target: `68B441F28132E841A5E10F3744A08180`
- blank target selected for South Carolina navigation:
  `8E9841A33FAEB41676F16BCABC626BA6`

The same tab records carried a different current ownership identity:

- lease ID: `handoff-dee3a731b4c4b053`
- owner session ID: `handoff-dee3a731b4c4b053`
- session name: `handoff-dee3a731b4c4b053`

The browser-level ID therefore encoded the older handoff identity while the
tab handle encoded the newer owner identity.

### Reproduction

1. Obtain the retained browser and tab from Service inventory.
2. Attempt to navigate the blank target using the advertised browser session
   identity `handoff-eb77590fba7e3b76`.
3. Observe `service_navigation_tab_identity_conflict` with uncertain effect.
4. Inspect the exact target and confirm that it remains `about:blank`.
5. Attempt the same navigation using the tab handle's current owner session
   `handoff-dee3a731b4c4b053`.
6. Observe the same `service_navigation_tab_identity_conflict`.
7. Inspect again and confirm that the target remains `about:blank`.

The two failures were not followed by another blind retry. The retained browser
was left open at that time.

### Expected behavior

- Service inventory must expose one canonical, executable browser and session
  identity for every ready tab handle.
- If a durable handoff alias differs from the current owner session, ordinary
  handle-based navigation must resolve the alias internally or return a
  refreshed handle before any effect attempt.
- A ready tab must not be returned with a browser ID and owner session pair that
  the navigation validator will reject in every supported combination.
- A typed conflict should identify the exact disagreeing fields and provide an
  executable handle-refresh action when refresh is safe.

### Likely fault boundary

This incident is consistent with an alias or owner-generation transition being
projected into the tab handle without the corresponding browser identity being
canonicalized for navigation validation. This is a hypothesis, not a confirmed
root cause. The investigation should trace durable-handoff resolution, tab
projection, handle refresh, and navigation identity validation as one flow.

## Defect 2: Exact Profile Path Produces Invalid Runtime Profile

After the original browser exited, a fresh access plan for `runtimeProfile`
`default` reported:

- `profileReuse.recommendedAction = launch_new_browser`
- no active profile lease conflict
- `lifecycleReplacement.reason = terminal_cleanup_satisfied`
- `lifecycleReplacement.replacementEligible = true`
- replacement browser ID `session:handoff-eb77590fba7e3b76`
- replacement session name `handoff-dee3a731b4c4b053`
- exact profile path
  `/home/ecochran76/.agent-browser/runtime-profiles/default/user-data`

Capability preflight passed and selected the validated WSL-native
chromium-stealthcdp executable.

The following native launch shape then failed:

```text
agent-browser --json \
  --session terminal-profile-e4748d6560de06942c5ae009 \
  --profile /home/ecochran76/.agent-browser/runtime-profiles/default/user-data \
  open https://businessfilings.sc.gov/BusinessFiling/Entity/Search
```

Recorded job: `r695390`

The command made three internal attempts and returned:

```text
Invalid runtime profile 'custom:18348830372933933750'. Must match /^[a-zA-Z0-9_-]+$/
```

The terminal outcome was:

- state: `failed`
- effect state: `effect_uncertain`
- retry disposition: `inspect_before_retry`
- failure code: `service_operation_failed`

Post-failure inspection found no tab for the requested session and no live
`default` profile tab. This supported a changed-tactic retry without creating a
duplicate profile lane.

### Expected behavior

- A managed profile path returned by an access plan must remain copyable into
  every documented adapter that accepts `profile`.
- Internal custom-profile identities must satisfy the runtime-profile grammar,
  or path-based profiles must remain path identities rather than being passed
  through runtime-profile validation.
- This deterministic validation failure should be classified as confirmed
  pre-effect when no process or tab was launched.
- An individual client request must not retry the same deterministic argument
  validation failure three times.

## Recovery Evidence

After inspecting job `r695390` and confirming that no browser effect occurred,
the same selected profile was addressed by name:

```text
agent-browser --json \
  --session terminal-profile-e4748d6560de06942c5ae009 \
  --profile default \
  open https://businessfilings.sc.gov/BusinessFiling/Entity/Search
```

This request succeeded as `r874978`.

Current verified state:

- session: `terminal-profile-e4748d6560de06942c5ae009`
- browser: `session:terminal-profile-e4748d6560de06942c5ae009`
- profile: `default`
- target: `DF70DB35CE382AF85B057C8A2DA1EAD9`
- title: `Business Name Search - Business Entities Online - S.C. Secretary of State`
- URL: `https://businessfilings.sc.gov/BusinessFiling/Entity/Search`
- tab lifecycle: `ready`
- tab-handle stale reason: none

The browser remains open for continued registry fieldwork.

### Post-launch ownership-transfer limitation

The successful URL and title reads above occurred before a concurrent installed
runtime upgrade entered admission drain. A later screenshot request was rejected
as `existing_session_profile_identity_unproven`. A stream-status request was
recorded as job `r203744` and failed with:

```text
runtime_admission_draining: transaction 'upgrade-55a6f48d-5988-4be8-a626-1d234c6e0a1b' is transferring runtime ownership at revision 12
```

That job returned `effect_uncertain` with `inspect_before_retry`. Inspection
confirmed that the chromium-stealthcdp process tree for the `default` profile
was still live under browser PID `92141`, while the contemporaneous Service
projection no longer returned the browser or its tab. No retry, process signal,
profile reset, or interference with the separate upgrade transaction followed.

The recovery evidence therefore proves that the browser launched and reached
the requested page. It does not prove that the browser remained controllable
through the concurrent owner transfer. After the upgrade reaches a terminal
state, Service should either project the same process and target under its new
canonical owner or expose a typed, executable recovery disposition.

## Recommended Regression Coverage

1. Construct a retained browser whose durable alias differs from its current
   owner session after a valid owner-generation transition. Project its tabs,
   then prove that a copied handle can navigate without an identity conflict.
2. Assert that access-plan browser IDs, session names, owner session IDs, lease
   IDs, and tab handles resolve to one canonical effect route.
3. Exercise `tab_handle_refresh` across the same transition and require either
   a valid refreshed handle or a precise terminal disposition.
4. Pass a managed profile's absolute `user-data` path through native CLI launch
   parsing and assert that no colon-bearing runtime-profile ID reaches runtime
   profile validation.
5. Assert that deterministic argument validation is attempted once and returns
   a confirmed pre-effect terminal outcome.
6. Run the same launch by profile name and by access-plan HTTP or MCP request,
   then require equivalent profile identity, browser build, and ownership
   records.
7. Verify that none of these paths creates a second managed profile or a
   duplicate live browser for the same exclusive profile directory.
8. Start an admitted owner transfer immediately after a successful terminal
   replacement launch. Require the browser and target to remain projected, or
   return a stable transfer-bound handle that becomes executable after commit.
   A live process must not silently disappear from Service inventory.

## Follow-up Boundary

The original field capture recorded evidence and acceptance criteria without
claiming a source fix. It does not authorize retirement of retained records,
profile reset, runtime replacement, or broad process cleanup.

## Review And Action Checkpoint

Plan0172 reproduced and repaired the two source defects without touching the
retained production browser:

- Authenticated path-backed custom profiles now remain profile-path identities
  instead of becoming invalid colon-bearing runtime-profile names.
- Named runtime profiles are validated before process creation, so deterministic
  input failure is attempted once and reports `no_effect` with no blind retry.
- Navigation persistence accepts a logical owner alias only when current owner,
  process, daemon, profile, session, browser, and target evidence resolve one
  exact physical browser. It preserves that physical browser ID and retains the
  typed conflict for foreign or ambiguous aliases.
- `runtime_admission_draining` now reports a confirmed pre-effect admission
  denial with transaction inspection and terminal-state waiting as safe next
  actions.

The post-upgrade projection concern was rechecked read-only. Upgrade transaction
`upgrade-55a6f48d-5988-4be8-a626-1d234c6e0a1b` is accepted at revision 14 with
no outstanding obligations. Browser PID `92141`, its `default` profile, owner
generation 102, session `handoff-1c85f31dcc34550b`, and the South Carolina
target are coherently projected. This demonstrates eventual convergence for
the observed transaction and does not justify a second projection repair.

Focused regressions, strict workspace Clippy, route-confusion gates, formatting,
and diff checks qualified source commit `02f40c370e73eea5e869c70dd7d8ebe220345060`,
which merged through PR 46 at
`8d3f552f6bef3abec0e398900ba638ac7962a62f`. Production installation and
fieldwork acceptance remain separately authorized gates.
