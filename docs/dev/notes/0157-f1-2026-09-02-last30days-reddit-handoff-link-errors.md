# Last30Days Reddit Durable-Handoff Link Errors

Date: 2026-09-02

Status: FIELD OBSERVATION RECORDED | PRODUCT REPAIR NOT AUTHORIZED

Related plans: P155 durable handoff resume intent; P157 profile permissions and
request provenance

Scope: Agent Browser durable handoff resolution, retained-target attachment,
workspace presentation recovery, and route/display identity

Authority: REDACTED FIELD EVIDENCE AND PROVIDER-FREE PRODUCT FOLLOW-UP ONLY

## Purpose

Record a sequence in which an existing authenticated social profile could open
Reddit on a healthy Guacamole/RDP desktop, yet three operator entry surfaces
gave mutually inconsistent results:

1. an opaque handoff opened a tile whose claimed service session had no HTTP
   route;
2. workspace presentation recovery reported that recovery capacity was not
   configured even though the route pool was healthy; and
3. a newly created handoff reported that its exact retained browser target was
   not attached while the same browser and route remained reachable through
   the workspace left rail.

This note contains no credentials, cookies, page bodies, raw provider URLs,
profile paths, capability bearer material, or private handoff identifiers. It
does not authorize profile mutation, browser termination, route takeover,
provider interaction, installation, or a production repair.

## Operator Goal

The consumer needed one visible Reddit browser on the existing social profile
to confirm or complete authentication before a bounded feed-scraper QA run.
The required result was an opaque service-owned handoff whose browser, route,
display, target, and public HTTP presentation all remained usable when the
operator opened it.

## Observed Failure Sequence

### A retained ready record did not prove an HTTP-backed handoff

The first browser was opened through detached runtime login on an existing RDP
display without DevTools. Runtime and service status agreed that the process
was alive, manual, remotely controllable, and associated with a Guacamole
route. A previously retained opaque handoff record also reported `ready`, and
an HTTP request to its public path returned 200.

Opening that path did not reach the browser. The dashboard created or selected
a claimed handoff session and returned:

```text
Claimed service session '<redacted>' has no HTTP route
```

Fresh Service State contained no matching durable handoff record for the
claimed session. The manual-browser projection exposed a provider route, but
the direct runtime launch had never established service-owned handoff or HTTP
route authority.

The defect is not simply a dead public endpoint. The endpoint, retained
handoff row, manual-browser projection, and actual dashboard resolution made
different claims about the same operator action.

### Workspace recovery lacked authority despite healthy provider capacity

Opening the browser from its dashboard tile then returned:

```text
operator_presentation_authority_unavailable:
presentation recovery capacity is not configured
```

At the same time, scoped remote-view doctor evidence reported three ready
Guacamole/RDP route-pool entries, public ingress HTTP 200, successful
Guacamole authentication, complete connection permissions, reachable RDP
backend TCP, and live X11 display sockets. Service Status projected
`presentationRecoveryCapacity` as null.

The route pool was available as provider configuration, but the workspace
recovery path had no capacity authority from which to reserve or repair a
presentation. Doctor readiness and effect-capable service capacity were not
the same thing, and the dashboard did not explain that distinction until the
effect failed.

### A stale display-allocation key contradicted its display value

A fresh service-owned `remote-view open` initially failed route selection with
`route_pool_target_mismatch`. The retained allocation whose identifier ended
in `:11` actually stored display name `:10` and an unrelated released owner.
Route A correctly targeted display `:11`, so the stale allocation key caused
an apparent route/display mismatch.

Supplying a new explicit display-allocation identifier with the verified
display name produced a blocker-free strict dry run. This shows that display
allocation identity was the broken seam, not route-pool readiness.

The first actual Route A launch then failed the stronger visible-window
ownership proof:

```text
operator_presentation_observation_failed:
No viewable X11 window belongs to the browser PID
```

Its receipt proved that the newly launched browser was closed and the display
and route lease were rolled back. A controlled Route B retry on another ready
display succeeded without replacing the profile.

### A newly ready handoff failed exact-target attachment

The Route B request returned all required success evidence:

- one healthy service-owned browser process;
- route and route-pool agreement;
- a unique display allocation on the verified display;
- process-bound, mapped, active, topmost, non-minimized visible-window proof;
- a selected Reddit target whose URL readback was ready;
- public operator access HTTP 200; and
- `operatorVisible.state=ready` plus a new opaque handoff in `ready` state.

Opening that new handoff returned:

```text
durable_handoff_target_unavailable:
the exact retained browser target is not attached
```

Fresh Service Status still reported all of the following at that moment:

