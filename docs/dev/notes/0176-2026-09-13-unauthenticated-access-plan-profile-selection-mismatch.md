# Unauthenticated Access Plan Profile Selection Mismatch

## Summary

Fresh Roof public-registry fieldwork exposed an access-plan selection and
lifecycle-coherence problem. A new access plan for unauthenticated California
Secretary of State research silently selected the retained profile
`soylei-contact-test-20260906` and recommended reusing its live browser. The
same response described that browser as a terminal-profile process whose
lifecycle owner requires inspection, and it warned that the selected profile's
freshness must be verified before authenticated work.

This fieldwork does not require login state. A genuinely disposable temporary
profile is therefore acceptable and may be preferable. The problem is not that
Agent Browser failed to preserve login data or declined to choose a durable
default profile. The problem is that a request without an explicit profile was
routed into an old task-specific retained profile, then received internally
mixed guidance about whether that retained browser was reusable, terminal, or
in need of authentication verification.

## Environment And Request

- Observed: September 13, 2026
- Installed browser build: `stealthcdp_chromium`
- Service: `Odollo`
- Agent: `codex`
- Task: `freshroof-registry-enrichment`
- Target service: `ca-sos`
- Account: `soylei-prod`
- Authentication requirement: none for the public registry search

The read-only reproduction command was:

```text
agent-browser --json service access-plan \
  --service-name Odollo \
  --agent-name codex \
  --task-name freshroof-registry-enrichment \
  --target-service-id ca-sos \
  --account-id soylei-prod
```

No runtime profile was explicitly requested.

## Observed Access Plan

The access plan succeeded, but selected:

- profile: `soylei-contact-test-20260906`
- browser build: `stealthcdp_chromium`
- reusable browser:
  `session:terminal-profile-2765b4386499dc2ea0f21b99`
- reusable session: `handoff-21f881c7cbebe11c`
- reuse recommendation: `reuse_existing_browser`
- compatible live browser count: `1`
- same-profile live browser count: `1`
- active lease count: `0`
- duplicate pressure: `false`

The same response also reported:

- top-level recommended action:
  `verify_or_seed_profile_before_authenticated_work`
- profile freshness attention: required
- profile compatibility: `not_declared`
- lifecycle state: `ready`
- lifecycle reason: `terminal_process_still_live`
- lifecycle required action: `inspect_lifecycle_owner`
- process absence proven: `false`
- replacement eligible: `false`

The browser-build decision itself was coherent. It selected the validated
WSL-native StealthCDP build and cited passing launch and CDP evidence. The
ambiguity is in profile selection and lifecycle disposition, not browser-build
selection.

A companion `agent-browser --json service status` read returned only a partial
runtime observation. It reported `process_inventory_unavailable` for
`browser-cdp` and `display_probe_unavailable` for the retained California
session's view. Those observations do not prove that the browser is absent or
unhealthy. They do mean the status projection could not independently resolve
the access plan's simultaneous terminal-owner and immediate-reuse claims.

## Prior Fieldwork Context

The immediately preceding registry run used a working StealthCDP browser to
navigate Maryland and Florida public registries and to retry California
BizFile. Browser control, target enumeration, navigation, and JavaScript
execution remained functional. California returned Imperva Error 15, which was
a target-site rejection rather than evidence that browser control had failed.

That run did not create another profile or terminate the browser. It therefore
does not establish that a durable profile is required for this workflow. It
does establish that the next access plan should make its profile posture and
relationship to retained runtime state deterministic.

## Expected Behavior

For a request with no explicit profile and no authentication requirement,
Agent Browser should choose one coherent posture:

1. Select a disposable one-time profile and state that no retained identity is
   expected or required.
2. Select the configured default profile according to an explicit service,
   account, task, or site binding.
3. Reuse an existing task-specific retained profile only when a current binding
   explains why that profile is authoritative for the request.

Whichever posture wins, the access plan should expose one executable next
action. It should not recommend immediate browser reuse while also classifying
the browser as a terminal-profile process requiring lifecycle-owner inspection.
Authentication freshness guidance should be suppressed or clearly advisory
when the target is explicitly unauthenticated.

## User Impact

The direct impact in this incident is moderate because the registry workflow
does not depend on cookies, saved credentials, or accumulated login state. A
throwaway profile would not lose valuable user data.

The control-plane impact is still material:

- An old experiment profile can silently become the authority for unrelated
  work.
- A caller cannot tell whether `reuse_existing_browser` or
  `inspect_lifecycle_owner` is the actual prerequisite.
- Authentication freshness warnings add irrelevant work to an unauthenticated
  task.
