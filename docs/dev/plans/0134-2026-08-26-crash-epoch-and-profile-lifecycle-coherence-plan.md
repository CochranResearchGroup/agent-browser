# Plan 0134 | Comprehensive Principal, Lease, Crash, And Install Coherence

Date: 2026-08-26

State: OPEN

Execution state: `slice_i_complete_slice_j_pending`

Lane: P134

Source baseline: `1efab249b7908cf703d5facae132dd658a3daf97`

Branch: `plan/crash-profile-lifecycle-coherence`

Target: `main`

Authority: SOURCE AND PROVIDER-FREE DEVELOPMENT ONLY. DEVELOPMENT-RUNTIME OR
PRODUCTION EFFECTS REQUIRE A SEPARATE EXACT-CANDIDATE AUTHORIZATION.

Current checkpoint:

- Slice A consumer failures and Slice B authenticated principal authority are
  implemented and validated.
- Slice C now projects lease collection, detail, explanation, doctor,
  lifecycle mutations, sealed reconciliation, lifecycle events, contract
  capability discovery, generated clients, and dashboard management.
- Dashboard controls require the exact ephemeral profile capability, honor the
  lease's current `authorizedActions`, disable observation-only mutations, and
  keep reconciliation planning separate from effect-capable apply. Capability
  input is held only in the open dialog and is never written to browser
  storage or Service State.
- Slice C is integrated. Slice D makes access planning and service-request
  admission use the same transport-authenticated principal decision. The
  capability is bound to both principal and profile, and never persists.
- Same-principal coherent reuse returns the exact browser and session route.
  Foreign principals wait, unauthenticated callers must authenticate, and
  profile contradictions return a typed lifecycle identity inconsistency.
- Slice E makes the exact current runtime owner authoritative for an existing
  managed session before any new-lane profile fallback.
- Slice F binds package-owned process, runtime-host socket, display, viewer,
  acquisition, lifecycle, and session-lease observations to one host boot
  epoch. Prior-boot evidence requires rediscovery and has no effect or cleanup
  authority.
- Slice G adds one persisted, compare-and-swap crash-regeneration transaction.
  It resumes in dependency order with stable operation IDs, retains invalid
  visibility as an interruption, and converges after either effect failure or
  receipt-persistence failure without duplicating logical effects.
- Slice H keeps first-class profile lease authority aligned across CLI, HTTP,
  MCP, generated clients, and dashboard controls. Public status adds redacted
  crash-regeneration progress while removing its private ephemeral evidence
  from both bounded and full-history Service State projections. Slices I
  through K remain pending.

Depends on:

- `docs/dev/notes/0134-2026-08-26-exclusive-profile-lease-holder-reuse-divergence.md`;
- `docs/dev/notes/2026-08-26-books-receipts-post-crash-browser-regeneration.md`;
- Plan 0081 route-pool state reconciliation;
- Plan 0116 runtime adoption and transactional upgrade architecture;
- Plan 0117 runtime lifecycle authority and convergence;
- Plan 0128 runtime lifecycle hotfix collection;
- Plan 0130 access-plan owner reuse coherence;
- Plan 0132 terminal-owner supersession route coherence;
- Plan 0133 operator-visible window focus postcondition; and
- installer runtime-routing hotfix `130fd4a6`.

## Goal

Restore one coherent managed-browser identity after a workstation crash,
runtime generation transition, or service reconnect. A registered service
principal that owns profile P must be able to rejoin its exact retained
browser, acquire a task-scoped tab, and continue using its profile when current
principal, profile, browser, process, route, and owner-generation evidence
agree. A transient session id must not turn that service into a foreign
contender against its own lease.

When those identities disagree, Agent Browser must return one typed lifecycle
inconsistency and a bounded, authority-preserving recourse action. It must not
wait on the same principal's contradictory or stale holder, silently
reattribute the browser to `default`, release another principal's work, or
launch a duplicate profile process.

Make profile leases first-class managed resources. Services and operators must
be able to list, inspect, explain, rejoin, renew, release, and reconcile leases
through stable CLI, HTTP, MCP, dashboard, and generated-client contracts. The
normal recovery path must not require editing state files, killing browsers, or
shipping a new lifecycle hotfix for every combination of stale session,
process, route, or boot evidence.

The same identity model must drive crash startup. Boot-scoped process, socket,
display, viewer, and lease observations are rediscovered; stable profile,
browser, route, connection, user, and opaque handoff identities are preserved.

Deliver the model through one transactional installation and migration path.
The candidate must understand legacy session-scoped state without mutating it,
stage and validate the new principal and lease schema, preserve rollback, adopt
or transfer every live owner, atomically commit the selected generation and
state, and prove the installed CLI, service, dashboard, HTTP, MCP, generated
client, and user-scoped skill all describe the same contract.

P134 is the comprehensive delivery authority for this defect family. It uses
Plan 0116's immutable-generation, runtime-census, owner-transfer, ingress, and
rollback machinery rather than creating a second installer architecture.

## Current Defects

### Session identity is mistaken for service authority

The public access-plan request carries `serviceName`, `agentName`, and
`taskName`, but those fields are descriptive labels rather than authenticated
authority. Profile lease blocking is computed from the selected profile and
each holder session's lease state. The decision does not establish whether the
requester is the same durable service principal that owns the retained
browser.

That makes a reconnecting client indistinguishable from an unrelated
contender. Last30Days, Books Receipts, and Odollo fulfillment can therefore be
denied use of their own authenticated profiles when the original transient
session remains the exclusive holder. Waiting is not recourse because the
service itself owns the work that must continue, and broad lease release would
violate another active task whenever the attribution is wrong.

The missing abstraction is a stable, authenticated service principal with a
profile capability that survives request and session churn. Sessions and tabs
are subordinate work leases under that principal. User-supplied service names
must never become authority merely because their strings match.

### Profile leases have observability but no control plane

Current service status, profile allocation, trace, and diagnostics expose
lease holders and wait pressure. `tab_handle_release` can release one tab
handle, and viewer leases have request, heartbeat, and release actions. There
is no equivalent profile-lease resource or action family.

A service cannot ask which proof blocks reuse, rejoin its own retained lane,
renew a legitimate task lease, release an exact idle lease, or request bounded
reconciliation. Operators are left with indirect cleanup, hand-edited runtime
state, process termination, or a source hotfix. Those are not acceptable
recourse mechanisms for routine lifecycle drift.

### Self-blocking exclusive profile lease

