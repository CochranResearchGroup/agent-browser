# Plan 0124 Slice F Source Acceptance

Date: 2026-08-23

Status: SOURCE ACCEPTED

Plan: `docs/dev/plans/0124-2026-08-23-scalable-desktop-evidence-and-presentation-capacity-plan.md`

Source baseline: `c07a1ee78e278f4682344b198e7743788b25bbf4`

## Outcome

Slice F publishes one redacted `desktopEvidencePolicy` through Service Status.
The same projection now reaches CLI JSON and text, HTTP, MCP, the generated
client, install doctor, and the dashboard. It exposes the evidence decision
contract without route IDs, display names, provider credentials, pixels, OCR
text, or other private scene data.

Agent and operator guidance now starts with the evidence need. DOM,
accessibility, viewport, canvas, and page pixels use CDP without presentation
capacity. Browser chrome, extension UI, password-manager and passkey prompts,
native dialogs, OS windows, stacking, and occlusion use a bounded desktop
evidence episode. Generic CDP failure remains diagnostic and cannot authorize
desktop fallback. Sensitive biometric, secure-desktop, PIN, master-password,
and consent surfaces return a durable human continuation.

The projection requires paired page-absence evidence and capture-ready proof
for browser-external classification. It also advertises configured production
input as `unavailable_pending_plan_0110`, preserving the independent input
acceptance boundary.

The dashboard now presents logical browser control health, presentation
capacity, and evidence transport as distinct status concepts. Existing
presentation-capacity fields continue to report arbitrary-N inventory,
pressure admission, reserves, queueing, and redacted binding warnings.

## Validation

- Desktop-evidence policy and coordinator tests: 15 passed.
- Service Status projection tests: 18 passed.
- CLI Service Status formatter tests: 2 passed.
- Install doctor focused tests: 18 passed.
- Generated service client contracts, types, exports, helpers, fixed-input
  harness, managed-profile flow, and examples: passed.
- Dashboard inspector contract and production build: passed.
- Documentation production build: passed.
- Service API and MCP parity plus remote-view handoff documentation: passed.
- Source-free workstation installer, host provisioning, fresh-VM harness,
  Guacamole assets, PostgreSQL durability, route-user sync, and workstation
  payload provenance fixtures: passed.
- Rust formatting, strict Clippy, and diff hygiene: passed.

## Scope Boundary

No browser, profile, desktop frame, provider input, display, Guacamole route,
RDP target, dashboard service, installed generation, ingress, or production
process changed. Slice G controlled installed acceptance remains separate and
must target the isolated development environment before any production effect.
The selector's live CDP-streaming check and local production-dashboard
publication were intentionally withheld for Slice G because source acceptance
does not authorize a browser effect or publication to production port 4848.
