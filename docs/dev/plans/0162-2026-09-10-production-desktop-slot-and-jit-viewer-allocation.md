# Plan 0162: Production Desktop Slots and Just-in-Time Viewer Allocation

Date: 2026-09-10

State: OPEN

Consolidation: required

Lane: P157

Branch: plan/profile-permissions-and-request-provenance

Target: main

Integration: merge

Parent acceptance: [Plan 0160](0160-2026-09-06-production-profile-identity-and-operational-readiness.md), A2 and A3

Predecessor: [Plan 0124](0124-2026-08-23-scalable-desktop-evidence-and-presentation-capacity-plan.md)

Sequencing dependency: [Plan 0161](0161-2026-09-09-first-class-profile-repair-and-reset-plan.md), preserving-repair source milestone

## Objective and authority

Adopt Plan 0124's arbitrary-N presentation-capacity model in production and
separate browser-to-desktop assignment from just-in-time Guacamole viewing.
Ordinary clients select a logical browser. Agent Browser selects and proves the
desktop slot, and only a human viewer request opens or reuses the Guacamole
stream for that desktop.

This plan is authorized planning and implementation under Plan 0160. Production
installation and live acceptance use Plan 0160's existing candidate-install and
synthetic operator-journey authority. It does not authorize tenant business
actions, credential entry, destructive profile reset, formal release, or
unbounded provider cleanup.

## Current state

Plan 0124 source and development acceptance already support list-shaped
arbitrary-N inventory, presentation capacity, four warm slots, elastic 4 to 6
to 4 lifecycle checks, retained-browser preservation, multi-viewer/controller
posture and exact cleanup. Production did not adopt that accepted topology.

The current workstation exposes three Guacamole RDP connection and route-pool
records but only two admitted presentation slots. `guacamole:3` remains marked
available even though no corresponding capacity slot exists. Automatic
selection can therefore choose it and fail with
`presentation_bound_slot_missing`. Historical provider records, desired
topology and current occupancy are insufficiently separated.

The current public request shape also retains expert route-selection inputs
created for deterministic fixtures and recovery. Those provider details have
leaked into ordinary remote-view allocation. A dashboard browser-tile click
should never require or accept a Guacamole route choice.

Plan 0161 remains the immediate profile ownership and identity priority. This
plan begins sustained implementation after Plan 0161's preserving-repair source
milestone unless a disjoint fixture or contract packet can proceed without
overlapping its Rust service-state owners.

## Consolidated batch

One production batch delivers three coherent outcomes:

1. configurable desktop-slot inventory with an exact slot, route user,
   observed display, provider connection and capacity binding;
2. logical-browser allocation that selects only admitted desktop capacity; and
3. just-in-time viewer allocation that opens or joins the selected browser's
   proven desktop without exposing provider route choice to ordinary clients.

The initial production configuration has three warm desktop slots and a hard
maximum of three. It uses the already provisioned route-specific users and
connections only after all three pass the same readiness contract. General
automatic creation of additional Linux users beyond the configured inventory
is deferred. The model and contracts remain list-shaped and arbitrary-N.

## Frozen decisions

### Desktop slots are the allocation authority

A selectable production slot has one stable opaque slot identity and an exact
current binding to:

- one route-specific XRDP identity;
- one observed current-boot X desktop, whose display number is never inferred
  from its ordinal;
- one provider connection and route;
- one capacity record and lifecycle generation; and
- current browser, acquisition, viewer, controller, handoff and cleanup
  references.

Provider discovery alone never admits capacity. Every selectable route must
have exactly one admitted slot, and every admitted slot must resolve to exactly
one current provider route and desktop. A mismatch is a typed topology finding,
not an allocation candidate.

### Browser and viewer lifecycles are separate

An active remote-headed browser occupies one desktop slot. A browser that needs
only CDP consumes no operator-presentation slot. The browser-to-desktop binding
survives viewer disconnects and remains attributable until the browser is
parked, closed or moved through an exact lifecycle transition.

Clicking a browser tile requests presentation for that logical browser. The
service reuses its proven desktop binding or atomically reserves the next
eligible desktop slot, verifies the browser scene, and returns the opaque
durable `/remote-view/<handoff-id>` URL. Guacamole viewer sessions are created
or joined only for that request. Additional authorized viewers join the same
desktop and consume viewer leases, not additional desktop slots. Controller
authority remains separate from passive viewing.