- Reusing stale task-specific state can carry unrelated tabs, cookies, local
  storage, or challenge history into a new public-site investigation.
- Clients may create another profile or browser merely to escape ambiguous
  guidance, increasing lifecycle and cleanup pressure.

## Recommended Repair And Regression Coverage

1. Record and return the exact rule that selected a profile when the caller did
   not provide one. Distinguish service binding, account binding, site binding,
   task binding, global default, retained affinity, and disposable fallback.
2. Add an explicit unauthenticated or identity-retention posture to access-plan
   intent so the broker can choose a disposable profile without treating that
   choice as degraded behavior.
3. Prevent historical task-specific profiles from winning unrelated requests
   through implicit recency or retained-process affinity alone.
4. Reconcile lifecycle state before emitting `reuse_existing_browser`. A
   terminal browser that requires owner inspection must not simultaneously be
   the immediately executable reuse target.
5. Suppress `verify_or_seed_profile_before_authenticated_work` as the primary
   recommendation when the target does not require authentication. If target
   policy does not declare authentication posture, report that missing policy
   fact directly.
6. Add a fixture with no explicit profile, an unauthenticated target, a durable
   default profile, and an old task-specific retained browser. Require one
   deterministic selection reason and one non-conflicting next action.
7. Add a fixture for an explicitly disposable request and prove that repeated
   execution does not accumulate named managed profiles or select unrelated
   retained state.
8. Assert that browser-build validation can pass independently of profile
   compatibility, freshness, and lifecycle readiness, while the final action
   recommendation still fails closed on contradictory profile state.

## Stock Chrome CDP-Free Desktop Follow-Up

The operator requested a direct comparison using stock Chrome without a
DevTools endpoint and with all interaction performed through desktop services
and `xdotool`. A disposable profile was appropriate because the public
California registry requires no retained login data.

### Capability preflight gap

The read-only capability guide found both installed stock Chrome executables,
including `/opt/google/chrome/chrome`. The exact preflight request for
`browserBuild=stock_chrome`, headed mode, and CDP-free operation returned:

```text
applied: false
reason: cdp_free_launch_not_supported
wouldLaunch: false
```

The dedicated `cdp_free_headed` build label also could not launch through the
registry because it had no matching executable or preference binding. Agent
Browser can therefore model stock Chrome and CDP-free operation separately but
cannot currently route this exact combination through its registered launch
surface.

### Direct desktop experiment

Stock `/opt/google/chrome/chrome` was launched directly on available RDP display
`:10` with a fresh `/tmp` user-data directory. The launch command contained no
remote-debugging or automation flag, and the profile had no
`DevToolsActivePort` file. Navigation, pointer movement, clicks, typing,
scrolling, reload, and screenshots used X11 desktop operations only.

Observed sequence:

1. The California BizFile URL opened an Imperva additional-security-check page
   with an hCaptcha checkbox rather than the prior server-side Error 15.
2. The checkbox and two image-selection pages were completed with `xdotool`.
3. hCaptcha accepted the result and BizFile rendered its real Business Search
   page.
4. The first post-challenge application state displayed the search form but did
   not dispatch pointer or keyboard activation. One desktop reload restored a
   functioning submit handler.
5. Searches for `NOR CAL FRESH ROOF` and `NOR CAL` both reached the result
   handler but returned BizFile's generic `An error has occurred` view.
6. A known-entity control search for `APPLE INC` returned the same generic
   application error, proving the result was not merely an empty dealer-name
   match.

This experiment establishes that stock Chrome without CDP can pass the current
Imperva and hCaptcha perimeter from the same host. It does not establish a
usable registry search session because the application search backend failed
for both target and control queries after admission.

### Additional expected behavior

- The browser capability registry should be able to bind a reviewed stock
  Chrome executable to a CDP-free headed capability without relabeling it as a
  different browser family.
- A CDP-free launch should retain PID, profile, display, route, and lifecycle
  ownership while declaring DOM, CDP screenshot, and CDP input operations
  unavailable.
- The desktop input provider should offer the same bounded focus, key, pointer,
  screenshot, and visible-result observation operations used in this test.
- Access-plan output should distinguish perimeter admission from application
  success. Passing a challenge is not proof that the target workflow is usable.
- Add a provider-free fixture for stock Chrome plus CDP-free capability routing,
  followed by a live test that proves no DevTools endpoint exists while desktop
  input remains available.

## Scope Boundary

This note records field evidence and expected behavior. It does not authorize
profile deletion, process termination, Service State repair, browser restart,
production installation, or source changes. The existing browser and profile
records were left untouched.
