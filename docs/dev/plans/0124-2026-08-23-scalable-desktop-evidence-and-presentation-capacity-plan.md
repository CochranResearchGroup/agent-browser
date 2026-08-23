# Plan 0124 | Scalable Desktop Evidence And Presentation Capacity

Date: 2026-08-23

State: IN PROGRESS

Execution state: `slice_c_source_accepted`

Lane: P124

Authority: SOURCE-ONLY | NO LIVE DESKTOP OR INSTALL EFFECTS

Depends on:

- `docs/dev/plans/0126-2026-08-23-pre-development-runtime-safety-and-browser-launch-stabilization.md`
- `VISION.md`
- `docs/dev/plans/0125-2026-08-23-development-runtime-isolation-and-build-capacity-plan.md`
- `docs/dev/plans/0110-2026-08-12-desktop-perception-interaction-foundation-plan.md`
- `docs/dev/plans/0029-2026-06-07-live-retained-pressure-cleanup-plan.md`
- `docs/dev/plans/0060-2026-06-27-s7-route-pool-exhaustion-plan.md`
- `docs/dev/plans/0067-2026-07-05-rdp-reattachment-stress-hardening-plan.md`
- `docs/dev/plans/0111-2026-08-13-multi-agent-shared-browser-profile-authority-plan.md`
- `docs/dev/plans/0115-2026-08-14-cdp-automation-etiquette-plan.md`
- `docs/dev/plans/0117-2026-08-19-runtime-lifecycle-authority-and-convergence-plan.md`

## Goal

Replace the fixed two-route desktop model with one scalable presentation
capacity authority and one deep Desktop Evidence Episode module. Agents should
state what evidence they need. Agent Browser should decide whether page-level
CDP evidence is sufficient, reserve presentation capacity only when desktop
evidence is necessary, stage and prove the target scene, capture or interact,
restore the scene, release capacity, and reclaim terminal slot resources
without disturbing retained browsers or human work.

The installed acceptance target is four warm presentation slots with a
controlled elastic scale-out test to six. The product model remains arbitrary
N and is bounded by configuration, current host pressure, and exact lifecycle
authority rather than by route labels or a compiled constant.

## Problem Statement

The current source has the right identities but the wrong capacity shape:

- `desktop_capture` binds a browser, RDP stream, route, display allocation,
  geometry, and ephemeral frame receipt;
- capture readiness requires `browser_window_visible`, but does not prove the
  selected browser scene is maximized, topmost, unoccluded, or exclusively
  staged;
- remote-view focus can raise and maximize a PID-bound browser window, but the
  capture path does not own that preparation or its restoration;
- the controller coordinator serializes machine input and human takeover, but
  read-only capture and visible desktop staging have no equivalent admission;
- the Guacamole readiness script recognizes route A and route B and truncates
  the candidate inventory to two entries;
- P67 proved that more retained browsers than route slots can remain alive and
  reattachable, but it deliberately retained two as the installed presentation
  ceiling;
- agents can call CDP screenshot or desktop capture directly and therefore
  must understand capture blindness, route occupancy, focus, and human-viewer
  side effects themselves;
- display, route, viewer, and browser cleanup exist as separate mechanisms,
  but elastic slot scale-in has no one cleanup obligation or pressure-aware
  garbage-collection contract.

Two slots are not an acceptable steady-state product assumption. One human
viewer and one desktop-aware agent can consume the whole pool, leaving no
ready capacity for another agent, recovery, staging, or handoff.

## Frozen Domain Decisions

### 1. Evidence Need, Not Capture Mode

The agent-facing decision is the required evidence surface:

- page DOM, accessibility, viewport, canvas, and page pixels use CDP;
- browser chrome, extension UI, password-manager prompts, passkey choosers,
  native dialogs, operating-system windows, stacking, and occlusion use a
  Desktop Evidence Episode;
- a JavaScript dialog, file transfer, permission, or other state already
  represented by a supported CDP mechanism remains a CDP operation even when
  it appears modal;
