# Agent Browser

Agent Browser coordinates browser automation and operator-visible remote control while preserving one authoritative identity for each managed browser lane.

## Language

**Daemon command**:
A requested browser or control-plane operation executed in the serialized native runtime.
_Avoid_: Action handler, request handler

**Service State**:
The durable authority for managed profiles, browsers, tabs, routes, leases, jobs, and their lifecycle evidence.
_Avoid_: Dashboard state, runtime cache

**Remote-view intent**:
The normalized desired browser, target, route, viewing, and control posture for an operator-visible browser acquisition.
_Avoid_: Remote-view request, open parameters

**Route-bound handoff**:
An operator-visible browser acquisition whose route, display, browser, target, lease, and proof identities agree.
_Avoid_: Remote desktop link, route open

**Acquisition lease**:
The exclusive pending or finalized claim that prevents two acquisitions from owning the same route-bound browser lane.
_Avoid_: Lock, checkout record

**Retained browser**:
A browser that remains alive across daemon or route transitions and whose existing ownership must be respected during recovery.
_Avoid_: Orphan browser, stale browser

**Service tab handle**:
The stable service identity that binds a managed tab to its browser, session, and current target evidence.
_Avoid_: Target ID, tab ID

**Operator-visible proof**:
The combined evidence that the intended browser target is visible and reachable through the authoritative operator route.
_Avoid_: URL readiness, route health

**Durable handoff**:
An opaque public identity that can reacquire current route and browser evidence without exposing an ephemeral provider address.
_Avoid_: Guacamole URL, provider URL

**Provider fallback**:
A best-effort retained-route outcome that preserves an existing browser without claiming normal managed control or creating another ownership lane.
_Avoid_: Successful reopen, automatic recovery

**Forward deadline**:
The route-bound open deadline for new effects, computed from the existing total job timeout after reserving bounded time for compensation.
_Avoid_: Operation timeout, extended timeout

**Compensation reserve**:
The final portion of the existing total job timeout available only to undo effects that the coordinator recorded as completed. It does not extend the public deadline.
_Avoid_: Cleanup timeout, grace period

**Scripted runtime**:
A deterministic test implementation of the route-bound runtime seam that records invoked effects and advances a fake clock without browser or live runtime effects.
_Avoid_: Mock browser, integration runtime

**Coordinator-owned completion**:
The worker signals timeout or cancellation through the route-bound token and retains the coordinator future until bounded compensation reports a terminal state.
_Avoid_: Post-timeout join, background cleanup

**Rollback quarantine**:
A terminal fail-closed acquisition record used when compensation cannot confirm every owned external effect before the total deadline. It removes active checkout and blocks an equivalent acquisition until explicit recovery.
_Avoid_: Partial rollback, cleanup warning
