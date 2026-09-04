# Plan 0125 | Development Runtime Isolation And Build Capacity

Date: 2026-08-23

State: ACCEPTED

Execution state: `source_installed_and_ingress_accepted`

Lane: P125

Authority: SOURCE AND DEVELOPMENT RUNTIME EFFECTS | PRODUCTION READ-ONLY

Depends on:

- `docs/dev/plans/0117-2026-08-19-runtime-lifecycle-authority-and-convergence-plan.md`
- `docs/dev/plans/0124-2026-08-23-scalable-desktop-evidence-and-presentation-capacity-plan.md`
- `docs/dev/policies/0031-runtime-vs-product-boundary.md`
- `docs/dev/policies/0032-runtime-state-governance.md`
- `docs/dev/policies/0036-architecture-guardrails.md`

## Goal

Create a fully isolated development Runtime Environment for experimental Agent
Browser builds and replace the repository-wide Cargo mutex with resource-aware
Build Admission. Development installation, dashboard operation, runtime-host
execution, browser state, acceptance, garbage collection, and Cooper ingress
must remain independent from active production use.

P125 is complete when an experimental source build can be installed as
`agent-browser-dev`, reached through the dedicated Agent Browser development
dashboard and Cooper ingress, exercised and replaced repeatedly, and reclaimed
without changing production process identities, executable selection, state,
profiles, handoffs, routes, or dashboard readiness. Two bounded Cargo
invocations must be able to run concurrently when current host pressure admits
them, while pressure automatically reduces admission to one or zero.

## Problem Statement

P117 correctly converged production on one dashboard, one runtime host, and one
selected immutable executable generation. That successful singular authority
also means normal source installation and installed acceptance target the live
production environment. Experimental builds can therefore trigger an expensive
transactional handoff, restart production dashboard processes, and expose new
behavior to authenticated browsers before the feature is accepted.

The compilation safety wrapper has the opposite problem. It applies one global
user lock to the full Cargo invocation even though every invocation already
runs with bounded jobs and a WSL user-systemd memory cap. The lock ignores
available host capacity and serializes independent worktrees and validation
lanes unnecessarily.

The observed 2026-08-23 host snapshot reported 70 GiB total memory, 47 GiB
available memory, 12 GiB free swap, 20 logical CPU cores, and 706 GiB free disk.
This supports two modest bounded builds at that moment, but the existing swap
use and history of browser/process pressure rule out unbounded parallelism or
static reliance on total memory.

Cooper currently contains only the production `agent-browser` inventory entry:
dashboard port 4848, backend port 4849, and a production Guacamole path on
8092. No `agent-browser-dev` inventory exists.

## Frozen Domain Decisions

### 1. Environment Is An Authority, Not A Flag

A Runtime Environment owns all identities that could otherwise collide:

- stable executable selector and immutable generation store;
- runtime state and pseudo-home;
- socket and runtime-host ingress namespace;
- systemd unit names and process identities;
- dashboard ingress, backend, shadow, and authentication state;
- profile, browser, service-state, handoff, transaction, and receipt roots;
- provider and future presentation-resource namespace;
- garbage-collection inventory and effects;
- local and external ingress identity.

The implementation may use internal environment variables and deployment
descriptors, but callers receive one coherent environment selection. They do
not assemble paths, ports, unit prefixes, or state overrides themselves.

The deletion test must hold: deleting the Runtime Environment authority would
force installation identity decisions back into build scripts, systemd unit
rendering, runtime processes, acceptance harnesses, and ingress publication.

### 2. Production Defaults And Remains Read-Only

Unqualified existing product commands preserve production behavior. P125 does
not rename, relocate, restart, upgrade, or reconfigure:

- `~/.local/bin/agent-browser`;
- `~/.local/lib/agent-browser`;
- `~/.agent-browser`;
- `/run/user/<uid>/agent-browser` and the selected production host directory;
- `agent-browser-*.service` and production timers;
- dashboard ports 4848 and 4849;
- production Guacamole port 8092;
- the production Cooper inventory or external hostname.

P125 may read exact production process and readiness evidence before and after
development effects. Any attempted production mutation is a blocking defect.

### 3. Development Identity Is Exact

The first installed development environment uses:

- executable: `~/.local/bin/agent-browser-dev`;
- generation store: `~/.local/lib/agent-browser-dev/generations`;
- selected generation: `~/.local/lib/agent-browser-dev/current`;
- isolated pseudo-home: `~/.local/share/agent-browser-dev/home`;
- product state inside the pseudo-home: `.agent-browser`;
- socket namespace: `${XDG_RUNTIME_DIR}/agent-browser-dev`;
- user units: `agent-browser-dev-runtime-host.service`,
  `agent-browser-dev-dashboard-backend.service`, and
  `agent-browser-dev-dashboard.service`;
- dashboard ingress port 4948, backend port 4949, and reserved shadow port
  4950;
