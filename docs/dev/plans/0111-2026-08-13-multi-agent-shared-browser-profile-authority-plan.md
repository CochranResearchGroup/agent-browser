# Plan 0111: Multi-Agent Shared-Browser Profile Authority

Date: 2026-08-13

State: OPEN

Lane: P111

Source baseline: `8042d5b7cb11`

Depends on:

- `docs/dev/plans/0037-2026-06-19-runtime-profile-sharing-plan.md`
- `docs/dev/plans/0069-2026-07-06-shared-profile-routing-and-handoff-deepening-plan.md`
- `docs/dev/plans/0070-2026-07-09-browser-session-authority-plan.md`
- `docs/dev/plans/0095-2026-08-07-remote-control-duplicate-pressure-readiness-repair-plan.md`
- `docs/dev/plans/0108-2026-08-10-runtime-process-identity-pid-reuse-repair-plan.md`

## Goal

Make one authenticated browser profile safely usable by many agents at the
same time.

The ordinary topology is:

```text
one canonical profile directory
  -> one owning browser instance and Chromium process group
    -> many agent sessions
      -> many tabs and windows
```

The browser instance owns the writable profile directory. Agents do not own
the whole profile merely because they use a tab. Each contested resource gets
the narrowest authority boundary that preserves correctness:

- browser-instance ownership for the profile directory;
- shared participation for agents and independent tabs;
- per-tab serialization for mutations of one tab;
- browser-global serialization for global browser state;
- one controller at a time for OS pointer, keyboard, and foreground focus on
  one display;
- concurrent read-only observation wherever its provider can prove safety.

## Maintainer Direction

The product should not describe normal multi-agent browser use as duplicate
profile use. A Chromium browser naturally has many operating-system processes,
tabs, and clients. Those are not duplicate profile owners.

Exclusivity applies only to independent root browser instances attempting to
write the same canonical user-data directory. Multiple agents should normally
attach to the existing owner and receive their own tab or window without
needing to understand profile locks, daemon sessions, or route hints.

## Relationship To Plan 0069

Plan 0069 established the correct product direction and implemented retained
browser reuse for ordinary navigation and service tab acquisition. It remains
the historical authority for route-bound handoff consolidation.

P111 is the focused successor for the unresolved sharing boundary:

- replace check-then-launch profile exclusion with atomic ownership
  reservation;
- separate browser-instance ownership from agent and tab access;
- make multi-agent acquisition automatic across ingresses;
- validate route hints against the current owner rather than treating their
  presence as sufficient;
- reconcile stale and externally attached browser records using process and
  canonical-profile evidence;
- make duplicate-pressure diagnostics describe independent root owners, not
  normal Chromium child processes or multiple agent clients.

P111 does not reopen completed P69 routing work or absorb P69's remaining
route-bound handoff refactor.

## Current Evidence

### Source

- Access plans already publish `profileProcessPolicy=exclusive_process` and
  `clientSharingPolicy=shared_browser_tabs`.
- Compatible retained-browser selection considers profile identity, browser
  health, requested host, display isolation, view-stream provider, and control
  input provider.
- The launch gate rejects a same-profile live browser by default, supports
  bounded lease waiting, and requires either route hints or the explicit
  `allowDuplicateProfileLane` override to proceed.
- `LockedServiceStateRepository` serializes service-state mutations across
  processes with a file lock and uses atomic state replacement, but the
  existing profile check is a snapshot read performed before browser launch.
  The check, owner reservation, process spawn, and owner finalization are not
  one transaction.

### Installed readback on 2026-08-13

- The retained `default` profile is modeled as `shared_service` and currently
  has three ready browser records.
- One record has current process and CDP evidence and an exclusive session.
  Two `attached_existing` records have neither a root PID nor a CDP endpoint.
- Resource diagnostics group the two evidence-poor records into
  `duplicate_live_browsers_for_profile` under the same posture.
- Garbage collection reports zero candidates because all of the records are
  protected.

This readback does not prove that three physical Chrome roots are writing the
same directory. It proves that the current model cannot distinguish a real
duplicate owner from stale or weakly adopted service-state rows well enough to
converge safely.

## Frozen Vocabulary

### Profile identity

