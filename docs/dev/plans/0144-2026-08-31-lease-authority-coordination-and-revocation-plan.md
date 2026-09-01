# Plan 0144 | Lease Authority Coordination And Revocation

Date: 2026-08-31

State: OPEN

Execution state: `slice_f_protected_effect_intent_committed_sink_consume_in_progress`

Lane: P144

Source baseline: `c21118a30b01eaf23acabdec80e81f5d79a130b3`

Branch: `plan/lease-authority-coordination`

Target: `main`

Integration model: cohesive validated checkpoints on a short-lived topic
branch, followed by a merge to `main`.

Authority: PLAN, SOURCE, DOCUMENTATION, PROVIDER-FREE FIXTURES, ISOLATED
DEVELOPMENT-RUNTIME VALIDATION, EXACT PRODUCTION-CANDIDATE INSTALLATION, AND
POST-INSTALL PROVIDER-FREE LIVE SMOKE ARE IN SCOPE. Tenant navigation,
provider mutation, broad process cleanup, manual database editing, and
unreviewed lease revocation are out of scope.

Depends on:

- Plan 0134 principal, profile, crash-epoch, and lifecycle coherence;
- Plan 0137 acquisition and recovery contracts;
- Plan 0142 Service State concurrency and client recourse reliability; and
- Plan 0143 workstation upgrade self-admission repair.

## Executive Decision

Agent Browser will replace inferred lease authority with one canonical lease
authority kernel used by every planner, admission gate, runtime owner,
session, tab, presentation route, installer, recovery action, doctor, and
client projection.

The kernel separates two different things that the current model conflates:

1. an active claim, which is current fenced mutation authority; and
2. a lease event or terminal record, which is append-only operational history.

History can explain what happened. It cannot reserve a profile, block a
client, create an owner, extend a deadline, or prove that a process exists.
Only a current active claim returned by an atomic acquisition can do that.

Ordinary ephemeral clients receive short, renewable claims whose authority
expires without a clean shutdown. Lease-aware software may request strict
claims, but must provide a recovery policy and use first-class renew, release,
revoke, and recovery operations. No strict claim may require state-file
editing or a product hotfix to recover.

## Goal

Make the reported and enforced lease state truthful under ephemeral agents,
sleeping workers, process crashes, daemon replacement, runtime transfer,
mixed-version installation, and retained history.

The design must make these defect classes unrepresentable:

- a historical record becoming an operational blocker;
- a dead or absent session being treated as a current holder;
- an access plan being executable while admission rejects the same evidence;
- two Agent Browser components independently deciding who owns a profile;
- a stale holder continuing to mutate after expiry, transfer, or revocation;
- an unbounded transition such as `closing`, `prepared`, or `transferring`;
- an abandoned strict claim requiring manual state editing;
- a read path creating or reattributing durable state;
- a successful effect being retried because its receipt was not durably
  observed; and
- a logical lease being used as proof of a physical browser process, route,
  display, or daemon collision.

The system cannot make process crashes, storage failures, or operator mistakes
impossible. It must make them converge to bounded, inspectable, revocable
states without inventing authority or indefinitely blocking ordinary work.

## Audited Failure Inventory

The hotfix history and Plans 0128, 0134, 0137, 0142, and 0143 show recurring
failures in seven families:

1. **History promoted to authority.** Retained sessions and legacy principal
   evidence produce lease warnings or blockers after work is terminal.
2. **Identity-axis collapse.** Service principal, client session, daemon
   route, durable browser, profile, process, owner generation, and
   presentation route are treated as interchangeable strings.
3. **Split planning and enforcement.** The access planner reports an
   executable launch while the daemon later rejects it with different or
   newer inferred identity evidence.
4. **Unbounded lifecycle transitions.** `closing`, `prepared`, and
   `transferring` survive the process that could finish or compensate them.
5. **Projection drift.** Read paths and dashboard projections create
   ready-looking browsers, owners, or warnings from stale and processless
   evidence.
6. **Effect and installation ambiguity.** A mutation may succeed before its
   caller observes a receipt, or a candidate runtime may inherit only part of
   the authority model and block its own transaction.
7. **Deadline and outcome erasure.** A broker wait can consume the caller's
   entire transport or subprocess budget, causing a typed conflict, queue, or
   recovery outcome to be replaced by a generic client timeout and unsafe
   retry ambiguity.

No individual hotfix closes these families. This plan closes them through one
shared authority model and public recovery plane.

## Ubiquitous Language

| Term | Meaning |
| --- | --- |
| active claim | the only record that can authorize or block a mutation |
| lease event | append-only evidence of acquisition, renewal, transfer, expiry, release, revocation, or recovery |
| fencing token | monotonically increasing token required by every effect under a claim |
| ephemeral claim | short renewable authority designed for clients that may disappear without cleanup |
| strict claim | explicitly requested authority with declared recovery ownership and bounded revocation policy |
| transition deadline | mandatory deadline for a nonterminal lifecycle transition |
| authority snapshot | revision-bound result consumed by planning, acquisition, admission, and effects |
| physical collision | current process, socket, lock, display, or route evidence independent of logical lease authority |
| recovery controller | the one component authorized to finish, compensate, or revoke a strict claim |

## Canonical State Model

The lease authority kernel owns these durable collections:

```text
activeClaims[resourceKey] -> ActiveClaim
claimRevisions[resourceKey] -> monotonic revision
leaseEvents[] -> append-only LeaseEvent
effectReceipts[idempotencyKey] -> EffectReceipt
cleanupObligations[id] -> CleanupObligation
```

`activeClaims` is the sole operational index. Terminal rows never remain in
that index. A historical query joins `leaseEvents`, terminal receipts, and
cleanup obligations without changing current authority.

Every active claim contains at least:

- typed resource key and optional parent claim id;
- stable principal id and capability provenance;
- claim mode, state, revision, and fencing token;
- acquisition, heartbeat, expiry, and transition deadline timestamps;
- boot epoch and current process evidence when process ownership matters;
- runtime owner generation when lifecycle ownership matters;
- strict recovery controller and revocation policy when mode is strict; and
- subordinate claim ids or a derivable child index.

## Frozen Invariants

### Truth and safety

1. Only `activeClaims` may authorize or block an operation.
2. Lease events, terminal sessions, terminal owners, old generations, and
   doctor findings are never consulted as operational authority.
3. Every effect validates the resource key, principal, claim revision,
   fencing token, expiry, parent authority, and relevant owner generation in
   one authority snapshot.
4. Expiry, release, revocation, transfer commit, and recovery monotonically
   advance the fencing token before later effects can commit.
5. A stale holder cannot mutate even if its process is still running.
6. A read path cannot acquire, renew, transfer, revoke, release, recover,
   synthesize, or reattribute authority.

### Liveness

7. Ephemeral claims have finite expiry and require no clean release.
8. Every nonterminal transition has a deadline and one named reconciler.
9. A strict claim declares its recovery controller and revocation policy at
   acquisition. Missing recovery metadata makes strict acquisition invalid.
10. Every abandoned strict claim has a supported inspect, revoke-plan,
    revoke-apply, and recovery path that does not edit raw state.
11. A terminal transition with an unsatisfied physical cleanup obligation may
    block only the exact unsafe physical resource, not unrelated profile work.

### Consistency

12. Planning and execution consume the same authority snapshot and evaluator.
13. A plan is advisory. Authority is granted only by atomic compare-and-swap
    acquisition against the current claim revision.
14. Exactly one module owns active-claim mutation. Other components call it
    and never project their own operational lease state.
15. Hierarchical child claims cannot outlive or exceed their parent claim.
16. Runtime-owner transfer is a receipted saga coordinated by the kernel, not
    an independent lease implementation.
17. Logical authority and physical collision evidence are separate axes and
    are reported separately.

### Recoverability and compatibility

18. Every mutation is idempotent by operation key and persists its effect
    receipt before reporting success.
19. Repeating an operation returns the same terminal receipt without duplicate
    effects.
20. Mixed-version runtimes fail before effect when they cannot understand the
    current claim schema or fencing token.
21. Migration preserves history while building a new active index only from
    evidence that is current, typed, nonterminal, and positively proven.
22. An ambiguous legacy record migrates to history or quarantine, never to an
    active blocker.
23. Administrative revocation is exact, revision-bound, audited, and narrower
    than generic force unlock.
24. Doctor health is based on current safety and recoverability. Historical
    warnings remain queryable but do not make the current system unhealthy.
25. Lease time is authority-owned. Callers request a policy class, not an
    arbitrary expiry, and wall-clock rollback cannot lengthen a claim.
26. Resource keys are canonicalized before admission. Aliases, profile
    shorthands, paths, and route projections cannot create two claims for one
    physical resource.
27. Revision and fencing counters fail before mutation on exhaustion. They
    never saturate, wrap, or reuse an earlier value.
28. Idempotency survives terminalization and history archival. Replaying an
    acquire, renew, release, recover, revoke, or effect operation returns its
    original receipt rather than creating new authority.
29. Multi-resource work is acquired as one ordered bundle or as a bounded,
    receipted saga. A partial profile, route, display, browser, or installer
    acquisition cannot become an indefinite blocker.
30. Parent authority is revalidated recursively at every child effect, not
    only when the child is created. Parent expiry or fencing invalidates the
    child immediately even if a stale child row remains.
31. History retention and compaction preserve fencing high-water marks and
    idempotency receipts. Unbounded history growth cannot exhaust the active
    authority store or change admission.
32. The active index, counters, and event ledger are private kernel state.
    Other subsystems receive immutable claims and invoke typed operations;
    they cannot directly replace or mutate authority collections.
33. A new operation from the same principal and capability may rejoin a
    current ephemeral claim without minting a fence or extending expiry.
    Strict claims require the explicit recovery path instead of implicit
    rejoin.
34. Capability revocation has one declared kernel behavior. It either fences
    every claim issued under that capability immediately or records the
    bounded delegation interval during which the claim remains valid. No
    effect path may improvise different revocation semantics.
35. Canonical authority reads and mutations are linearizable at the effect
    boundary. A cache, replica, daemon-local snapshot, or temporarily
    unavailable store cannot authorize an effect or manufacture a blocker.
36. After migration, an effect request that lacks the canonical claim
    envelope is rejected as an incompatible caller. It never falls back to a
    legacy session, owner, or profile gate. Compatibility adapters must obtain
    a canonical claim before invoking the effect.
37. Every denial names either the conflicting current claim or the exact
    positively observed physical collision, together with its expiry,
    deadline, or supported recovery action. An absent runtime owner, invented
    session identity, historical row, warning, or projection mismatch cannot
    be a blocker.
38. An executable access plan never returns an unproved identity as a launch
    prerequisite. It either atomically establishes the exact prerequisite or
    omits it and lets the acquisition mutation create the new identity.
39. External effects distinguish durable intent, confirmed completion, and
    uncertain completion. A receipt may suppress a retry only when its state
    proves the corresponding effect semantics; uncertainty receives bounded
    reconciliation rather than duplicate execution or an indefinite denial.
40. A strict recovery controller is itself replaceable. Administrative
    revision-bound recovery remains available when the named ephemeral
    controller disappears, without granting a wildcard force-unlock.
41. Planning, reviewed recovery, and denied acquisition create no active
    claim. A claim is created only for an already admissible reuse or at the
    atomic boundary immediately before an admitted effect. A diagnostic or
    recovery offer cannot become the blocker it is reporting.
42. With no current runtime owner, owner-linked lifecycle records are history.
    They cannot block cold acquisition or supply a replacement browser or
    session identity.
43. Physical evidence can block only while its observation epoch and freshness
    bound are current. Stale process, lock, socket, route, display, or provider
    observations become history or an explicit need-to-reobserve outcome, not
    an indefinite collision.
44. Claim mutation, counter advancement, the corresponding event, and the
    idempotency receipt commit in one storage transaction. Partial persistence
    cannot leave authority without its fence or receipt.
45. Renewable ephemeral claims have a maximum continuous tenure and bounded
    waiter policy. A live but defective client cannot renew forever or release
    and immediately reacquire in a loop that starves ordinary work.
46. Installation selection is owned by a stable supervisor outside the
    candidate being evaluated. A candidate cannot admit, drain, fence, or veto
    its own activation or rollback transaction.
47. Effectful entry points accept a kernel-issued authorized-effect type, and
    denial construction accepts only a current-claim conflict or fresh physical
    collision type. Raw JSON, strings, and compatibility projections cannot
    construct either authority outcome after migration.
48. A serialized effect envelope is not authorized by observable claim
    metadata alone. It is bound to current request authentication or carries an
    opaque or authenticated bearer proof whose secret is never exposed through
    status, history, diagnostics, logs, or generated projections. Copying or
    reconstructing public claim fields cannot impersonate the holder.
49. A row in `activeClaims` is operational only while its authority-owned time
    predicate is current. An expired row that has not yet been terminalized is
    ignored by planning and admission and cannot become a blocker merely
    because cleanup has not run.
50. Claim recovery mode and access compatibility are separate dimensions.
    `ephemeral` versus `strict` does not imply exclusive versus shared access.
    A stable resource supervisor owns the profile parent claim, while
    concurrent clients receive attributable child claims under an explicit
    compatibility policy. A transient client session is never the durable
    profile owner.
51. Every canonical resource belongs to exactly one authority domain and store
    epoch. Claims, envelopes, receipts, and physical-resource registrations bind
    that domain and epoch. Two daemons, homes, mounts, or restored state stores
    cannot independently authorize the same physical profile.
52. Backup restore, rollback, replication repair, and store replacement cannot
    lower an authority epoch, fencing high-water mark, capability revision, or
    completed-operation index. A restored older snapshot is read-only history
    until a stable supervisor establishes a strictly newer authority epoch.
53. Effect authorization is least-authority and request-bound. Its proof binds
    the action class, exact resource or child resource, audience runtime,
    operation idempotency key, issued-at bound, and expiry in addition to claim
    identity and fencing. The bearer has redacted debug output and is scrubbed
    from retained commands, traces, errors, incidents, and logs.
54. For irreversible or external effects, effect-time validation and durable
    intent creation are one authority transaction. Only the selected executor
    may consume that intent, and completion, uncertainty, compensation, and
    replay all resolve through the same operation receipt. A validate-then-act
    gap cannot let expiry or revocation race an unrecorded effect.
55. Waiter identity, enqueue order, priority, deadline, and terminal outcome are
    durable kernel state. Restarting a daemon or scheduler cannot forget a
    waiter, reset maximum tenure, or let a releasing holder jump the queue.
56. Physical process evidence includes boot epoch, process start identity, and
    canonical file or resource identity when applicable. PID, path text, lock
    file presence, or socket address alone is never sufficient because each can
    be reused or aliased.
57. Administrative recover and revoke operations use a distinct, scoped,
    authenticated authority that is not granted by the target claim and cannot
    be blocked by that claim. It is exact-resource, revision-bound, auditable,
    and incapable of wildcard mutation.
58. Claims bind the lease-policy revision that selected duration, renewal,
    compatibility, recovery, and priority semantics. Policy changes have an
    explicit grandfather, shorten, or revoke transition and cannot silently
    reinterpret an existing claim.
59. An unavailable, corrupt, or non-linearizable authority store returns a
    typed authority outage. It cannot be described as another holder, stale
    session, identity mismatch, or physical collision, and it cannot synthesize
    authority. Explicit isolated fallback remains a separate unauthenticated
    resource, never use of the requested profile.
60. Compatibility and emergency adapters run before canonical acquisition or
    not at all. Once a canonical envelope exists, no scheduler, fallback,
    profile selector, alias resolver, or retry helper may rewrite its resource,
    principal, audience, or operation. The effect boundary still verifies the
    envelope and does not trust adapter admission.
61. Deadline reconciliation is owned by a stable supervisor with a durable
    scan cursor and idempotent receipts, not by the ephemeral worker whose
    transition may be abandoned. Repeated supervisor crashes cannot reset or
    indefinitely defer an eligible recovery.
62. The repository maintains an exhaustive effect-entrypoint manifest. A
    presubmit architecture test proves every browser, profile, process, route,
    display, session, tab, installer, and control-input mutation enters through
    the sealed kernel authorization boundary or an explicitly non-lease
    physical-safety boundary.
63. An active claim proves a time-bounded authority reservation, not process,
    worker, session, or browser liveness. Public projections report its last
    heartbeat and expiry separately and never describe a holder as running from
    the claim alone. After an ephemeral holder disappears, the maximum logical
    blocking interval is the policy-bounded remaining tenure, never retained
    history or a renewed diagnostic observation.
64. Exact holder release and scoped administrative recover or revoke require
    the canonical claim plus their respective authenticated authority. They do
    not require a derived daemon session, browser, runtime owner, process, or
    presentation row to exist, so an absent projection cannot prevent removal
    of logical authority.
65. Observation and reconciliation are non-renewing. Reading a claim, finding
    a stale row, repeating a warning, scanning after restart, or failing a
    cleanup attempt cannot extend claim expiry, transition deadlines, maximum
    tenure, or waiter position.
66. Lease expiry uses a supervisor-owned monotonic time basis within one boot
    epoch and a durable nondecreasing authority-time floor across restart.
    Human-readable wall-clock timestamps are evidence only; clock rollback,
    suspend, restore, or namespace skew cannot lengthen operational authority.
67. Bearer signing authority is cryptographically separate from observable or
    persisted capability digests. Read access to claims, capability hashes,
    history, backups, diagnostics, or generated projections cannot mint an
    effect, release, recovery, or revocation authorization.
68. Terminal operation replay authenticates the exact original request or a
    scoped receipt-read authority, then consults the completed-operation index
    before requiring a controller that may since have rotated or disappeared.
    Controller lifecycle changes cannot turn a completed mutation into an
    unobservable result or recreate its effect.
69. Administrative authority has an explicit bootstrap, custody, rotation,
    revocation, loss-recovery, and audit lifecycle rooted outside the target
    claim and candidate runtime. Loss of one administrator cannot require raw
    authority-state editing, and no target holder can mint administrative
    authority.
70. A supervisor-owned parent claim is itself time-bounded, fenced, and
    administratively recoverable. An absent profile supervisor cannot become a
    permanent parent-level blocker, and a parent claim with no current child or
    physical obligation cannot deny compatible ordinary work.
71. Cleanup obligations have their own typed state, attempt budget, deadline,
    evidence freshness, reconciler, and exact escalation or quarantine action.
    Repeating a failed cleanup or observing its history cannot extend logical
    authority or create an indefinite unsupported physical blocker.
72. Authority-domain registration is enforced against canonical physical
    resource identity through an external exclusive guard or coordinator, not
    only a store-local epoch field. A second home, mount alias, restored store,
    or daemon cannot register the same physical profile concurrently.
73. Plan issuance evaluates claim currency at the authority-owned issuance
    time, never at a retained heartbeat or caller-provided observation time.
    Any helper capable of returning an executable plan uses the same current
    predicate and sealed operation inputs that apply will revalidate.
