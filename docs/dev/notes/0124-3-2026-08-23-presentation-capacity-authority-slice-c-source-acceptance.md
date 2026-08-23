# Plan 0124 Slice C Source Acceptance

Date: 2026-08-23

Status: SOURCE ACCEPTED

Plan: `docs/dev/plans/0124-2026-08-23-scalable-desktop-evidence-and-presentation-capacity-plan.md`

Source baseline: `b3af8bcdf5a631e080eaa4fe74a6b53050317006`

## Outcome

Slice C promotes `PresentationCapacityAuthority` from a test-only contract into
durable Service State. It owns arbitrary-size slot inventory, configured and
pressure-admitted limits, human and recovery reserves, priority ordering,
bounded queueing, FIFO and bounded aging, per-browser and per-slot exclusion,
scene generation, lifecycle fencing, quarantine obligations, and redacted
capacity projections.

Slot derivation requires exact agreement among ready route-pool, remote-route,
and display-allocation records. A configured warm minimum never fabricates
provider capacity. CDP-only logical browsers consume no slot until a caller
explicitly requests presentation.

Admission reads existing acquisition, route, viewer, controller, durable
handoff, display, and retained-browser authority without launching a browser or
opening a route. Human controllers block automated staging. Passive viewers
permit capture only when staging is unnecessary. Controller drift after
reservation is rechecked before the scene generation advances.

Service Status, CLI text, the generated client contract, and the dashboard
status strip expose slot counts, admitted and hard limits, protected reserves,
queued demand, oldest wait age, and redacted binding warnings. The dashboard
keeps logical browsers distinct from presentation slots.

## Validation

- Presentation capacity and integration tests: 14 passed, including 12 direct
  authority cases plus CDP-no-capacity and Service Status projection cases.
- Service model serial contract tests: 34 passed.
- CLI output tests: 36 passed.
- Service client generation, types, helpers, examples, and fixed-input harness:
  passed.
- Service API and MCP parity: passed.
- Dashboard view-stream, browser-row, table, inspector, and production build
  gates: passed.
- Docs production build and remote-view handoff docs: passed.
- Rust formatting, strict Clippy, and diff hygiene: passed.

The repository validator also selected the live CDP streaming gate and local
dashboard publication. Both were intentionally withheld because Slice C is
provider-free source acceptance and production publication is reserved for the
isolated development runtime established by Plan 0125.

## Scope Boundary

No browser, profile, display, Guacamole connection, RDP target, process,
dashboard service, installed generation, ingress, or production state was
changed. Slice D is the next authorized source slice and connects the frozen
Desktop Evidence Episode coordinator to capacity and scene authority through
injected adapters only.