- the browser process running and healthy;
- attachability `attached_ready`;
- the route checked out and ready;
- a live viewer lease created by the operator's failed open;
- the display content as `browser_window_visible`;
- the handoff's stored exact target as a valid tab handle on that browser; and
- multiple current Reddit targets, including the stored handoff target.

A direct command against the retained daemon session did not return a target
list. It failed earlier with:

```text
existing_session_profile_identity_inconsistent
```

The operator then opened the same browser successfully from the workspace left
rail. That proves browser presentation remained available while the opaque
handoff's target-resolution path rejected it.

## What The Evidence Proves

The failures occupy four separate authority seams:

1. **Public handoff routing:** an HTTP 200 handoff path and a retained `ready`
   row did not prove that the claimed session had an HTTP route.
2. **Presentation capacity:** provider doctor readiness did not populate the
   effect-capable capacity authority required by workspace recovery.
3. **Display identity:** a stable-looking allocation identifier disagreed with
   the allocation's stored display name and owner.
4. **Target attachment:** the durable resolver rejected an exact target that
   Service Status still exposed as valid, while route-level left-rail access
   remained functional.

The evidence does not prove Reddit authentication. Page visibility and a home
URL are only candidates for the consumer's later bounded authentication probe.

## Product Requirements Exposed By The Fieldwork

### Truthful handoff readiness

Before a handoff is returned or remains `ready`, the control plane must prove
that its browser, session, route, display allocation, HTTP route, and exact
target are resolvable through the same authority path the public handoff will
use. A public HTTP 200 check alone is insufficient.

If the target is not attached, the response should identify whether the cause
is target disappearance, daemon-session identity disagreement, invalid tab
handle ownership, or incomplete browser-route attachment. It must not leave a
retained `ready` row whose public open deterministically fails.

### One browser-first recovery contract

The workspace tile, left rail, durable handoff resolver, and
`service_remote_view_browser_reattach` should consume the same browser-first
attachment decision. A browser that is safely reachable from the left rail
should not require the client to reconstruct a different target, session, or
route tuple for the durable handoff.

This does not authorize an arbitrary-target fallback. If the exact target is
gone, recovery should either select a replacement under an explicit,
deterministic policy with a new receipt or fail before the handoff is shared.

### Capacity projection and recourse

When doctor reports ready provider routes but Service Status has no
presentation recovery capacity, access planning and the dashboard should show
that typed distinction before the operator clicks recovery. The response
should include one executable recourse that can populate or repair capacity
without closing a browser or taking over another viewer.

### Collision-resistant display allocations

Auto-generated display-allocation identifiers must not collide with retained
records whose stored display name or owner differs. Allocation lookup should
validate identifier, display name, current owner, boot epoch, and readiness as
one identity. A released historical row must not silently govern a fresh route
request because its identifier resembles a display name.

### Complete provenance

Every failed public handoff open should retain one causal chain containing the
public handoff id, resolver request, claimed session, browser, target, route,
display allocation, viewer lease, structured failure, effect state, and retry
disposition. Redacted user-facing output may omit private values, but the
service trace must preserve enough identity to explain why the left rail and
handoff resolver disagreed.

## Suggested Provider-Free Regression Matrix

1. A retained handoff row marked ready but lacking an HTTP route is rejected
   before publication and cannot return a user-facing handoff URL.
2. Provider routes are doctor-ready while presentation capacity is null; the
   access plan reports unavailable capacity and executable recourse before a
   workspace recovery effect.
3. A retained allocation id collides with a different stored display name;
   fresh acquisition ignores or quarantines the stale row instead of returning
   `route_pool_target_mismatch` for a valid route.
4. A handoff target remains in the browser's valid service tab handles while
   direct daemon attachment returns an identity inconsistency; the handoff is
   not reported ready until those authorities reconcile.
5. A handoff target truly disappears while the browser and RDP route survive;
   exact-target recovery follows the explicit replacement policy or returns a
   typed unavailable result without invalidating left-rail browser access.
6. Left-rail route access and public handoff resolution consume the same
   browser-first attachment fixture and cannot disagree about route usability.

## Current Operational State

At the final readback, the Route B browser remained healthy and reachable from
the workspace left rail. The opaque handoff remained unsuitable for operator
use because exact-target resolution failed. No Agent Browser source repair,
installation, profile replacement, route takeover, or provider mutation was
performed while producing this note.

The Last30Days consumer may continue only through its separately authorized
bounded Reddit authentication probe and scraper QA. That consumer result must
not be treated as validation that durable handoff links are repaired.

## Recommended Next Product Slice

Open one bounded Agent Browser repair plan that starts at the public handoff
resolver and traces the exact target through browser-first reattachment. Use
the six provider-free scenarios above as the acceptance matrix, then run one
isolated development-runtime Route B replay. Do not use the live social profile
as a product test fixture.