74. Effect, release, recovery, and revocation verifiers cannot mint the
    authorizations they verify. Signing authority remains inside the stable
    kernel or a dedicated signer; runtime executors receive only public
    verification material or call a verification service. A shared symmetric
    key readable by candidate daemons, workers, or effect executors is not an
    acceptable component boundary even when its file is user-private.
75. Low-level mutation sinks are sealed as well as public entry points. Browser
    process spawn, CDP mutation, profile write, route or display allocation,
    runtime-owner transition, installer selection, and control input require a
    kernel-issued typed intent or an explicitly typed physical-safety permit.
    A new caller cannot bypass authority by invoking an older raw mutation
    helper below the public manifest.
76. Process and daemon creation is a single-flight, capacity-admitted durable
    saga. Intent is committed before spawn, exact boot and process-start
    identity is attached after spawn, and every crash boundary converges to one
    live process or one bounded cleanup obligation. Repeated client retries,
    supervisor restarts, or lost responses cannot create a process storm.
77. Idempotency receipts are namespaced by authenticated authority domain,
    principal, action, canonical resource, and operation key. A colliding or
    guessed operation key cannot reveal another principal's receipt, suppress
    its work, or replay an authorization across domains.
78. Active claims, durable waiters, pending intents, and cleanup obligations
    have policy-owned cardinality and resource budgets. Admission failure is a
    typed capacity outcome and starts no daemon, browser, waiter worker, or
    renewal loop. An authenticated but defective client cannot recreate the
    historical daemon and process multiplication failure through unbounded
    distinct requests.
79. A successful authority mutation is acknowledged only after the authority
    store's declared crash-durability boundary is satisfied. Torn writes,
    directory-entry loss, unsupported filesystem locking, and integrity-check
    failure become typed authority outages; an in-memory or merely buffered
    success cannot be used to authorize an external effect.
80. Every gate that can prevent an ordinary operation, including readiness,
    health aggregation, installer admission, scheduler admission, routing, and
    compatibility checks, returns one canonical typed outcome: a current-claim
    conflict, a fresh exact physical collision, an authority outage, a bounded
    capacity outcome, or a non-authority product error. Generic unhealthy,
    unavailable, warning, or readiness booleans cannot promote history or an
    invented identity into an operational denial. The exhaustive architecture
    manifest covers denial gates as well as effect entry points.
81. Signing-key custody is separated from candidate runtimes by an enforced
    operating-system or process boundary. Source visibility, Rust privacy, and
    a mode `0600` file owned by the same user are not custody boundaries. A
    candidate daemon, worker, executor, dashboard, or compatibility process can
    neither read signer material nor invoke an unrestricted signing oracle.
82. Signing and administrative key rotation uses a monotonic key epoch and a
    bounded verifier keyring. In-flight authorizations remain verifiable only
    for their original bounded lifetime, completed operations replay from their
    receipt, revoked keys cannot mint or authorize new work, and restoring an
    older key file cannot lower the accepted key epoch.
83. The stable supervisor has its own installation, rollback, recovery, and
    upgrade protocol. Updating that supervisor is selected and committed by a
    still-trusted bootstrap coordinator or banked generation outside the
    candidate being evaluated. Calling a component stable does not permit it to
    approve its own replacement or make its loss an unrecoverable workstation
    gate.
84. Authority time explicitly accounts for suspend, reboot, and supervisor
    failover. A monotonic source that pauses during suspend is insufficient.
    Pre-boot ephemeral claims and transitions are fenced on restart unless a
    trusted suspend-aware deadline source and durable floor prove remaining
    tenure without lengthening it. Strict claims resume only through their
    declared bounded recovery policy.
85. The canonical state machine has an executable reference model used for
    property and fault-injection tests across acquisition, renewal, release,
    expiry, transfer, recovery, revocation, key rotation, store restart,
    replay, time discontinuity, mixed versions, and process-spawn crash points.
    Example regressions alone are not evidence that arbitrary event ordering
    preserves single authority, bounded liveness, and receipt uniqueness.
86. Canonical authority state has the same enforced custody boundary as signing
    authority. Candidate daemons, workers, dashboards, compatibility adapters,
    and effect executors cannot rewrite the active index, counters, authority
    epoch, principal capability registry, canonical resource registrations,
    owner-generation bindings, administrator registry, completed-operation
    index, or verifier trust configuration through same-user file access. They
    use authenticated, typed IPC to the stable authority service.
87. Verifier trust distribution and key rotation are one supervised generation
    transition. The new signer, verifier set, retirement or emergency-revocation
    cutoffs, and external epoch floor become current atomically through a
    durable selected-generation pointer. A crash cannot pair a new signer with
    an old keyring or make a candidate-supplied public key trusted. The keyring
    has a hard cardinality bound; rotation waits or prunes only after every
    proof under the oldest key is outside its acceptance window.
88. Authority disaster recovery does not depend solely on the online signer,
    administrator registry, or state store being repaired. An independently
    authenticated bootstrap can inspect, quarantine, and replace an
    unavailable or corrupt authority generation at a strictly newer external
    epoch while preserving fencing and completed-operation high-water marks.
    If those high-water marks cannot be proven, the physical resource remains
    quarantined rather than inventing a clean lease state. No recovery path
    requires raw state editing.
89. Every architectural use of bounded has a concrete policy maximum, a
    machine-readable deadline, and a tested terminal outcome. Ephemeral tenure,
    transition recovery, physical-evidence freshness, waiter lifetime, cleanup
    retries, key retirement, uncertainty reconciliation, and authority outage
    escalation cannot remain finite only in prose while being operationally
    unbounded.
90. Trust-generation selectors and manifests use one canonical, path-safe
    generation identifier derived from the exact key epoch and active verifier
    identity. Resolution is pinned beneath the authority trust root and rejects
    relative paths, aliases, symlinks, reparse points, and content whose
    identity does not reproduce the selected name. A selector cannot redirect
    authority loading or bind an epoch to a different key.
91. Staged, incomplete, retired, and orphaned trust generations have a bounded
    reconciliation and retention policy. They are never selected implicitly,
    never become a lease blocker, and never require raw filesystem editing.
    Reclamation preserves every still-accepted proof and rollback generation;
    disk or generation-capacity exhaustion is a typed authority-capacity
    outcome with first-class inspect and recovery actions.
92. Every supported authority filesystem proves equivalent atomic replace,
    directory durability, owner, ACL, link, and lock semantics. A platform or
    volume without those primitives is rejected before authority mutation or
    installation. A Unix-only `fsync`, permission check, or rename assumption
    cannot silently become the Windows or network-filesystem contract.
93. Verifier rollout is monotonic across long-lived effect executors. An
    executor that observes a proof from a newer selected epoch refreshes only
    from the protected trust source and never lowers its accepted external
    floor. Signer activation waits for every mandatory sink generation to be
    compatible, while an obsolete sink is removed from routing with a typed
    bounded transition rather than manufacturing a claim conflict or a global
    readiness denial.
94. Protected authority state is hostile input on every load, restore,
    migration, and generation switch. The kernel validates schemas, map keys,
    cross-collection references, one-to-one resource identities, revisions,
    fences, epochs, completed-operation namespaces, and trust bindings before
    serving any request. Invalid state is quarantined as a typed authority
    outage and is never normalized into an empty or apparently clean authority.
95. The authority endpoint has a stable protected identity. Requests are
    authenticated and replay-bound to the exact authority domain, epoch,
    operation, and payload; effect sinks authenticate the response or proof
    rather than trusting whoever answered a replaceable socket. Endpoint
    squatting, stale connections, and candidate impersonation can cause only a
    typed bounded outage, never an allow decision or invented lease conflict.
96. Canonical resource registration is one-to-one between a logical resource
    key and a protected physical identity. Registration and rebinding are
    bootstrap or administrator operations, not acquisition side effects. A
    rebind is an explicit atomic migration that advances the resource fence,
    reconciles active descendants and cleanup obligations, and cannot alias one
    physical profile, route, display, browser, or installer under two keys.
97. Non-authoritative history and diagnostic projections occupy a separate
    load and validation failure domain from active authority, fences, policy,
    principal grants, canonical resources, current owner bindings, and
    completed-operation high-water marks. Reading or authorizing current work
    never deserializes lease events, terminal lifecycle history, warnings, or
    compatibility projections. Corrupt or unavailable history degrades the
    audit surface only. A mutation whose required audit append cannot become
    durable fails before effect as a typed audit-durability outage, while
    existing current authority remains readable without interpreting history.
98. Every durable authority publication is a compare-and-swap against the
    exact previously selected content-bound generation, not merely a valid
    snapshot with a plausible epoch or collection revision. A stale writer,
    restored kernel, or delayed mutation cannot repoint the selector to an
    older or divergent but individually valid generation. The mismatch has
    zero selector effect and requires a fresh load before retry.
99. Operational read capability and mutation publication capability are
    distinct. A read may load current authority without parsing history, but
    that object cannot publish. A mutation load binds protected state to the
    exact selected generation and publishes the new minimum authority journal
    record with the state change. It does not deserialize or require the
    availability of prior narrative history. Prior history bytes are retained
    or referenced immutably, and archival history may degrade without blocking
    a new ordinary lease operation. Only failure to durably commit the current
    authority mutation, its fence, receipt, and minimum audit record can stop
    the mutation.
100. A service-identity proof is accepted only when its signed endpoint and
    executable identities equal evidence observed locally from the exact
    connected transport. On Linux, the client binds that evidence to kernel
    `SO_PEERCRED`, the root peer PID and UID, the socket device and inode,
    root and group ownership plus mode, the root-owned non-writable executable
    digest, and the root-only state root. A valid signature for a different or
    replaced endpoint is invalid for the current connection.
101. A signed authorization is not offline proof that a claim is still
    current. Every effect consumes a single-use kernel intent against the
    current claim, fence, capability revision, parent authority, and
    revocation state. Offline signature verification may authenticate the
    issuer and immutable request fields, but cannot bypass this current-state
    consume. Expiry or revocation that wins the serialization order fences the
    effect before its sink.
102. Raw profile and administrator capabilities have an explicit client
    custody and delegation contract. A mode `0600` file readable by every
    same-user candidate process is not sufficient when same-user candidates
    are outside the authority boundary. Capability use is bound to an enrolled
    workload or broker identity, is narrowly delegated by action and resource,
    and has supported rotation and loss recovery without exposing a wildcard
    signing or mutation oracle.
103. Canonical authority and user-scoped Service State are not a distributed
    dual-write transaction. Authority receipts are the source for idempotent,
    revision-checked projection into jobs, sessions, browsers, lifecycle rows,
    and dashboards. Projection failure cannot roll back authority, create a
    second claim, or turn missing projection state into a denial; replay from
    the receipt converges the projection. Every supported concurrent runtime
    host uses the same serialized Service State mutation protocol during the
    migration window.
104. Compatibility is defined per action and resource hierarchy, not inferred
    from profile-level exclusivity or a shared principal string. The policy
    matrix states which profile, browser, session, tab, viewer, controller,
    route, installer, and input operations may coexist. An ordinary client
    receives only the shortest-lived child authority needed for its operation;
    an idle parent supervisor is not a reason to deny compatible work.
105. A fresh physical collision denial has a supported bounded resolution,
    not only a better error label. Exact process adoption, wait, owner-assisted
    close, administrator-fenced termination, stale-lock cleanup, or quarantine
    is selected from boot, process-start, executable, resource, and lock-owner
    evidence. If none is safe, the product reports a physical-safety outage
    with a deadline and inspection locator rather than inventing a lease
    holder or promising automatic recovery.
106. The protected supervisor is singleton, least-privilege, and
    resource-bounded. Socket activation, peer authentication, frame size,
    request rate, in-flight work, secret lifetime, and parser failure are
    bounded before expensive work. A malformed or flooding group member cannot
    starve expiry, revocation, deadline reconciliation, or administrator
    recovery, and the root process never performs browser or provider effects.
107. External epoch and cross-host resource exclusion name a concrete trusted
    coordinator and failure contract before installation. A second host with
    a valid root service and a restored local store cannot authorize the same
    physical profile during partition. When exclusive registration cannot be
    proven current, the outcome is a physical-resource quarantine or explicit
    isolated profile, never two local authorities or an invented remote owner.
108. Authority failure is contained to the narrowest provable scope. A corrupt
    or invalid claim, registration, owner binding, completed-operation entry,
    or counter quarantines its exact canonical resource while unrelated valid
    resources remain inspectable and usable. Only damage to shared custody,
    trust selection, authority-domain identity, or an unprovable global
    high-water mark may create a domain-wide outage. Resource quarantine
    preserves the suspect bytes and fencing evidence and has first-class
    inspect and recovery operations; it never normalizes the resource to an
    apparently free state.
109. Ordinary Agent Browser operations use broker-managed ephemeral authority.
    Their public contract does not require a caller to supply or understand a
    lease id, session identity, daemon route, owner generation, heartbeat,
    renewal loop, or recovery controller. The broker derives the narrow child
    resource and action, acquires or rejoins a server-bounded ephemeral claim,
    consumes the effect intent, and performs best-effort release. Explicit
    strict lease APIs remain available only for lease-aware software with a
    durable enrolled recovery controller.
110. Every authority readback is revision-bound and freshness-explicit. It
    reports the authority domain and epoch, snapshot revision, authority-owned
    observation time, and a bounded validity or refresh requirement. A cached
    CLI, dashboard, HTTP, MCP, generated-client, or compatibility projection
    may describe what was observed, but cannot label itself current after its
    bound, authorize an effect, or feed a denial without a fresh kernel
    decision. Historical and stale projections remain visibly observational.
111. Every potentially waiting operation has one end-to-end deadline budget
    with a mandatory response reserve. The broker either completes before that
    reserve or returns a durable queue, deferral, or recovery receipt. An
    internal lease wait cannot run until the HTTP, MCP, CLI subprocess, or
    generated-client timeout, and an abandoned caller cannot leave an
    unbounded waiter behind.
112. A public access plan is either an explicitly observational snapshot or a
    kernel-reserved executable offer. An observational plan cannot call itself
    executable, available, or unblocked without publishing its revision and
    validity bound. A reserved offer is created by the same atomic acquisition
    used by execution and carries no inferred session, owner, or runtime
    identity. Execution may return a typed stale-offer or current-conflict
    outcome, but cannot reinterpret the plan through a second legacy gate.
113. CLI, HTTP, MCP, generated clients, skills, and consumer brokers preserve
    the canonical structured outcome and retry posture through cancellation,
    timeout, and process boundaries. A typed conflict, queued result,
    uncertainty receipt, or recovery offer cannot be collapsed into a generic
    timeout or automatically retried as though no effect occurred.
114. Authority and projection transactions have one enforced lock order and
    never hold an authority-store, Service State, resource-registry, or process
    mutation lock while waiting on an external effect, subprocess, browser,
    route provider, or another independently scheduled authority domain. Every
    lock wait has an authority-owned deadline and a typed zero-effect or
    effect-uncertain outcome. Cancellation and panic release in-process guards,
    and crash recovery relies on transactional or kernel-released custody
    rather than a retained logical holder row. A lock convoy, inversion, or
    abandoned lock waiter cannot masquerade as lease contention or make the
    whole workstation say no.
115. Every Agent Browser process tree is launched inside a supervisor-owned,
    generation-bound operating-system containment boundary with hard PID,
    memory, CPU, file-descriptor, and restart-rate limits appropriate to its
    role. The durable single-flight spawn saga and kernel claim prevent logical
    duplication; the operating-system boundary limits damage if a browser,
    daemon, helper, or buggy executor forks outside the expected child count or
    ignores cooperative shutdown. Containment exhaustion produces one typed
    exact-resource capacity or physical-safety outcome, fences further spawn,
    and preserves administrator and lease-recovery capacity. It cannot create
    an unbounded daemon tree or consume the reserves needed to revoke, inspect,
    reconcile, or roll back.
116. Effect-channel custody is fenced as strongly as logical authority. A stale
    daemon or executor cannot retain or reacquire a raw CDP, route, display,
    input, process-signal, or profile-write channel after its owner generation
    is superseded. The selected architecture either mediates every such effect
    through a generation-checking broker or uses an operating-system boundary
    that makes the channel inaccessible to nonowners. Closing a logical claim
    while an old executor can still command or terminate the browser is not a
    completed transfer or revocation.
117. Live-browser adoption is a two-phase, single-use custody transition. The
    authority pins a stable process handle where supported and binds boot,
    process-start, executable, physical profile, profile-lock, endpoint-socket,
    current owner, and selected candidate-peer evidence. Commit reobserves every
    replaceable axis after candidate attachment. PID reuse, executable change,
    endpoint or socket substitution, profile replacement, candidate fork or
    exec, inherited descriptors, and transferable bearer material can produce
    only a typed no-transfer or uncertain outcome, never a second owner.
118. Public state uses separate machine-readable axes for reservation currency,
    holder observation, physical occupancy, effect progress, cleanup obligation,
    and historical evidence. No API, dashboard, doctor, planner, or client may
    derive `active session`, `running browser`, `authenticated profile`, or
    `current owner` from a lease row or receipt alone. Contract tests prohibit
    combined convenience states that silently restore those inferences.
119. Canonical physical identity survives path aliasing and detects replacement.
    Registration accounts for the platform's device or volume identity, file or
    directory identity, mount or namespace view, links and reparse points, and a
    protected resource sentinel where required. A bind mount, hard link,
    delete-and-recreate at the same path, copied authority home, or changed
    namespace cannot create two authorities or let old evidence describe a new
    profile. Unprovable identity requires explicit rebind or quarantine.
120. Bounded safety is paired with an ordinary-work usability objective. For an
    authenticated client using its own compatible profile with no fresh exact
    physical collision, the broker cannot return a lease blocker. An
    incompatible in-flight operation returns a durable queue or exact current
    conflict with a machine-readable deadline, and abandoned ephemeral work
    reaches its policy terminal within a frozen installed-acceptance maximum.
    A formally finite but operator-hostile wait does not satisfy this plan.
121. Migration closes authority paths rather than merely preferring the new one.
    The accepted candidate contains no effect-capable legacy lease, lifecycle
    transfer, direct Service State owner mutation, or unfenced raw sink path.
    Mixed-version adapters may observe or obtain canonical authority, but an
    unsupported or downgraded component fails before effect and cannot keep a
    hidden second writer alive.

## Claim Modes

### Ephemeral

- default for ordinary Agent Browser clients;
- finite server-selected TTL with bounded renewals;
- automatic expiry and fencing without requiring client shutdown;
- no persistent blocker after the client, worker, session, or daemon exits;
- optional best-effort release for fast reuse.

### Strict

- opt-in for lease-aware software only;
- finite heartbeat or recovery-grace policy, never an undocumented permanent
  hold;
- named recovery controller and supported crash-recovery workflow;
- explicit subordinate work and cleanup obligations;
- first-class administrative revoke plan and apply surfaces;
- revocation advances fencing before cleanup and reports any remaining exact
  physical obligation separately.

