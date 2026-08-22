# Select and manage service profiles

Use this guide when browser work needs retained authentication or a stable
account identity.

## Select a profile

Call `service_access_plan` with caller attribution and target identity. The
access plan is the primary decision surface. Use profile lookup only when you
need to search or explain the catalog before forming the exact intent.

Read these fields together:

- `selectedProfile.id` identifies the chosen durable profile.
- `decision.profileReuse.defaultAcquisition` says whether to launch or reuse.
- `decision.profileReuse.sharedAcquisition` supplies `browserId` and
  `sessionName` hints for a retained browser.
- `decision.recommendedAction` identifies readiness work such as a bounded auth
  probe or manual seeding.
- `decision.attention.owner` identifies who owns the next action.

Do not select a fresh profile because another task is active. A shared profile
uses one retained browser process with attributed tabs. An exclusive profile
waits for its lease or returns a typed conflict.

## Reuse a retained browser

When `sharedAcquisition.mode` is `tab_new`, copy the returned top-level
`browserId` and `sessionName` into `service_request`. Request a new attributed
tab instead of launching another Chrome process on the same profile directory.

Release the service tab handle when your task finishes. Tab release preserves
the shared browser and other clients' tabs.

## Verify or seed authentication

Treat `verify_or_seed_profile_before_authenticated_work` as an authentication
freshness action, not a route-capacity or runtime-health failure.

1. Run the bounded post-seeding or freshness probe when the access plan offers
   one.
2. If the probe fails and the plan requires manual seeding, request the profile
   seeding handoff.
3. Let the operator authenticate in the detached browser.
4. After the browser closes, verify the same profile through the service
   control plane.

Do not use DevTools during Google or similar sensitive initial sign-in unless
the selected profile's seeding contract explicitly permits it.

## Bring your own profile

Use an explicit `runtimeProfile` only when you know the required managed
identity. Use a raw `profile` path only when bringing an external profile is
part of the request. Explicit profile selection overrides automatic selection
and makes the caller responsible for choosing the correct identity, not for
managing its process lifecycle.