- generic CDP timeout, crash, missing endpoint, or target-discovery failure is
  not evidence that desktop capture is needed;
- classification of a browser-external prompt requires paired evidence when
  practical: a desktop candidate plus page, DOM, or CDP absence evidence;
- biometric, secure-desktop, PIN, master-password, and consent surfaces return
  typed human continuation unless a separately approved policy says otherwise.

Raw capture-mode choice remains available only as a narrow diagnostic or
expert mechanism. Task-shaped agent guidance and future workflows use the
evidence decision.

### 2. Desktop Evidence Episode

A Desktop Evidence Episode is one bounded transaction that owns:

1. evidence classification;
2. presentation-slot admission when required;
3. exact browser-scene resolution;
4. optional pre-trigger staging;
5. capture-ready proof;
6. observation and any separately authorized input;
7. after-state verification;
8. scene restoration;
9. slot release and cleanup evidence.

An episode may reserve and stage a scene before the browser action expected to
open an extension or native popup. It must not rely on focusing the browser
after the popup appears because focus changes can dismiss browser-external UI.

The module is deep. Its interface does not expose display names, raw routes,
provider URLs, coordinates, window handles, route labels, or provider-specific
staging commands. CDP, X11, Guacamole, route-pool, window-semantic, capture,
input, and handoff implementations remain internal adapters at real seams.

### 3. Presentation Slot

A Presentation Slot is scarce operator-visible desktop capacity. It binds one
route, display, current scene, viewer and controller posture, geometry epoch,
readiness, lifecycle generation, and cleanup obligation.

A slot is not browser ownership. Parking or releasing a presentation must
preserve the retained browser, profile, tabs, service identity, and durable
handoff. A CDP-only browser consumes no presentation slot.

Slot lifecycle states are:

```text
absent
  -> provisioning
  -> warm_idle
  -> reserved
  -> staging
  -> capture_ready
  -> active
  -> restoring
  -> warm_idle
  -> cooling
  -> reclaiming
  -> absent
```

Any uncertain provision, restore, or reclaim outcome enters `quarantined`
rather than being reused or silently deleted.

### 4. Arbitrary-N Capacity

No source path may assume two slots, route A and B, two users, two displays, or
two Guacamole connections. Stable opaque slot and route identities replace
alphabetic labels in the canonical model. An indexed compatibility adapter may
read legacy A and B configuration during migration, but canonical state and
new configuration are list-shaped.

Capacity has three independent values:

- configured warm minimum;
- configured hard maximum;
- current admitted maximum derived from host-pressure evidence.

Initial controlled installed acceptance uses:

- warm minimum: 4;
- configured hard maximum: 6;
- one human-priority reserve;
- one recovery and handoff reserve;
- at least two general desktop-work slots.

Reserved capacity may serve a short preemptible observation episode when idle,
but the allocator must not begin non-preemptible agent work that could prevent
an operator handoff or recovery admission. No episode is preempted in the
middle of an acknowledged input effect.

These values are an installed acceptance profile, not compiled product limits.

### 5. Human And Multi-Agent Priority

Admission priority is:

1. active human control and continuation;
2. runtime recovery and durable-handoff restoration;
3. already-started desktop effects and their verification or cleanup;
4. desktop observation;
5. optional presentation requested only for inspection convenience.

An active human controller blocks automated staging without explicit takeover
authority. A passive human viewer permits capture only when the exact scene is
already capture-ready. If staging would visibly rearrange the desktop, the
episode waits by default.

Requests within one priority class use FIFO ordering with bounded aging.
Capacity pressure returns typed queue position, limiting resource, and next
safe action. It does not masquerade as browser failure, shared-runtime cleanup,
or route corruption.

### 6. Capture-Ready Proof

`browser_window_visible` remains operator-view proof but is insufficient for
desktop evidence. Capture-ready proof binds:

- exact browser and process generation;
- allowed browser top-level and child or popup windows;
- route, display allocation, presentation slot, and scene generation;
- active and topmost window evidence;
- maximized work-area geometry or a policy-approved exact scene geometry;
- absence of unowned occluding windows over the authorized capture region;
- frame dimensions, scale, crop, coordinate mapping, and geometry epoch;
- viewer and controller posture;
- proof time and freshness.

The proof is re-read after capture and after every input effect. Drift discards
the frame or stops the transaction before another effect.

### 7. Restoration

Staging records the prior scene before mutation. Completion restores window
geometry, stacking, focus, route attachment, and viewer posture when they still
belong to the episode generation. Human takeover, route replacement, or other
authority drift cancels restoration that would overwrite newer intent and
returns a typed terminal receipt.

### 8. Elastic Provisioning And Garbage Collection

Scale-out provisions one slot at a time after proving:

- queued desktop demand cannot use a current or parkable slot;
- configured maximum has not been reached;
- memory, swap, process, file-descriptor, display-helper, and browser-pressure
  admission are within configured limits;
- the provisioning adapter can produce an isolated display, RDP target,
  Guacamole route, required permissions, and current readiness evidence;
- rollback can remove only the resources created by that attempt.

Scale-in begins only after the cooldown interval and only when the slot has:

- no browser presentation attachment;
- no acquisition or episode lease;
- no viewer or controller lease;
- no durable handoff that currently resolves through it;
- no rollback quarantine or recovery reference;
- no pending restoration or cleanup obligation;
- exact process and provider-resource identity.

Garbage collection reclaims presentation resources, not retained browsers or
profiles. It removes provider records and helper processes only through exact
owned identity, records a terminal cleanup receipt, and stops on ambiguity.
Repeated provision and reclaim cycles must converge to the warm minimum without
stale Xorg, XRDP, Guacamole, viewer, daemon, or browser-process accumulation.

## Architecture Shape

The target contains three concrete owner modules:

1. `DesktopEvidenceCoordinator` owns the episode transaction and evidence
   decision.
2. `PresentationCapacityAuthority` owns arbitrary-N slot inventory, admission,
   priority, leases, queueing, scale decisions, and capacity projections.
3. `PresentationLifecycleAuthority` owns provider provisioning, rollback,
   cooldown, reclaim, quarantine, and cleanup receipts.

Existing route selection, retained-browser reattachment, desktop capture,
desktop control coordination, browser-window focus, service-state persistence,
runtime lifecycle, and durable handoff modules remain concrete owners. The new
owners call them through narrow internal seams instead of copying their
invariants.

The deletion test must hold: deleting any new owner would force its decisions
back into several callers. A pass-through module does not satisfy this plan.

## Capacity And Pressure Projections

Service State, CLI, HTTP, MCP, generated client, dashboard, doctor, and
diagnostics eventually expose one coherent redacted capacity projection:

- configured warm minimum and hard maximum;
- current pressure-admitted maximum;
- total, provisioning, warm-idle, reserved, staging, capture-ready, active,
  restoring, cooling, reclaimable, reclaiming, and quarantined slots;
- human-protected and recovery-reserved capacity;
- queued demand by priority and oldest wait age;
- scale-out and scale-in decisions with typed reasons;
- owned cleanup obligations and stale-process warnings;
- current pressure readings and threshold source without private command lines
  or provider credentials.

The dashboard must distinguish logical browsers from presentation slots. It
must not render one browser tile per transient route or display record.

## Compatibility And Migration

- Preserve current two-route installations as valid legacy input.
- Read legacy A and B environment variables through one compatibility adapter.
- Project them into canonical list-shaped slot candidates before readiness or
  allocation logic.
- Do not manufacture slots merely to satisfy a configured count when provider
  readiness, permissions, isolation, or pressure evidence is missing.
- Preserve durable handoffs and retained browser identities while routes move
  between legacy and canonical inventory.
- Keep P60's typed capacity exhaustion and P67's browser preservation behavior
  as regressions, while allowing scale-out before final exhaustion.

## Implementation Slices