`ProfileIdentity` is the internal canonical identity of one writable browser
user-data directory. It is derived from the canonicalized path and runtime
profile namespace. Public surfaces expose only an opaque ID or digest, never a
private filesystem path.

Profile name alone is not sufficient identity. Case variants, symlinks,
legacy default aliases, and caller-supplied paths must resolve before ownership
is decided.

### Browser instance

`BrowserInstance` means one root browser process and its Chromium process
group. Renderer, GPU, network, storage, utility, crash-handler, and extension
processes belonging to that root are children, not duplicate browser owners.

### Profile owner

`ProfileOwner` is the service-owned reservation binding one `ProfileIdentity`
to one browser instance generation. It contains enough private evidence to
reject PID reuse and ABA transitions:

- opaque owner ID;
- profile identity digest;
- state: `reserving`, `ready`, `releasing`, `orphaned`, or `failed`;
- monotonically changing owner generation;
- browser ID and daemon/session route;
- process-instance identity when a process exists;
- browser host and build;
- display and stream posture when applicable;
- reservation, readiness, and last-observed timestamps;
- reservation token used only for compare-and-finalize or rollback.

### Agent participation

An `AgentParticipation` record lets one accountable service, agent, and task
use the existing browser owner. Participation is shared by default and does
not imply authority over other agents' tabs, browser shutdown, or display
input.

### Scoped operation authority

- `TabMutationLease`: serializes effectful operations against one tab.
- `BrowserGlobalLease`: serializes browser-wide settings, extension state,
  downloads when destination semantics are global, and shutdown or restart.
- `DisplayControllerLease`: serializes OS pointer, keyboard, and focus effects
  for one display and generation.
- `ViewerLease`: may remain concurrent and read-only unless promoted through
  the existing controller workflow.

These are separate resources. Holding one must not silently grant another.

## Product Invariants

1. At most one ready or reserving browser owner exists for one canonical
   writable profile identity unless the profile has been physically copied to
   a distinct directory and registered as a distinct identity.
2. Many agents may participate in one browser owner concurrently.
3. Each participating agent receives an independently attributable tab handle,
   window handle, or explicit shared-target grant.
4. Different tabs may execute concurrently when neither operation requires a
   browser-global or display-controller lease.
5. Mutations against the same tab are serialized in a deterministic queue.
6. Read-only operations may overlap only when the provider declares that
   capability and their receipt binds the same current browser and tab
   generations.
7. Browser-global mutation and lifecycle operations are serialized per browser
   owner.
8. Desktop pointer, keyboard, and focus effects are serialized per display,
   regardless of how many tabs or agents share the browser.
9. A route hint is a locator, not authority. Its browser, session, profile,
   process generation, host, stream, display, and health must agree with the
   current owner before reuse.
10. A stale PID, recycled PID, stale browser row, or expired reservation cannot
    authorize reuse or block a new owner indefinitely.
11. External or attached-existing browsers may become owners only after
    canonical profile and process-instance evidence is proved. Evidence-poor
    rows remain observed inventory and cannot make ownership decisions.
12. `allowDuplicateProfileLane` never authorizes two roots to write the same
    canonical directory. It may authorize only a separately copied, ephemeral,
    or otherwise distinct profile identity.
13. Closing one agent's tab or ending its participation does not close the
    shared browser or another agent's tab.
14. Browser-owner crash or replacement invalidates stale tab, global, and
    display authority through generation checks before further effects.

## Architecture

### 1. Profile ownership repository

Add one service-state ownership registry keyed by canonical profile identity.
All ownership changes use `LockedServiceStateRepository::mutate` so the
following decision is atomic across daemon processes:

```text
no owner -> create reserving owner with generation and token
ready compatible owner -> return reuse result
reserving compatible owner -> return bounded wait result
live incompatible owner -> return typed conflict
orphaned owner with disproven process -> advance generation and reserve
```

Do not hold the service-state file lock while starting Chrome. The first
transaction creates the reservation. Process launch happens outside the lock.
A second compare-and-finalize transaction succeeds only when owner ID,
generation, and reservation token still match. Failure and cancellation use
the same token to roll back without deleting a successor's reservation.

Reservations have a bounded deadline. Reconciliation may mark an expired
reservation orphaned only after process identity, browser health, and current
owner token evidence fail closed.

### 2. One shared acquisition coordinator