## Public Control Plane

The public operations are:

- `lease list|inspect|history|doctor` for read-only truth;
- `lease acquire|renew|release` for ordinary lifecycle control;
- `lease revoke plan|apply` for exact administrative recovery;
- `lease recover plan|apply` for strict owner recovery;
- `lease explain` for dominant blocker, active claim, physical evidence, and
  safe recourse; and
- `lease watch` for revision-bound changes without polling-derived authority.

The same operations and schemas must remain aligned across CLI help, HTTP,
MCP, generated client, dashboard, README, docs site, shared skill, and inline
documentation.

Revocation plan and apply are separate. Apply requires the plan id, exact
claim revision, fencing token, principal or administrative authority,
observed subordinate work, and expiry. A stale plan has zero effect and
returns the refreshed current claim.

## Coordination Boundaries

The kernel is authoritative for:

- profile and runtime-lane claims;
- service-session and tab child claims;
- viewer and controller child claims;
- lifecycle-owner transfer authority;
- installer transaction claims;
- transition deadlines and reconciliation eligibility;
- revocation and recovery plans;
- effect fencing and receipts; and
- lease health diagnostics.

Browser process discovery, daemon census, sockets, profile locks, displays,
routes, and provider state remain owned by their current subsystems. They
submit typed current evidence to the kernel. They do not convert historical
lease records into physical truth, and the kernel does not infer a process
from a logical claim.

## Execution Slices

### Slice A | Separate history from operational authority

- Add a regression proving that retained released or expired legacy sessions
  remain visible without blocking axes or an unhealthy doctor result.
- Prove that adding arbitrary terminal history cannot change an access or
  admission decision.
- Classify live legacy evidence separately from historical legacy evidence.
- Keep history inspectable without deleting or rewriting it.

Exit condition: historical lease records are observational only and cannot
change current admission.

### Slice B | Introduce the canonical active-claim kernel

- Add the active claim, event, fencing, expiry, and transition-deadline model.
- Centralize current-authority evaluation behind one interface.
- Migrate direct `ServiceState` construction to builders or constructors, then
  make the lease-authority envelope private as well as its collections.
- Route profile acquisition and authenticated work authority through it.
- Preserve current public responses through an explicit compatibility
  projection.

Exit condition: the current lease decision has one mutation owner and one
evaluator.

### Slice C | Make acquisition atomic and effects fenced

- Replace executable-plan assumptions with atomic acquisition.
- Require claim revision and fencing token at every profile, session, tab,
  owner, and route mutation seam.
- Add idempotent effect receipts and uncertain-effect recourse.

Exit condition: two contenders cannot both receive authority, and a stale
holder cannot commit.

### Slice D | Add ephemeral expiry and transition reconciliation

- Apply finite TTLs and heartbeat rules to ordinary clients.
- Add deadlines and named reconcilers for closing, prepared, transferring,
  recovery, and revocation transitions.
- Separate exact physical cleanup obligations from logical claim expiry.

Exit condition: client or daemon disappearance converges without manual
release or an indefinite gate.

### Slice E | Add strict claims, recovery, and revocation

- Validate strict recovery metadata at acquisition.
- Implement revision-bound recover plan and apply.
- Implement administrative revoke plan and apply.
- Fence before cleanup and retain exact cleanup obligations on failure.

Exit condition: an abandoned strict claim is recoverable through supported
surfaces without raw state editing.

### Slice F | Coordinate runtime-owner transfer and hierarchy

- Make owner transfer consume parent claim authority.
- Make sessions, tabs, viewers, controllers, and routes bounded children.
- Compensate or quarantine incomplete transfer sagas at their deadlines.

Exit condition: no Agent Browser subsystem maintains an independent competing
definition of current ownership.

### Slice G | Align public surfaces and migration

- Add CLI, HTTP, MCP, generated client, dashboard, documentation, and skill
  parity.
- Migrate only positively proven live authority into `activeClaims`.
- Retain all other legacy material as history or explicit quarantine.
- Add mixed-version read, write, rollback, and installer compatibility.

Exit condition: old and new generations cannot silently disagree about
operational authority.

### Slice H | Validate, integrate, install, and accept

- Run focused, presubmit, comprehensive, and isolated development checks.
- Install one exact development-approved production candidate.
- Prove provider-free acquisition, expiry, revocation, transfer recovery,
  restart convergence, and zero process residue.
- Audit current production history to prove it is nonblocking without deleting
  it.

Exit condition: source, installed identity, live authority, and current
runtime receipts all satisfy the acceptance matrix.

## Acceptance Matrix

| Case | Required outcome |
| --- | --- |
| released or expired legacy session retained for years | visible in history, absent from operational blockers |
| arbitrary terminal history added | identical access and admission result |
| ephemeral holder process crashes | claim expires, fencing advances, next acquisition succeeds |
| stale process continues after expiry | every later effect rejected by fencing token |
| two simultaneous acquisitions | exactly one claim, one conflict outcome, no duplicate browser |
| planner snapshot becomes stale | acquisition fails before effect with refreshed claim |
| active foreign principal holds exact profile | bounded conflict with current claim proof |
| same principal starts a new client session | rejoin or child claim, never self-conflict by session name |
| strict holder disappears | recover or revoke available through public surfaces |
| stale revoke or recovery plan | zero effect and refreshed current revision |
| strict revoke cleanup fails | logical authority fenced, exact cleanup obligation retained |
| transition controller crashes | deadline reconciliation finishes, compensates, or quarantines |
| owner transfer interrupted | one authoritative owner generation after replay |
| parent claim expires or is revoked | all child effects fenced immediately |
| processless ready-looking browser record | historical or quarantined projection, never live authority |
| logical claim exists with physical profile lock collision | separate exact physical blocker and recourse |
| effect succeeds but response is lost | retry returns same durable receipt without duplicate effect |
| candidate runtime cannot read claim schema | installation fails before runtime-owner mutation |
| rollback to compatible old generation | authority and history remain readable without split ownership |
| doctor sees historical inconsistencies only | current health remains healthy with history count reported |
| caller requests an excessive or backdated TTL | server selects a bounded expiry from current authority time |
| wall clock moves backward after acquisition | remaining authority never increases |
| profile shorthand and canonical path name the same profile | one canonical resource key and one possible winner |
| revision or fencing counter reaches its numeric limit | mutation fails atomically before authority changes |
| completed acquisition operation is replayed after expiry | original terminal receipt, no new claim |
| route bundle fails after profile admission | bounded compensation or exact cleanup obligation, no stranded claim |
| child row remains after parent expiry | every child effect is rejected immediately |
| historical events are archived | fencing and idempotency high-water marks remain unchanged |
| a subsystem attempts to edit the active index directly | impossible through the Rust module boundary |
| same capability starts another operation before ephemeral expiry | joins the exact claim without a new fence or later expiry |
| same principal attempts to rejoin a strict claim directly | rejected with supported strict recover or revoke recourse |
| issuing capability is revoked | one kernel policy fences the claim or proves its bounded delegation interval |
| authority store or replica is stale or unavailable at effect time | no effect and no daemon-local authority synthesis |
| migrated effect caller omits the canonical claim envelope | incompatible-caller error, never legacy gate fallback |
| access planner proposes launch with an absent session identity | plan is invalid before return; acquisition establishes or omits the identity |
| denial has no current claim or exact physical collision evidence | denial is invalid and ordinary work remains admissible |
| external effect completion is uncertain | bounded reconciliation, never blind duplicate execution or permanent denial |
| named strict recovery controller disappears | exact revision-bound administrative recovery remains available |
| acquisition returns reviewed recovery or denial | no active claim, fence, or lease blocker is created |
| terminal lifecycle history exists but current owner does not | cold acquisition remains available, with no invented replacement session |
| physical collision observation exceeds its freshness bound | reobserve or admit; never continue blocking from the stale observation |
| event or receipt persistence fails during claim mutation | the whole authority transaction aborts with no changed claim or fence |
| live client renews continuously while another principal waits | bounded tenure ends or policy transfers priority; no permanent starvation |
| candidate runtime evaluates its own installation admission | rejected by architecture; stable selected supervisor owns the transaction |
| migrated effect or denial path attempts to use raw legacy fields | cannot construct the sealed authority type |
| foreign caller copies claim id, revision, fence, and principal from status | effect rejected because observable metadata is not bearer authority |
| expired claim row remains in the active collection before cleanup | ignored for authority and admission; terminalization may follow without delaying acquisition |
| two clients legitimately share one profile | stable parent supervisor plus attributable compatible child claims, never session-name self-conflict |
| two authority stores point at one physical profile | second domain cannot register or authorize the resource; no duplicate process |
| older backup is restored after newer fences were issued | restored store cannot become effect-capable until a strictly newer authority epoch is established |
| valid envelope is replayed for another action, runtime, or operation | proof fails or the original idempotent receipt is returned; no broader effect |
| logs or retained jobs serialize an effect bearer | bearer is structurally redacted or scrubbed before persistence and output |
| claim expires after validation while browser launch is in progress | durable intent is reconciled to completion, compensation, or uncertainty under the original fence; no unrecorded launch |
| scheduler restarts with a waiting principal | durable waiter position and deadline survive and renewal fairness remains enforced |
| PID or lock path is reused by an unrelated process | no collision conclusion without matching boot, start, and canonical resource identity |
| abandoned strict holder blocks its own administrator | exact authenticated recovery or revoke remains executable independently of holder authority |
| lease policy changes while claims exist | explicit revision transition; existing claims are never silently reinterpreted |
| authority store is corrupt or unavailable | typed authority outage, no invented holder or identity blocker, no requested-profile effect |
| emergency adapter receives a canonical envelope | request remains byte-for-byte resource coherent and daemon verification still decides the effect |
| transition owner and reconciler both crash repeatedly | stable supervisor resumes from durable deadline cursor and idempotent receipt |
| a new effectful entrypoint omits kernel authorization | presubmit architecture manifest fails before integration |
| read-only caller obtains a capability digest or authority backup | cannot derive or mint any bearer accepted at an effect boundary |
| recovery completes and its controller is then rotated or revoked | exact replay returns the original receipt without requiring the old controller to remain active |
| administrative credential is rotated, lost, or revoked | scoped replacement follows the independent administrative lifecycle without target-state editing |
| stable profile supervisor disappears with no child work | bounded parent authority expires or is recovered and cannot permanently deny compatible acquisition |
| physical cleanup repeatedly fails | logical fence remains terminal while the exact obligation reaches bounded retry, escalation, or quarantine recourse |
| two store epochs address one profile through path or mount aliases | external canonical resource registration admits one authority domain only |
| recovery planner reads an expired claim whose old heartbeat was current | no executable plan is returned and apply observes no mutation |
| stale candidate daemon can read verifier material | it can verify but cannot mint an authorization accepted by any executor |
| caller reaches a raw process, CDP, route, display, owner, installer, or input helper | compilation or the sealed sink rejects mutation without typed kernel intent |
| launcher crashes before spawn, after spawn, or before process identity persistence | replay converges to one process or one exact bounded cleanup obligation |
| two authenticated principals reuse the same operation key | receipts remain isolated by principal, action, resource, and authority domain |
| defective client floods distinct acquisitions or waiters | bounded capacity denial, no worker or process multiplication, existing authority remains responsive |
| authority transaction is buffered but not crash durable | no success or effect authorization is returned before the declared durability barrier |
| a historical warning makes a generic readiness aggregate false | the gate rejects the aggregate as non-authoritative and ordinary admission is unchanged |
| a candidate daemon runs as the same workstation user as the authority service | the candidate can read verifier material but cannot read or invoke signer custody |
| signer or administrator key rotates with an authorization in flight | old proof is accepted only within its original bound, completed replay uses its receipt, and no new proof can use the retired epoch |
| the stable supervisor itself is upgraded or lost | an outside bootstrap generation selects rollback or replacement without candidate self-admission |
| the workstation suspends past an ephemeral deadline or reboots with an active claim | the old claim cannot gain tenure; it is expired or fenced before ordinary admission resumes |
| randomized crash and reorder sequences exercise the authority model | implementation and reference model converge on one claim, one fence order, and one receipt per authenticated operation |
| same-user candidate rewrites authority state or verifier files | operating-system custody denies the write; authenticated IPC cannot construct raw authority state or alter trust roots |
| rotation crashes between signer, verifier, and epoch updates | the prior generation remains selected or the complete next generation becomes selected; no mixed trust generation is effect-capable |
| verifier keyring reaches its cardinality bound while old proofs remain live | rotation waits with a typed deadline or emergency-revokes explicitly; it never grows without bound or silently invalidates accepted proofs |
| authority store or online signer is corrupt or permanently lost | independent bootstrap establishes a strictly newer recoverable generation or quarantines the physical resource without inventing a holder |
| a policy describes a transition or denial as bounded | status exposes its exact maximum and deadline, and fault injection reaches the declared terminal outcome within that bound |
| selector names a relative path, alias, or generation whose key identity differs | loading rejects it before leaving the protected trust root or accepting any proof |
| rotation crashes with temporary, staged, or retired generations present | bounded reconciliation selects only an already committed generation and exposes first-class cleanup without blocking leases |
| authority storage is on a platform or volume without proven replace, durability, ACL, or lock semantics | installation or mutation fails with a typed unsupported-storage outcome before authority can diverge |
| long-lived executor retains verifier epoch N while the authority selects N+1 | it securely refreshes or is removed from routing within the declared deadline; no false lease conflict or global readiness denial is emitted |
| stale writer publishes an older valid protected snapshot | selector compare-and-swap rejects it with zero effect and the current generation remains selected |
| read-only authority snapshot is mutated and republished | publication is structurally unavailable; mutation must bind the exact selected authority generation and durably add its own minimum audit fact without parsing prior narrative history |
| same-user process creates a compatible authority socket or replays a valid proof from another socket | local peer, socket, executable, and signed endpoint identities disagree; no authority response is accepted |
| holder authorization is signed and then the claim is revoked before the effect | the current-state intent consume loses to revocation and the effect sink receives no usable permit |
| same-user candidate reads another client's capability file | file possession alone is not an enrolled workload identity and grants no authority outside an explicit narrow delegation |
| authority commit succeeds but Service State projection loses a revision race | the authority receipt remains canonical and idempotent projection replay converges without a second claim or false denial |
| two compatible tab operations share one profile | the action compatibility matrix grants attributable child authority without a profile-level self-conflict |
| logical claim expires while the exact Chrome process still owns the profile lock | logical authority no longer blocks; fresh physical evidence selects bounded adoption, close, wait, cleanup, or quarantine recourse |
| authenticated group member floods or malforms authority IPC | bounded parsing and admission preserve expiry, revocation, reconciliation, and administrator capacity |
| two root authority services on partitioned hosts address one shared profile | the external coordinator admits one domain or quarantines the resource; local root custody alone cannot authorize it |
| one profile claim or resource registration is corrupt while other profiles are valid | only the exact resource is quarantined; unrelated profiles remain usable and no suspect resource is treated as free |
| an ordinary client opens its own authenticated profile and then crashes | broker-managed ephemeral authority requires no lease choreography from the client, expires without release, and permits bounded reuse |
| a dashboard or client retains an authority response past its freshness bound | it displays a stale observation and refreshes before action; it cannot emit a current holder or denial from the cached projection |

## Design Completeness Audit | 2026-08-31

The six hotfix families are covered by the architecture, but the first draft
did not state several cross-cutting failure modes strongly enough. They are now
part of the frozen invariants and acceptance matrix:

1. authority-owned time and bounded TTL policy, including wall-clock rollback;
2. canonical resource identity across shorthand, path, route, and owner aliases;
3. non-saturating revision and fencing counters;
4. terminal idempotency retained independently of active claims and event
   history; and
5. ordered bundle or bounded-saga semantics for operations that need several
   resources;
6. same-capability ephemeral rejoin without renewal and strict recovery
   without implicit rejoin;
7. capability revocation and replaceable strict-recovery ownership;
8. linearizable effect-time authority with no cache or legacy fallback;
9. denial provenance and executable access plans that cannot invent required
   identities;
10. explicit intent, completion, and uncertainty states for external effects;
    and
11. zero-authority planning and recovery offers that cannot create a blocker.

The recurrence audit also found four usability boundaries that must be
explicit rather than assumed: freshness-bounded physical evidence,
transactional coupling of claims with events and receipts, maximum continuous
ephemeral tenure with bounded waiter progress, and installation authority that
is external to the candidate. Compiler-sealed effect and denial types are the
enforcement mechanism that turns the no-fallback rule into a structural
property instead of a convention. Cross-process callers additionally require
request-bound authentication or an unforgeable opaque or authenticated bearer;
type sealing alone cannot secure a serialized envelope.

The audit also makes recursive parent fencing and history compaction explicit.
Without these controls, a single kernel could still admit duplicate aliases,
revive a completed request, strand a partial acquisition, or allow a child to
outlive its parent. These are required before the redesign can be described as
structurally recurrence-resistant.

The kernel collections are also compiler-enforced private state. This closes a
gap between saying there is one mutation owner and actually preventing sibling
subsystems from writing competing authority projections. The additional
invariants close the remaining routes by which an otherwise centralized
kernel could still deny ordinary work based on invented prerequisites, accept
stale cached authority, silently renew a claim during rejoin, or strand strict
recovery behind another vanished worker.

A second recurrence pass over the concrete Plans 0128, 0130, 0132, 0134,
0136, 0137, 0142, 0143, and 0145 reports found additional structural gaps. The
most important is that claim lifetime (`ephemeral` or `strict`) had been
conflated with access compatibility, which could still turn cooperating
clients into profile-level competitors or let several tasks silently share one
undifferentiated authority. The revised model uses a stable profile supervisor
and attributable child claims. The pass also adds authority-domain and
anti-rollback epochs, request-scoped and redacted bearer proofs, durable waiter
state, PID-reuse-resistant physical evidence, scoped administrator authority,
policy revision binding, typed authority outages, stable deadline scanning, and
an exhaustive effect-entrypoint manifest. Without these additions, the design
would reduce recurrence but would not make the reported defect classes
structurally unavailable.

A third recurrence pass found that request binding alone is insufficient when
the verifier holds the same symmetric secret as the signer. User-private file
permissions protect against other operating-system users, but Agent Browser's
candidate daemons and effect executors commonly share one user identity. The
current in-progress HMAC prototype therefore does not satisfy the final
single-authority design: it must be replaced by asymmetric signatures or by a
kernel verification service whose consumers cannot mint proofs. This is a
design correction, not a key-file hardening task.

The same pass tightened the bottom and resource boundaries. An exhaustive
entrypoint manifest can detect known public bypasses, but structural closure
also requires the process, CDP, route, display, owner, installer, and input
mutation sinks to demand an unforgeable typed intent. Process creation must be
a capacity-admitted durable saga with crash injection at every boundary, and
idempotency receipts must be authenticated namespaces rather than globally
caller-selected strings. Finally, an authority commit is not effect-capable
until its crash-durability barrier has completed. These additions address the
older process-multiplication failure family as well as false lease denials.