### Physical route selection is internal

Ordinary CLI, HTTP, MCP, generated-client and dashboard workflows provide a
logical browser or durable handoff identity. They do not choose a Guacamole
connection, route-pool row or display. Exact physical selection remains an
internal allocator decision. A narrowly named diagnostic fixture may request a
physical route, but that surface cannot be used by ordinary remote-view open or
browser-tile actions.

### Desired, observed and historical state stay distinct

The installed manifest declares desired slot inventory and configured
capacity. Current provider and process probes establish observed readiness and
occupancy. Events, receipts and retired records preserve history. An extra
discovered or retained Guacamole connection becomes `legacy_unadmitted` and is
ineligible for selection until an explicit topology transaction binds and
proves it as a slot.

Install, update and reconcile perform a revision-fenced topology transaction.
They never merge all responsive provider connections into the selectable pool.
Interrupted transactions either preserve the prior admitted topology or retain
one exact quarantined cleanup obligation.

## Work units

| Unit | Work and expected write surface | Exit evidence | Dependency |
| --- | --- | --- | --- |
| W0 | Freeze provider-free regressions for three provider connections with two capacity slots, raw-route client selection, current-boot display renumbering, one browser with three viewers and three browsers with three slots. | The current `guacamole:3` mismatch fails for the expected reason; desired, observed and historical fixtures are distinct. | Plan ready |
| W1 | Make presentation capacity the sole ordinary allocation source. Migrate or quarantine legacy route-pool rows and preserve exact handoff/browser references. | No unbound route is selectable; a complete slot C is selectable; restart and interrupted migration converge without state-file editing. | W0 |
| W2 | Separate logical browser-to-desktop assignment from just-in-time viewer allocation. Remove physical route choice from ordinary browser-tile and client workflows while retaining a bounded diagnostic seam. | First tile click allocates or reuses one desktop and returns an opaque handoff; two later viewers join it without allocating more desktops. | W1 |
| W3 | Generalize installer, update, doctor, runtime manifest and supported repair to configured N. Adopt three warm production slots while retaining a clean-install default of two. | Fresh two-slot and retained three-slot upgrade fixtures pass; doctor reports desired, admitted, observed, quarantined and selectable counts with exact recourse. | W1 |
| W4 | Synchronize CLI help, README, installed skill, docs site, inline comments, schemas, generated client and dashboard. Run changed-surface validation once against a frozen candidate. | Every public surface describes logical-browser selection, JIT viewing and configurable capacity consistently. | W2 and W3 |
| W5 | Install through Plan 0160 and run synthetic production acceptance. | Three browsers are simultaneously visible on three distinct desktops; one browser accepts three concurrent authorized viewers through one durable handoff; restart and release leave no stale selectable rows. | W4 and Plan 0160 runtime gate |

## Delivery sequence and budget

Use three bounded source packets and one separately measured installation gate.
Cumulative source implementation, review, validation and documentation target
is three hours before mandatory replanning.

1. **Packet A, 45 minutes:** the primary freezes W0 predicates and owns the W1
   allocation invariant while one Luna-medium worker writes only the named
   provider-free regression fixtures. Stop if the current state model cannot
   distinguish provider discovery from admission.
2. **Packet B, 60 minutes:** the primary owns W2 logical-browser and JIT viewer
   semantics. Use no implementation worker unless CodeGraph identifies a
   disjoint adapter with a frozen interface and test. Stop after the first
   architecture ambiguity that would require changing browser ownership
   semantics owned by Plan 0161.
3. **Packet C, 75 minutes:** the primary owns W3 installer, doctor and migration
   semantics. After contracts freeze, one Luna-low worker synchronizes only the
   enumerated generated-client and documentation paths while the primary runs
   focused source validation. Freeze one candidate after the complete
   changed-surface gates pass.
4. **Installation gate:** perform one production install and one bounded W5
   acceptance sequence. Do not rebuild or reinstall repeatedly around live
   symptoms; return any deterministic defect to a new source packet.

At two consecutive checkpoints without acceptance progress, consolidate the
remaining blocker before further implementation. Prior Plan 0124 and Plan 0160
effort remains historical evidence and is not repeated.