Introduce one deep acquisition operation used by plain CLI navigation, HTTP,
generic and dedicated MCP adapters, generated client helpers, dashboard
launches, and remote-headed acquisition:

```text
resolve_profile_identity
  -> reserve_or_reuse_profile_owner
    -> resolve_or_create_agent_participation
      -> acquire_or_create_tab
        -> return bound acquisition receipt
```

The public caller supplies semantic intent such as profile, service, target,
account, URL, and desired posture. The service chooses and validates internal
browser/session route hints. Callers should not have to copy hints from an
access-plan response into a second request for ordinary sharing.

The acquisition receipt binds:

- profile identity digest and owner generation;
- reused or newly launched browser ID;
- agent participation ID and attribution;
- acquired tab or window handle and generation;
- host, build, stream, display, and input posture;
- acquisition outcome and reasons;
- cleanup ownership for the tab, participation, and browser.

### 3. Resource-scoped coordinators

Add process-owned coordinators keyed by stable resource identity:

- browser owner ID for global work;
- browser owner ID plus tab ID for tab mutation;
- display allocation ID plus controller epoch for OS input.

Every coordinator claim captures the persisted generation. Before each effect,
the engine re-reads the bound authority and rejects drift. Provider calls occur
inside the narrow event guard where takeover or replacement races matter.

The service may initially serialize all mutations per tab for correctness.
Read-only concurrency and more granular capability scheduling are follow-up
optimizations only after deterministic tests prove the base model.

### 4. Reconciliation and duplicate pressure

Rework duplicate-pressure classification around owner evidence:

- count root browser instances, not Chromium child processes;
- group by canonical profile identity, not profile label alone;
- distinguish `confirmed_duplicate_owner`, `reservation_conflict`,
  `stale_modeled_owner`, and `insufficient_owner_evidence`;
- include process-instance and owner-generation agreement in readiness;
- treat evidence-poor attached-existing rows as observed rather than ready
  owners;
- provide reviewed repair actions for stale rows without terminating an
  unproven live process;
- keep automatic termination outside reconciliation.

GC and install doctor consume the same classification. A protected stale row
must not remain permanently non-actionable: it needs a typed reviewed repair
path even when process termination is not authorized.

### 5. Public contract and terminology

Replace ambiguous user-facing `profile lease` language where it describes
ordinary browser sharing. Preserve compatibility fields during migration, but
make these concepts visible:

- `profileOwnership`: owner state, generation, evidence posture, and opaque
  browser route;
- `agentParticipation`: shared or detached and its accountable principal;
- `tabAuthority`: tab generation, mutation queue, and cleanup ownership;
- `browserGlobalAuthority`: available, held, or waiting;
- `displayControlAuthority`: observer or current controller;
- `sharingMode=shared_browser_tabs`;
- `processPolicy=single_profile_owner`.

Access-plan and workspace projections should answer one primary question:

> Can this agent receive a tab in the current browser owner now?

The normal recommended action is `acquire_shared_tab`. Waiting, seeding,
incompatible-owner, evidence-insufficient, and copied-profile isolation are
explicit alternatives.

## Implementation Slices

### Slice A: Red fixtures and ownership vocabulary

- Freeze canonical profile identity fixtures for named, custom-path, symlink,
  legacy-default, copied, and case-variant inputs.
- Freeze browser root versus Chromium child-process fixtures.
- Add failing tests for two concurrent reserve attempts, stale reservation ABA,
  evidence-poor attached-existing rows, and invalid route-hint bypass.
- Add model and response schemas behind internal feature use without changing
  launch behavior.

Exit gate: fixtures prove the current check-then-launch and stale-row gaps
before production behavior changes.

### Slice B: Atomic owner reservation

- Implement reserve, wait, finalize, fail, release, and reconcile transitions.
- Bind them to process-instance identity and owner generation.
- Route all service-owned browser starts through the reservation.
- Prohibit the duplicate-lane override for the same canonical directory.

Exit gate: concurrent independent requests for one profile produce exactly one
owner. The loser deterministically reuses, waits, or receives a typed conflict.

### Slice C: Automatic multi-agent tab acquisition

- Introduce accountable shared participation records.
- Route CLI, HTTP, MCP, client, dashboard, and remote-headed tab acquisition
  through the shared coordinator.