A fourth recurrence pass compared the design with the actual hotfix seams
rather than only the lease mutation API. It found five remaining ways the
written kernel could still be bypassed or overclaimed. A generic readiness or
health aggregate could deny work before constructing a canonical lease denial;
same-user candidate processes could read a private signer file despite Rust
module privacy; a single verifier file did not define safe rotation and
anti-rollback; the stable supervisor had no stated protocol for upgrading
itself; and an unspecified monotonic clock could pause during suspend and
lengthen a claim in real time. Invariants 80 through 84 close those design
holes. Invariant 85 adds an executable state-machine oracle because a long list
of example tests cannot prove all crash, replay, time, transfer, and mixed-
version interleavings.

These additions narrow the meaning of structurally recurrence-resistant. The
known failure sources become incapable of constructing operational authority or
a typed denial after every gate and sink is migrated. They do not promise that
arbitrary software defects, storage loss, kernel compromise, or unavailable
hardware can never stop an operation. Those conditions must surface under
their own typed outage or product-error classes and must not be persisted as a
lease holder, session identity, or transferable owner.

A fifth recurrence pass followed the same-user attack surface through the
current keyring prototype and the public recovery promise. It found that
protecting only the private signer is insufficient. A candidate that can edit
the canonical authority file can manufacture active state without signing, and
a candidate that can replace the verifier keyring can make its own signer
trusted. Invariant 86 therefore extends enforced custody to authority state and
trust configuration. Invariant 87 makes rotation a selected, crash-durable
generation transition with explicit retirement cutoffs and a genuinely bounded
keyring rather than an ever-growing map.

The same pass found a recovery recursion: a public administrative command is
not a disaster-recovery surface if it requires the damaged online signer,
administrator registry, or authority store to authorize its own repair.
Invariant 88 adds an independent bootstrap with monotonic external epochs and a
quarantine outcome when fencing high-water marks cannot be proven. Finally,
Invariant 89 converts every liveness promise from an adjective into a published
maximum and deadline. These four additions are required before the design can
claim structural resistance to the historical false-holder and permanent-gate
families.

A sixth recurrence pass examined the trust-generation implementation as a
hostile input and crash surface. It found that a syntactically valid selector
could still escape or alias the trust namespace unless its name was
content-bound; orphaned staging and retired generations could become another
permanent recovery gate or unbounded disk sink; Unix durability and mode checks
did not establish the Windows or network-volume contract; and a long-lived
effect executor could reject a legitimately rotated proof from a stale verifier
cache. Invariants 90 through 93 make namespace binding, bounded generation
reconciliation, platform storage qualification, and monotonic verifier rollout
part of structural acceptance. The current source now rejects unsafe selector
components and requires the selected name to be derived from the active key,
but the remaining three invariants are not implemented and the candidate
remains noninstallable.

A seventh recurrence pass treated the authority protocol and its serialized
state as hostile boundaries. It found that private collections alone do not
prevent a restored, migrated, tampered, or partially written snapshot from
encoding two logical resources for one physical profile. It also found that an
authenticated caller can still be misled by a candidate-owned or stale IPC
endpoint unless the authority service identity and response are protected, and
that resource bootstrap must not be an implicit consequence of acquisition.
Invariants 94 through 96 require full load-time invariant validation,
authenticated endpoint identity, and explicit fenced resource registration and
rebinding. The first protocol regression now proves that a duplicate physical
profile identity is rejected on protected-state load. The broader
cross-collection validator, protected transport, and administrative rebind
surface remain incomplete, so the candidate remains noninstallable.

An eighth recurrence pass followed the history boundary through protected
state loading. Excluding lifecycle rows from admission logic is insufficient
when those rows still share the same serialized envelope and parser as current
authority: a malformed historical row could make authority unavailable before
the kernel reaches its no-history decision. Invariant 97 therefore separates
non-authoritative event, lifecycle, warning, and compatibility history from the
operational state load path. Required audit appends remain transactionally
coupled to new mutations, but reading or authorizing already-current work does
not parse history. The private store now serializes the canonical lease event
log through a separate history file and manifest; operational load never opens
them. Public service wiring and protected custody remain open, so the candidate
remains noninstallable.

A ninth recurrence pass exercised the durable publication protocol and found
that serialization alone did not prevent a stale writer from selecting an
older, internally valid protected snapshot. Load-time epoch checks cannot
close a rollback path created by the publisher itself. Invariant 98 therefore
binds every publication to the exact selected predecessor generation. The
failing stale-publisher regression demonstrated the selector rollback before
the compare-and-swap was added.

The same pass audited current effect surfaces rather than treating the future
entrypoint manifest as already enforced. The legacy emergency profile gate,
route-bound and manual-seeding browser launch, foreign CDP input grants,
runtime-owner transfer, workstation installation, and dashboard lease actions
still have compatibility or independent authority paths. The dashboard is an
advisory client and can never be an enforcement boundary. These are migration
inventory, not accepted exceptions: source acceptance requires each effectful
sink to consume the sealed kernel intent or become explicitly non-effectful,
followed by a presubmit manifest that fails when a new sink lacks that type.

The publication review found a second capability-boundary defect. Protected
operational load correctly omitted history, but the resulting kernel could
still be passed back to publication with an empty event vector. That could
truncate retained audit evidence in a later generation. Invariant 99 makes
read-only load non-publishable. The original follow-on implementation made a
mutation parse and validate all selected history before changing authority.
The tenth recurrence pass below rejects that coupling: a new mutation must
durably publish its own minimum audit fact, but corrupt or unavailable prior
narrative history cannot become a new ordinary-work gate.

A tenth recurrence pass included the later Plan 0146 shared-Service-State
writer failure and treated the protected authority service as an adversarial
and unavailable dependency rather than merely a trusted implementation detail.
It found seven remaining design gaps. First, a signed bearer verified offline
cannot prove that its claim was not revoked after issuance, so effect authority
must be a single-use current-state intent consume. Second, protecting signer
and authority files from a same-user candidate is incomplete if that candidate
can read another client's raw capability file. Third, authority state and
user-scoped Service State cannot be maintained as an implicit dual write; the
authority receipt must drive idempotent projection, including while selected
and fallback runtime hosts coexist. Fourth, access compatibility needs a
complete action-by-resource matrix or a new centralized kernel can still
overblock at profile granularity. Fifth, logical expiry alone does not restore
usability when a real orphan process or lock remains, so every fresh physical
collision needs bounded first-class recourse. Sixth, the root IPC service needs
singleton, parser, rate, and resource limits so an authenticated defective
client cannot starve recovery. Seventh, local root custody is not cross-host
exclusion; the external epoch and resource coordinator must be concrete before
shared or restored profiles can be enabled.

Invariants 101 through 107 and their acceptance rows close these gaps in the
design. They are not implemented. In particular, the current mutation-load
implementation still binds the complete selected history generation, the
current bearer is not a single-use online consume, and profile capabilities
remain same-user files. Plan 0146 was merged at `b9d6d0ae`; the combined branch
passes strict Clippy, 29 Service Store tests, 25 protected-authority protocol
tests, the required serial 122-test workstation installer partition, the
source-free workstation fixture, the service-client suite, and the docs build.
That merge closes the known shared-Service-State writer race but does not
implement the authority-to-projection receipt contract in invariant 103. The
candidate therefore remains noninstallable and cannot yet claim structural
recurrence resistance.

An eleventh recurrence pass focused on usability blast radius rather than only
authority integrity. It found three remaining ways a correct kernel could
still recreate the reported experience. A malformed record in one monolithic
snapshot could turn one profile incident into a workstation-wide authority
outage. A stale but well-formed read projection could continue telling an
operator that an expired holder is current. Finally, exposing the canonical
lease protocol directly to every ordinary caller could replace false denials
with mandatory lease bookkeeping and make ephemeral agents responsible for
cleanup they cannot reliably perform.

Invariants 108 through 110 require resource-scoped quarantine, freshness-bound
observations, and broker-managed ephemeral authority for ordinary work. These
are design corrections, not presentation polish. They are not implemented,
and the current protected store remains a single validation and availability
unit. Structural source acceptance now also requires fault injection proving
that one corrupt resource cannot deny another, public readbacks proving stale
status is observational, and a zero-lease-choreography client acceptance case.

A twelfth recurrence pass returned to the original Last30Days failure at the
consumer boundary. That incident did not merely expose a false lease conflict:
the broker's roughly thirty-second wait exhausted the consumer's equal
subprocess timeout, so the useful typed broker result was replaced by
`unexpected_timeout_expired`. A truthful authority kernel could still recreate
that operator experience if each layer owns an unrelated timeout or if an
observational access plan is described as an executable promise. Invariants
111 through 113 therefore add end-to-end deadline dominance, distinguish
observational plans from atomically reserved executable offers, and require
typed outcome fidelity through every public adapter and consumer broker. These
contracts are not implemented. Structural acceptance now requires a deadline
matrix and fault tests proving that waits, caller disappearance, cancellation,
and response loss cannot erase recourse or trigger blind duplicate execution.

A thirteenth recurrence pass checked the older resource-exhaustion and Service
State lock hotfixes rather than treating them as consequences already covered
by canonical lease state. It found two independent failure boundaries. First,
a correct claim and single-flight intent can still deadlock or create a global
timeout convoy if code holds an authority or projection lock while waiting for
Chrome, a subprocess, a provider, or another store. Second, software admission
cannot by itself bound a child that forks unexpectedly or a supervisor that
restarts a crashing generation faster than durable reconciliation converges.
Invariants 114 and 115 therefore add an enforced cross-store lock graph, forbid
locks across external-effect waits, and require generation-bound
operating-system process containment with a protected recovery reserve. These
are separate from lease truth: the kernel prevents duplicate authority, while
the operating system bounds damage from defective effect code. Neither is fully
implemented, so structural acceptance now has eight proof gates.

## Validation Contract

Each slice names the protected invariant and demonstrates the defect before
the fix when practical. Tests use the cheapest stable public or contract seam.

Touched Rust code requires:

```bash
scripts/ci/cargo-safe.sh fmt --manifest-path cli/Cargo.toml -- --check
scripts/ci/cargo-safe.sh clippy --manifest-path cli/Cargo.toml -- -D warnings
```

Every checkpoint runs focused tests for its authority seam. Before push and
installation, run:

```bash
pnpm validation:select -- --base c21118a30b01eaf23acabdec80e81f5d79a130b3
```

Service action or schema changes require parity across Rust contract metadata,
JSON schema, CLI, HTTP, MCP, generated client, dashboard, help, README, docs
site, skill, and inline comments. Isolated development publication and browser
smoke precede any production candidate installation.

## Migration And Rollback

Migration first writes a sealed candidate active index and a source-state
digest. It imports only claims with current principal capability, nonterminal
state, unexpired time, current owner generation when required, and current
process evidence when process ownership is asserted. Everything else remains
history or quarantine.

The candidate validates cardinality, hierarchy, fencing monotonicity, expiry,
transition deadlines, and reverse-reader compatibility before the selected
state pointer changes. A failure leaves the old state untouched. Rollback
restores the prior compatible state pointer while retaining new append-only
events and receipts in a reader-compatible terminal ledger.

## Bounds

- Maximum two implementation attempts per slice before a local reframe.
- Maximum one broad drift-discovery review for the full goal.
- Maximum one closed-world remediation cycle for accepted blocking findings.
- Durable checkpoint every three slices, ninety minutes, or any material
  authority or installation transition, whichever comes first.
- No automatic retry after uncertain external effect.
- Historical warnings never consume an implementation retry or block an
  unrelated ready slice.

Checkpoint fields are state transition, acceptance state, progress
classification, evidence, material blockers, and next action or stop reason.

## Hard Stops

- Do not delete retained history to make doctor green.
- Do not add a generic force-unlock or wildcard revoke.
- Do not infer principal authority from a client session name.
- Do not infer physical liveness from a lease, owner row, route, or status
  label.
- Do not allow a planner response to reserve authority.
- Do not install a candidate that has not passed exact development identity,
  compatibility, and rollback checks.
- Do not use tenant profiles or provider navigation for source acceptance.
- Do not terminate a process without exact current process and ownership
  evidence.

## First Execution Packet

1. Add the retained-history regression at the profile-lease projection and
   doctor seam.
2. Demonstrate the current false warning before the fix.
3. Classify a legacy profile with no active session or owner as historical,
   with no blocking axes and no mutation actions.
4. Preserve current blocking behavior for positively active unproven legacy
   authority.
5. Run focused Rust tests, formatting, and changed-surface selection.
6. Record the red and green evidence here and commit the slice.

This packet changes no live runtime state and performs no installation.

## Slice A Checkpoint | 2026-08-31

State transition: `slice_a_history_separation_in_progress` to
`slice_a_complete_slice_b_ready`.

Acceptance state: retained released and expired legacy sessions remain visible
as `historical`, expose read actions only, carry no blocking identity axes, and
leave profile-lease doctor healthy. Current unproven legacy work remains an
observation-only identity-reconciliation blocker. Adding terminal session
history does not change the access-plan decision.

Progress classification: `outcome_progress`.

Evidence:

- Red: the focused historical regression failed because the projected state
  was `identity_reconciliation_required` instead of `historical`.
- Green: the focused historical regression passed after current and historical
  evidence were separated.
- Three legacy-profile tests passed, including the current-authority negative
  control.
- The terminal-history access-plan invariance regression passed.
- The 45-test `service_access_plan` family passed serially.
- Rust formatting and strict Clippy passed.
- Documentation build, API/MCP parity, generated client checks, JavaScript
  type checks, remote-view documentation checks, and every validation-selector
  workstation fixture passed.

Material blockers: none for Slice B. The unrelated P110 worktree retains one
uncommitted note and does not overlap this lane.

Next action: introduce the canonical active-claim kernel behind a compatibility
projection, beginning with the smallest profile authority seam.

## Slice B Kernel And Access Checkpoint | 2026-08-31

State transition: `slice_a_complete_slice_b_ready` to
`slice_b_kernel_and_access_in_progress`.

Acceptance state: Service State now has a backward-readable canonical lease
authority envelope containing a resource-keyed active-claim map, durable
fencing counters, authority revision, and append-only events. Atomic
acquisition validates expiry, expected revision, parent authority, strict
recovery metadata, and idempotent replay. Access planning consults the current
profile claim even when no session projection exists. A matching principal may
continue, an unauthenticated caller must authenticate, and a foreign principal
must wait.

Progress classification: `outcome_progress`.

Evidence:

- Red: the first kernel regression failed with `Unsupported` while retained
  terminal history was present.
- Green: five kernel tests pass for history independence, revision
  compare-and-swap, strict recovery requirements, Service State round-trip,
  and repository-level two-contender atomicity.
- Red: a fencing high-water mark at the numeric limit had no typed failure and
  would have saturated, reusing the prior token.
- Green: counter exhaustion now fails before any authority mutation; the six
  kernel tests include an exact state-equality regression for this boundary.
- Red: an access plan with a canonical claim but no session incorrectly
  returned `launch_new_browser`.
- Green: the access plan now returns `authenticate_for_profile_reuse`, exposes
  the claim id, revision, fencing token, and principal, and reports one active
  lease. Matching and foreign principal controls also pass.

Material blockers: profile-lease doctor and effect admission still use the
legacy compatibility projection. The public acquire, renew, release, recovery,
and revocation operations do not yet issue or consume canonical claim tokens.
The inner authority collections are private, but the containing Service State
field cannot become private until existing direct struct construction is
migrated to builders or constructors. This checkpoint must not be installed as
the completed lease redesign.

Next action: project canonical claims through profile-lease doctor, then make
profile acquisition and daemon effects consume the same atomic claim before
adding renew, release, recovery, and revocation.

## Slice B Doctor Projection Checkpoint | 2026-08-31

State transition: `slice_b_kernel_and_access_in_progress` to
`slice_b_kernel_access_and_doctor_in_progress`.

Acceptance state: profile-lease collection, inspect, explain, and doctor now
give a current canonical profile claim precedence over retained session,
capability, binding, and owner compatibility rows for the same profile. Those
rows remain visible as subordinate context on the canonical projection but
cannot add operational blockers. The canonical row reports `state=active` and
`recourse=continue_with_active_claim` without claiming that a logical lease
proves a browser process exists.

Progress classification: `outcome_progress`.

Evidence:

- Red: a current canonical claim plus one retained active unproven session was
  projected as a legacy lease id and made doctor unhealthy.
- Green: the same fixture yields exactly one canonical claim row, retains the
  session id as context, has no blocking identity axes, and leaves doctor
  healthy.
- All 17 profile-lease tests and all 10 service-principal tests pass.
- The generated service-observability contract check, JavaScript type check,
  API and MCP parity check, documentation build, Rust formatting, and strict
  Clippy pass.
- The full service-client suite, remote-view documentation guard, source-free
  workstation installer fixture, host-provision fixture, fresh-VM harness,
  Guacamole asset check, PostgreSQL durability check, and route-specific user
  synchronization check pass.

Material blockers: canonical acquisition, renewal, release, recovery,
revocation, and daemon effects remain unmigrated. Canonical rows therefore
advertise read operations only at this checkpoint. The compatibility candidate
must not be installed yet.

Next action: replace the public profile-acquisition mutation with one atomic
canonical acquisition and a durable idempotent receipt, then require that
claim at the first daemon effect seam.

## Slice C Profile Acquisition And Prelaunch Fence Checkpoint | 2026-08-31

State transition: `slice_b_kernel_access_and_doctor_in_progress` to
`slice_c_profile_acquisition_and_prelaunch_fence_complete`.

Acceptance state: public profile acquisition now reauthenticates the registered
profile capability inside the atomic Service State mutation, replays completed
operations before new admission, and creates no claim for reviewed recovery or
denial. An admitted acquisition returns one five-minute ephemeral claim,
durable acquisition receipt, and exact effect envelope. A new operation from
the same principal and capability rejoins the current ephemeral claim without
changing its fence or expiry. Strict claims cannot implicitly rejoin. The
daemon revalidates the claim, expiry, owner generation, and exact principal
binding before retained attach, shared attach, or browser launch.

The recurrence audit also repaired an authority leak outside the new kernel.
When no current runtime owner exists, retained lifecycle records are history
only. They cannot block cold acquisition or emit a replacement browser or
session identity. Unsupported authority, receipt, and effect-envelope schemas
fail before mutation or effect.

Progress classification: `outcome_progress`.

Evidence:

- all 13 canonical lease-authority tests pass;
- all 20 profile-acquisition and recovery tests pass;
- both canonical prelaunch effect tests pass, including retained-session attach;
- the terminal-history-without-current-owner regression passes;
- all 45 service-access-plan and all 35 service-model tests pass;
- wrapper Rust formatting and strict Clippy pass;
- the complete service-client suite, generated contract, JavaScript types,
  API/MCP parity, documentation build, and remote-view documentation guard pass;