Current access planning derives reusable browsers from
`BrowserProcess.profile_id` while deriving exclusive holders from
`ServiceSession.profile_id`. A live session can therefore hold profile P while
its own browser is excluded from P's reusable set. The planner reports normal
`wait_for_profile_lease` contention even though waiting cannot repair the
contradiction.

An unscoped command can also resolve an omitted runtime profile to `default`.
For a genuinely new lane that fallback is valid. For an existing managed lane
with a current owner binding, it can republish or attach the retained browser
under an identity that contradicts its session and lease.

### Crash startup regenerates from stale ephemeral evidence

After a host crash, persisted PIDs, sockets, X displays, viewer sessions, and
route allocations can describe the previous boot. Current recovery does not
yet perform one idempotent dependency-ordered transaction that invalidates
those observations, binds the runtime-host unit to the selected generation and
selected ingress, reacquires or adopts the retained browser, rediscovers its
display, restores the Guacamole web tier and route, and proves the exact
operator-visible postcondition.

### Installer hotfix boundary

Commit `130fd4a6` binds post-commit workstation reconciliation to the staged
candidate runtime host. It prevents candidate commands from colliding with an
old-generation route daemon during upgrade. It does not repair inconsistent
profile attribution, stale lease ownership, or general crash regeneration.

### Installation can preserve a bad model or strand a candidate

The principal and first-class lease schema changes durable Service State,
owner authority, public contracts, generated clients, dashboard behavior, and
runtime reconciliation. Replacing only the executable would leave old
session-scoped records ambiguous and could make old and candidate generations
interpret the same lease differently.

Runtime-census rejection must remain fail-closed, but a rejected preflight must
also have a terminal, inspectable disposition. It must not leave an
effect-free candidate recorded as an indefinitely active upgrade or make
doctor report unexplained install drift. Conversely, marking a transaction
terminal must never bypass unresolved live-owner ambiguity.

The installer currently has immutable-generation and transaction seams, but
P134 still needs an exact state-migration contract, mixed-version behavior,
resume and rollback tooling, installed contract parity, and acceptance proof
for live principal-bound leases.

## Frozen Invariants

1. Profile-process exclusivity, service-principal authority, task/session
   leases, tab ownership, and viewer/controller authority are distinct axes.
2. One managed browser lane has one canonical principal identity, profile
   identity, logical browser identity, session route, process identity, owner
   generation, and cleanup obligation.
3. Principal authority comes from an authenticated registration or unforgeable
   capability. `serviceName`, `agentName`, `taskName`, and session ids remain
   attribution and routing fields, not proof of ownership.
4. A retained browser may survive client reconnect, session replacement,
   daemon, runtime-host, display, viewer, and route regeneration without
   changing its profile or logical identity.
5. The same principal may rejoin its retained browser and acquire a new
   task-scoped tab without waiting on its own coherent profile lease.
6. A different principal cannot reuse, release, supersede, or act through that
   browser without an explicit owner-transfer contract.
7. Ephemeral observations are valid only for the boot or host epoch in which
   they were recorded. PID presence alone is never identity proof.
8. An access-plan read is observation-only. It cannot rewrite a browser,
   session, lease, profile, route, or owner record.
9. Exact retained-browser reuse requires mutually consistent principal,
   profile, browser, process, route, and current owner-generation evidence.
10. A same-principal contradiction returns
    `lifecycle_principal_identity_inconsistent`,
    `lifecycle_profile_identity_inconsistent`, or an equally specific typed
    blocker with one exact reconciliation action. It cannot produce normal
    lease waiting or a launch-capable request.
11. Normal `wait_for_profile_lease` is reserved for a proven different
    principal holding a coherent exclusive lease.
12. An omitted profile selector preserves the canonical owner profile for an
   existing managed lane. `default` remains the fallback only for genuinely
   new, unbound work.
13. Reconciliation is compare-and-swap bound to the observed process identity
   and owner generation. Newer human or client authority wins.
14. Recovery never launches a second process to escape a contradictory holder.
15. Operator-visible readiness requires the Plan 0133 process-bound desktop
    postcondition after any regeneration or focus action.
16. Software and operators receive only the durable opaque
    `/remote-view/<handoff-id>` URL. Provider and Guacamole URLs remain
    internal evidence.
17. Every lease has a stable `leaseId`, principal identity, profile identity,
    lease revision, owner generation, mode, state, subordinate session and tab
    bindings, heartbeat policy, expiry policy, and cleanup obligation.
18. Lease inspection and explanation are read-only. Every mutation requires an
    authenticated principal or operator authority plus exact expected lease
    revision and owner generation.
19. There is no generic `force unlock`. Release, replacement, transfer, and
    reconciliation are distinct actions with different preconditions.
20. Safe lifecycle repair is expressed as idempotent, compare-and-swap
    transitions over the frozen identity axes. Unknown or contradictory state
    is quarantined with explicit missing proofs rather than improvised cleanup.
21. Lease capabilities are advertised in the service contract so clients can
    discover supported reads, actions, and preconditions without reconstructing
    server behavior or depending on matching source internals.
22. Candidate preflight reads legacy state without modifying it and produces a
    deterministic migration plan, compatibility report, runtime census digest,
    and rollback requirements.
23. No durable state migration occurs in place. The installer snapshots the
    exact authoritative inputs, migrates into a staged copy, validates all
    identities and references, then atomically commits or preserves the old
    state byte-for-byte.
24. A candidate may commit only when every discovered live browser, lease,
    route, viewer, controller, process, and owner generation has one proven
    adoption, preservation, transfer, or explicit blocking disposition.
25. Mixed generations never both hold effect authority over one logical
    browser or profile lease. Old clients may read capability metadata, but an
    unsupported mutation fails with a typed compatibility result.
26. Rollback after state commit requires either proven old-generation read
    compatibility or a validated reverse migration. An unproven reverse path
    blocks commit.
27. A preflight rejection before effects is a terminal `blocked_preflight`
    transaction with zero mutations, preserved diagnostics, and an explicit
    retry-or-close path. It is not reported as an active stranded upgrade.
28. Installer resume, rollback, and close operations require the exact
    transaction id, expected transaction revision, candidate generation, and
    current census digest. They cannot operate on an inferred latest record.
29. Candidate acceptance binds source commit, candidate binary SHA, immutable
    generation id, migrated-state receipt, installed executable SHA, contract
    digests, runtime-owner generations, and final doctor receipt.
30. Installation completion requires consumer acceptance for synthetic
    Last30Days, Books Receipts, and Odollo fulfillment principals plus one
    foreign-principal isolation control. Provider navigation is not required.

## Installation And Migration Contract

### State schema and compatibility