- Validate any compatibility route hints against current owner identity.
- Give each agent independent tab cleanup ownership.

Exit gate: multiple agents acquire different tabs in one retained browser
without profile-wide exclusive leases or caller-managed route hints.

### Slice D: Scoped operation authority

- Add per-tab mutation queues and browser-global coordination.
- Reuse the existing controller epoch and event fencing for OS input.
- Define capability metadata for read-only overlap versus effectful
  serialization.
- Invalidate claims on browser, tab, route, display, or controller generation
  changes.

Exit gate: different tabs progress independently, same-tab mutations remain
ordered, and browser-global or desktop effects cannot interleave unsafely.

### Slice E: Reconciliation and repair

- Change duplicate pressure to owner-evidence classification.
- Add no-effect diagnostics for stale and insufficient evidence.
- Add reviewed repair for stale modeled rows and expired reservations.
- Align GC, install doctor, service resources, profile allocation, browser
  session authority, incidents, and dashboard attention projections.

Exit gate: the current default-profile warning becomes either a proved owner
conflict or a safely repairable stale or insufficient-evidence condition.

### Slice F: Contract, documentation, and installed-runtime convergence

- Update service schemas, metadata, HTTP, MCP, generated client helpers, CLI
  output, dashboard actions, and observability projections.
- Update `cli/src/output.rs`, `README.md`, `skills/agent-browser/SKILL.md`,
  relevant `docs/src/app/` pages, and inline source documentation.
- Deprecate ambiguous fields without breaking existing clients in the first
  compatible release.
- Install the reviewed binary and prove source, packaged binary, user command,
  workstation payload, dashboard payload, and active service listeners agree.

Exit gate: user-facing language and installed behavior consistently describe
many-agent sharing and single-browser ownership.

## Required Test Matrix

1. Two daemon processes concurrently request the same empty profile. Exactly
   one reservation and one root browser launch occur.
2. Ten agents concurrently request distinct tabs. All ten bind one owner
   generation and no second root starts.
3. Two agents mutate different tabs concurrently without queue starvation.
4. Two agents mutate the same tab and observe deterministic FIFO ordering.
5. Read-only operations overlap only when provider capability permits it.
6. Browser-global mutation waits behind another global mutation but does not
   block unrelated read-only tab work unnecessarily.
7. Two OS-input requests on one display never overlap; observer leases remain
   concurrent.
8. Renderer, GPU, utility, network, crash-handler, and extension processes do
   not create duplicate-owner pressure.
9. A second independent root on the same canonical profile is detected as a
   confirmed conflict even when profile labels differ.
10. Symlink, legacy-default, and case-variant aliases resolve to one identity;
    a physical profile copy resolves to a distinct identity.
11. A stale PID, PID reuse, expired reservation, or browser-generation change
    cannot finalize or retain ownership.
12. A stale owner rollback cannot remove a successor owner.
13. `browserId` or `sessionName` without exact owner agreement fails before
    browser or tab effects.
14. An attached-existing browser without canonical profile and process proof
    remains observed and cannot block or authorize reuse.
15. Ending one participation closes only its owned tabs under its cleanup
    policy and preserves other agents and the browser.
16. Owner shutdown waits for or cancels scoped work, advances generation, and
    invalidates old handles before a replacement launches.
17. Reconciliation classifies the current duplicate-warning fixture without
    killing a process or deleting unproved state.
18. CLI, HTTP, generic MCP, dedicated MCP, generated client, schema, dashboard,
    help, skill, and docs surfaces return the same acquisition semantics.
19. Existing P69 shared-profile, service tab-handle, remote-view handoff,
    viewer/controller lease, profile seeding, and browser-session authority
    regressions remain green.
20. Installed-runtime readback proves one noncritical temporary profile, one
    browser root, multiple accountable agents, and multiple surviving tabs.

## Validation Strategy

### Source gates per Rust slice

```bash
scripts/ci/cargo-safe.sh fmt --manifest-path cli/Cargo.toml -- --check
scripts/ci/cargo-safe.sh clippy --manifest-path cli/Cargo.toml -- -D warnings
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml profile_owner -- --test-threads=1
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml shared_profile -- --test-threads=1
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml profile_lease -- --test-threads=1
```