- the validation selector's workstation fixture family passed earlier in this
  same uncommitted slice and no workstation implementation changed afterward.

Material blockers: the serialized effect envelope is not yet bound to request
authentication or an unforgeable bearer. Effect completion receipts, release
compensation, strict recovery and revoke, transition deadlines, hierarchical
claims, runtime-owner transfer coordination, and removal of the legacy
no-envelope fallback remain incomplete. This checkpoint must not be installed
as the completed redesign.

Next action: commit and push this coherent checkpoint, reconcile the isolated
emergency fail-open branch, then add authenticated effect proof and exact
release, strict recover, and revision-bound revoke operations.

## Slice C Authenticated Effect Proof Checkpoint | 2026-08-31

State transition: `slice_c_profile_acquisition_and_prelaunch_fence_complete` to
`slice_c_authenticated_effect_proof_complete`.

Acceptance state: effect authorization v2 is an authenticated bearer rather
than a reconstruction of public claim fields. The envelope binds resource,
claim, principal, capability ID and revision, claim revision, fencing token,
and owner generation with an HMAC derived from the private registered
capability digest. The digest and proof are not projected through status,
history, or diagnostics. Effect admission rechecks that the exact capability is
still active at the same revision before verifying the proof. Capability
revocation, rotation, envelope alteration, stale claim state, and owner-binding
divergence all fail before browser effects.

Progress classification: `outcome_progress`.

Evidence:

- all 13 canonical lease-authority tests pass, including proof tampering,
  capability revocation, and later owner-binding divergence in one effect-time
  sequence;
- all 20 profile-acquisition tests and all 22 canonical-path tests pass;
- wrapper Rust formatting and strict Clippy pass;
- generated client contracts and JavaScript types pass with the v2 envelope;
- the complete service-client suite, API/MCP parity, documentation build, and
  remote-view documentation guard pass.

Material blockers: exact release compensation, strict recovery and
revision-bound revoke, effect completion receipts, transition deadlines,
hierarchical claims, runtime-owner transfer coordination, and removal of the
legacy no-envelope fallback remain incomplete. This checkpoint remains
non-installable as the completed redesign.

Next action: commit and push the authenticated-effect checkpoint, then add
exact terminal claim mutations and public strict recovery and revoke plans.

## Emergency Branch Reconciliation And Second Recurrence Audit | 2026-08-31

State transition: `slice_c_authenticated_effect_proof_complete` to
`emergency_branch_merged_and_semantically_reconciled`.

The accepted emergency branch was merged at `be1f87cb`. Semantic review found
one post-merge split-authority seam: the legacy scheduler gate still evaluated
commands that already carried a canonical effect authorization. With emergency
mode enabled, it could rewrite the authorized profile to an isolated fallback;
with emergency mode disabled, it could reject before the daemon evaluated the
canonical claim. The scheduler now admits canonical-envelope commands unchanged
past the legacy gate. This is not effect admission. The daemon remains the only
component that verifies the authenticated envelope against current claim,
capability, profile, owner generation, and expiry before browser effect.

The second recurrence audit expanded frozen invariants 49 through 62. It found
that lifetime and recovery mode had not been separated from access
compatibility, and added the stable profile-parent plus attributable-child
model. It also added authority-domain and anti-rollback epochs, request-scoped
and redacted bearers, durable external-effect intent, durable waiter fairness,
PID-reuse-resistant physical evidence, independent administrative authority,
policy revision binding, typed authority outages, stable deadline scans, and
an exhaustive effect-entrypoint manifest.

Focused evidence:

- canonical scheduler admission regression: 1 passed;
- emergency fail-open regressions: 3 passed;
- canonical profile-claim regressions: 2 passed;
- wrapper Rust formatting and strict Clippy passed;
- documentation production build and remote-view documentation guard passed;
- `git diff --check` passed.

Material blockers remain unchanged in effect: exact release compensation,
strict recover and revision-bound revoke, durable effect completion and
uncertainty receipts, deadline reconciliation, parent and child claim
integration, runtime-owner transfer coordination, legacy no-envelope removal,
authority-domain anti-rollback, public-surface parity, mixed-version migration,
and installed acceptance. The merged checkpoint remains intentionally
non-installable as the completed redesign.

## Slice C Exact Holder Release And Third Recurrence Audit | 2026-08-31

State transition: `emergency_branch_merged_and_semantically_reconciled` to
`slice_c_exact_holder_release_complete`.

Canonical exact-holder release now authenticates the current capability and
claim inside one serialized repository mutation. It advances the resource
fence, removes only the exact active claim, and commits the terminal event and
idempotency receipt with the authority mutation. The operation is intentionally
not coupled to a global expected authority revision, so unrelated resource
activity cannot manufacture a false release conflict. A lost response can be
replayed through CLI, HTTP, MCP, or the generated client surface using the same
capability, exact original lease revision, and operation key. Replay returns
the original terminal receipt and terminal projection without recreating
authority or consulting a derived session, browser, owner, or history row.

The third recurrence audit found four semantics that were still implicit and
froze them as invariants 63 through 66: an active claim is a bounded authority
reservation rather than process-liveness proof; logical release, recovery, and
revocation cannot depend on optional runtime projections; observations and
failed cleanup cannot renew authority; and expiry needs a monotonic
supervisor-owned time basis with a durable nondecreasing floor across restart.
These additions bound the unavoidable crash-detection window without allowing
retained evidence or repeated warnings to extend it.

The broader profile-gate regression also exposed two remaining compatibility
seams. Exact broker route hints were still re-evaluated as an unproved new
session, and non-acquiring service, handoff, and stream control commands could
enter profile admission when they carried an explicit lease policy field. The
compatibility gate now recognizes only a current browser/profile/session
association as exact reuse and unconditionally excludes those non-acquiring
control actions. This restores current clients while the legacy gate remains.
It is not the final structural guarantee: Slice G must still remove the legacy
no-envelope authority path and make the canonical entrypoint manifest the
presubmit enforcement boundary.

Focused evidence:

- all 14 canonical lease-authority tests pass, including tamper rejection,
  unrelated-authority mutation, terminal fence advancement, exact replay, and
  later acquisition with a newer fence;
- the public canonical release and replay regression passes with the exact same
  terminal receipt and lease projection;
- all 39 profile-lease and legacy admission regressions pass together,
  including exact broker route reuse and non-acquiring control actions;
- the generated service-client contract and type suite passes;
- Rust formatting passes.

Material blockers: automatic compensation around uncertain launch effects,
strict recover and revision-bound revoke, durable effect intent and completion
receipts, transition-deadline reconciliation, parent and child claim
integration, runtime-owner transfer coordination, legacy no-envelope removal,
authority-domain anti-rollback, remaining public-surface parity,
mixed-version migration, and installed acceptance remain incomplete. This
checkpoint is not installable as the completed redesign.

## Slice F Non-Minting Verification Reframe | 2026-08-31

State transition: `slice_f_request_bound_bearer_in_progress` to
`slice_f_request_bound_bearer_signer_separation_required`.

The third full recurrence pass found that the in-progress request-bound HMAC
bearer still gave every verifier the secret needed to mint another bearer.
User-private file permissions did not establish a component boundary because
candidate daemons and effect executors run under the same user. Frozen
invariants 74 through 79 now require non-minting verification, sealed low-level
mutation sinks, crash-safe single-flight process creation, authenticated
idempotency namespaces, bounded authority resources, and an explicit durable
commit barrier.

Source now uses Ed25519 authorization schemas v4 and recovery schemas v3. The
private signer type, key loader, and signing helpers are private to the lease
authority module. Profile acquisition and recovery request signed envelopes
without receiving signer material. Runtime effect, release, and recovery paths
load only the separate public verification-key file. Private and public key
publication syncs file contents and the containing directory before authority
is returned, and verification cannot bootstrap a missing authority root.

Current evidence at the protected-registry checkpoint:

- generated service-client contracts and observability helper tests pass;
- dashboard profile-lease tests and dashboard TypeScript pass;
- service API and MCP parity passes for 66 browser controls, 26 service tools,
  19 resources, 83 native actions, and 106 service-request actions;
- client JavaScript and TypeScript coverage passes;
- the documentation production build passes;
- every contract JSON document parses and `git diff --check` passes;
- Cargo metadata accepts the locked offline `ring` dependency graph;
- all 21 canonical lease-authority tests pass, including signer separation,
  signing-oracle rejection, exact release, strict recovery, administrative
  revoke, administrator revocation, key-epoch rotation, bounded old-proof
  verification, rollback rejection, fencing, tamper rejection, and replay;
- all 41 profile-lease, 21 profile-recovery, 45 access-plan, and 35
  service-model tests pass;
- the exact canonical prelaunch effect test passes; and
- wrapper Rust formatting and strict Clippy pass.

The non-minting verifier, raw-capability signing boundary, versioned verifier
keyring, and signing-key epoch are source accepted. The rotated keyring retains
only explicitly enrolled old public keys, rejects future or mismatched epochs,
and prevents a stale verifier from accepting a newer proof. Trust files now
stage under one immutable generation and become current through one atomic
selector only after signer, verifier, manifest, file digests, and directories
are durable. Selection rejects tampered digests and stale rotation compare-and-
swap; the verifier ring has a hard eight-key cardinality limit. Standalone
legacy key files cause an explicit migration-required outcome instead of a
parallel trust root. Stable-supervisor signer and authority-state custody,
public durable rotation apply, retirement cutoffs, external epoch anti-rollback,
loss recovery, legacy migration apply, in-flight proof drain, and installed
acceptance remain open, so this is not an installable completion of the lease
redesign.

The follow-on structural review removed two signing and mutation oracles before
opening the public administrator plane. Effect and recovery issuance now
reauthenticate the exact raw private profile capability before the authority
module will sign an envelope; possession of a claim projection or an
in-crate authenticated-principal struct is insufficient. Release, recovery,
and administrative revoke verify their cryptographic authorization inside the
private terminal mutation method itself. A sibling subsystem cannot deserialize
a plausible envelope and invoke an unverified state mutation below the
repository wrapper.

The first administrative revoke kernel primitive is staged behind that sealed
boundary. It is exact-resource, claim-id, claim-revision, and fencing-token
bound; independent of the target capability and all optional session, browser,
owner, and process projections; advances the resource fence before removing
the active claim; emits a revoked event and durable terminal receipt; rejects
tampering before mutation; and replays the exact completed receipt before
requiring a signer that may have rotated. The serialized administrative
authorization contract is
`docs/dev/contracts/lease-administrative-authorization.v2.schema.json`.

Administrative issuance now also requires the exact raw private administrator
capability registered in the kernel's private administrator collection. Apply
rechecks that the administrator remains active at the sealed revision inside
the same terminal mutation. Revoking or rotating the administrator therefore
invalidates an outstanding plan before effect, while revoking the target
holder capability does not disable independent administrative recourse.

This primitive is not yet a public revoke feature. Administrator bootstrap,
credential custody, rotation, loss recovery, public plan and apply parity, and
cleanup-obligation projection remain mandatory before any operator can use it.
The new Rust regressions pass. The administrator kernel remains non-public and
non-installable until the bootstrap, custody, rotation, public parity, and
cleanup-obligation requirements above are implemented and accepted.

## Slice G Protected Authority Protocol And Resource Registry | 2026-08-31

State transition: `slice_f_non_minting_verifier_source_accepted_admin_revoke_kernel_in_progress`
to `slice_g_protected_authority_protocol_resource_registry_in_progress`.

The first private protocol kernel now accepts an explicit allowlist of typed
operations and rejects generic signing or generic state-mutation requests. Its
typed acquire request carries the raw capability only as a redacted, zeroed
secret. Authentication and derivation of principal, capability identity, and
capability revision occur inside the kernel rather than trusting caller-supplied
identity projections.

The kernel serializes the canonical lease authority, principal registry, and a
protected canonical resource registry as one state envelope. Profile
acquisition requires a pre-registered protected resource and cannot create a
profile identity as a side effect. Resource registration rejects empty or
noncanonical identities, and load-time validation rejects mismatched map keys,
unsupported resource kinds, invalid revisions, noncanonical digests, and two
logical profile resources mapped to one physical identity. Idempotent replay
survives a protected-state round trip without persisting the raw bearer.

Current evidence:

- the focused protocol suite has eleven passing tests;
- the duplicate-physical-profile and noncanonical-digest defects were each
  demonstrated by a failing regression before their fixes;
- external epoch rollback, unregistered owner authority, invented capability
  binding, and runtime-history serialization each have focused regressions;
- protected-state replay retains acquisition receipts without retaining the
  raw capability or consulting lease-event history;
- lease events remain serializable through a separate versioned history
  encoding rather than being silently discarded; and
- repository Rust formatting and strict Clippy pass.

This is a source-only private protocol seam. It does not yet provide complete
cross-collection load validation, a durable atomic store, protected IPC,
stable-supervisor custody, an independently durable external epoch source,
transactional operational-state and history publication, typed effect and
terminal operations, public administrator bootstrap and rotation, or installed
acceptance. The candidate remains noninstallable.

## Slice G Crash-Durable Authority Publication Checkpoint | 2026-08-31

State transition: `slice_g_protected_authority_protocol_resource_registry_in_progress`
to `slice_g_crash_durable_authority_publication_source_accepted_custody_in_progress`.

The private authority store now publishes immutable, content-bound
generations. Protected operational state and non-authoritative history have
separate manifests and digests. Files and generation directories become
durable before one atomic selected-generation pointer changes. Operational
load validates only the protected generation, so corrupt history degrades the
audit read without making current authority unavailable. Corrupt selected
authority fails closed and never falls back to an older generation.

Publication fault injection covers protected-state write, history write,
manifest write, and final generation publication. Every interruption leaves
the prior generation selected. Publication also compares the kernel's exact
loaded predecessor generation with the current selector. A stale writer
cannot reselect an older valid snapshot even when its domain and external
epoch remain valid.

Evidence:

- Red: a stale loaded kernel successfully repointed the selector to its older
  valid generation.
- Green: the same publication returns
  `lease_authority_protocol_store_stale_publication`; the newer resource state
  remains selected.
- All 17 focused protected-protocol tests pass, including four publication
  interruption boundaries, history-only degradation, corrupt-authority
  no-fallback, stale-publisher compare-and-swap, and read-versus-mutation
  history binding.
- Wrapper Rust formatting and strict Clippy pass.

This remains private source acceptance, not operating-system custody or an
installable authority service. Protected supervisor ownership, authenticated
IPC, an independently durable external epoch, bounded orphan-generation
reconciliation, platform filesystem qualification, transactional wiring of
every kernel mutation, public administrator bootstrap and rotation, concrete
effect-sink migration, mixed-version migration, and installed acceptance
remain open. No production install is authorized at this checkpoint.

## Slice G Protected Service Identity Checkpoint | 2026-08-31

State transition: `slice_g_crash_durable_authority_publication_source_accepted_custody_in_progress`
to `slice_g_protected_service_identity_source_accepted_custody_install_in_progress`.

The private protocol now has an operating-system custody adapter rather than
accepting caller-asserted service identity. On Linux it reads `SO_PEERCRED`
from the exact connected Unix stream, resolves the peer executable through
`/proc/<pid>/exe`, hashes its bytes, and inspects the socket and state-root
metadata. Custody requires a root peer, the exact peer PID, a root-owned
non-group-writable executable, a root-only state root, and a root-owned socket
with the exact configured operator group and mode. The endpoint identity is
content-bound to those observations, including socket device and inode.

The authority signs a nonce-bound identity response containing its domain,
authority epoch, boot epoch, executable digest, and endpoint identity. Client
verification requires both a valid enrolled Ed25519 key and equality with the
client's locally observed custody identity. A proof signed for a replaced
socket is not valid on the current connection.

Evidence:

- Red then green: a same-user service and peer cannot satisfy root custody.
- Red then green: a user-owned state root cannot hold operational authority.
- Red then green: candidate-owned sockets and candidate-writable executables
  cannot become the protected endpoint.
- Red then green: peer PID must match the exact process on the connected Unix
  stream, and changing the socket inode changes endpoint identity.
- Red then green: the real Linux inspector rejects a user-owned socket that
  speaks through a valid connected Unix stream.
- Red then green: the signed service challenge rejects endpoint replacement
  and signed-field tampering.
- All 25 focused protocol and custody tests pass. Wrapper Rust formatting and
  strict Clippy pass.

This checkpoint establishes the source enforcement contract but does not yet
run a root authority service. The root-owned supervisor executable and state
root, system socket activation or equivalent transport lifecycle, framed typed
request dispatch, request authentication, external epoch store, privileged
bootstrap and rollback generations, installer and doctor integration, and
installed adversarial acceptance remain open. The same-user production
runtime still lacks this custody boundary, so no production install is
authorized.

## Slice G Nonblocking Historical Archive Checkpoint | 2026-08-31

State transition: `slice_g_protected_service_identity_source_accepted_custody_install_in_progress`
to `slice_g_history_decoupling_source_accepted_custody_install_in_progress`.

The tenth recurrence audit exposed a contradiction in the first durable-store
implementation. Read-only authority loading did not parse history, but mutation
loading reconstructed the complete event vector from the selected history
file. A corrupt historical record therefore could not affect an existing
claim, yet it could still prevent the next ordinary acquisition or resource
registration. That violated the requirement that historical records are
operationally irrelevant.

The store now publishes a versioned immutable history segment with each
authority generation. Its manifest references the prior generation without
copying or parsing the prior segment. A mutation load binds the exact selected
protected generation and gains publication authority without opening history.
Publication compare-and-swap, current-state durability, and read-versus-mutate
capability separation remain enforced. Audit reads traverse the bounded,
cycle-checked segment chain; corruption anywhere degrades that audit read only.
The v2 manifest can link to a legacy v1 full-snapshot generation, preserving
source migration compatibility.

Evidence:

- Red: corrupting the selected generation's history made
  `load_for_mutation` fail with `lease_authority_protocol_history_unavailable`.
- Green: the same corrupt history permits a new protected resource mutation and
  durable publication; current authority loads with the new resource while the
  history read remains explicitly unavailable.
- A two-generation healthy chain returns the historical and current events in
  order without rewriting the first segment.
- A v2 segment linked to a legacy v1 full-snapshot history remains readable.
- All 26 focused protocol and custody tests pass. Wrapper Rust formatting and
  strict Clippy pass.

This is still private source acceptance. Segment retention and compaction,
root-service custody, framed authenticated IPC, current-state intent consume,
client capability custody, public administration, full effect-sink migration,
and installed acceptance remain open. No production install is authorized.

## Slice G Bounded Service Dispatch Checkpoint | 2026-08-31

State transition: `slice_g_history_decoupling_source_accepted_custody_install_in_progress`
to `slice_g_bounded_service_dispatch_source_accepted_root_service_in_progress`.