- Add an explicit Service State schema version and a profile-lease schema
  version. Unknown newer versions fail read-only with a typed compatibility
  result instead of being rewritten through serde defaults.
- Define legacy session-scoped state as an input format, not an authority
  source. Migration derives a principal binding only from authenticated
  registration, current owner evidence, and exact profile capability. Labels
  alone produce an `unproven_principal` lease that is observation-only.
- Preserve stable profile ids, logical browser ids, authenticated profile
  directories, durable handoff ids, Guacamole connection and user identities,
  and owner generations. Regenerate only explicitly ephemeral current-epoch
  observations.
- Validate referential integrity across profiles, principals, leases,
  sessions, tabs, browsers, routes, handoffs, viewer/controller leases,
  cleanup obligations, and runtime-owner records before commit.
- Publish forward and reverse compatibility matrices in service contracts and
  install status. A migration with no safe reverse reader or reverse transform
  cannot enter the installed acceptance lane.

### Transaction lifecycle

Use the Plan 0116 immutable generation and transaction engine with these P134
phases:

```text
discovered
  -> preflighted
  -> census_stable
  -> state_snapshot_created
  -> state_migration_staged
  -> state_migration_validated
  -> owners_prepared
  -> candidate_runtime_ready
  -> owners_committed
  -> generation_and_state_committed
  -> installed_acceptance_passed
  -> rollback_window_open
  -> complete
```

Terminal alternatives are `blocked_preflight`, `blocked_census`,
`blocked_migration`, `rolled_back_before_commit`, `rolled_back_after_commit`,
and `failed_operator_required`. Every terminal state records whether any effect
occurred, selected generation, authoritative state pointer, rollback
availability, outstanding owner or cleanup obligations, and the exact next
safe action.

### Operator and client install surfaces

- `install transactions list|inspect` are read-only and expose transaction
  revision, phase, candidate identity, census status, migration status,
  rollback readiness, blockers, and safe actions.
- `install workstation --dry-run` creates no active transaction and mutates no
  runtime state. It returns the candidate, census, compatibility, migration,
  and rollback plan.
- `install transactions resume|rollback|close` use exact transaction and
  compare-and-swap evidence. `close` is allowed only for a proven zero-effect
  terminal transaction or after all obligations are satisfied.
- `install doctor` distinguishes workspace-versus-installed binary identity,
  selected-versus-candidate generation, active convergence, terminal blocked
  history, state-schema compatibility, owner/lease convergence, contract
  parity, rollback readiness, and operator-visible readiness.
- The installer publishes the repository skill to the installed user-scoped
  skill only after generation and contract commit. Failed candidates leave the
  accepted skill untouched.

### Rollback and recovery

- Keep the old immutable generation, exact pre-migration state snapshot, unit
  definitions, ingress selection, contract metadata, and owner-transfer
  receipts until installed acceptance and the rollback review both pass.
- Before owner commit, rollback discards only staged candidate material.
- After owner commit, rollback reverses owner generations through existing
  receipted authority, restores the accepted state pointer and generation
  selector, and re-proves the durable handoff and operator-visible
  postcondition.
- Interrupted migration, install, resume, or rollback replays by transaction id
  and converges without duplicate leases, browsers, routes, generations, or
  cleanup obligations.
- Garbage collection refuses any generation, state snapshot, or receipt still
  referenced by a live process, owner, lease, rollback window, or nonterminal
  transaction.

## Execution Slices

### Slice A | Freeze the three consumer failures

- Add a public access-plan fixture with one live exclusive holder whose
  session profile is P and whose browser profile is `default`, missing, or a
  different profile.
- Prove the current result is self-blocking `wait_for_profile_lease` with no
  reusable browser.
- Add a coherent retained-browser fixture in which a new session from the same
  registered service principal requests its own profile and is incorrectly
  treated as a lease conflict.
- Exercise three provider-free consumer shapes: Last30Days repeated tasks,
  Books Receipts post-crash reconnect, and Odollo fulfillment FedEx tracking
  lookup. Use synthetic identities and URLs only.
- Add the negative control: an unrelated principal requesting the same
  exclusive profile must remain bounded contention.
- Add a command-path fixture that addresses the existing session without a
  runtime-profile selector and records the first write or projection that can
  reattribute the browser.
- Add a crash fixture with a new boot epoch, stale runtime-host evidence, a
  dead Guacamole web tier, a stale display allocation, and a retained managed
  browser.

Exit condition: provider-free tests reproduce the session/principal ownership
failure, the profile-attribution contradiction, and crash-epoch failure through
public CLI, HTTP, or MCP behavior before implementation.

### Slice B | Introduce principal-scoped profile authority

- Define a stable `principalId` and principal-to-profile capability in the
  registered client/service contract. Derive it from authenticated service
  registration or a capability token, never from caller-supplied labels.
- Bind the runtime owner registry to principal identity as well as profile,
  logical browser, process, route, and owner generation.
- Model sessions and tabs as subordinate, expiring work leases. Replacing a
  dead session from the same principal does not transfer profile ownership.
- Add a conservative migration for legacy session-scoped leases. Ambiguous
  legacy ownership remains observation-only with typed recourse.
- Define exact recourse states: `rejoin_owned_browser`,
  `replace_stale_same_principal_session`, `wait_for_foreign_principal`, and
  `reconcile_principal_identity`.

Exit condition: the service control plane can prove same-principal continuity
without trusting labels and without weakening cross-principal isolation.

### Slice C | Add a first-class profile-lease control plane

- Add canonical profile-lease collection and detail projections. Each record
  includes principal provenance, profile, logical browser, sessions, tabs,
  mode, state, revision, owner generation, heartbeat and expiry state, cleanup
  obligation, blocking identity axes, and currently authorized actions.
- Add read-only `list`, `inspect`, `explain`, `doctor`, and `watch` surfaces.
  `doctor` evaluates all identity invariants and returns typed findings plus
  exact safe actions without mutating state.
- Add owner-scoped `rejoin`, `renew`, and `release` actions. `release` refuses
  active subordinate work unless the exact cleanup policy and authority permit
  it; it never releases another principal's lease.
- Add `reconcile plan` and `reconcile apply`. Planning returns a sealed,
  expiring action descriptor bound to lease revision, owner generation,
  principal, profile, browser, process, route, boot epoch, proposed
  transitions, and idempotency key. Apply accepts only that exact descriptor
  and converges on replay.
- Keep explicit owner transfer on the existing owner-transfer authority path.
  Do not disguise transfer, takeover, browser close, or profile deletion as
  lease reconciliation.