- seeded development runtime lane `development-default` on fixed stream port
  4951 so the isolated runtime host has a durable initial authority;
- local Cooper host: `agent-browser-dev.localhost`;
- external Cooper service and subdomain: `agent-browser-dev`.

The development dashboard must display an unmistakable development identity.
Its authentication material is development-owned and must not read or rewrite
the production dashboard auth store.

The first P125 ingress publishes only the development dashboard root. It does
not route `/guacamole` to production port 8092. A development Guacamole,
XRDP, display, or presentation provider requires its own future provider
namespace and readiness proof before publication.

### 4. Immutable Development Installation

Development publication builds the current source, hashes the resulting
binary, copies it into a new immutable development generation, atomically
selects that generation, and maintains the stable `agent-browser-dev`
environment-owning launcher. The launcher and units select the same isolated
pseudo-home, socket, authentication, and environment identity. Units execute
the selected development generation directly.

Replacing development is allowed to restart only development units. Retained
development browsers may later use the P117 transfer mechanism, but P125 does
not weaken production ownership or route a development lane through the
production runtime host.

Failed publication restores the prior development selector and units when
possible. It never falls back to production.

### 5. Build Admission Replaces Full-Lifetime Serialization

The repository wrapper retains one short exclusive lock only while inspecting
and changing Build Admission claims. The Cargo process does not hold that lock.

Admission uses:

- current `MemAvailable`, not total memory;
- swap availability and pressure posture;
- logical CPU count and requested Cargo jobs;
- free disk at the target filesystem;
- live claims whose recorded processes still exist;
- a configured host reserve;
- per-build and aggregate user-systemd cgroup limits.

Initial workstation policy is:

- at most two concurrent Agent Browser Cargo invocations;
- four Cargo jobs per invocation by default;
- 16 GiB host memory reserve;
- 14 GiB admission weight per invocation;
- aggregate Cargo slice `MemoryHigh=28G`, `MemoryMax=32G`,
  `MemorySwapMax=4G`;
- existing per-invocation WSL cgroup safety remains fail-closed;
- a third build waits with a typed capacity message;
- admission automatically drops below two when current pressure cannot
  preserve the reserve.

These are initial host policy defaults, not compiled product limits. Explicit
environment overrides remain available for controlled validation. Stale build
claims are reclaimed only after their recorded process identity is no longer
live.

Current-policy clarification: the four-job value above records this plan's
initial baseline and is not current operator guidance. The implemented wrapper
now defaults each admitted invocation to eight Cargo build jobs. Its limit of
two concurrent repository Cargo invocations is a separate admission dimension;
it does not imply two build jobs per invocation. Do not set
`AGENT_BROWSER_CARGO_BUILD_JOBS=2` solely because two invocations may run at
once. Use an explicit job override only for a diagnosed build-specific need.

### 6. Environment-Bound Acceptance

Every installed validation receipt binds:

- environment name;
- selected executable and generation digest;
- state and socket roots;
- systemd unit names and main process identities;
- dashboard ports and readiness;
- ingress inventory and hostname;
- production before and after identity snapshots;
- development cleanup obligations.

A green development receipt cannot claim production acceptance. A production
receipt cannot be generated from the development dashboard or binary.

### 7. Cooper Publication Is Inventory-Driven

The development route is added as a separate
`cooper-webservices/services/agent-browser-dev.json` inventory record with one
pinned upstream. The shared renderer and publication helpers remain the ingress
authority. Generated local or bastion configuration is never hand-edited.

The development application must behave correctly under the ingress hostname,
including forwarded scheme, redirects, cookies, assets, and authentication.
External access uses the same protection posture as the existing Agent Browser
pathway unless a stronger reviewed policy is required.

## Architecture Shape

P125 adds two deep owner modules and one deployment adapter:

1. `DevelopmentRuntimePublisher` owns development generation staging,
   selection, unit rendering, activation, status, doctor evidence, rollback,
   and cleanup.
2. `BuildCapacityAuthority` owns pressure sampling, live claim reconciliation,
   admission, waiting, release, and aggregate cgroup policy.
3. The Cooper inventory entry is a deployment adapter at the existing
   cooper-service-ingress seam. It does not duplicate routing logic in Agent
   Browser.

Production and development descriptors are the two concrete adapters that make
the Runtime Environment seam real. Tests exercise the same publisher and
capacity interfaces used by operators.

## Implementation Slices

### Slice A | Contracts, Domain, And Red Fixtures

- freeze production and development environment descriptors;
- freeze environment receipts and production non-interference snapshots;
- add path, port, unit, state, and ingress collision guards;
- add a development dashboard identity fixture;
- update P124 to depend on P125.

No installed or ingress effect occurs in this slice.

### Slice B | Resource-Aware Build Capacity

- replace the full-lifetime Cargo flock with short admission locking;
- reconcile live and stale claims;
- sample current resources and compute admitted concurrency;
- run admitted builds inside the aggregate Cargo slice and per-build scope;
- retain fail-closed WSL behavior;
- add deterministic one-build, two-build, pressure, stale-claim, release, and
  unavailable-systemd fixtures.

