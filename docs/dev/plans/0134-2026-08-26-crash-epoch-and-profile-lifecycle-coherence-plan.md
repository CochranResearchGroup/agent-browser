# Plan 0134 | Service Principal And Crash Lifecycle Coherence

Date: 2026-08-26

State: PLANNED

Execution state: `provider_free_reproducer_pending`

Lane: P134

Source baseline: `4a2d18ffdbd7f2aeab02be240593bbfb1936b8de`

Branch: `plan/crash-profile-lifecycle-coherence`

Target: `main`

Authority: SOURCE AND PROVIDER-FREE DEVELOPMENT ONLY. DEVELOPMENT-RUNTIME OR
PRODUCTION EFFECTS REQUIRE A SEPARATE EXACT-CANDIDATE AUTHORIZATION.

Depends on:

- `docs/dev/notes/0134-2026-08-26-exclusive-profile-lease-holder-reuse-divergence.md`;
- `docs/dev/notes/2026-08-26-books-receipts-post-crash-browser-regeneration.md`;
- Plan 0081 route-pool state reconciliation;
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

The same identity model must drive crash startup. Boot-scoped process, socket,
display, viewer, and lease observations are rediscovered; stable profile,
browser, route, connection, user, and opaque handoff identities are preserved.

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

### Slice C | Make access and admission principal-aware

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

### Slice D | Make canonical profile identity authoritative

- Resolve an existing managed session against the exact current owner binding
  before applying the new-lane `default` fallback.
- Reject an explicit profile that conflicts with the proven owner rather than
  republishing the retained browser.
- Keep ambiguous `attached_existing` observations preserve-only until profile,
  process, endpoint, target, route, and owner evidence agree.
- Preserve `default` behavior for one genuinely new unbound session.

Exit condition: omitted selectors cannot reattribute an owned browser, and no
second profile or owner registry is introduced.

### Slice E | Bind ephemeral evidence to a boot epoch

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

### Slice F | Add one idempotent crash-regeneration transaction

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

### Slice G | Align public surfaces and clients

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

### Slice H | Validate in bounded environments

- Run focused provider-free access-plan, profile resolution, lifecycle,
  adoption, reconciliation, crash-replay, and contract tests.
- Run the validation selector, canonical Rust partitions, formatting, strict
  Clippy, and all selected client and documentation checks.
- In the isolated development runtime, replay one disposable crash with a
  harmless `about:blank` browser and prove exact reacquisition, release, and
  cleanup.
- Stop before production installation. A later exact-candidate authorization
  must name the candidate SHA and installed acceptance scope.

Exit condition: source and isolated development acceptance are complete with
no production or provider effect.

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
scripts/ci/cargo-safe.sh fmt --manifest-path cli/Cargo.toml -- --check
scripts/ci/cargo-safe.sh clippy --manifest-path cli/Cargo.toml -- -D warnings
scripts/ci/rust-tests.sh
```

Contract changes also run the generated-client, service-request parity,
dashboard, MCP, HTTP, documentation, and no-launch checks selected by changed
paths.

## Bounds

- Two implementation attempts per failing behavioral seam before local
  replanning.
- Provider-free fixtures before any browser or installed-runtime effect.
- One disposable development crash replay after source acceptance.
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
