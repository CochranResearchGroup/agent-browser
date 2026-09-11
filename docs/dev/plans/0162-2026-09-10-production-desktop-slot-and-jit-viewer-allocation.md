# Plan 0162: Production Desktop Slots and Just-in-Time Viewer Allocation

Date: 2026-09-10

State: OPEN

Consolidation: required

Lane: P157

Branch: unallocated; create from current `main` when execution resumes

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
| W3 | Generalize installer, update, doctor, runtime manifest and supported repair to configured N. Adopt three warm production slots while retaining a clean-install default of two. | Fresh two-slot and retained three-slot upgrade fixtures pass; doctor reports desired, admitted, observed, quarantined and selectable counts with exact recourse. | W1; runs parallel to W2 after exact file ownership is frozen |
| W4 | Synchronize CLI help, README, installed skill, docs site, inline comments, schemas, generated client and dashboard. Run changed-surface validation once against a frozen candidate. | Every public surface describes logical-browser selection, JIT viewing and configurable capacity consistently. | W2 and W3 |
| W5 | Install through Plan 0160 and run synthetic production acceptance. | Three browsers are simultaneously visible on three distinct desktops; one browser accepts three concurrent authorized viewers through one durable handoff; restart and release leave no stale selectable rows. | W4 and Plan 0160 runtime gate |

## Delivery sequence and budget

Use three bounded source packets and one separately measured installation gate.
Cumulative source implementation, review, validation and documentation target
is 165 minutes before mandatory replanning.

1. **Packet A, 40 minutes, fan-out then join:** the primary freezes W0
   predicates and owns the W1 allocation invariant while one Luna-medium worker
   writes only the named provider-free regression matrix. Join on the same
   failing 3-provider/2-slot predicate. Stop if the state model cannot
   distinguish provider discovery from admission.
2. **Packet B, 70 minutes, two-way implementation fan-out:** after W1 freezes
   topology types and exact write ownership, the primary owns W2 service
   allocation and JIT viewer semantics while one Terra-medium worker owns the
   disjoint W3 workstation installer, manifest and doctor adapters. A
   Luna-medium worker may own one disjoint dashboard/generated-client adapter
   only after the request contract freezes and only while a concurrency slot is
   free. W2 and W3 join on one canonical topology fixture and service response.
   Stop any lane at the first cross-owner interface change.
3. **Packet C, 55 minutes, consolidation:** after the W2/W3 join, one Luna-low
   worker performs the enumerated mechanical parity and documentation sync
   while the primary runs focused integration checks and prepares the single
   evidence matrix. Then run one closed-world Sol review, accept or reject its
   one finding set, allow at most one bounded rework, and run the final selected
   gates once. Freeze one candidate only after those gates pass.
4. **Installation gate:** perform one candidate build if executable inputs
   changed, one production install and one bounded W5 acceptance sequence. Do
   not rebuild or reinstall around live symptoms; return any deterministic
   defect to the cheapest source reproducer before considering another
   candidate cycle.

At two consecutive checkpoints without acceptance progress, consolidate the
remaining blocker before further implementation. Prior Plan 0124 and Plan 0160
effort remains historical evidence and is not repeated.

The dependency graph is:

```text
Plan 0161 preserving-repair source milestone
  -> W0 fixture fan-out + W1 capacity owner
  -> join on canonical slot invariant
  -> W2 service/viewer || W3 installer/doctor
  -> join on canonical topology and response
  -> W4 parity/docs || focused integration checks
  -> one closed-world review and at most one rework
  -> one final gate set
  -> one candidate build/install/acceptance cycle
```

## Maintenance and validation consolidation

- Establish one batch baseline and one changed-surface manifest before W1.
  Reuse it through W4; do not regenerate broad inventories at each packet.
- Query CodeGraph for owners and impact. Do not repeat source exploration in
  worker contexts or manually rebuild call graphs.
- Extend the smallest existing stable test seam. Prefer one parameterized
  topology matrix over separate tests for every historical route symptom.
- During implementation, run only the focused test that detects the edited
  invariant. Target five minutes or less per feedback loop. A wider failure is
  diagnosed once and does not trigger an unchanged full-suite retry.
- Run formatting, strict Clippy, contract parity, client checks, dashboard
  checks and docs build once at the completed source batch. Reuse every passed
  gate unless a later edit affects its dependency surface.
- Run `pnpm validation:select -- --base <batch-baseline>` once before the final
  gate set. Record its selection and any explicit risk-based additions in the
  single evidence table.
- Freeze schemas and public behavior before generated files and prose. Do not
  update documentation after every intermediate implementation commit.
- Preserve Plan 0124's accepted development evidence and tests. Re-run only
  cases whose dependencies changed; do not replay its elastic or soak campaign.
- Treat unrelated cleanup, historical record repair, test modernization and
  provider hardening as backlog unless they directly block W0 through W5.
- Perform one closed-world review with one rework allowance. Do not start a
  fresh discovery review, a second opinion or a broad 77-item audit.
- Build the optimized candidate once after source freeze. Run one install,
  doctor readback and acceptance sequence. Do not poll CI or repeat doctor
  merely to observe unchanged state.
- A live acceptance failure must name the violated invariant and return to a
  provider-free or isolated reproducer. It cannot begin another maintenance,
  build or installation loop by itself.

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
| W3 installer and doctor | `gpt-5.6-terra`, medium | After W1 freezes topology types, edit only the enumerated workstation installer, manifest and doctor adapter paths plus focused fixtures. The primary continues W2 service allocation in parallel. | 45 minutes, first cross-owner interface change, or first failing check outside the assigned adapters. |
| W2 public adapter, conditional | `gpt-5.6-luna`, medium | Only after the request contract freezes and CodeGraph proves a disjoint dashboard or generated-client seam, remove ordinary physical-route choice and add its focused test. No Rust capacity or installer edits. | 30 minutes, contract change, semantic ambiguity or unavailable concurrency slot. |
| W4 mechanical parity | `gpt-5.6-luna`, low | After schemas freeze, run deterministic generators and edit only enumerated generated-client, parity-registry and required documentation paths. No contract or behavior decisions. | 30 minutes, generator failure, schema mismatch or need for semantic prose. |
| Closed-world review | `gpt-5.6-sol`, medium, fresh context | Review one frozen source batch against route admission, slot-neutral multi-viewing, migration rollback and client abstraction. Return evidence-shaped findings or an explicit no-finding result; make no edits. | 25 minutes or one complete finding set; one primary disposition and at most one rework, with no broad discovery or second review pass. |

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