The private protocol now has a length-prefixed transport boundary capped at 64
KiB before payload allocation. Zero-length, truncated, oversized, unreadable,
and unwritable frames return typed protocol failures. One connection handler
reads exactly one request, dispatches it through the typed allowlist, and emits
one bounded response frame. Invalid and unsupported operations receive a typed
error response rather than invoking a fallback parser or generic signer.

The first implemented dispatch operation is `service_challenge`. It returns
only the nonce-bound signed service identity already tied to authority domain,
authority epoch, boot epoch, executable digest, and locally observed endpoint
custody. Generic `sign` and `mutate_state` operations remain unrepresentable.
Acquire, effect-intent consume, release, recovery, revoke, and inspect remain
explicitly typed but unavailable through the transport until each operation
has its complete authentication and durable-mutation contract.

Evidence:

- Red: the bounded frame constant, reader, and typed dispatcher did not exist.
- Green: an oversized length prefix is rejected after exactly four header
  bytes without reading or allocating the declared payload.
- A framed generic signing request returns
  `lease_authority_protocol_operation_unsupported` as a bounded error frame.
- A framed service challenge verifies against the exact nonce, local custody
  identity, authority domain, epoch, and enrolled Ed25519 verifier.
- All 29 focused protocol and custody tests pass. Wrapper Rust formatting and
  strict Clippy pass.

This checkpoint is a transport core, not a running authority service. Root-only
process entry, fixed protected paths, system socket lifecycle, peer request
authentication, per-peer rate and concurrency budgets, store and trust reload,
external epoch selection, administrator bootstrap, and installer integration
remain open. No production install is authorized.

## Slice G Root Authority Service Source Checkpoint | 2026-08-31

State transition: `slice_g_bounded_service_dispatch_source_accepted_root_service_in_progress`
to `slice_g_root_service_source_accepted_installer_bootstrap_in_progress`.

The binary now has an internal Linux authority-service process mode entered
before any user-scoped `.env` loading. Startup requires effective UID zero, a
PID greater than one, and an executable resolved under exactly one banked
generation below
`/usr/local/libexec/agent-browser/lease-authority/generations/`. A workspace,
user-home, selected-candidate, relative, or wrongly named executable cannot run
the protected service.

The process accepts exactly one systemd socket-activation descriptor whose
`LISTEN_PID` is the current process, whose `LISTEN_FDS` is one, and whose local
address is `/run/agent-browser/lease-authority.sock`. Custody then proves the
fixed `/var/lib/agent-browser/lease-authority` state root, socket owner, group,
mode, device and inode, current root process, and root-owned executable digest.
The service loads a root-private fixed configuration, pre-existing protected
store, and pre-existing selected trust generation. It reloads the selected
signer and protected authority for every connection and applies bounded read
and write timeouts.

The online process deliberately cannot initialize a missing store, trust root,
configuration, socket, signing key, resource registration, authority domain,
or external epoch. Those are bootstrap-coordinator responsibilities. Its only
currently executable request remains the signed service challenge; mutation
operations stay unavailable until their online current-state consume and
durable publication paths are complete.

Evidence:

- Red: no root service module or process entry existed.
- Non-root startup fails before consulting any installed path.
- Root startup accepts one exact banked-generation shape and rejects user,
  relative, parent-traversal, missing-generation, and wrong-executable paths.
- Socket activation accepts one exact current-PID descriptor and rejects
  missing, foreign-PID, zero, or multiple descriptors.
- Opening a missing online store fails without creating any filesystem path.
- The built source binary with the internal process marker exits one with
  `lease_authority_service_root_required` under the ordinary user and emits no
  standard output.
- All 33 protocol, custody, framing, history, and service tests pass. Wrapper
  Rust formatting, strict Clippy, and the source binary build pass.

This source is not installed or operational. The privileged bootstrap must
install immutable banked service generations, root state and trust, the root
configuration, socket and service units, exact verification and rollback, and
an independently durable external epoch selection without exposing a generic
NOPASSWD upgrade or signing oracle. Peer workload authentication, concurrency
and rate budgets, mutation dispatch, public administration, doctor integration,
and adversarial installed acceptance remain open. No production install is
authorized.

## Slice G Atomic Root Bootstrap Source Checkpoint | 2026-08-31

The protected executable now has a distinct one-shot bootstrap process marker.
Bootstrap runs before user environment loading, requires effective root, and
accepts only the same immutable banked executable identity as the online
authority service. It is not a public command and is not exposed through the
passwordless privileged helper.

The bootstrap publishes an entirely new authority root from one private staged
directory. It creates fresh domain and boot identities, a private Ed25519
signing generation, the corresponding verification keyring, an empty protected
authority store generation, and the root-private service configuration. The
final state-root rename is single-use: any existing state, including incomplete
or damaged state, is rejected rather than repaired or overwritten. The online
service continues to call `open_existing` and therefore cannot bootstrap itself.

Focused tests prove that the resulting trust and protected store can be loaded,
that repeated bootstrap cannot alter the selected generation, that an invalid
operator group creates no state, and that non-root or unbanked execution is
rejected. Repository formatting and strict Clippy pass for the source
checkpoint.

This checkpoint remains noninstallable. The next packet must bank the exact
reviewed binary, install fixed systemd socket and service units, invoke the
one-shot bootstrap only inside the explicit interactive sudo boundary, and
verify custody without adding any generic passwordless signer, state mutation,
bootstrap, or upgrade operation.

## Slice G Privileged Installer Bootstrap Checkpoint | 2026-08-31

The existing one-time privilege installer now owns first installation of the
protected authority. It selects an explicit reviewed executable, computes its
SHA-256 identity, installs it under the matching immutable generation path,
installs fixed hardened systemd service and socket units, invokes the one-shot
bootstrap after the operator group exists, and enables the socket. These actions
all occur after the installer's one interactive `sudo -v` authorization and use
only noninteractive continuations within that explicit boundary.

The installed contract validates the full service and socket unit content,
root ownership and mode, the banked path shape, and the banked file digest. A
healthy existing generation becomes the source of truth on rerun, so a newly
available candidate cannot cause self-admission. An exact inactive socket has
a bounded daemon-reload and enable recovery. Existing authority state paired
with changed units, changed banked content, or otherwise invalid artifacts is
rejected without replacing state or invoking bootstrap again.

The sudoers policy is unchanged and still names only the bounded RDP helper.
There is no passwordless authority bootstrap, signing, state-mutation, or
upgrade command. Clean-root fixtures prove exactly one interactive first-install
authorization, idempotent rerun without another authorization, one bootstrap,
socket recovery without rebootstrap or binary replacement, tamper refusal,
and compatibility with the full workstation dependency installer.

This is installer source acceptance, not installed-runtime acceptance. The
authority still exposes only its signed identity challenge. Peer workload
authentication, concurrency budgets, online mutation dispatch, public
administration, independent external epoch recovery, doctor coverage, and the
final exact binary installation remain open.

## Slice E Root Administrator Bootstrap Checkpoint | 2026-08-31

Fresh authority bootstrap now creates the first administrative identity in the
same atomic staged generation as trust, protected state, and service
configuration. The raw 256-bit administrator capability is written only to a
root-private administrator directory. Protected authority state stores its
digest, stable administrator id, revision, and active lifecycle state. The
root-private service configuration binds the same id and revision without
containing the capability.

Administrator enrollment is permitted only while the authority is completely
empty. It initializes authority revision one and cannot be replayed over an
existing claim, receipt, event, fence, or administrator. The shared
administrator authenticator now enforces id, revision, active state, minimum
secret strength, and exact capability digest; the older Service State helper
uses the same check rather than maintaining parallel authentication logic.
Protected-state loading also validates administrator map keys, digests, and
revision bounds.

Focused bootstrap tests prove that protected state and the root capability load
as one identity, that debug output redacts the raw capability, that a replaced
capability no longer authenticates, and that bootstrap remains single-use.
Formatting, focused tests, and strict Clippy pass.

This checkpoint does not yet expose administrator authority over IPC. The next
packet must add root-peer-authenticated, authority-timed revoke planning and
revision-bound apply, load the root capability only for that operation, publish
the resulting protected generation before success, and keep ordinary challenge
and lease operations independent of administrator-credential availability.

## Slice E Kernel-Authenticated Administrative Peer Gate | 2026-08-31

Every accepted Linux authority connection now captures the peer UID, GID, and
PID from `SO_PEERCRED` on that exact connected Unix stream. These fields are
not accepted from request JSON, environment variables, filesystem metadata, or
caller assertions. An invalid or processless peer fails before request dispatch.

The protocol now distinguishes `revoke_plan` from revoke apply and places both
behind a root-peer gate before either can reach an operation handler. A regular
member of the socket group may request the signed service challenge but receives
`lease_authority_protocol_administrator_peer_required` for either administrative
operation. A root peer reaches the typed operation boundary, which remains
explicitly not implemented until authority-owned time, capability loading,
single-use intent persistence, and durable apply publication land together.

Focused tests prove that the request peer is read from a real connected Unix
socket, that ordinary challenge dispatch remains available to a non-root peer,
that a non-root administrative request cannot reach its handler, and that root
reaches only the typed placeholder rather than a generic mutation surface.

This gate is necessary but not a public revoke surface. The next packet remains
the authority-timed, root-capability-authenticated revoke plan, followed by
revision-bound apply and crash-durable publication.

## Slice E Durable Administrative Intent Kernel Checkpoint | 2026-08-31

Administrative revoke planning is now a canonical authority mutation rather
than an offline signature operation. The kernel authenticates the current
administrator id, revision, lifecycle state, and raw root capability; binds the
plan to the exact resource, claim id, principal, claim revision, fencing token,
reason, issue time, expiry, and idempotency key; signs that envelope; increments
the authority revision; retains the authorization only in protected state; and
adds a bearer-free `revocation_planned` audit event.

An exact plan retry returns the original signed authorization without changing
authority revision or minting a proof under a newer signer. Reusing the
idempotency key for any changed target, fence, reason, administrator, or time is
an idempotency conflict. Revoke apply now requires the signed authorization to
equal the exact retained protected intent. A valid signature created outside
that current-state planning transaction is rejected before it can fence a
claim. Terminal apply replay remains available from its durable receipt even
after intent compaction or administrator rotation.

The ordinary user-scoped Service State serializer skips retained administrative
authorizations entirely. The protected serializer persists them for online
consume, while the separate history segment receives only the redacted audit
event. Protected load validates every retained intent's key, schema, signer
identity shape, administrator revision, claim axes, bounds, and expiry span.

Focused tests prove offline-signature rejection, exact plan replay, unchanged
revision on replay, holder-capability independence, terminal apply replay,
protected restart persistence, projection redaction, bearer-free history, and
malformed retained-intent rejection. Authority-owned clock observation and the
root-service plan/apply dispatcher remain required before this becomes a public
operation.

## Hotfix Recurrence Structural Audit | 2026-08-31

The known hotfix reports are covered by the frozen model, but they are not yet
structurally impossible in the product. The current source has the protected
kernel, atomic acquisition primitive, fenced claim model, durable authority
publication, service identity challenge, root peer gate, administrator
bootstrap, durable administrative revoke, and strict-controller recovery.
Production protected dispatch still leaves effect authorization, exact-holder
release, and inspection unimplemented, while CLI, HTTP, MCP, generated-client,
dashboard, and shared-skill recovery parity remain open. Browser spawn,
runtime-owner transfer, profile and session mutation, physical-collision
denial, and installer selection are not yet exhaustively sealed behind the
kernel.

The historical incident families map to the following mandatory invariants:

| Reported failure | Structural prevention contract | Current disposition |
| --- | --- | --- |
| Retained sessions, terminal owners, or lease warnings block ordinary work | 1, 2, 24, 37, 42, 49, 63, 65, 80, 97, and 99 | Authenticated cold access planning, profile selection, legacy lease admission, prelaunch lifecycle admission, and postlaunch registration now treat retained rows as observations; remaining entrypoints and projections are not yet exhaustively sealed |
| An absent client or crashed worker leaves a permanent lease | 7, 25, 45, 61, 63, 65, 66, 84, and 89 | Modelled; authority clock and stable deadline reconciler remain |
| An abandoned strict lease requires a hotfix or raw state edit | 9, 10, 23, 40, 57, 64, 69, 88, and 102 | Protected recover and revoke exist; administrator replacement, public parity, and disaster recovery remain |
| Access plan says launch is executable, then execution rejects an invented session or owner identity | 12, 13, 35, 37, 38, 41, 42, 47, 60, 73, 80, and 95 | The authenticated cold route now carries its exact observed registry generation through selection and legacy lease admission; a shared planner, acquire, admission, and denial dispatcher remains |
| `closing`, `prepared`, or `transferring` survives its owner indefinitely | 8, 16, 29, 39, 54, 61, 71, 76, and 89 | Stale transfer rows no longer block authenticated cold planning or launch, and successful registration supersedes the exact observed generation; canonical transfer saga and supervisor reconciliation remain |
| Dashboard or read reconciliation creates fictitious browsers or ready-looking owners | 6, 17, 32, 41, 63, 80, 94, 97, and 103 | Modelled; projections remain to be made receipt-only |
| Candidate upgrade blocks itself or old and new generations disagree | 20, 35, 46, 52, 60, 83, 87, 90, 93, 98, and 103 | Protected supervisor foundation exists; installer ownership and mixed-version migration remain |
| Retried requests create duplicate daemons, browsers, routes, or external effects | 18, 19, 28, 39, 44, 54, 55, 76, 77, 78, and 79 | Authority receipts exist in part; single-flight spawn and every effect sink remain |
| A browser, daemon, helper, or restart loop multiplies until workstation resources are exhausted | 76, 78, 106, and 115 | Modelled; generation-bound operating-system containment, reserve protection, and installed pressure acceptance remain |
| Logical lease records are mistaken for live process, lock, route, display, or socket evidence | 11, 17, 37, 43, 56, 63, 71, 72, 96, 105, and 107 | Modelled; fresh physical-evidence permits and bounded resolution remain |
| Concurrent or stale Service State writers lose a mutation or manufacture a blocker | 35, 44, 51, 52, 79, 94, 98, 99, and 103 | Protected generation compare-and-swap exists; all production writers remain to converge |
| Lock convoy, inversion, cancellation, or crash turns internal serialization into a generic timeout or global denial | 39, 79, 103, 106, 111, 113, and 114 | Historical lock ordering repair exists in the legacy store; the protected authority, projections, and effect sinks still need one enforced lock graph and crash tests |
| A broker wait is hidden by an equal or shorter client timeout, erasing typed recourse | 39, 78, 89, 106, 109, 111, and 113 | Historical failure identified; unified deadline budget, waiter abandonment, and adapter outcome fidelity remain |
| An observational access plan is presented as an executable promise and then reinterpreted by a second gate | 12, 13, 35, 38, 41, 47, 73, 95, 110, and 112 | Snapshot semantics exist in the model; reserved offers and public wording or schema separation remain |

No historical incident family identified in Plans 0128, 0134, 0137, 0142, or
0143 requires a second authority model or an exception to these invariants.
Profile registration, authentication, and readiness remain eligibility or
product-state facts. They never constitute a lease holder, process, session,
or blocker. Conversely, a lease claim never proves that a profile is
authenticated or that a browser process exists.

Structural closure requires all of these proof gates, not only regression
tests for the examples above:

1. an exhaustive denial-gate manifest proves that every ordinary denial is
   constructed only from a current claim conflict, fresh exact physical
   collision, authority outage, bounded capacity outcome, or non-authority
   product error;
2. an exhaustive low-level effect-sink manifest proves that every browser,
   profile, process, route, display, session, tab, installer, and input mutation
   consumes a kernel authorization or typed physical-safety permit;
3. dependency and visibility tests prove no production module can mutate the
   active index, counters, registrations, owner bindings, capabilities,
   receipts, verifier trust, or selected authority generation except through
   authenticated kernel IPC;
4. a reference-model and fault-injection suite proves arbitrary crashes,
   response loss, replay, expiry, suspend, restart, transfer interruption,
   store faults, and mixed-version ordering preserve one authority, bounded
   liveness, and unique receipts;
5. migration tests inject arbitrary terminal and ambiguous legacy history and
   prove byte-for-byte identical planning and admission outcomes before and
   after that history is added; and
6. installed acceptance proves ephemeral abandonment, strict administrative
   recovery, transferring-deadline recovery, candidate rollback, duplicate
   retry suppression, zero process residue, historical-warning isolation, and
   an ordinary authenticated client using its own profile without a retained
   session or runtime-owner prerequisite; and
7. public-adapter and consumer tests prove one end-to-end deadline budget,
   mandatory response reserve, abandoned-waiter expiry, observational-plan
   labeling, reserved-offer execution, and lossless structured recourse across
   CLI, HTTP, MCP, generated-client, skill, and subprocess boundaries; and
8. lock-graph and operating-system containment tests prove no authority or
   projection lock crosses an external-effect wait, cancellation or panic
   cannot strand in-process custody, every managed process is in the selected
   generation's bounded process tree, fork and restart storms preserve the
   recovery reserve, and pressure produces one typed bounded outcome.

Until all eight gates pass against one exact installed candidate, the correct
claim is that the design closes the known failure taxonomy and the kernel is
being built toward structural prevention. It is not yet correct to claim that
the bugs cannot recur.

## Slice E Protected Revoke Plan And Apply Checkpoint | 2026-08-31

The root authority service now implements typed `revoke_plan` and `revoke`
dispatch instead of returning the administrative placeholder. A plan request
contains only the canonical resource, claim id, claim revision, fencing token,
idempotency key, and reviewed reason. Caller-supplied issue or expiry time is
rejected by the closed request schema. The protected service observes current
time, advances a durable nondecreasing authority-time floor, chooses the
bounded 120-second authorization lifetime, authenticates the current root
administrator capability, and persists the exact signed intent before it
returns a proof-redacted plan id.

Apply accepts only that plan id. The kernel resolves the exact protected
authorization, revalidates administrator authority, signature, expiry, current
claim revision, and fencing token, advances the resource fence, removes the
active claim, and durably publishes the terminal receipt before replying.
Exact plan replay returns the original plan even after the target claim has
become terminal. Exact apply replay returns the original terminal receipt after
restart and after the short plan lifetime has elapsed. A changed claim revision
fails before fencing with `stale_claim`.

The service reads the root administrator credential only after the connected
Unix peer is proven root and the decoded operation is administrative. A normal
group-member service challenge neither reads nor depends on that credential.
Missing or invalid administrative custody returns a typed per-request error
instead of terminating the protected service. Administrative business errors
still publish an advanced authority-time floor before their response, so an
observed clock advance cannot be forgotten and later lengthen an authorization.

Focused tests prove root peer gating, caller-time rejection, nondecreasing time
across protected restart, exact plan replay, terminal plan replay, durable plan
and apply publication through the production service wrapper, stale-plan
rejection, proof redaction, late apply replay, and challenge independence from
administrator-credential availability.

