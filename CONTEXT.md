# Agent Browser

Agent Browser coordinates browser automation and operator-visible remote control while separating access authority, work coordination, and exact runtime lifecycle ownership.

## Language

**Client subject**:
The stable identity to which access policy grants are assigned, together with an explicit assurance level such as locally self-declared or authenticated.
_Avoid_: Runtime owner, daemon identity, session name

**Connection instance**:
The service-generated identity for one live attachment of a Client Subject, used for command attribution and temporary coordination without becoming durable access authority.
_Avoid_: Client ID, principal, browser session

**Access policy**:
The revisioned rules that authorize a Client Subject to observe, use, coordinate, administer, or reclaim a browser resource.
_Avoid_: Lease, runtime ownership proof, profile identity

**Shared-local profile**:
A profile whose default policy allows locally trusted Client Subjects to reuse one managed browser and receive attributable tabs without first enrolling a strict identity.
_Avoid_: Unowned profile, public profile, exclusive profile

**Policy revision**:
The monotonic version of one Access Policy against which an admission, denial, or policy edit is evaluated.
_Avoid_: Owner generation, lease generation

**Profile occupancy**:
The current set of client work, tabs, controller leases, and viewer leases that must be considered before a profile can become more restrictive.
_Avoid_: Profile ownership, process ownership

**Drain-and-restrict transition**:
A revision-fenced change that stops new admission, clears incompatible Profile Occupancy, and commits a narrower Access Policy only after the required occupancy reaches zero.
_Avoid_: Immediate revocation, force unlock

**Eviction authority**:
The explicit permission to end another Client Subject's current Profile Occupancy during a Drain-and-restrict Transition.
_Avoid_: Policy edit, lease release, process kill

**Runtime ownership proof**:
Internal evidence that binds an exact managed browser or runtime process to its profile, executable, process instance, endpoint, and owner generation before lifecycle effects occur.
_Avoid_: Client identity, access permission, profile lease

**Request provenance**:
The immutable causal identity carried from ingress through admission, execution, terminal response, job, event, trace, and incident records.
_Avoid_: Log message, caller label, lane selector

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

**Profile acquisition intent**:
A request by one authenticated service principal to obtain or reuse one managed browser lane for an exact profile and task without choosing internal browser, session, route, or lifecycle identities.
_Avoid_: Launch request, session request

**Dominant blocker**:
The single current fact that determines why a profile acquisition cannot proceed, while all other relevant inconsistencies remain available as supporting evidence.
_Avoid_: Error list, first validation failure

**Recovery plan**:
A sealed, expiring, state-revision-bound proposal for the exact transitions needed to make one inconsistent profile acquisition safe to retry.
_Avoid_: Force unlock, repair script

**Recovery receipt**:
The durable result of checking and applying one Recovery Plan, including its preconditions, effects, compensation, terminal state, and acquisition retry outcome.
_Avoid_: Command log, success message

**Mitigation action**:
One bounded, idempotent, authority-preserving transition owned by the recovery plane and guarded by current evidence.
_Avoid_: Force operation, manual state edit

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

**Desktop evidence episode**:
A bounded observation or interaction transaction that owns evidence selection, presentation admission, scene proof, capture, verification, restoration, and release for one browser workspace.
_Avoid_: Desktop screenshot request, capture mode

**Presentation slot**:
Scarce operator-visible desktop capacity binding one route, display, current scene, human posture, readiness generation, and cleanup obligation without owning the retained browser lifecycle.
_Avoid_: Route, virtual desktop

**Warm presentation pool**:
The ready minimum of presentation slots retained for low-latency human, recovery, and desktop-evidence admission while elastic slots may be added or reclaimed within pressure limits.
_Avoid_: Two-route pool, fixed desktops

**Capture-ready proof**:
Fresh evidence that the exact authorized browser scene is staged, topmost, maximized or at approved geometry, unoccluded, and still bound to its route, display, process, viewer, controller, and geometry identities.
_Avoid_: Browser visible, route ready

**Runtime environment**:
An isolated installation authority that owns one executable selector, generation store, runtime state root, socket namespace, supervisor identities, dashboard and authentication state, port allocation, provider namespace, garbage-collection scope, and acceptance receipts.
_Avoid_: Install mode, alternate port

**Development runtime**:
A non-production Runtime Environment for experimental builds whose installation, execution, validation, and reclamation cannot select, restart, mutate, or claim acceptance for the production Runtime Environment.
_Avoid_: Debug binary, staging flag

**Build admission**:
A bounded claim on current host compilation capacity derived from memory, swap, CPU, disk, active build claims, and configured reserves.
_Avoid_: Cargo lock, unlimited parallel build

**Environment receipt**:
Evidence that binds an installation, process, dashboard, ingress, validation, or cleanup result to exactly one Runtime Environment identity.
_Avoid_: Install log, shared status

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

**Concrete owner module**:
A cohesive native module that owns a domain invariant and the operations that preserve it. Command dispatch and peer workflows import this owner directly.
_Avoid_: Action bucket, handler collection

**Runtime host**:
The singular active authority that executes daemon commands for all runtime lanes in one user installation.
_Avoid_: Per-session daemon, session process

**Runtime lane**:
A logical serialized command and ownership scope for one named browser session within the runtime host.
_Avoid_: Daemon instance, socket process

**Cleanup obligation**:
The durable duty to preserve a valid retained resource or reclaim it after its lifecycle becomes terminal.
_Avoid_: Best-effort cleanup, drop behavior

**Reclaimable runtime resource**:
A package-owned process tree, profile, display helper, transaction payload, or runtime generation whose lifecycle is terminal and which has no active ownership, lease, handoff, rollback, or process reference.
_Avoid_: Old resource, orphan

**Runtime convergence window**:
The bounded upgrade interval during which the old and candidate runtime generations may coexist while ownership transfers and rollback remains possible.
_Avoid_: Mixed steady state, maintenance mode

**Transitional facade**:
A temporary re-export-only module used while callers migrate to concrete owners. It is not an architectural owner and must be deleted before the final architecture gate.
_Avoid_: Compatibility owner, permanent facade

**Cohesive green checkpoint**:
A durable commit that binds one interdependent remediation invariant to its focused validation and rollback boundary. Plan 0106 uses these checkpoints instead of reconstructing retroactive one-commit-per-responsibility history; the 615-record responsibility and packet ledger remains intact.
_Avoid_: Retroactive packet commit, grouped unvalidated change
