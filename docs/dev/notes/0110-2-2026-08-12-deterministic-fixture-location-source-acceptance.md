# P110 PoC 2 Deterministic Fixture Location Source Acceptance

Date: 2026-08-12

Status: SOURCE ACCEPTED | NO LIVE, AMBIENT OCR, OR INPUT PROOF

Accepted commit: `4281196a`

## Outcome

`desktop_locate` atomically captures and locates one repository-owned
synthetic verification control. It accepts only the named
`p110-control-v1` profile, binds every observation to the exact context,
receipt, frame hash, dimensions, scale, geometry epoch, coordinate space, and
capture provider, and returns stable detector receipts and ordered candidates.

The profile combines geometry, deterministic integer RGBA template matching,
and pinned normalized OCR-token evidence. `not_found` and `ambiguous` are
successful observations without a selected target. Optional visualizations
are bounded and response-only. No frame, overlay, or raw OCR text is persisted.

## Audit And Remediation

One independent audit found missing capture-provider agreement, a server and
client request-allowlist mismatch, nonexistent challenge-like docs examples,
insufficiently pinned candidate receipts, and missing OCR failure evidence.
One bounded remediation at `4281196a` resolved all five. No second broad audit
was opened.

## Validation

The primary and independent source-only passes proved:

- guarded formatting and strict Clippy;
- 10 deterministic locator tests and 12 ingress/CLI tests;
- full light and dark fixture coverage at 1.00, 1.25, and 1.50 scale;
- pinned observation and visualization hashes, ambiguity, decoy, and edge
  behavior;
- 26 PoC 1 capture, 46 remote-view handoff, and 30 page-screenshot tests, with
  two ignored browser E2E tests not run;
- full service client, API/MCP parity, no-launch contracts, action inventory,
  WSL Cargo safety, docs build, and patch checks.

No live or external-provider action occurred. The next action is Plan 0110-3,
not pointer or keyboard implementation without a frozen authority contract.
