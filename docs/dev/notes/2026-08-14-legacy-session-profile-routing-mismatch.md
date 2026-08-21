# Legacy Session Profile Routing Mismatch

Date: 2026-08-14

Status: installed-runtime defect observed; temporary serialized operating rule
accepted by the operator

Related plan:
`docs/dev/plans/0111-2026-08-13-multi-agent-shared-browser-profile-authority-plan.md`

## Summary

An installed agent-browser 0.28.0 service could see one healthy retained
browser with an authenticated application tab and the expected named profile
ID, but it could not acquire a separate service-owned tab for another client.
The browser record and active session agreed on the profile ID. The legacy
session omitted the runtime-profile name and concrete profile value that the
new request consistency guard expected.

This is current live evidence for the legacy migration and route-hint cases in
Plan 0111. The fail-closed outcome prevented a duplicate browser, but it also
made the advertised `shared_browser_tabs` acquisition unusable for the retained
owner.

No private profile path, site payload, account value, or page content is
retained in this note.

## Reproduction Shape

1. A retained browser is healthy, has a reachable CDP endpoint, and reports a
   named profile ID.
2. Its active session reports the same profile ID but has no runtime-profile or
   concrete profile value because it predates the current request contract.
3. An access plan constrained to the browser's current host, stream, input, and
   display posture returns `reuse_existing_browser`,
   `clientSharingPolicy=shared_browser_tabs`, and
   `sharedAcquisition.mode=tab_new` with exact browser and session route hints.
4. Submitting the copied request fails before tab creation because its explicit
   runtime-profile and profile values do not match the absent legacy session
   values.
5. Submitting a minimal route-only blank-tab request with the exact browser and
   session IDs does not bind the retained owner first. Generic profile
   selection chooses the default profile and then fails against that profile's
   unrelated active lease.

Both requests stopped before browser or tab effects. No second root browser
was launched.

## Temporary Operating Rule

Until Plan 0111 updates the installed acquisition path, use this named
authenticated browser with one agent at a time. The active agent may operate
the already open, explicitly identified tab through the existing daemon
session after confirming the target URL and title. It must not launch another
browser, create a second profile lane, edit retained session records, or touch
another tab concurrently.

This is an operator-approved temporary workflow, not a claim that direct
same-tab multi-agent use is safe. A handoff must leave the browser and tab in a
known state before the next agent begins.

## Product Implication

The shared acquisition coordinator described by Plan 0111 should resolve and
validate the current browser owner before running generic profile selection.
Legacy session reconciliation needs enough canonical profile evidence to bind
the existing owner without requiring fields that were never recorded. Route
hints remain locators rather than authority, but an exact current-owner match
must not fall through to an unrelated default profile.

Add a regression fixture with this shape:

1. browser profile ID is present;
2. session profile ID agrees;
3. legacy runtime-profile and profile values are absent;
4. access plan recommends retained-owner tab acquisition;
5. copied `tab_new` creates one independently attributed tab in that owner;
6. releasing the new tab preserves the original tab and browser; and
7. process readback proves no second root browser was started.

## Evidence Boundary

- Installed runtime observed: `agent-browser 0.28.0`.
- Source checkout observed: branch `architecture-deepening-20260809`, commit
  `67d8dc66ccc26e207dd4be220133ab4f25ff67bd`.
- The installed runtime and source checkout identifiers are recorded
  separately. This note does not assert byte parity between them.
- Graphiti group `agent_browser_main` returned related shared-profile routing
  context but no prior fact for this exact legacy-session mismatch. Current
  plan and live readback therefore remain the authority for this note.