This checkpoint exposes the protected IPC semantics only. CLI, HTTP, MCP,
generated client, dashboard, documentation, and shared-skill parity remain in
Slice G. Full suspend and reboot fencing, signer and administrator rotation,
loss recovery, bounded intent compaction, and the strict recovery-controller
plan and apply path also remain. Production installation is still withheld.

## Slice E Protected Strict Recovery Plan And Apply Checkpoint | 2026-08-31

The protected authority service now implements typed `recover_plan` and
`recover` operations for the exact recovery controller named by a strict
claim. The request schema accepts the raw controller capability, canonical
resource and claim axes, idempotency key, and requested successor owner
generation. It rejects caller-supplied time. The authority service chooses the
120-second authorization lifetime, 60-second transition deadline, and
300-second recovered-claim lifetime from its durable nondecreasing time floor.

Recovery planning authenticates the live controller capability against the
protected principal registry and current strict claim. It persists the exact
signed authorization in protected operational state before returning a
proof-redacted plan id. Recovery apply accepts only that plan id plus the raw
controller capability, authenticates the controller again, requires byte-exact
agreement with the retained authorization, revalidates the current claim and
fence, and atomically advances the claim revision, fencing token, transition
deadline, owner generation, event, and recovery receipt.

The production service classifies both recovery operations as protected
mutations, obtains a compare-and-swap mutation load, and publishes the new
generation before returning the result. Recovery intent and terminal receipt
survive restart. An exact completed apply replays the original receipt instead
of recovering twice. A wrong controller, altered plan, stale claim, expired
plan, or missing retained authorization cannot mutate authority.

Focused recovery and lease-authority tests pass for durable protected intent,
proof and capability redaction, wrong-controller rejection with zero state
change, caller-time rejection, restart apply, restart terminal replay, stale
effect fencing, and recovery replay after controller lifecycle change.

This closes the protected strict-controller IPC placeholder, not the whole
hotfix taxonomy. Administrative recovery-controller replacement, bounded
intent compaction, hierarchy and parent fencing, runtime-owner transfer,
single-flight process spawn, receipt-only Service State projection, exhaustive
denial and effect manifests, model and crash testing, mixed-version migration,
and exact installed acceptance remain mandatory. Production installation is
still withheld.

## Slice F Nonblocking Runtime-Owner History Checkpoint | 2026-08-31

The access planner no longer converts a nonterminal runtime-owner or lifecycle
row into `lifecycle_owner_blocks_replacement`. A `ready`, `closing`,
`prepared`, or `transferring` observation that cannot be joined to reusable
current authority remains visible under `decision.lifecycleReplacement`, but
it cannot erase the cold-launch request, invent a required session route, set
an acquisition blocker, or demand reconciliation before ordinary work.

The regression reproduces the reported failure with a generation-bound
`transferring` lifecycle row, no live PID, and no active session. It proves
that the planner's recommended action, profile-reuse decision, and copyable
service request are identical with and without that retained owner history.
The public schema and generated client no longer advertise
`blocked_by_lifecycle_owner` as a planner recommendation. The retained
`blockedByLifecycleOwner` boolean is compatibility-shaped and is false for
current plans.

This change deliberately does not weaken the physical safety boundary. An
exact current process, profile lock, socket, route, display, or canonical
active claim may still produce its own typed denial at the responsible gate.
A runtime-owner row or browser projection does not prove any of those facts.

The legacy owner registry still coordinates transfer effects and therefore
remains a competing authority implementation below the planner. The next
Slice F work must move prepare, commit, abort, reverse, owner-generation
fencing, and cleanup accountability into the protected kernel and leave the
Service State registry as receipt-driven projection only.

## Slice F Planner And Executor Coherence Checkpoint | 2026-08-31

The authenticated cold execution route now consumes the same exact
runtime-owner registry revision, owner id, and owner generation that the
access planner observed. A retained nonterminal owner is no longer interpreted
as proof of a live holder during profile selection. Historical session and
browser rows also cannot reintroduce a legacy profile-lease veto after the
authenticated plan has been copied by the service adapter.

The prelaunch lifecycle check now treats the runtime-owner registry as an
observation surface. It does not deny Chrome startup from a retained owner row.
The browser's process-level profile lock remains the physical collision gate.
If startup succeeds, managed-lane registration compare-and-swaps against the
exact observed owner generation, advances the generation, publishes the fresh
process identity, and removes the superseded lifecycle projection. A concurrent
owner change makes that compare-and-swap fail, so a stale daemon cannot acquire
effect authority from the supersession.

Provider-free regressions reproduce generation 57 with a transferring
lifecycle, no process identity, and a retained exclusive session row. They
prove authenticated profile selection and legacy lease admission stay ready,
prelaunch history cannot veto, successful registration advances to generation
58, and the old lifecycle projection is no longer current.

This closes the concrete planner/executor disagreement behind the reported
`existing_session_profile_identity_unproven` and
`runtime_lifecycle_existing_owner_requires_explicit_transition` sequence for
the authenticated cold route. It does not establish global structural
closure. Unauthenticated and legacy entrypoints still reach compatibility
gates, the runtime-owner registry still authorizes effects and transfer, and
the exhaustive denial manifest, kernel-owned spawn, physical permits,
receipt-only projections, reference-model faults, migration matrix, and exact
installed acceptance remain open.

The broader regression run exposed an additional public-surface defect. The
legacy `service_profile_lease_recover_plan` wrapper signs and returns a strict
recovery authorization from Service State, but recovery authorizations are
intentionally excluded from Service State serialization. Its paired apply
therefore rejects the freshly returned plan as
`lease_authority_invalid_recovery_proof`. Persisting that bearer in Service
State would violate the protected-state and bearer-free-history invariants.
The correct repair is to route public CLI, HTTP, MCP, generated-client, and
dashboard recovery plan/apply through the protected authority service's
already durable `recover_plan` and `recover` protocol, returning only its
proof-redacted plan id to untrusted projections. Until that parity is wired,
the legacy public wrapper is not an accepted strict-recovery surface and the
full `service_profile_lease` filter remains red on its public recovery test.

The false wrapper has now been removed. Until protected parity exists, every
legacy CLI, HTTP, MCP, generated-client, and dashboard recovery plan/apply
request fails before mutation with
`lease_authority_protected_recovery_surface_required`. The regression proves
the exact strict claim and all Service State bytes remain unchanged. The full
41-test profile-lease filter is green again without persisting or returning an
unexecutable bearer.

This fail-closed checkpoint is not the final recovery surface. Protected
recovery cannot be enabled by forwarding only the plan call: the protected
authority domain must first own profile principal registration, physical
resource registration, and claim acquisition. Bootstrap currently begins with
empty principal and resource registries. The next integration order is
therefore registration, resource binding, acquisition/effect
authorization/release, then public recovery plan/apply. Skipping that order
would recreate split authority with matching method names.

## Slice F Protected Acquisition Authority Checkpoint | 2026-08-31

Protected `acquire` dispatch is now implemented. The wire request contains the
raw profile capability, canonical resource, optional parent, requested mode,
expected resource revision, idempotency key, and optional recovery controller.
Its closed schema rejects caller-provided current time, expiry, transition
deadline, boot epoch, and owner generation. The protected kernel authenticates
the capability against its own principal registry, requires an explicitly
registered physical resource, advances its durable authority-time floor,
chooses the bounded claim and transition deadlines, and derives boot and owner
generation evidence from protected state.

The production service classifies acquisition as a mutation, so its selected
generation compare-and-swap and durability barrier complete before the framed
response is returned. The response carries only the acquired claim, durable
receipt, and replay marker. It never echoes the raw capability. Focused tests
prove rejection of caller-owned authority evidence, authenticated identity
derivation, unregistered-resource rejection, durable replay, framed response
encoding, capability redaction, and the complete 34-test protocol partition.

This is a prerequisite, not a public acquisition path. Effect authorization,
exact-holder release, inspection, resource-scoped fault containment,
freshness-bound public readbacks, and broker-managed zero-choreography
ephemeral acquisition remain. Production installation is still withheld.

## Slice F Protected Profile Enrollment Checkpoint | 2026-09-01

The protected service now owns first-time profile principal and physical
resource enrollment. A group-authorized peer supplies only the profile name,
absolute profile path, raw capability, expected per-resource revision, and
idempotency key. The service derives the principal from the kernel-observed
peer UID, canonicalizes the path, verifies that the target is a directory owned
by that UID and is not group- or world-writable, derives the canonical physical
identity digest, and selects registration time from the durable authority
clock. Caller-supplied principal ids, physical digests, UIDs, and timestamps
are rejected by the closed protocol schema.

Enrollment atomically stages the protected principal registry, one-to-one
physical resource registration, and a durable replay receipt. The raw
capability and private profile path are absent from protected state and framed
responses. A lost response replays only for the exact operation, capability,
UID, canonical physical identity, and expected resource revision. Path aliases
resolve to one digest, a different UID cannot enroll the directory, and an
unregistered profile still cannot acquire authority.

The receipt validator deliberately does not require its historical capability
or physical binding to remain current. Later capability rotation or an
administrator-authorized resource rebind cannot turn a completed enrollment
receipt into an authority-store outage. The receipt remains self-contained,
content-bound replay evidence while current registration remains a separate
operational collection.

Service-level enrollment testing exposed a remaining global compare-and-swap
axis in acquisition. Bootstrap administration advanced the authority revision
and made the first acquisition of an unrelated new profile fail with
`stale_authority_revision`. Acquisition now compares the caller's exact
`expectedClaimRevision` with only the current unexpired claim for that canonical
resource. An empty resource expects zero regardless of unrelated administrator,
profile, receipt, or claim activity. Same-resource contenders still admit one
winner and return `stale_claim_revision` to the stale contender.

Focused evidence covers closed enrollment request fields, UID and permission
binding, canonical alias identity, atomic state and receipt persistence,
capability and path redaction, restart replay, later rotation and rebinding
compatibility, enrollment followed by acquisition through the protected
service, same-resource contention, and unrelated-authority noninterference.
Public CLI, HTTP, MCP, generated-client, dashboard, and shared-skill enrollment
are intentionally absent. The next step is protected effect authorization and
exact-holder release, followed by a broker that makes enrollment and ephemeral
acquisition internal to ordinary Agent Browser work.

## Slice F Protected Exact-Holder Release Checkpoint | 2026-09-01

Protected `release` dispatch now performs the complete exact-holder mutation
inside the selected authority generation. The caller supplies only its raw
profile capability, canonical resource, exact claim id, claim revision,
fencing token, and operation idempotency key. The closed request schema rejects
caller-provided time, principal, capability identity, and authorization
envelopes. The authority owns time, authenticates the capability against its
protected principal registry, matches every current claim axis, creates the
narrow `lease_release` authorization internally, advances the fence, and
persists the terminal receipt before replying.

Lost-response replay is independent of the now-absent active claim and does
not require the holder capability to remain current. The replay key must still
match the exact resource, claim, revision, and released fence. It returns the
same non-authoritative terminal receipt without advancing the authority
revision or recreating authority. A colliding operation key returns a typed
idempotency conflict. Raw capability material is absent from protected state,
history, receipts, debug output, and framed responses.

The focused service regression demonstrates the defect before the change as
`lease_authority_protocol_operation_not_implemented`, then proves enrollment,
acquisition, release, durable publication, exact replay, unchanged authority
revision on replay, and no remaining active profile claim. Protected effect
authorization remains intentionally unimplemented because returning the old
offline bearer would preserve a revocation race. The next slice must persist a
single-use current-state effect intent and make every selected effect sink
consume that intent before execution.

## Slice F Protected Durable Effect Intent Checkpoint | 2026-09-01

Protected `authorize_effect` now commits one exact effect intent in the
selected authority generation before returning its signed authorization. The
closed request carries only the raw profile capability, canonical resource,
exact claim id, claim revision, fencing token, action class, audience, and
operation idempotency key. Caller-provided principal, capability identity,
executor identity, timestamps, expiry, or proof are rejected. The kernel owns
time, authenticates the profile capability, revalidates the current claim and
fence, and derives the executor identity from the connected Linux peer's UID,
GID, PID, process start time, canonical executable, and executable digest.

The durable effect receipt is namespaced by authority domain, principal,
canonical resource, action, and operation key. It binds the authority epoch,
capability revision, exact claim axes, action, audience, executor UID and
process identity, occurrence time, authorization expiry, and authority
revision. The executor digest is part of the signed authorization payload. A
different process cannot replay the operation, and changing the signed
executor digest invalidates the proof. The first successful authorization
delivery is single-use. Exact restart replay returns the same receipt without
advancing the authority revision, but never returns another executable bearer:
after delivery the kernel cannot distinguish a lost pre-effect response from a
crash after the external effect, so replay requires reconciliation rather than
permission to launch again.

This checkpoint deliberately exposes only the currently modeled
`browser_launch` action with a bounded `daemon-session:` audience. It rejects
arbitrary action strings instead of becoming a holder-scoped generic signing
oracle. The protected operational collection is bounded at 4,096 receipts and
returns a typed capacity outcome before mutation.

The defect was first demonstrated by the protected service regression failing
to compile because no durable `effect_receipts` collection existed. Focused
tests now prove closed request fields, durable publication before reply,
bearer-free restart replay, principal and executor namespacing, proof tamper
rejection, scope rejection, capability redaction, and compatibility with the
existing profile recovery and lease callers.

This is not yet an effect-sink migration or final invariant 101 acceptance.
The selected browser-launch sink must consume the committed intent and persist
completion, uncertainty, or compensation through the same operation receipt.
Legacy callers still construct authorizations without an executor digest and
must be removed from effectful paths. Capability custody, receipt compaction,
the exhaustive effect manifest, and broker-managed ordinary acquisition also
remain before production installation.

## Slice F Protected Effect Terminalization Checkpoint | 2026-09-01

Protected `complete_effect` now moves an exact consumed effect intent to either
`completed` or `uncertain` inside the selected authority generation. The
closed request contains only the receipt id, terminal result, bounded evidence
digest, and completion idempotency key. The service derives the executor from
the connected peer and selects completion time and terminal authority revision
inside the kernel. Caller-provided executor identity, UID, time, or
authorization bearer is rejected.

The first terminal transition requires the exact executor UID and process
identity bound by authorization. It commits the result, evidence digest,
completion key, completion time, and terminal authority revision before reply.
It also scrubs the executable authorization from protected state. Exact replay
returns the same bearer-free receipt without advancing authority revision;
changed result, evidence, or completion key conflicts. Re-authorizing the
original operation after terminalization returns only its terminal receipt and
no executable authorization, preventing a completed or uncertain effect from
being launched again after response loss or restart.

The focused 78-test protected-authority partition, strict Clippy, Rust format,
docs build, and diff hygiene pass. This establishes the protected IPC and state
transition required by invariant 39, but no browser-launch sink consumes it
yet. Compensation, bounded uncertainty reconciliation, selected sink
integration, the exhaustive sink manifest, and the newly identified
end-to-end deadline and outcome-fidelity invariants 111 through 113 remain.
Production installation is still withheld.

## Slice F Single-Use Effect Delivery Correction | 2026-09-01

The sink trace found that a consumed but nonterminal effect receipt still
returned its original executable authorization on exact replay. That behavior
was safe only when response loss was known to occur before the effect. After a
daemon crash, the kernel cannot know whether Chrome started before completion
was recorded, so replaying the bearer could create the duplicate process storm
this plan is intended to make unrepresentable.

The protected kernel now returns the durable consumed receipt and
`authorization: null` on every authorization replay. The first delivery remains
executor-bound and effect-capable; every later delivery is evidence-only and
must enter bounded physical reconciliation. A service-level regression first
failed by observing the repeated bearer, then passed after the correction.
Selected sink integration must treat a consumed replay as
`effect_uncertain`/inspect-before-retry, not as an authorization outage and not
as permission to request a new operation key automatically.

## Slice F Protected Effect Client Adapter Checkpoint | 2026-09-01

The Linux daemon now has one internal client adapter for the protected
browser-launch consume and terminal-completion protocol. It connects only to
the fixed lease-authority Unix socket, applies a two-second read and write
budget, derives the socket group from the connected root-owned inode, and
validates the exact peer through `SO_PEERCRED` plus root-owned state root,
socket mode, socket device and inode, executable ownership, executable mode,
and executable digest. A same-user candidate socket or daemon cannot satisfy
that custody check.

The adapter constructs the closed `authorize_effect` request without caller
time or executor identity, validates that the first response matches the exact
resource, claim, fence, action, audience, and operation, then retains only the
non-authoritative receipt id. The signed executable bearer is discarded at the
adapter seam and cannot enter a command, log, job, or Service State projection.
Receipt-only replay returns
`lease_authority_effect_uncertain_inspect_before_retry`. Exact completed or
uncertain terminal responses must match the original receipt and requested
state.

## Slice F Broker-Managed Ephemeral Acquisition Checkpoint | 2026-09-01

The protected acquisition protocol no longer requires an ordinary ephemeral
caller to discover or submit the current claim revision. When
`expectedClaimRevision` is omitted for an ephemeral request, the protected
kernel selects the current revision at authority-owned time inside the same
serialized mutation that acquires or rejoins the claim. A same-capability new
operation rejoins the exact current claim without advancing its fence,
revision, or expiry. An expired claim is ignored and terminalized by normal
acquisition. A current foreign claim still returns the canonical conflict.

Strict acquisition deliberately retains explicit compare-and-swap semantics.
Omitting `expectedClaimRevision` for a strict request fails before mutation
with `lease_authority_protocol_strict_expected_revision_required`; strict
software remains responsible for its recovery controller and revision-aware
workflow.

The Linux protected client adapter now supports exact profile enrollment and
broker-managed ephemeral acquisition. Its closed ordinary acquisition request
contains no session identity, daemon route, owner generation, caller expiry,
heartbeat, recovery controller, or expected claim revision. Profile enrollment
lets the root service derive operator UID and canonical physical identity from
the connected peer and exact path. Client debug output redacts both capability
and profile path, and acquisition replay after expiry returns a typed
no-current-claim outcome rather than reviving authority.

This is not yet the public acquisition cutover. The current
`acquire_profile_command` still creates a legacy Service State claim and issues
a legacy effect authorization. The protected enrollment, acquisition, effect
consume, browser launch, and terminal completion must replace that entire
sequence in one slice; passing the legacy claim into the protected adapter is
forbidden because it would preserve two authority implementations.

Two focused client tests cover request closure, secret-safe debug output,
first-delivery matching, replay uncertainty, and exact terminal response
matching. The adapter is deliberately not exported to legacy profile
acquisition yet: that path still acquires its claim from user-scoped Service
State, while the protected service recognizes only protected claims. The next
slice must migrate enrollment and acquisition or provide one kernel-owned
broker operation before invoking this adapter; passing a legacy claim into it
would recreate split authority.

## Slice F Atomic Browser Owner Commit Checkpoint | 2026-09-01