### Slice A | Red Contracts And Arbitrary-N Fixtures

- freeze evidence-decision, episode, slot, lease, queue, pressure, lifecycle,
  cleanup, and receipt fixtures;
- cover zero, one, two, four, six, and eight configured slots;
- add selection fixtures for CDP, desktop, paired evidence, diagnostic failure,
  and human-only outcomes;
- add human controller, passive viewer, multi-agent fairness, pre-trigger popup,
  restoration drift, pressure rejection, and GC ambiguity scenarios;
- add an architecture guard that detects new canonical A or B assumptions and
  fixed two-entry truncation.

No browser, display, Guacamole, RDP, provider, or installed-runtime effect is
authorized.

### Slice B | Generalize Static Route Inventory

- replace two-entry candidate selection with canonical list-shaped inventory;
- generalize route users, display discovery, permissions, readiness, doctor,
  installation reconciliation, configuration, and diagnostics to arbitrary N;
- retain legacy A and B parsing only in the compatibility adapter;
- keep route selection and parking deterministic across N entries;
- prove no regression for an existing two-slot installation.

This slice does not dynamically provision or remove provider resources.

### Slice C | Presentation Capacity Authority

- add durable slot inventory and scene generation;
- implement priority admission, reserves, FIFO aging, bounded queueing, and
  per-browser plus per-slot exclusion;
- integrate current viewer, controller, acquisition, route parking, retained
  browser, and handoff authority;
- return typed capacity and authority outcomes without opening a route or
  launching a browser merely to inspect capacity;
- add redacted service and dashboard projections.

### Slice D | Desktop Evidence Episode

- add the deep coordinator with injected CDP, desktop-frame, window-semantic,
  staging, slot, capture, input, verification, restoration, and handoff
  adapters;
- make CDP the default for page evidence and allocate no presentation slot;
- support reservation and staging before a trigger expected to open external
  UI;
- require capture-ready proof instead of only operator-visible proof;
- bind before, capture, after, restoration, release, and cleanup evidence into
  one episode receipt;
- keep existing low-level capture actions as narrow diagnostic mechanisms.

Configured production input remains unavailable until its existing P110 gates
are independently satisfied.

### Slice E | Elastic Lifecycle And Garbage Collection

- implement one-at-a-time scale-out behind pressure admission;
- implement cooldown and exact-reference scale-in;
- add rollback quarantine and deterministic cleanup obligations;
- join slot lifecycle with service GC and runtime resource inventory without
  making either one a shallow proxy for the other;
- prove repeated fixture cycles return to the warm minimum with zero leaked
  resource identities.

### Slice F | Agent And Operator Product Surface

- make agent guidance lead with evidence need and the CDP-first decision table;
- expose capacity, queueing, scene readiness, and human precedence coherently
  across CLI help, README, the agent skill, docs site, HTTP, MCP, generated
  client, doctor, and dashboard;
- show logical browsers separately from presentation capacity;
- provide a durable handoff when the episode reaches a human-only outcome;
- never expose raw Guacamole URLs, display names, provider credentials, private
  pixels, or secret-bearing OCR text.

### Slice G | Controlled Installed Acceptance

After source acceptance and separate explicit live authority:

1. install through the accepted transactional workstation path;
2. provision four warm slots and prove each has a distinct display, route,
   target, user or isolation identity, and operator-visible proof;
3. run one human viewer, two concurrent desktop observations, and one recovery
   reservation without route or browser identity drift;
4. configure a six-slot maximum and prove a fifth concurrent eligible demand
   provisions exactly one additional slot;
5. release demand, wait through the bounded cooldown, reclaim the elastic slot,
   and converge to four warm slots;
6. repeat scale-out and scale-in while checking OS process trees, memory, swap,
   display helpers, Guacamole records, leases, cleanup obligations, retained
   browsers, and durable handoffs;
7. prove ordinary CDP-only work does not change slot counts;
8. prove active human control blocks staging and that a passive viewer is not
   disrupted by an unstaged capture request;