- Project the contract through CLI `service leases`, HTTP
  `/api/service/profile-leases`, MCP resources and tools, dashboard actions,
  service request metadata, and generated client helpers.
- Advertise supported lease operations and schema versions through
  `/api/service/contracts` and `agent-browser://contracts` so clients can
  feature-detect instead of guessing from a binary version.
- Record append-only lease lifecycle events and idempotent receipts without
  storing private page data, credentials, or raw capability material.

Exit condition: an authorized service or operator can diagnose and safely
resolve every modeled lease state without editing runtime files, terminating a
browser, or installing another binary.

### Slice D | Make access and admission principal-aware

- Derive browser reuse and holder coherence from one principal-aware
  acquisition decision.
- Reuse the exact retained holder for the same principal only when every
  frozen identity agrees, then grant a new task-scoped tab lease.
- Return a typed lifecycle inconsistency when the same principal's retained
  browser is excluded by contradictory profile, process, route, or owner
  evidence.
- Keep a proven unrelated coherent exclusive holder as normal bounded
  contention.
- Keep replacement launch unavailable while any current or contradictory
  owner remains.
- Apply the same decision to access planning and effect admission so a read
  cannot promise a route the request path will reject.

Exit condition: a service can rejoin its own lane, a foreign service cannot,
and no path waits on itself or recommends duplicate launch.

### Slice E | Make canonical profile identity authoritative

- Resolve an existing managed session against the exact current owner binding
  before applying the new-lane `default` fallback.
- Reject an explicit profile that conflicts with the proven owner rather than
  republishing the retained browser.
- Keep ambiguous `attached_existing` observations preserve-only until profile,
  process, endpoint, target, route, and owner evidence agree.
- Preserve `default` behavior for one genuinely new unbound session.

Exit condition: omitted selectors cannot reattribute an owned browser, and no
second profile or owner registry is introduced.

### Slice F | Bind ephemeral evidence to a boot epoch

- Introduce one boot or host epoch field for package-owned PID, socket,
  runtime-host, display, viewer, and lease observations.
- Treat prior-epoch observations as stale evidence that requires rediscovery,
  not as current authority or automatic deletion candidates.
- Preserve stable profile, logical browser, route, connection, route-user, and
  durable handoff identities across epoch change.
- Migrate legacy records conservatively and expose typed missing-epoch
  diagnostics until current evidence is captured.

Exit condition: a simulated reboot cannot authenticate PID reuse, a stale
socket, or a stale display allocation as current state.

### Slice G | Add one idempotent crash-regeneration transaction

- Atomically derive the runtime-host unit from the selected immutable
  generation and selected runtime-host ingress.
- Execute recovery in dependency order: host authority, retained-browser
  acquisition or exact adoption, display discovery, bounded Guacamole web-tier
  recovery, route projection, durable handoff resolution, then independent
  operator-visible proof.
- Rediscover display numbers as runtime evidence instead of treating them as
  durable identity.
- Repair only exact stale allocations and browserless viewer lanes with
  compare-and-swap evidence. Preserve active routes, databases, profiles, and
  unrelated processes.
- Make interruption and replay converge to the same result without duplicate
  browsers, routes, leases, or cleanup obligations.

Exit condition: the crash fixture reaches ready on replay with unchanged
stable identities and newly observed ephemeral identities.

### Slice H | Align public surfaces and clients

- Expose principal identity provenance, the typed inconsistency, blocking
  identity axes, and safe next action consistently through profile allocation,
  access plan, service status, request admission, dashboard, CLI, HTTP, MCP,
  schemas, and generated client.
- Keep raw process details, private profile paths, provider URLs, and page
  content out of public responses.
- Update every required user-facing documentation surface when the public
  contract changes.

Exit condition: maintained clients consume one generated contract and do not
reconstruct profile or lifecycle ownership.

### Slice I | Integrate state migration and transactional installation

- Add explicit state and lease schema versions plus legacy readers that never
  silently promote labels to principal authority.
- Implement staged forward migration, full invariant validation, atomic state
  selection, and the validated reverse or old-reader compatibility path.
- Extend the Plan 0116 workstation transaction with P134 migration phases,
  candidate runtime routing, owner and lease transfer, atomic generation and
  state commit, exact resume, rollback, and zero-effect terminal close.
- Add install transaction list and inspect surfaces first, then guarded resume,
  rollback, and close actions.
- Teach install doctor to distinguish a harmless workspace candidate,
  terminal blocked history, active convergence, true installed drift, schema
  incompatibility, and lease or owner divergence.
- Sync the installed skill only after accepted contract commit and preserve the
  prior skill on rollback.

Exit condition: every injected install and migration boundary has one durable,
replayable, inspectable outcome with exactly one selected generation and state.

### Slice J | Validate source and isolated development runtime

- Run focused provider-free access-plan, profile resolution, lifecycle,
  lease-control, migration, installation, adoption, reconciliation,
  crash-replay, and contract tests.
- Run the validation selector, canonical Rust partitions, formatting, strict
  Clippy, and all selected client and documentation checks.
- In the isolated development runtime, replay one disposable crash with a
  harmless `about:blank` browser, migrate one legacy fixture, and prove exact
  reacquisition, lease recourse, rollback, release, and cleanup.

Exit condition: source and isolated development acceptance are complete with
no production or provider effect.

### Slice K | Qualify and install one exact candidate

- Build one release-mode candidate and bind its source commit, binary SHA,
  generation id, support-manifest digest, contract digests, and migration
  schema versions into the qualification record.
- Run dry-run preflight against the current workstation and require a stable
  closed-world census, complete migration plan, rollback readiness, and no
  unexplained owner, lease, route, or process ambiguity.
- If preflight blocks before effects, record terminal `blocked_preflight` or
  `blocked_census`, preserve the accepted installation unchanged, and return
  the exact first-class recourse path. Do not bypass the guard.
- Under separate exact-candidate authorization, apply the transaction, run the
  provider-free installed consumer matrix, inject and prove one rollback, then
  apply the same candidate again and prove idempotent convergence.
- Run `agent-browser install doctor` from the accepted installed path and
  require selected generation, installed SHA, state schema, lease schema,
  owner generations, contract parity, skill identity, rollback readiness, and
  operator-visible readiness to agree.
- Retain rollback material until explicit acceptance closeout; garbage
  collection is a later reviewed action.

Exit condition: the accepted installed candidate supports first-class lease
recourse and crash recovery for all three synthetic consumers while preserving
foreign-principal isolation and a proven rollback path.

## Required Acceptance Matrix

1. A new session from the same authenticated principal returns retained-browser
   reuse with exact browser and route hints plus a new task-scoped tab lease.