Every Cargo command on WSL must use `scripts/ci/cargo-safe.sh`. Run one Cargo
process at a time.

### Contract and UI gates

```bash
pnpm test:service-api-mcp-parity
pnpm test:service-client
pnpm test:dashboard-inspector-actions
pnpm test:dashboard-workspace-nodes
pnpm --dir docs build
pnpm validation:select -- --base <last-green-ref>
git diff --check
```

### Process-level source smoke

Use a temporary home, temporary profile directory, two independently spawned
daemon processes, and an instrumented fake browser root. Prove one reservation,
one spawn, deterministic waiter behavior, crash recovery, and zero mutation of
the operator's default runtime profile.

### Controlled live gate

The live gate is a later explicit slice, not implied by source acceptance.
Use a newly created noncritical temporary profile with no private account or
credentials. Start one installed browser owner, acquire multiple tabs from at
least three independent agent principals, and prove:

- one canonical user-data directory;
- one root browser process and its expected child process group;
- at least nine independently attributed live tabs;
- cross-tab progress and same-tab serialization;
- one tab release preserves every other tab and the browser;
- source and installed binary hashes agree;
- no duplicate-owner warning remains after reconciliation.

Do not use the current `default`, QBO, Last30Days, AuraCall, or another private
authenticated profile for the first live gate.

## Migration And Compatibility

- Existing `profileLeasePolicy=reject|wait` remains accepted during migration.
  Its effect is reinterpreted only where it currently guards browser-owner
  acquisition. It must not impose profile-wide exclusion on ordinary shared
  tab participation.
- Existing access-plan route hints remain available for compatibility, but new
  helpers consume them internally and validate current owner generation.
- Existing browser and session rows without ownership metadata load with a
  legacy evidence state. They are reconciled before becoming authoritative.
- Existing `allowDuplicateProfileLane` remains parseable, but same-directory
  duplication becomes a typed rejection. Documentation directs isolation use
  to copied or ephemeral profile identities.
- Durable job and incident projections retain opaque IDs, generations, typed
  states, and hashes. They do not persist raw profile paths, page contents, or
  private account data.

## Non-Goals

- Supporting two independent Chrome roots writing one user-data directory.
- Giving every agent control of the same tab without ordering.
- Allowing multiple OS pointers or foreground keyboard owners on one display.
- Automatically killing an unproved external browser.
- Automatically deleting stale state without a reviewed, evidence-backed
  repair predicate.
- Using browser contexts or incognito profiles to simulate sharing of an
  authenticated persistent profile.
- Site-specific account, CAPTCHA, passkey, or credential-manager behavior.
- Replacing Guacamole, RDP, CDP, or desktop-perception provider architecture.
- Formal release work.

## Hard Stops

- Stop if canonical profile identity cannot be resolved without exposing or
  mutating a private path.
- Stop before browser launch if ownership reservation cannot be persisted.
- Stop if a reservation token or owner generation changes before finalize.
- Stop rather than attach when external process or profile evidence is
  missing, contradictory, or stale.
- Stop effectful work when tab, browser, route, display, or controller
  generation changes.
- Stop live validation if more than one root process addresses the same
  temporary profile directory.
- Stop live validation before touching a named authenticated profile.

## Done Definition

P111 is complete only when:

- product language consistently says one profile owner with many shared agents
  and tabs;
- ownership reservation closes the cross-daemon check-then-launch race;
- normal multi-agent tab acquisition needs no profile-wide exclusive lease and
  no caller-managed route hints;
- same-tab, browser-global, and display-input effects use separate scoped
  coordinators;
- duplicate pressure counts proved root owners and distinguishes stale or
  insufficient evidence;
- the current default-profile warning has a safe, typed reconciliation outcome;
- the required source, contract, UI, process-level, and controlled live tests
  pass;
- installed runtime and source are synchronized and current readback shows one
  browser root serving multiple independently attributable agents and tabs;
- no private authenticated profile was used to establish the first acceptance
  proof.

## First Recommended Slice

Start with Slice A only. Freeze the canonical profile identity, root-process,
reservation-race, stale attached-existing, and route-hint mismatch fixtures.
Do not change launch behavior until those tests are red against the existing
check-then-launch implementation and the public ownership vocabulary has been
reviewed as one coherent contract.
