# Plan 0124 Slice D Source Acceptance

Date: 2026-08-23

Status: SOURCE ACCEPTED

Plan: `docs/dev/plans/0124-2026-08-23-scalable-desktop-evidence-and-presentation-capacity-plan.md`

Source baseline: `f645a7488dbbfdb8f81a7eee6dcd5f4fc64d392b`

## Outcome

Slice D promotes `DesktopEvidenceCoordinator` from a test-only decision model
into a provider-free transaction authority with injected CDP, presentation
slot, scene staging, window semantics, external trigger, desktop frame, input,
verification, restoration, handoff, and cleanup adapters.

Page evidence remains CDP-only and reserves no presentation slot. Browser
external evidence requires paired CDP absence evidence, reserves capacity,
captures the prior scene, stages before the external-UI trigger, requires exact
capture-ready proof before and after capture, verifies the resulting scene,
restores only while the recorded scene authority still owns the transaction,
then releases capacity and completes cleanup.

The successful receipt binds admission, before-scene, staging, page absence,
trigger, capture proof, frame capture, optional authorized input, verification,
after-capture proof, after-scene, restoration, release, and cleanup identities.
Capture or authority drift returns a terminal receipt, cancels unsafe
restoration, and still binds release and cleanup.

Human controllers and passive viewers are checked before reservation or
staging. Human precedence returns a typed handoff without triggering external
UI. Sensitive surfaces do the same. Configured production input is explicitly
unavailable and cannot reach the injected input adapter until Plan 0110's
separate gates are accepted.

Existing desktop capture remains a narrow display-bound diagnostic mechanism.
Slice D does not connect the coordinator to a configured provider or product
request surface.

## Validation

- Focused desktop-evidence tests: 14 passed.
- Rust formatting: passed.
- Strict Clippy: passed.
- Diff hygiene: passed.
- Development runtime doctor: passed with one selected development generation,
  one runtime host, one dashboard backend, and one dashboard.
- Build Admission admitted the Rust checks with active capacity two and four
  Cargo jobs per invocation.

## Scope Boundary

No browser, profile, display, Guacamole connection, RDP target, dashboard
service, installed generation, ingress, production process, or provider input
was changed. The accepted development runtime was inspected read-only. Slice E
is the next source-only packet and joins elastic slot lifecycle and exact
garbage-collection obligations without installing the result.