9. prove a retained authenticated browser survives parking, route movement,
   scale-in of an unrelated slot, runtime reconciliation, and dashboard
   refresh.

The live acceptance artifact records measured pressure and recommends the
installed warm minimum and maximum. It does not convert those values into
compiled product limits.

## Required Validation

Each slice runs the repository selector plus focused checks for every touched
surface. Rust compilation on WSL uses `scripts/ci/cargo-safe.sh`.

Minimum source gates across the completed plan:

```bash
scripts/ci/cargo-safe.sh fmt --manifest-path cli/Cargo.toml -- --check
scripts/ci/cargo-safe.sh clippy --manifest-path cli/Cargo.toml -- -D warnings
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml desktop_capture -- --test-threads=1
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml desktop_control_coordinator -- --test-threads=1
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml remote_view_reattach -- --test-threads=1
pnpm test:service-client
pnpm test:service-api-mcp-parity
pnpm test:service-contracts-no-launch
pnpm test:dashboard-view-streams
pnpm test:dashboard-inspector-actions
node scripts/check-actions-architecture.js --check
pnpm --dir docs build
git diff --check
```

Additional focused commands must be added with the slices rather than hidden
inside a broad final gate. Live tests remain serial where they start Chrome or
mutate presentation resources.

## Acceptance Criteria

P124 is complete only when:

- no canonical source path truncates route or slot inventory to two;
- legacy two-route installations remain compatible through one adapter;
- the capacity model supports arbitrary N and the installed acceptance proves
  four warm plus elastic scale-out to at least five under a six-slot maximum;
- CDP-only work allocates no slot;
- agents receive one coherent evidence decision and do not need to understand
  provider plumbing;
- desktop evidence requires a fresh capture-ready proof that includes scene,
  focus, topmost, maximize or exact geometry, occlusion, viewer, controller,
  route, display, process, and geometry identity;
- human control, recovery, existing effects, observations, and convenience
  viewing follow the frozen priority order without starvation;
- staging before a popup-triggering action is supported and restoration cannot
  overwrite newer human or route intent;
- retained browsers, profiles, tabs, and durable handoffs survive parking,
  route reassignment, scale-out, scale-in, reconciliation, and dashboard
  refresh;
- repeated provision and reclaim cycles return to the warm minimum with zero
  stale browser, Xorg, XRDP, Guacamole, viewer, daemon, or helper-process
  accumulation;
- every uncertain cleanup is quarantined and visible rather than reused;
- CLI, HTTP, MCP, generated client, dashboard, help, skill, README, docs, and
  inline comments describe the same shipped behavior;
- source, installed, live, and release acceptance remain separate claims.

## Hard Stops

- Do not implement another desktop feature directly against raw capture,
  coordinates, XTEST, display names, route labels, or provider URLs.
- Do not increase route count without generalizing identity, readiness,
  permissions, reconciliation, diagnostics, and cleanup to the same N.
- Do not automatically fall back from generic CDP failure to desktop capture.
- Do not stage or rearrange a desktop under active human control without
  explicit takeover authority.
- Do not reclaim a slot with any browser, lease, viewer, controller, handoff,
  rollback, restoration, or cleanup reference.
- Do not kill retained browsers or profiles to satisfy a presentation-capacity
  target.
- Do not provision beyond current pressure admission or the configured hard
  maximum.
- Do not treat `browser_window_visible` as capture-ready proof.
- Do not claim installed or live acceptance from provider-free fixtures.
- Do not start live provisioning, capture, input, authentication, provider, or
  installation work without the separately applicable authority.

## First Bounded Packet

Execute P125 first. After its development Runtime Environment passes installed
non-interference acceptance, execute Slice A only. Freeze provider-free red
fixtures and architecture guards
for evidence selection, arbitrary-N inventory, four-slot warm capacity,
priority admission, scene readiness, restoration, elastic lifecycle, and exact
GC. Do not change live readiness, install configuration, provider resources,
or the current two-slot workstation.