## Worker assignments and model routing

Optimize for balanced wall-clock and token efficiency. Use deterministic
CodeGraph, generators and focused repository checks before model work. Worker
briefs use fresh compact context rather than the conversation history. They
must enumerate exact files before spawning, prohibit adjacent edits, request a
patch plus decisive evidence, and return partial evidence at their timeout.

| Lane | Model and effort | Exact scope and parallel primary work | Timeout and stop condition |
| --- | --- | --- | --- |
| Critical path | Primary on the current strongest available model | Own domain decisions, Rust allocation and migration semantics, integration, production effects and acceptance. While a fixture or parity worker runs, inspect or implement only the corresponding non-overlapping owner surface. | Reassess at packet bounds, ownership overlap with Plan 0161, or a change to the frozen invariant. |
| W0 fixture matrix | `gpt-5.6-luna`, medium | After the primary freezes exact predicates, edit only the enumerated provider-free test modules for 3-provider/2-slot mismatch, display renumbering, 3 browsers/3 slots and 1 browser/3 viewers. No production source or snapshots outside the packet. | 30 minutes or first semantic ambiguity; return tests and observed red predicates. |
| W2 adapter, conditional | `gpt-5.6-terra`, medium | Only if CodeGraph proves a disjoint adapter seam, implement one frozen logical-browser or viewer adapter plus its focused test. The primary continues the Rust capacity owner. | 35 minutes, first interface change, or first failing check outside the assigned adapter. |
| W4 mechanical parity | `gpt-5.6-luna`, low | After schemas freeze, run deterministic generators and edit only enumerated generated-client, parity-registry and required documentation paths. No contract or behavior decisions. | 30 minutes, generator failure, schema mismatch or need for semantic prose. |
| Closed-world review | `gpt-5.6-sol`, medium, fresh context | Review one frozen commit against route admission, slot-neutral multi-viewing, migration rollback and client abstraction. Return evidence-shaped findings or an explicit no-finding result; make no edits. | 25 minutes or one complete finding set; no broad discovery or second review pass. |

Use at most two workers concurrently with the primary, nesting depth one and no
worker-created children. Do not spawn a worker merely to wait for Cargo, a
build, installation or live acceptance. The primary reviews each actual diff
and decisive result without repeating the worker's investigation, and owns all
shared-source integration. No worker receives production mutation,
installation, credential, provider-cleanup or session-management authority.

For each executed worker record its task name, runtime handle, requested model
and effort, runtime-reported effective model when available, start and finish
time, terminal status, accepted output, rework and available token or allocation
metadata in the current checkpoint. Unknown runtime metadata stays unknown;
requested cheaper routing is not evidence of savings.

## Evidence and exit

Use one evidence table in `RUNBOOK.md` for source, candidate, installed and
user-outcome states. Required acceptance evidence includes:

- a one-to-one admitted slot, desktop and route census for all three slots;
- current-boot process and display proof without ordinal display assumptions;
- rejection or quarantine of every extra unbound provider connection;
- three simultaneously visible synthetic browsers with distinct slot bindings;
- three concurrent authorized viewer leases for one browser and one desktop;
- identical durable handoff reuse without another desktop allocation;
- unauthorized viewer denial and controller separation;
- restart, interrupted topology transaction and exact release convergence;
- clean two-slot install and retained three-slot upgrade migration fixtures;
- zero stale selectable routes, viewer leases or cleanup obligations after
  acceptance; and
- exact source, binary, support-manifest and installed-generation identities.

Plan 0162 is complete when W0 through W5 pass on one installed candidate and
Plan 0160 records A2's slot/viewer portion and the related A3 topology findings
as accepted. External-vantage evidence still required by Plan 0160 remains a
separate acceptance dimension.

## Non-goals

- automatic creation of arbitrary Linux route users beyond the configured
  inventory;
- permanent profile ownership of a numbered X display;
- exposing Guacamole routes, raw provider URLs or display numbers to ordinary
  clients;
- changing profile access, lease or principal policy owned by Plan 0161;
- tenant browser actions, authentication entry or private page capture;
- formal release; or
- rerunning Plan 0124's already accepted development campaign.
