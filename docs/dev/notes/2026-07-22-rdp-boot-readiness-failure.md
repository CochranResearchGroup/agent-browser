# RDP boot readiness failure

Date: 2026-07-22
Status: Mitigated; runtime convergence and diagnostic follow-up remain open

## Summary

After a workstation reboot, the installed agent-browser runtime and dashboard
were healthy, but neither configured Guacamole RDP route had a live X11
display. Retained route state still described Route A as checked out and Route
B as available or ready. A live `remote-view open` then failed because Route
B's configured display, `:11`, had no X11 socket.

Agent-browser is not ready for remote operator use on boot while its control
plane can retain route readiness that the live display plane does not satisfy.
Boot convergence must either restore the route desktop sessions or mark every
route unavailable until current display and browser-window checks pass.

## Expected boot contract

Once the user-scoped agent-browser services start, an agent should be able to
open an operator-visible browser without requiring a separate RDP login to
create the route display first.

If agent-browser intentionally requires an operator to establish each RDP
desktop session, it must report that prerequisite directly. It must not expose
retained `ready` state or recommend a route whose display socket is absent.

## Observed state

The standalone install doctor passed after the reboot:

```text
success=true
version=0.27.0
issues=[]
runtime_status=converged
dashboard_state=ready
stealth_ready=true
service_ready=true
```

The dashboard runtime manifest returned HTTP 200 with contract
`service-ui-runtime.v1`, and `agent-browser-dashboard.service` was enabled and
active with zero restarts. The Guacamole, guacd, and PostgreSQL containers were
also running.

The remote-view doctor did not find usable route displays:

```text
status=blocked
route_pool_ready=false
route_displays_ready=false
display_access_count=0
```

The route users, helper, group membership, sudoers policy, browser binaries,
and viewer prerequisites were present. The missing component was the live
route display layer.

## Retained state contradicted live state

Route A retained this binding:

```text
routePoolEntryId=guacamole-rdp-a
routeId=guacamole:4
targetDisplay=:10
state=checked_out
remoteViewRouteState=orphaned
remoteViewRouteReason=display_allocation_unavailable
```

Route B retained this binding:

```text
routePoolEntryId=guacamole-rdp-b
routeId=guacamole:5
targetDisplay=:11
state=available
retainedReadiness=ready
```

A live open against Route B disproved that retained readiness:

```text
route_display_unavailable: route pool entry 'guacamole-rdp-b' target display
':11' has no local filesystem or abstract X11 socket
```

The same condition affected Route A. Both routes had retained component
evidence from an earlier runtime, but neither had a post-reboot display socket.

## Prior validation boundary

The [last30days runtime profile routing note](2026-07-06-last30days-profile-routing-failure.md)
records a passing live `remote-view open` fixture with Route A on `:10` and
Route B on `:11`. That proof established correct route-bound handoff behavior
while the displays existed. It did not establish that agent-browser recreates
or invalidates those displays and route records after a host reboot.

The current failure does not overturn that route-binding result. It exposes a
missing boot and reconciliation guarantee around the validated path.

## Downstream impact

Authenticated browser workflows that depend on the RDP gateway could not
start:

- Facebook was pinned to Route A and failed because the retained route was
  orphaned and unavailable.
- YouTube subscription discovery could not acquire an authenticated remote
  browser and returned `browser_subscription_discovery_failed`.
- An X smoke test using a local retained browser profile succeeded, which
  confirms that the browser binary and saved authentication profile survived
  the reboot. The failure was in the remote display path, not the target-site
  credentials.

The Facebook profile also exposed a separate retained-session mismatch, but
aligning the requested session with the retained owner did not repair the RDP
failure. Route A remained stale, and Route B still lacked a live display.

## Diagnostic defect

`agent-browser doctor remote-view --json` reported
`install_doctor_not_ready` because its embedded install-doctor helper timed
out. Standalone `agent-browser install doctor --json` passed immediately
before and after that result.

The remote-view doctor should distinguish a child-command timeout from proven
install drift. In this incident, the actionable failure was missing route
displays, not an unhealthy installed binary or dashboard.

## Likely failure boundary

The evidence supports this failure sequence:

1. The reboot removed the X11 sockets and desktop sessions associated with
   displays `:10` and `:11`.
2. The dashboard and Guacamole containers started without restoring those
   route desktop sessions.
3. Persisted route-pool and component-readiness records survived the reboot.
4. Route selection consulted retained readiness before proving that the live
   display socket and browser window existed.
5. The final launch failed only after selecting the apparently available
   route.

This sequence is an inference from the current service, route, and socket
evidence. The implementation still needs tracing before assigning the defect
to a specific startup unit or reconciliation function.

## Required product behavior