### Slice C | Development Runtime Publisher

- build and stage one immutable development generation;
- atomically select `agent-browser-dev`;
- render and activate only development runtime-host and dashboard units;
- isolate HOME-derived state and the socket namespace;
- surface a visible development dashboard identity;
- implement status and doctor readback;
- retain bounded rollback and old development-generation cleanup.

### Slice D | Cooper Development Ingress

- create the pinned `agent-browser-dev` inventory entry;
- validate and render local ingress;
- verify raw and `agent-browser-dev.localhost` application behavior;
- publish the external route through the existing Cooper helper;
- verify unauthenticated protection and authenticated dashboard reachability;
- confirm no production inventory or generated route changed unexpectedly.

### Slice E | Installed Non-Interference Acceptance

1. Capture production binary, selector, unit, PID, socket, dashboard, handoff,
   and retained-browser identities.
2. Publish a development build and start the development units.
3. Verify one development dashboard, one development runtime host, and one
   selected development binary generation.
4. Launch only an isolated disposable development profile and verify it appears
   only in development state.
5. Replace the development generation and verify production identities remain
   unchanged.
6. Run development doctor, local ingress, external ingress, auth, and dashboard
   identity smokes.
7. Stop and restart development units while production remains ready.
8. Dry-run development generation and state cleanup and prove production is
   outside the candidate set.
9. Re-read operating-system process and resource state.

## Required Validation

```bash
pnpm test:wsl-cargo-safety
pnpm test:development-runtime
pnpm validation:select -- --base HEAD
scripts/ci/cargo-safe.sh fmt --manifest-path cli/Cargo.toml -- --check
scripts/ci/cargo-safe.sh clippy --manifest-path cli/Cargo.toml -- -D warnings
git diff --check
```

Installed and ingress gates are recorded separately from source checks. The
production non-interference snapshot is mandatory after every development
installation or unit activation during acceptance.

## Acceptance Criteria

P125 is complete only when:

- the development binary, generations, state, sockets, units, auth, dashboard,
  profiles, receipts, and cleanup scope are isolated from production;
- unqualified existing product commands still target production;
- development installation cannot select or restart production units;
- the development dashboard is visibly identified and runs on its pinned raw
  port;
- `agent-browser-dev.localhost` and the protected external
  `agent-browser-dev` hostname reach the development dashboard;
- no development `/guacamole` route points at the production provider;
- two bounded Cargo invocations run concurrently when pressure admits them;
- pressure, configured maximum, or missing user-systemd capacity queues or
  rejects additional builds without running uncapped;
- stale build claims converge without allowing live claims to be stolen;
- repeated development publication retains only the selected development
  generation plus the configured rollback allowance;
- development doctor detects cross-environment path, unit, port, process,
  selector, auth, or ingress drift;
- production process identities, executable digest, runtime-host selection,
  dashboard readiness, retained browsers, and durable handoffs survive all
  accepted development effects;
- source, development installed, production installed, ingress, and release
  acceptance remain separate claims.

## Hard Stops

- Do not run the production workstation installer to validate development.
- Do not reuse production state, sockets, auth, profiles, unit names, ports,
  Guacamole routes, or cleanup inventory.
- Do not restart or reload production Agent Browser units.
- Do not publish a development `/guacamole` route to production port 8092.
- Do not replace the Cargo mutex with unbounded concurrency.
- Do not compute admission from total memory alone.
- Do not let each build receive an aggregate-equivalent memory maximum without
  a shared aggregate cap.
- Do not claim production acceptance from development evidence.
- Stop on ambiguous process or environment ownership.

## First Bounded Packet

Execute Slices A and B. Land the frozen environment contracts and the
resource-aware Build Capacity Authority with deterministic fixtures. Then use
the admitted build path to implement and publish the development Runtime
Environment without touching production.

## Acceptance Result

Accepted on 2026-08-23. The exact acceptance evidence is recorded in
`docs/dev/notes/0125-2026-08-23-development-runtime-isolation-acceptance.md`.

The development environment owns one immutable selected executable, one
runtime host, one dashboard backend, one dashboard, private state and auth,
and the dedicated local and external Cooper routes. Development publication,
generation replacement, garbage collection, stop, and restart left the
production executable selection, process identities, dashboard manifest,
durable handoff digest, and retained browser and session identities unchanged.

Build Admission admitted two real Cargo invocations concurrently when current
capacity permitted it and reduced admission under observed workstation
pressure. The repository no longer serializes the full lifetime of every
Agent Browser Cargo command.

One fresh-Chrome follow-up ended before CDP startup with Chrome exit code zero
and no diagnostic stderr. An earlier isolated managed-profile launch did reach
the development runtime and proved profile and process isolation. The later
host-level Chrome behavior is retained as an operational observation, not an
ingress, environment-identity, or production-non-interference failure.
