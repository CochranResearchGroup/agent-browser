# Plan 0144 | Lease Authority Coordination And Revocation

Date: 2026-08-31

State: OPEN

Execution state: `slice_g_crash_durable_authority_publication_source_accepted_custody_in_progress`

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
failures in six families:

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
    that object cannot publish. A mutation load binds protected state and the
    required audit history to the exact same selected generation before it
    can mutate or publish. History failure therefore blocks new mutation
    without truncating prior audit evidence or blocking current reads.

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
| read-only authority snapshot is mutated and republished without loading history | publication is structurally unavailable; mutation must bind the exact selected history generation first |

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
read-only load non-publishable and adds an exact-generation mutation load that
must validate history before changing authority. This preserves history's
nonblocking read semantics without weakening required audit durability for new
mutations.

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