Agent-browser must satisfy one of these boot behaviors:

1. Start and verify each configured RDP desktop session before advertising its
   route as available.
2. Mark every route blocked after boot until an operator establishes the RDP
   session and agent-browser verifies the live display.

In either model:

- Route readiness must require a current local or abstract X11 socket.
- Retained `ready` evidence must be invalidated when the display disappears.
- Route selection must run live display preflight before checkout.
- `remote-view open` must never return a ready operator handoff without current
  browser-window and route-display proof.
- Doctor output must report missing RDP sessions as the primary remediation
  when the install runtime itself is healthy.
- A helper timeout must remain distinct from a failed helper result.

## Regression coverage

Add a reboot-equivalent fixture that preserves route records while removing
the display sockets. The fixture should prove that:

- Route A cannot remain checked out and ready when `:10` is absent.
- Route B cannot remain available and ready when `:11` is absent.
- Route selection rejects both entries before launching a browser.
- The diagnostic identifies missing route displays instead of install drift.
- Restoring a route session updates readiness and allows one operator-visible
  open without manual state cleanup.

Add a host boot smoke that verifies this sequence:

1. The dashboard runtime manifest is readable.
2. Every configured route has a live X11 display or an explicit blocked state.
3. The route-pool state agrees with the live display inspection.
4. One route-bound browser opens to a harmless page.
5. `operatorVisible` proves the selected route, display, browser window, and
   tab are aligned.

## Safe reproduction

These checks do not require target-site credentials:

```bash
agent-browser install doctor --json
agent-browser doctor remote-view --json
agent-browser service status --json

agent-browser --json remote-view open https://example.com/ \
  --runtime-profile stealthcdp-default \
  --session rdp-boot-smoke \
  --browser-build stealthcdp_chromium \
  --view-stream-provider rdp_gateway \
  --route-pool-entry-id guacamole-rdp-b
```

The final command must fail closed if Route B's display socket or visible
browser proof is absent. A retained `ready` record is not sufficient evidence.

## Recommended next slice

Trace boot-time route reconciliation and the readiness inputs used by route
selection. Implement stale-readiness invalidation first, then decide whether
the supported boot contract should start route desktop sessions automatically
or expose an explicit operator prerequisite. Validate the result with the
reboot-equivalent fixture and the host boot smoke above.

## Implementation update

The first mitigation slice is implemented on 2026-07-22. Agent-browser now
uses the explicit operator-prerequisite boot contract:

- Service reconciliation checks every retained RDP route-pool entry against
  the current filesystem or abstract X11 socket.
- A missing display changes the entry to `unavailable`, preserves its prior
  state and readiness as diagnostic evidence, and orphans a linked active
  route with reason `route_display_socket_missing`.
- Route planning rejects the invalidated entry with
  `route_pool_entry_unavailable` before browser launch.
- When the display socket returns, reconciliation changes the entry to
  `available`, clears stale checkout ownership, and restores the retained
  readiness payload for normal acquisition and final proof.
- Agent-browser does not create an XRDP login session automatically. An
  operator must establish the route desktop before reconciliation can restore
  the route.

The reconciliation response contract and generated service client now expose
`unavailableRoutePoolEntries` and `restoredRoutePoolEntries` in
`remoteViewRepair`.

The deterministic reboot-equivalent regression preserves Route A as checked
out and Route B as available, removes both display sockets through an injected
probe, verifies both entries become unavailable, verifies both explicit opens
fail during planning, then restores the probe and verifies both entries become
available with stale checkout ownership cleared.

Local validation passed for the service-health, remote-view, service-model,
service-reconcile, route-confusion, API/MCP parity, generated-client,
TypeScript, Rust formatting, Rust clippy, and docs build surfaces. A current
binary also reconciled a copy of the retained workstation state with
`unavailableRoutePoolEntries=2`; both explicit Route A and Route B dry-run
opens failed before launch. The same no-launch reconciliation was then applied
to the live retained state, where both entries now report `unavailable` and
`route_display_socket_missing`, and Route A reports an orphaned
`route_display` state.

The repo-native full Rust runner completed its parallel-safe partition and
several serial environment-mutating partitions, then stopped making progress
in `native::parity_tests` and was terminated after the focused changed-module
partitions had passed. That runner-level contention remains a validation
limitation; it is not recorded as a full-suite pass.

The standalone and ignored workspace binaries were updated. The dashboard and
four unrelated active daemon sessions still hold the previous executable. They
were not interrupted because two retained browsers remain active. The normal
local convergence command is currently blocked by the pre-existing pnpm
ignored-build configuration, so installed-runtime convergence remains a
separate closeout step.

The remote-view doctor's child-command timeout classification is unchanged in
this slice and remains an open diagnostic defect.