2. Matching service, agent, or task labels without principal authority cannot
   claim or act through the retained browser.
3. An unrelated principal remains bounded profile-lease contention.
4. A stale same-principal session receives exact rejoin or replacement recourse
   rather than an indefinite self-wait.
5. A mismatched holder returns a typed lifecycle inconsistency, not lease wait.
6. Last30Days, Books Receipts, and Odollo fulfillment synthetic consumer
   fixtures all reuse their own principal-bound profiles.
7. Omitted runtime-profile selector preserves an existing owner profile.
8. New unbound work still resolves to `default`.
9. Ambiguous attachment or legacy principal attribution remains
   observation-only and cannot execute effects.
10. Current owner evidence can recover route hints without rewriting state.
11. Stale owner generation, process identity, or boot epoch cannot authorize
   repair, input, close, or replacement.
12. Crash replay preserves the authenticated profile and stable principal,
    browser, session, route, user, connection, and opaque handoff identities.
13. Crash replay rediscovers runtime host, socket, PID, display, viewer, and
    lease evidence for the current epoch.
14. Dead Guacamole web-tier recovery preserves the database and route records.
15. A displaced or minimized browser is not ready until independent
    process-bound re-observation passes.
16. No path launches a duplicate profile process, releases another task's
    lease, or adopts ambiguous metadata.
17. CLI, HTTP, MCP, dashboard, schemas, and generated clients agree.
18. Lease list, inspect, explain, and doctor are read-only and identify the
    same holder, principal provenance, identity blockers, and authorized
    actions across every public transport.
19. Same-principal rejoin and renew succeed with current evidence and fail on
    stale principal capability, lease revision, owner generation, or boot
    epoch.
20. Exact idle lease release is idempotent. Active subordinate tabs or sessions
    block release unless their recorded cleanup policy explicitly authorizes
    the same transition.
21. Reconcile planning produces no effects. Reconcile apply accepts only the
    sealed current plan, rejects changed evidence, and replays without a second
    mutation.
22. No profile-lease command offers broad force release, process killing,
    profile deletion, or cross-principal takeover.
23. Contract capability discovery lets an older client detect unavailable
    lease operations and return a typed unsupported-capability result.
24. Legacy session-scoped state migration preserves authenticated profiles and
    stable logical identities while leaving ambiguous principals
    observation-only.
25. Every migration failure before commit leaves authoritative state
    byte-identical and the accepted generation selected.
26. Failure after state or owner commit completes a receipted rollback or
    enters typed `failed_operator_required` with old and candidate generations
    preserved.
27. Dry-run produces no active transaction, state snapshot, owner transfer,
    unit change, selector change, skill change, or browser effect.
28. A rejected zero-effect preflight is terminal and inspectable and does not
    keep install doctor red as a nonterminal upgrade.
29. Resume and rollback reject stale transaction revision, census digest,
    generation id, state pointer, lease revision, or owner generation.
30. Installed CLI, runtime host, dashboard, HTTP, MCP, generated client,
    contracts, and user-scoped skill all report the same accepted generation
    and lease capabilities.
31. Injected failure at each migration and installation phase preserves exactly
    one effect-capable owner and one authoritative profile lease.
32. Final installed doctor is green only after source identity, generation,
    state schemas, runtime census, owners, leases, ingress, contracts, skill,
    rollback, and operator-visible readiness all agree.

## Validation

Use the repository Cargo safety wrapper for every compiling Cargo command.
Start each implementation slice with:

```bash
pnpm validation:select -- --base <last-green-ref>
```

At minimum run:

```bash
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml service_access_plan -- --test-threads=1
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml runtime_profile -- --test-threads=1
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml runtime_lifecycle -- --test-threads=1
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml runtime_reconciliation -- --test-threads=1
pnpm test:workstation-install-fixture
scripts/ci/cargo-safe.sh fmt --manifest-path cli/Cargo.toml -- --check
scripts/ci/cargo-safe.sh clippy --manifest-path cli/Cargo.toml -- -D warnings
scripts/ci/rust-tests.sh
```

Contract changes also run the generated-client, service-request parity,
dashboard, MCP, HTTP, documentation, and no-launch checks selected by changed
paths.

Migration and installer work additionally runs failure injection for every new
transaction phase, old-to-new and new-to-old compatibility fixtures, repeated
resume and rollback, contract-digest parity, installed-skill staging, and one
isolated development-runtime transaction. These are source or disposable
development checks until Slice K receives exact-candidate authority.

## Bounds

- Two implementation attempts per failing behavioral seam before local
  replanning.
- Provider-free fixtures before any browser or installed-runtime effect.
- One disposable development crash replay after source acceptance.
- One forward migration and one rollback per legacy fixture class.
- One injected failure per transaction phase, with exact replay only.
- One exact candidate qualification packet only after separate authorization.
- No provider navigation or authentication inspection is required for source
  or development acceptance.

## Hard Stops

- Do not edit Service State, owner registries, profile locks, unit files,
  Guacamole rows, or route allocations by hand.
- Do not delete, reset, copy, replace, or reseed a named authenticated profile.
- Do not close or kill a retained browser to make identity evidence agree.
- Do not release another task's tab, session, viewer, route, controller, or
  profile lease.
- Do not launch a second browser on a profile with a current or contradictory
  owner.
- Do not weaken route, display, lifecycle, owner-generation, or
  operator-visible agreement checks.
- Do not use broad Docker cleanup, database recreation, garbage collection, or
  workstation installation as crash recovery.
- Do not migrate the only authoritative Service State copy in place.
- Do not mark a transaction complete or closed while it retains an owner,
  lease, route, rollback, or cleanup obligation.
- Do not let an old generation write a schema it cannot understand or let a
  candidate write authoritative state before migration validation and commit.
- Do not delete the accepted generation, pre-migration snapshot, accepted
  skill, or rollback receipts during candidate qualification.
- Do not interpret `--dry-run`, preflight, doctor, list, inspect, or explain as
  authority for an install or runtime mutation.
- Do not classify a longer client timeout as a lifecycle repair.
- Do not consume a provider request or private page as acceptance evidence.
- Do not expose credentials, cookies, raw handoff URLs, provider URLs, private
  page content, or raw browser artifacts.

## First Execution Packet

