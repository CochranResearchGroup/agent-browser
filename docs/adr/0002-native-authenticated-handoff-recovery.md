# Native authenticated handoff recovery

Status: accepted for source implementation; installed activation unqualified
Owning plan: [P237](../dev/plans/0237-2026-10-11-native-dashboard-handoffs.md)

## Context

A valid opaque handoff returns 410 when its target expires. The existing browser owner can recreate the target, but the public locator is read-only and the existing recovery HTTP entry requires Dashboard authentication. A synthetic same-URL control proves that owner recovery followed by exact-target native observation preparation repairs navigation.

## Decision

Keep the public opaque route read-only. When a valid retained link lacks its live target, redirect to the configured Remote View origin at `/handoff/agent-browser/<handoff-id>`. Remote View owns that native continuation. Existing ingress validation and fresh native authentication must succeed before recovery is requested. Admit document navigation only; reject prefetch and non-document access without recovery.

The native gateway makes one private POST to the configured numeric-loopback Agent Browser endpoint at `/api/native-remote-view/<handoff-id>/resume`. A dedicated private bearer file authenticates this narrow gateway-to-owner capability. Do not forward browser cookies or accept an endpoint, token, session name or target identity from the browser. Agent Browser checks loopback peer, exact capability and fixed request shape before invoking its existing retained-task recovery and exact-target observation preparation. Native operator identity is attributable context, not a Dashboard role or successful-use assertion.

The owner returns only the current native location. Remote View validates the destination against its configured HTTPS origin and canonical native slot/observation shape before redirecting. Link expiry, deletion, unavailable ownership and pending recovery remain explicit failures. No automatic retry, URL replacement, viewer grant or use acknowledgement is introduced. Browser recovery and physical focus stay with Agent Browser; authentication, native viewer and desktop custody stay with Remote View.

The integration is optional and fails closed when the endpoint or private capability is absent or invalid. Source changes do not configure an installed endpoint or activate ingress. Record precise configuration and user-facing documentation in the implementation slice.

## Alternatives and consequences

Recovery inside the public GET would allow unauthenticated reads to launch or focus a browser. Dashboard login would introduce another authentication flow. A second viewer or browser-supplied resume endpoint would blur ownership or permit untrusted destination selection. These alternatives are rejected.

The continuation adds one native route and one scoped private callback. Tests must prove no callback before authentication or for prefetch, invalid-capability rejection, no arbitrary redirect, preserved link validity, exact-target recovery and both peer-preserving and last-browser workflows. The existing direct native route remains sufficient for a live qualified target. The opaque URL remains the operator bookmark.
