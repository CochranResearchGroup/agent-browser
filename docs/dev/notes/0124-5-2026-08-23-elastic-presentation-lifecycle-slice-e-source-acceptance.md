# Plan 0124 Slice E Source Acceptance

Date: 2026-08-23

Status: SOURCE ACCEPTED

Plan: `docs/dev/plans/0124-2026-08-23-scalable-desktop-evidence-and-presentation-capacity-plan.md`

Source baseline: `be13af5caa3e72469b77f50acee212f3c35540bf`

## Outcome

Slice E promotes elastic presentation lifecycle into a provider-free authority
that mutates the durable `PresentationCapacityAuthority` instead of keeping a
second slot inventory. It owns exact elastic resource identities, lifecycle
generations, cooldown epochs, rollback quarantine, cleanup obligations, and
one-at-a-time scale decisions.

Scale-out admits one slot per call only while pressure capacity and the
configured hard maximum permit it. The provisioning adapter must return the
exact requested slot and lifecycle generation plus nonempty route, display,
and resource ownership identities. Partial effects, provider failure, or
identity mismatch quarantine the slot with a deterministic cleanup obligation.

Scale-in begins only from a known elastic warm-idle slot, waits through the
configured cooldown, preserves the warm minimum, and asks an explicit
reference adapter for browser, acquisition, episode, viewer, controller,
handoff, rollback, recovery, restoration, and cleanup blockers. Referenced
slots remain deferred. Ambiguous identity or partial garbage-collection
failure quarantines the slot and retains the cleanup obligation. Exact success
removes the resource ownership, lifecycle generation, cooldown record, and
capacity slot together.

The GC seam consumes exact owned-resource records rather than treating service
GC or runtime inventory as a proxy for lifecycle ownership. This keeps
discovery, reference qualification, and destructive reclamation as separate
proof boundaries.

## Validation

- Focused presentation-lifecycle tests: 7 passed.
- Repeated three-cycle scale-out, cooldown, exact reclaim, and warm-minimum
  convergence removed six owned resources with no retained elastic identity.
- Pressure rejection called no provider.
- Exact reference blocking called no garbage collector.
- Partial provisioning failure produced a quarantined cleanup obligation.
- Rust formatting, strict Clippy, and diff hygiene passed.

## Scope Boundary

No configured provider, garbage collector, browser, display, Guacamole route,
RDP target, dashboard, installed generation, ingress, or production process was
changed. Slice F is the next source-only packet and publishes the evidence and
capacity decision model consistently to agents and operators.