Implement Slice A only. Extend the public Plan 0130 access-plan fixture with
the inconsistent exclusive holder, then add the same-principal reconnect and
foreign-principal negative controls. Cover synthetic Last30Days, Books
Receipts, and Odollo fulfillment request shapes without navigating providers.
Add the unscoped existing-session command-path fixture and the boot-epoch crash
fixture without implementing recovery effects. Stop after the tests
deterministically reproduce the principal self-block, the profile contradiction,
and the crash contradiction, and identify the first profile-attribution write.
Also freeze the public profile-lease contract shape and prove that current
clients have no first-class inspect, rejoin, renew, release, or reconcile path.
Add provider-free legacy-state and install-transaction fixtures that freeze the
required schema-version, migration-plan, terminal blocked-preflight, exact
resume, rollback, contract-capability, skill-staging, and doctor readbacks.
Do not implement migration or installer effects in Slice A.

## 2026-08-27 Slice A execution checkpoint

- Slice A is complete at source scope. The provider-free corpus at
  `docs/dev/fixtures/profile-lifecycle/plan-0134-red-fixtures.v1.json`
  freezes five public access-plan cases: one profile-attribution
  contradiction, same-principal Last30Days, Books Receipts, and Odollo
  fulfillment reconnects, plus one foreign-principal isolation control.
- The current public request rejects `principalId`. All four same-principal
  cases reproduce `wait_for_profile_lease` with no reusable browser and no
  route hint. The foreign-principal control produces the same bounded wait.
  This proves the planner cannot currently distinguish self-continuity from
  foreign contention.
- The unscoped existing-session fixture resolves an omitted profile to
  `default`. The first durable attribution write is
  `cli/src/native/cdp/chrome.rs write_runtime_state`; the later Service State
  projection is `BrowserProcess.profile_id` in
  `cli/src/native/action_runtime/runtime/navigation.rs`.
- The crash fixture preserves synthetic stable principal, profile, logical
  browser, route, Guacamole connection and user, and opaque handoff identities
  while freezing prior-epoch runtime-host, socket, display, viewer, and lease
  observations. Current Service State serializes no top-level state schema,
  profile-lease collection, or boot epoch.
- The lease contract fixture proves that current maintained surfaces have no
  first-class list, inspect, explain, doctor, watch, rejoin, renew, release,
  reconcile-plan, or reconcile-apply family. Adjacent tab-handle and viewer
  lease operations remain explicitly separate.
- The install and migration corpus at
  `docs/dev/fixtures/profile-lifecycle/plan-0134-install-migration-red-fixtures.v1.json`
  freezes the required legacy reader, explicit schema versions, no-effect dry
  run, terminal blocked preflight, exact resume and rollback keys, capability
  discovery, accepted-skill staging, and doctor classifications without
  implementing effects.
- Validation passed: the three focused P134 reproductions, all 45
  `service_access_plan` tests, the fixture contract audit, the workstation
  source-free fixture, Rust formatting, strict Clippy, and `git diff --check`.
- No browser, provider, development-runtime, installed-runtime, Service State,
  owner, lease, route, display, unit, Guacamole, or profile effect occurred.
  The next authorized packet is Slice B only.

## 2026-08-27 Slice B execution checkpoint

- Slice B is complete at source scope. `cli/src/native/service_principal.rs`
  defines stable registered principals, hashed principal-to-profile
  capabilities, authenticated authority records, subordinate expiring session
  and tab work leases, conservative legacy migration planning, and the exact
  four continuity recourse states.
- Caller-provided service, agent, task, session, and principal-shaped fields
  remain attribution or untrusted input. The current public request still
  rejects `servicePrincipalId`; only a transport-authenticated capability can
  populate the separate internal authority record.
- Principal authority is bound to the existing runtime owner registry by
  canonical profile identity digest and exact ready owner generation. Binding
  is compare-and-swap guarded, rejects unproven legacy provenance, and does not
  create a second ownership system.
- Registration is transactional. A conflicting capability leaves the
  principal registry unchanged, raw capabilities are never persisted, and a
  stale capability revision cannot bind new subordinate work.
- Legacy Service State remains readable. Labels and even principal-shaped
  session fields remain observation-only unless active registration,
  capability, profile, provenance, and current owner binding agree exactly.
- The canonical internal contract is documented at
  `docs/dev/contracts/service-principal-authority.v1.md`. First-class lease
  commands and reads, authenticated public transport ingestion, generated
  client and dashboard parity, state migration effects, and installed-runtime
  adoption remain later P134 slices.
- Validation passed: 10 focused principal authority tests, the broader
  17-test principal filter, exact owner-generation binding, all 50 service
  request tests, all 35 service model tests, all 45 access-plan tests, the
  three Slice A reproducers, Rust formatting, strict Clippy, route-confusion
  gates, HTTP and MCP parity, generated-client contract and type checks, the
  service collections no-launch smoke, and `git diff --check`.
- No browser, provider, development-runtime, installed-runtime, Service State,
  owner, lease, route, display, unit, Guacamole, or profile effect occurred.
  The next authorized packet is Slice C only.

## 2026-08-27 Slice C core model checkpoint

- The canonical source model now projects one first-class profile lease from
  authenticated principal capability, exact runtime owner binding, sessions,
  tabs, browser, process, route, expiry, and cleanup evidence. Legacy profiles
  appear only when retained owner or session evidence exists and remain
  observation-only.
- Every record publishes an evidence-derived `leaseRevision`, blocking
  identity axes, exact authorized actions, and one typed continuity recourse.
  Owner-scoped `rejoin`, `renew`, and `release` require current capability and
  revision evidence. Release refuses active subordinate tabs.
- `doctor` returns typed findings and safe actions. Reconciliation planning is
  sealed and bound to lease revision, owner generation, principal, profile,
  browser, process, routes, boot epoch, expiry, and idempotency key. Apply is
  idempotent through persisted receipts.
- Reconciliation remains non-effect-capable while boot epoch is unavailable.
  This is an intentional cross-boot safety gate until the later crash-epoch
  slice supplies the missing identity.
- The internal contract is documented at
  `docs/dev/contracts/service-profile-lease.v1.md`. CLI, HTTP, MCP, dashboard,
  generated-client, event, JSON schema, and watch parity remain open within
  Slice C; Slice C is not complete.
- Focused validation passed for the new profile-lease model and existing
  profile-lease gate filters, plus all service model tests. No browser,
  provider, development-runtime, installed-runtime, Service State, owner,
  lease, route, display, unit, Guacamole, or profile effect occurred.

## 2026-08-27 Slice C read-side parity checkpoint

- The first-class profile lease collection is available without launching a
  browser through CLI `agent-browser service leases`, HTTP
  `GET /api/service/profile-leases`, MCP `agent-browser://profile-leases`, and
  generated client helpers `getServiceProfileLeases()` and
  `findServiceProfileLease()`.
