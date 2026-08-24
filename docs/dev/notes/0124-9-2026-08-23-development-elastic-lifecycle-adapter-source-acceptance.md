# Plan 0124 Slice G Elastic Lifecycle Adapter Source Acceptance

Date: 2026-08-23

Status: SOURCE AND DEVELOPMENT INSTALL ACCEPTED | LIVE CYCLE PENDING

Plan: `docs/dev/plans/0124-2026-08-23-scalable-desktop-evidence-and-presentation-capacity-plan.md`

Authority: DEVELOPMENT RUNTIME EFFECTS | PRODUCTION READ-ONLY

Installed generation: `0.28.0-eeedd32d8e06`

## Outcome

The isolated development provider now has explicit one-route scale-out and
scale-in commands. Scale-out admits only one absent descriptor-owned route and
uses current memory, swap, load, CPU, and file-handle evidence to choose the
admitted maximum. Scale-in selects only the highest elastic ready route after
cooldown and checks exact Service browser, session, viewer, controller,
acquisition, handoff, restoration, and cleanup references before reclamation.

Reclamation closes the exact development viewer session and profile, then
uses a new privileged-helper contract to terminate only an exactly validated
development RDP route user. The helper operation is idempotent when the user
has no login session. Broad process cleanup, shared XRDP restart, production
route mutation, and authority-file deletion are not part of this path.

Pressure, cooldown, and exact active references defer without lifecycle
effects. Ambiguous references and partial provision or reclaim effects return
a failed quarantined receipt with a deterministic cleanup obligation. Provider
authority is reconciled from the current observation so partially created
resources remain visible rather than becoming hidden residue.

## Source Validation

- development provider fixture passed, including four-to-five provisioning,
  five-to-four reclaim, pressure defer, cooldown defer, reference defer,
  ambiguity quarantine, exact nested handoff and viewer matching, and partial
  effect quarantine;
- development runtime fixture passed and includes both explicit scale commands;
- RDP route helper contract guard passed;
- clean privilege-installer fixture passed;
- Rust helper compatibility tests passed, 3 of 3;
- Rust formatting and strict Clippy passed.

## Remaining Boundary

The installed root helper is still the previous contract and cannot perform
the exact route-session termination. Install the reviewed helper through the
intentional interactive privilege boundary. Live acceptance must then perform
three measured four-to-six-to-four cycles, retain production and
authenticated-browser identity, and collect fresh process, memory, swap,
display, Guacamole, lease, handoff, and cleanup readbacks after every cycle.

## Installation Guard Repair

The first installation attempt correctly stopped on its production
non-interference guard, but investigation found that the snapshot treated a
short-lived production CLI client as a durable runtime process. The rollback
also restored the selected development generation and units without restoring
the development lane manifest, temporarily leaving that lane mixed and failed.

The guard now ignores only explicitly young process observations while still
requiring every durable production generation process, production unit,
browser identity, session identity, selected generation, dashboard manifest,
and handoff file to remain stable. Rollback now restores the prior lane
manifest atomically. Regression fixtures cover both cases. The repaired retry
installed `0.28.0-eeedd32d8e06`; the required development doctor passed with
all three units active, ports 4948, 4949, and 4951 bound, four warm displays,
six exact connection IDs, current development skill, and production unchanged.