A successful protected browser launch can no longer be terminalized as a
generic completed effect. The executor must submit the launched PID through
`complete_browser_launch`. On Linux, the authority derives the process owner,
direct parent, start token, canonical executable path and digest, and canonical
profile identity from `/proc`. It rejects a process owned by another UID, a
process not parented by the connected executor, a missing profile argument, an
unregistered profile identity, or a conflicting current owner.

The kernel commits the completed effect receipt and the exact runtime-owner
binding in one selected authority generation before replying. The owner binds
the logical browser id, daemon route, process instance digest, PID, start token,
executable path and digest, principal, capability, and owner revision. Exact
lost-response replay returns the same completed receipt and owner without
advancing authority. Generic `complete_effect` may still report uncertainty,
but it cannot report successful `browser_launch` completion without registering
the owner.

A current protected owner now prevents another launch authorization for the
same profile. A distinct operation also cannot bypass a consumed launch intent
that still requires reconciliation. Exact authorization replay remains
evidence-only and never returns a second executable bearer. An unchanged
durable mutation publication is an idempotent no-op, so persisting the
authority time floor after a rejected request cannot manufacture a generation
whose predecessor is itself or mask the original typed denial.

The protected-authority partition passes 82 tests. This checkpoint closes the
kernel transaction between one selected launch effect and its first owner
record. It does not yet prove that the production Chrome spawn is the selected
sink, that an exited owner is reconciled, that transfer and cleanup ownership
are kernel-coordinated, or that every owner projection is receipt-only.

## Fifth Hotfix Recurrence Gap Audit | 2026-09-01

The earlier recurrence matrix covers the reported symptoms, but six
cross-cutting mechanisms need explicit acceptance contracts before the design
can claim structural prevention rather than strong regression resistance:

1. **Process-proof lifetime and replacement.** A PID, command line, path digest,
   and start token are a point-in-time observation. The selected sink must hold
   a stable process handle where the platform supports one, validate an allowed
   browser executable generation, and make exit reconciliation incapable of
   reviving or transferring the prior owner.
2. **Bounded operational-state compaction.** Receipt and intent capacity must
   not become a permanent self-denial after enough successful work. Compaction
   needs a kernel-owned watermark, replay-retention contract, crash-safe
   publication, and a typed bounded recovery path that preserves audit history
   outside the active index.
3. **Atomic multi-resource admission or compensation.** Profile, process,
   session, route, display, stream, and input claims form one operation. The
   kernel must either reserve the required set in a globally ordered bundle or
   durably compensate partial acquisition. A client crash between independent
   claims must not strand a usable profile behind unrelated infrastructure.
4. **Authority outage and disaster recovery.** Ordinary work may fail closed
   while current authority is unavailable, but the product must provide bounded
   service self-recovery and an out-of-band administrative path. Restoring a
   backup or replacing administrator credentials must advance a trusted epoch
   and must not reactivate claims or owners from the restored history.
5. **Upgrade, rollback, and schema coexistence.** A candidate must prove it can
   read the selected authority schema before any mutation or drain. An older
   rollback binary must not reinterpret newer state, and mixed generations must
   have exactly one effect-capable authority while candidate observation and
   cancellation remain bounded.
6. **External-effect commit gap and orphan adoption.** Every selected sink must
   handle crashes before the effect begins, after the effect begins but before
   terminal publication, and after publication but before the caller receives
   the receipt. A generation-bound process handle, operation journal, provider
   idempotency key, or effect-specific probe must let the reconciler prove one
   of absent, exactly adopted, compensated, or durably uncertain before another
   authorization can exist. Effect history and replay of a consumed bearer are
   never sufficient proof. For effects that cannot be probed or made
   idempotent, uncertainty must remain terminal and must not trigger an
   automatic retry.

These mechanisms extend, rather than replace, the eight structural proof gates
in the hotfix recurrence audit. The truthful current disposition remains that
the design has no known incident family requiring another authority model, but
the product does not yet make recurrence structurally impossible. That claim
requires production cutover, exhaustive denial and sink manifests, the six
contracts above, fault and migration suites, and exact installed acceptance.

## Slice F Protected Owner Reconciliation Checkpoint | 2026-09-01

The protected authority now owns the transition from a process-backed browser
owner to historical reconciliation evidence. A profile-capability holder may
submit only the exact resource, expected owner id and generation, and an
idempotency key. The caller cannot assert a PID, process digest, liveness
result, observation time, or reconciliation evidence.

On Linux, the root authority reads the recorded PID from protected owner state
and compares its current UID, start token, canonical executable path and
digest, canonical profile identity, and process-instance digest with the
committed owner. The request is rejected while that exact process remains
current. An absent, reaped, zombie, or PID-reused process produces
authority-derived stale evidence. Permission or observation failures remain
typed failures and cannot be reinterpreted as process absence.

Successful reconciliation removes the active owner and commits its immutable
receipt, evidence digest, authority revision, owner revision, and
`owner_reconciled` history event in one selected generation before reply.
Exact replay is independent of the now-absent owner and cannot recreate
authority. Owner bindings now retain their source claim id, claim revision,
and fence, and protected-state validation requires every active or reconciled
owner to trace to one completed browser-launch effect receipt.

Owner generations are monotonic across reconciliation. The first replacement
after generation 1 is generation 2 rather than another generation 1, so a
historical transfer or effect receipt cannot become current after the active
map is emptied. Focused kernel, physical-process, client-closure, and durable
service tests pass for live refusal, process replacement, durable removal,
lost-response replay, and next-generation launch.

This checkpoint supplies the crash-recovery primitive required by the selected
sink cutover. It is not yet supervisor-driven reconciliation, public
`service_profile_acquire` migration, runtime-owner transfer migration, receipt
compaction, or receipt-only Service State projection. Production installation
remains withheld.

## Sixth Hotfix Recurrence Gap Audit | 2026-09-01

The selected protected launch sink exposed another cross-boundary ordering
requirement. After the authority durably commits a browser owner, every local
custody record needed to reconcile that owner must be retained before any
fallible compatibility or observability projection runs. Otherwise a Service
State write failure can leave a real process and canonical owner without the
daemon retaining the exact capability required to close or reconcile it.

The launch path now stores the protected owner lease before projecting browser
health. A focused regression injects projection failure and proves that exact
owner id, generation, profile, and secret capability custody remain available.
This closes the immediate ordering defect, but structural acceptance still
requires the same commit, custody, projection, and reply failure injection at
every protected effect sink. Derived projection failure must never erase
canonical custody, authorize a retry, or become evidence that no owner exists.

## Slice F On-Demand Crash Reconciliation Checkpoint | 2026-09-01

The protected acquisition dispatcher now authenticates the exact profile
capability before consulting current browser-owner process state. When a
committed owner exists, the root authority derives process liveness from its
own protected binding. An exact current process returns the typed
`owner_process_still_current` conflict without changing authority. An absent,
zombie, or replaced process is reconciled from root-derived evidence before
the acquisition proceeds, using an idempotency key bound to the acquisition,
owner id, owner generation, and process instance.

This makes the next authenticated acquisition a safe convergence trigger after
an ephemeral daemon and its browser have disappeared. No historical session,
runtime-owner projection, client liveness assertion, or retained warning
participates. Focused tests prove stale-owner removal followed by acquisition,
live-owner refusal without revision change, and authentication before owner
state disclosure.

This checkpoint does not yet adopt a still-running orphaned browser, run a
periodic supervisor scan, or provide resource-scoped quarantine. Those paths
remain required so a live process whose daemon vanished has bounded adoption
or cleanup recourse rather than an accurate but indefinitely unresolved
physical-owner conflict.

## Slice F Public Protected Acquisition And Launch Cutover | 2026-09-01

Linux `service_profile_acquire` now uses the protected root authority for the
public broker-managed path. The route resolver enrolls the exact configured
profile identity and derives a cold daemon route from the protected principal
and profile. Acquisition obtains a broker-managed ephemeral claim, authorizes
one `browser_launch` effect for that daemon audience, launches the exact
profile through the selected local Chrome sink, and atomically completes the
effect with the root-derived process owner.

The protected launch command carries the exact profile id and path but no
serialized effect bearer. It cannot attach a retained session, attach a shared
or managed runtime, use CDP auto-connect, or select a provider. Historical
session and runtime-owner projections do not participate in profile identity
selection or admission. Start and completion uncertainty prohibit automatic
retry. Normal close reconciles only after exact process exit and profile-lock
release, and a fallible Service State projection cannot erase protected owner
custody.

The public response exposes redacted claim and owner receipt axes with
`leaseAuthority.kind=protected`; it does not expose a capability or executable
authorization. Non-Linux builds retain the legacy compatibility path and are
described as such in the CLI help, README, docs site, generated client, and
agent skill.

Source acceptance includes 42 protected protocol tests, 22 profile-recovery
tests, 27 protected-path tests, strict Rust formatting and Clippy, API and MCP
parity, generated-client contracts and type coverage, docs build, serial
workstation install fixtures, fresh-workstation harness, host provisioning,
Guacamole asset and PostgreSQL durability contracts, and route-specific user
synchronization. This is not installed-runtime acceptance. A development
publication must still prove the root socket service, public broker request,
real Chrome launch, exact owner readback, close reconciliation, daemon-crash
reacquisition, and zero production drift before candidate installation.

## Slice F Receipt-Only Protected Owner Projection | 2026-09-01

Protected browser-owner state now projects into Service State through a
separate `protectedBrowserOwnerObservations` collection rather than by
promoting browser or runtime-owner rows. Each observation is linked to the
completed launch receipt and carries the exact owner id and generation,
logical browser id, daemon route, lowercase process-instance digest, PID,
owner revision, observation time, and a freshness bound of no more than sixty
seconds. The schema requires `operationalAuthority=false`.

Projection persistence validates every axis against the browser record being
written. A missing protected projection removes an older observation when a
nonprotected launch replaces the row, and a processless health update removes
the observation. Removing the browser operational row also removes its
observation. Invalid or noncanonical input aborts the projection without
leaving an observation, and the public inventory rejects caller-supplied
observations that do not match the exact persisted browser key and PID or that
claim operational authority. `service browsers` returns the collection
separately, and the generated client, CLI help, README, docs site, and agent
skill all state that it is a candidate locator only. No planner, admission
gate, effect sink, or denial path consumes it as authority.

Source validation passes the projection lifecycle regression, the public
`service_browsers` action regression, 28 protected-path tests, 35 service-model
tests, 83 service-health tests, strict formatting and Clippy, API and MCP
parity, the full generated service-client suite, the docs build, remote-view
documentation checks, and every selector-requested workstation fixture run
serially. The source skill intentionally differs from the shared user-scoped
production skill. Development publication must use
`development-runtime:skill-sync`; this source checkpoint does not overwrite
production guidance.

## Seventh Hotfix Recurrence Gap Audit | 2026-09-01

The receipt-only projection audit found one adoption requirement that must be
made explicit before a live orphan can be reused. A protected owner proves the
browser process selected by the original launch, but it does not by itself
prove that the original daemon or executor has relinquished effect custody. A
replacement daemon must not adopt from Service State merely because the
observation is fresh, and two same-user daemons must not both rejoin the same
browser.

Protected orphan adoption therefore requires a kernel-owned executor-custody
transition. The root authority must prove the original executor absent or
replaced, reobserve the exact process, profile, executable, CDP endpoint, and
profile lock, issue one single-use adoption intent to the selected replacement
executor, and atomically commit or terminalize that intent. A current original
executor, mismatched endpoint, changed process instance, or uncertain attach
returns typed bounded recourse without transferring authority or extending the
claim. Service State remains a locator and never supplies a rejoin permit.

The audit also confirms that product-wide structural closure cannot be claimed
while non-Linux acquisition and legacy runtime-owner transfer retain competing
authority paths. Platform ports may fail with an explicit unsupported protected
authority outcome during migration, but they cannot silently preserve a second
effect-capable lease implementation in an accepted candidate.

## Slice F Protected Executor Custody Binding | 2026-09-01

Every newly committed protected browser owner now binds the launching
executor's root-observed UID, GID, PID, process start token, canonical
executable path and digest, and their content-bound executor identity digest.
The binding remains traceable to the completed launch receipt, whose executor
UID and digest must match. Protected-state loading recomputes the executor
digest and rejects a changed PID, start token, path, executable digest, receipt
identity, or malformed field before serving authority.

The root service retains these axes for later liveness observation but does not
return them to the candidate daemon. Browser-launch completion now emits a
redacted owner projection containing only the owner id and generation, logical
browser id, daemon route, browser process digest and PID, and owner revision.
Focused tests prove exact executor persistence, restart validation, response
compatibility, and rejection after executor-PID tampering. This is the custody
prerequisite for orphan adoption; it does not itself transfer a live browser or
make Service State an adoption authority.

## Eighth Hotfix Recurrence Gap Audit | 2026-09-01

The full incident review confirms that the canonical kernel model covers every
reported hotfix family, but it also exposes a distinction that the prior audits
did not state strongly enough: logical fencing does not revoke an already-open
physical effect channel. An old daemon that retains a CDP connection, can
reconnect to the debugging endpoint, can write the profile, or can signal the
browser process may diverge from a perfectly correct owner record. Structural
closure therefore requires effect-channel custody, not only signed intents and
owner generations. The accepted architecture must mediate each raw channel
through a generation-checking broker or isolate it so a superseded executor can
no longer use it.

The adoption protocol must also close the observation-to-commit race. Initial
process and endpoint evidence is insufficient if the PID is reused, the
candidate execs, the profile is replaced through an alias, the endpoint socket
changes, or a descriptor or bearer is delegated before commit. Invariants 116
through 119 require stable handles where supported, replaceable-axis
reobservation, nondelegable executor selection, exact physical profile identity,
and a typed no-transfer or uncertain outcome whenever exclusivity cannot be
proved.

Finally, a mathematically bounded lease can still recreate the operator's
experience of a gate maze. Invariants 118 and 120 require truthful independent
state axes and a frozen ordinary-work usability maximum. An authenticated
client with a compatible profile and no fresh exact physical collision must not
receive a lease blocker, and an abandoned ephemeral operation must converge
within the installed acceptance bound rather than merely before a generous
internal TTL. Invariant 121 makes the migration exit explicit: the legacy
runtime-owner transfer, direct Service State owner mutation, and unfenced raw
effect paths must be absent from the accepted candidate, not merely lower
priority than the new kernel.

With these additions, no known hotfix requires a different authority model.
The design is sufficient to make the reported failure taxonomy structurally
unrepresentable, but the current product is not. That claim becomes truthful
only after effect-channel mediation or isolation, two-phase adoption, physical
identity hardening, the usability bound, exhaustive denial and sink manifests,
legacy-path deletion, reference-model and fault tests, mixed-version migration,
and exact installed acceptance all pass for one candidate.

## Ninth Hotfix Recurrence Gap Audit | 2026-09-01

Implementation found three additional cross-layer failures that the abstract
model did not make visible. First, prepare and complete adoption requests were
dispatched as kernel mutations but were omitted from the socket service's
durable-mutation classification. A request could therefore return a successful
adoption response without publishing the resulting protected generation. The
service now routes both operations through the durable mutation transaction,
and a focused decoder-to-transaction regression freezes that requirement.

Second, the initial admission check treated a daemon route name as executor
identity and rejected a replacement process that reused the same stable route.
That would recreate the reported invented-session failure by forcing the access
planner to manufacture a replacement route. Adoption now distinguishes the
candidate exclusively by root-observed process and executable identity. A
stable route may survive an executor generation change, while the owner
generation and executor custody still advance atomically.

Third, executor process death does not prove effect-channel revocation. A child,
inherited descriptor, reconnectable CDP endpoint, profile writer, or process
signal capability can outlive the recorded executor. Protected inspection now
reports reservation, recorded owner, holder observation, physical occupancy,
effect-channel observation, and requester-holder equality as independent axes.
Acquisition cannot turn uncertain effect custody into adoption. This means the
two-phase adoption kernel is implemented, but live orphan adoption remains
deliberately unavailable until the selected broker or operating-system
containment proves the old effect channel absent. A bounded exact-browser close
and cold relaunch is an acceptable ordinary-work fallback; a permanent denial
is not.

## Slice F Two-Phase Adoption Kernel And Truthful Inspection | 2026-09-01

The protected authority now supports one single-use prepare receipt and an
atomic completion for an exact live browser. Prepare authenticates the profile
capability before observation, binds the current claim, owner generation,
browser process instance, physical profile, CDP listener socket, profile lock,
candidate peer, and a sixty-second transition deadline, and refuses while the
original executor remains current. Completion reobserves replaceable axes and
requires the selected candidate to own an established connection to the exact
CDP listener before advancing owner generation and executor custody. Uncertain
completion terminalizes without transferring the owner or issuing a second
candidate.

The public internal adapter carries only caller claims and redacted authority
projections. Callers cannot supply PIDs, endpoint identity, attachment,
absence, time, or lock evidence, and response decoding rejects leaked executor
or physical evidence. The profile-acquisition entry point now inspects
protected authority before considering cold launch. A stale exact browser is
reconciled before relaunch, a genuinely current foreign executor returns an
exact conflict, and a live orphan is eligible for adoption only after effect
custody is independently proven absent. Receipt projections use the generic
`authorityReceiptId` name so launch and adoption receipts are not conflated.

Focused source validation currently passes ten adoption tests, the truthful
inspection adapter regression, the mutation-transaction regression, and the
ordinary-acquisition disposition matrix. This is not an installable acceptance
checkpoint yet. Prepared-deadline and uncertain-receipt reconciliation,
effect-channel mediation or isolation, current-holder reconstruction, docs and
contract propagation for the receipt field, full protected service restart
tests, legacy-path deletion, and installed development acceptance remain.

## Slice G Root-Derived Effect-Channel Custody | 2026-09-01

The Linux protected root no longer maps a stale executor directly to uncertain
effect custody. It observes the exact CDP listener, enumerates established
loopback client sockets for that port, and joins every socket inode to its
user-owned process holder through procfs. No connected socket is reported as
absent. An established socket whose holder cannot be resolved, a surviving
holder other than the recorded executor, or more than one holder is uncertain.
The recorded exact executor is current only when it is the sole holder.

Prepare now consumes and persists a valid root-derived absent-custody digest.
It refuses current or uncertain custody before issuing an adoption receipt.
Completion independently enumerates the listener again and commits only when
the selected candidate is the sole process holding an established connection.
This closes the inspection-to-commit race for cooperating Agent Browser
executors and detects inherited descriptors rather than inferring their absence
from parent death. It does not claim protection from arbitrary same-user code
that bypasses Agent Browser and connects after the commit.

Focused validation passes eleven adoption tests, including the real Linux
listener, profile-lock, and candidate-socket fixture plus a regression proving
that surviving effect custody cannot prepare an adoption. Deadline-driven
prepared and uncertain receipt reconciliation is the next and only remaining
implementation packet before the broader recurrence and installed acceptance
gates.