- The shared response includes the exact projected lease records, count,
  observation time, and doctor report. Contract metadata advertises the same
  route, resource, helpers, no-launch posture, and Slice C operation family.
- Record and collection JSON schemas are canonical under
  `docs/dev/contracts/`. Rust tests guard required wire fields, and the
  generated client is derived from those schemas.
- README, CLI help, repository skill guidance, docs-site commands and service
  mode pages, and the contract registry describe the same authority and
  observation-only legacy behavior.
- Validation passed for focused lease, HTTP collection, MCP resource, service
  model, and contract tests; generated-client checks and type checking; API and
  MCP parity; no-launch MCP and service collection smokes; docs build; Rust
  formatting; strict Clippy; and `git diff --check`.
- The MCP no-launch smoke had a pre-existing stale expected inventory for the
  already-supported `desktop_evidence_observe` tool. The frozen allowlist now
  matches the current source inventory and the smoke passes.
- Owner mutation transports, detail, explain, watch, event, and dashboard
  parity remain open. No browser, provider, development-runtime,
  installed-runtime, Service State, owner, lease, route, display, unit,
  Guacamole, or profile effect occurred.

## 2026-08-27 Slice C authenticated lifecycle checkpoint

- CLI, HTTP, MCP, and generated-client surfaces now provide exact lease
  detail, explanation, doctor, bounded or abortable watch, owner rejoin,
  renewal, release, and sealed reconcile plan and apply contracts.
- Principal registration generates a capability once and writes it only to an
  operator-selected absolute private file. Retained Service State stores only
  the digest. CLI reads the secret from a private file, HTTP accepts it only in
  `Authorization: Bearer`, and MCP accepts it only as ephemeral tool input.
- Every mutation requires exact revision compare-and-swap evidence. Release
  refuses active subordinate tabs. Lifecycle events contain operation,
  principal, profile, revision, recourse, and trace labels without raw
  capability material.
- Reconcile plans are sealed by the authenticated capability and bind all
  currently modeled authority axes. They remain `effectCapable: false` with
  `boot_epoch_unavailable`; apply fails closed until Slice F supplies a current
  boot epoch.
- Contract metadata and dedicated collection, detail, explanation, doctor,
  and mutation schemas support feature detection. README, CLI help, repository
  skill guidance, contract documentation, and docs-site pages describe the
  same secret-transport and fail-closed behavior.
- Focused validation passed for 22 Rust profile-lease tests, all 35 service
  model tests, service-contract metadata, generated client checks and types,
  API/MCP parity, service collection and MCP read no-launch smokes, strict
  Clippy, Rust formatting, and the docs production build. Dashboard lease
  workspace parity remains open, so Slice C is not complete.
- No browser, provider, development-runtime, installed-runtime, Service State,
  owner, lease, route, display, unit, Guacamole, or profile effect occurred.

## 2026-08-27 Slice D principal-aware access and admission checkpoint

- Access planning and service-request normalization now call one
  principal-aware profile-reuse decision. HTTP authenticates an optional
  bearer capability before either operation, and MCP authenticates the
  ephemeral `profileCapability` field before planning or admission.
- Raw capability material is removed before queue persistence. Generated
  observability and request clients accept `profileCapability` as an option
  and transmit it only in the HTTP authorization header.
- Exact same-principal and same-profile authority reuses the retained browser
  and session. A foreign principal returns `wait_for_foreign_principal`; an
  unauthenticated caller returns `authenticate_for_profile_reuse`; and
  contradictory profile evidence returns
  `lifecycle_profile_identity_inconsistent` with the profile identity axis.
- Capabilities are checked against both the registered principal and the
  selected profile. A capability for another profile cannot authorize reuse
  merely because the principal label matches.
- Contract schemas and metadata advertise the new decisions, evidence fields,
  header-only transport, ephemeral MCP field, and principal-aware admission.
  README, CLI help, repository skill guidance, and both docs-site surfaces
  describe the same authority boundary and recourse behavior.
- Validation passed for all access-plan tests, focused HTTP, request-admission,
  MCP, and contract tests, strict Clippy, Rust formatting, generated client
  contracts and types, API and MCP parity, the full service-client suite, the
  service collection no-launch smoke, route-confusion gates, source-free
  workstation fixtures, the docs production build, and `git diff --check`.
  The repository skill correctly differs from the installed shared skill;
  installed skill synchronization remains an explicit Slice J and K candidate
  effect rather than a Slice D source mutation.
- No browser, provider, development-runtime, installed-runtime, Service State,
  owner, lease, route, display, unit, Guacamole, or profile effect occurred.
  The next authorized packet is Slice E only.

## 2026-08-27 Slice E canonical profile identity checkpoint

- Existing managed launches, automatic launches, profile-lease admission, and
  CDP-free launches now resolve the daemon's exact current session through the
  single runtime owner registry before applying request selection or the
  generic `default` fallback.
- A ready effect-capable owner must agree with the Service State session,
  browser, profile, canonical profile-directory digest, route, owner
  generation, and any retained principal binding. Contradictions return typed
  profile identity errors instead of republishing the browser.
- Conflicting explicit profile IDs or paths are rejected. Request route hints
  cannot override the daemon's actual route, but remain available to the
  existing duplicate-lane lease policy. Retained session or browser evidence
  without a current owner remains preserve-only.
- A genuinely new unbound session still reaches the existing `default`
  runtime-profile behavior. Session records and generated clients expose
  `existing_owner` alongside the complete profile-selection reason enum.
- The broad service partition found and repaired one adjacent Slice D test
  contract gap: MCP `profileCapability` is explicitly tested as an ephemeral
  transport-only extension rather than a persisted service-request field. A
  date-bound lease fixture was also moved out of the current time window so it
  continues testing revision compare-and-swap instead of accidental expiry.
- Validation passed: all 71 route-host tests; all 35 service-model and 45
  access-plan tests; the 700-test service partition with 698 passed and 2
  intentionally ignored; strict Clippy; generated-client contract, type, and
  full client checks; API and MCP parity; service collection and profile lookup
  no-launch smokes; source-free workstation, host, VM, Guacamole, durability,
  route-user, and remote-view documentation fixtures; and the docs production
  build.
- No browser, provider, development-runtime, installed-runtime, Service State,
  owner, lease, route, display, unit, Guacamole, or profile effect occurred.
  The next authorized packet is Slice F only.

## 2026-08-27 Slice F boot-scoped evidence checkpoint

- Linux captures the kernel boot ID; macOS and Windows derive a stable host
  epoch from the platform process-start identity. The typed classifier
  distinguishes current, prior, missing, and unavailable evidence.
