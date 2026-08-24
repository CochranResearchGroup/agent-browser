# P124 Slice G | Configured Capacity Adapter Source Acceptance

Date: 2026-08-23

Classification: SOURCE ACCEPTANCE

## Outcome

The Desktop Evidence Episode capacity boundary now admits real provider and
persistence failure. A reservation that is queued, rejected, or not durably
committed returns `admission_unavailable` before scene snapshot, staging,
paired evidence, or external trigger work. A release that cannot be durably
committed returns a terminal `release_failed` receipt and still runs episode
cleanup.

Scene-admission and capture-readiness providers are now fallible as well. A
missing scene observation stops before capacity reservation. Losing the
capture-ready probe after reservation produces a terminal adapter-failure
receipt, releases the exact slot, and runs cleanup. Provider unavailability can
therefore no longer be interpreted as capture readiness.

`ConfiguredPresentationSlotAdapter` binds one episode request to the durable
Service State presentation authority. It commits the exact granted slot and
browser identity, releases that same slot through the authority, and refuses
duplicate or cross-browser release. An unavailable one-shot observation rolls
back the advisory queue mutation so it cannot accumulate a request for which
no configured resume caller exists.

## Validation

- `native::desktop_evidence::tests`: 19 passed
- `native::desktop_evidence_configured::tests`: 2 passed
- Rust formatting passed

## Remaining Boundary

This packet does not create a product request or claim configured desktop
capture. The configured scene-semantic, frame, paired CDP, handoff, and cleanup
adapters still need to compose behind one deep observation-only episode
caller. Capture-ready proof must remain stronger than
`browser_window_visible`, and configured production input remains blocked by
Plan 0110.