- Service browser, display, viewer, remote-view acquisition, session work
  lease, and runtime lifecycle records now retain the boot epoch that
  authenticated their ephemeral observations. Runtime-host ingress retains the
  same epoch for its selected PID and socket identity.
- Prior-boot browser rows cannot be reused by access planning, prior display
  allocations cannot satisfy remote-view reuse, and prior runtime-host sockets
  cannot be selected. Exact runtime-host identity refresh is the bounded
  readmission path. Prior evidence is retained and never becomes an automatic
  cleanup candidate.
- Stable profile, logical browser, route, connection, route-user, and durable
  handoff identities are unchanged. Legacy missing-epoch records remain
  visible with typed `rediscover_current_evidence` findings.
- Profile-lease reconciliation plan and apply now capture and compare the real
  current boot epoch instead of remaining permanently blocked by
  `boot_epoch_unavailable` on supported hosts.
- The Plan 0134 reboot fixture proves that PID, display, viewer, and lease
  observations from a prior epoch are rediscovery-only, that the stale browser
  is not returned as reusable, and that the stable logical browser row
  survives. Runtime-host tests prove stale PID and socket authority stays
  blocked until exact identity refresh.
- Focused validation passed for Slice F fixture tests, boot-epoch diagnostics,
  all runtime-host ingress tests, all service-access, profile-lease, resource,
  and remote-view test partitions, strict Clippy, and Rust formatting. Broader
  service and documentation gates remain part of continuing Slice G through J
  validation.
- No browser, provider, development-runtime, installed-runtime, Service State,
  owner, lease, route, display, unit, Guacamole, or profile effect occurred.
  The next authorized packet is Slice G only.

## 2026-08-27 Slice G idempotent crash-regeneration checkpoint

- A single persisted transaction now orders runtime-host authority, browser
  authority, display discovery, Guacamole recovery, route projection, durable
  handoff resolution, and independent operator-visible proof.
- Every phase uses a stable transaction-and-phase operation ID. Completed
  receipts are persisted with compare-and-swap revision checks, and replay
  resumes at the first incomplete dependency while preserving principal,
  profile, logical-browser, daemon-route, remote-route, connection,
  route-user, and durable-handoff identities.
- The Plan 0134 crash fixture reaches ready after an injected Guacamole
  interruption. A separate injected receipt-persistence failure proves that a
  successful provider operation is replayed through the same operation ID and
  still produces exactly seven logical effects.
- Invalid operator-visible evidence is retained as a typed interrupted
  transaction. A changed boot epoch or changed stable identity cannot reuse an
  existing transaction ID, and transaction state survives Service State JSON
  serialization.
- Focused validation passed all five crash-regeneration tests, strict Clippy,
  and Rust formatting.
- This is a provider-free transaction coordinator checkpoint. Runtime effect
  adapters and installation migration remain bounded to Slice I; no browser,
  provider, development-runtime, installed-runtime, Service State, owner,
  lease, route, display, unit, Guacamole, or profile effect occurred. The next
  authorized packet is Slice H only.

## 2026-08-27 Slice H public surface and client checkpoint

- CLI, HTTP, typed MCP status, the generated observability client, and the
  dashboard now consume one `crashRegenerationTransactions` contract for
  transaction state, stable identity, phase progress, operator-visible
  readiness, and typed recourse.
- The public projection omits boot identity, host and browser PIDs, socket
  identity, display number, Guacamole generation, and viewer-session identity.
  Both bounded and full-tab-history status paths remove the private persisted
  transaction map from `service_state` without mutating the authoritative
  input snapshot.
- Existing first-class profile lease collection, detail, explanation, doctor,
  register, rejoin, renew, release, and sealed reconciliation surfaces remain
  the authority for principal provenance, blocking identity axes, authorized
  actions, and lifecycle recourse. The dashboard continues to enable only the
  actions advertised by the exact lease record and keeps capabilities in
  ephemeral dialog state.
- README, CLI help, repository skill guidance, service-mode documentation,
  status schema, generated types, and dashboard status presentation describe
  the same redaction and recourse boundary.
- Validation passed the focused crash and status projection tests, strict
  Clippy and Rust formatting, fixed-input status harness, API and MCP parity,
  full service client suite, dashboard inspector, view-stream, browser-row,
  browser-table and production builds, docs production build, and remote-view
  handoff documentation fixture.
- No browser, provider, development-runtime, installed-runtime, Service State,
  owner, lease, route, display, unit, Guacamole, or profile effect occurred.
  The repository skill is intentionally not copied into the installed skill
  until the guarded candidate transaction in Slice K. The next authorized
  packet is Slice I only.

## 2026-08-27 Slice I transactional migration and installation checkpoint

- Service State and profile leases now carry explicit schema versions. Unknown
  schemas fail read-only, while legacy labels remain observation-only and
  cannot manufacture principal authority.
- Workstation installation snapshots the exact original state, stages a fully
  validated migration candidate, commits with a digest compare-and-swap, and
  restores the original bytes or absence on rollback. Full referential
  validation is owned by this migration commit boundary so ordinary runtime
  mutations remain able to converge related projections incrementally.
- The immutable-generation transaction now persists migration, runtime
  handoff, convergence, generation commit, validation, rollback, and
  zero-effect close phases. Injected failures before and after state commit
  preserve one inspectable replay or rollback outcome.
- `agent-browser install transactions` provides first-class list, inspect,
  guarded resume, guarded rollback, and guarded close actions. Every mutation
  requires the exact transaction revision, candidate generation, and census
  digest. Advertised safe actions are phase-accurate.
- Install doctor distinguishes schema incompatibility, active convergence,
  terminal transaction history, installed drift, a harmless workspace
  candidate, and profile lease or owner divergence without exposing process
  identity or private state paths.
- README, CLI help, repository skill guidance, and installation documentation
  describe the same transaction and compare-and-swap recourse contract. The
  installed shared skill remains unchanged until guarded candidate install.
- Validation passed the canonical parallel-safe and serial environment-mutating
  Rust partitions, strict Clippy, Rust formatting, all 72 route-host tests,
  all migration tests, exact resume and rollback cases, full service client
  checks, API and MCP parity, docs production build, source-free workstation
  fixture, host-provision and fresh-VM harnesses, Guacamole assets and
  PostgreSQL durability checks, route-specific user sync, remote-view handoff
  documentation, and patch hygiene.
- No browser, provider, development-runtime, installed-runtime, Service State,
  owner, lease, route, display, unit, Guacamole, or profile effect occurred.
  The next authorized packet is Slice J only.
