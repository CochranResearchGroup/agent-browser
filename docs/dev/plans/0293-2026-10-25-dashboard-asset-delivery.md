# P293: Diagnose public dashboard asset delivery

Status: CLOSED
Product lane: PL-BUGFIX
Owner: ecochran76
Work item: https://github.com/CochranResearchGroup/agent-browser/issues/422
Primary: agent-browser-dashboard, fix/dashboard-assets-422
Baseline: 5a6e9cd9d5c0c605fe319ad22e56afb708838945

## Bounded outcome

Compare public HTTPS and production loopback HTML plus every directly referenced static chunk and font, using five serial paired rounds. Pin serving PID/start/executable and asset hashes; record DNS, connect, TLS, first-byte and body timings. Classify transport timeout, HTTP failure, redirect, wrong content and generation drift separately. This is a read-only diagnostic, not installation or application-account acceptance. Preserve original P289 failures; do not infer a cause from later success.

Reuse the retained primary serially; preserve five foreign checkouts and pinned candidates. No overlap with active P283 mode/build work: this packet owns only P293 planning and evidence prose, with raw timings and a repeatable diagnostic under user state. No service restart, ingress/auth changes, candidate build or runtime publication. If this finite loop cannot reproduce the symptom, retain422 open with precise evidence and a repeat command rather than inventing a repair.

## Outcome

The five-round170-request comparison completed without a timeout, status error, redirect or content mismatch. Public median87.88ms/max1031.53ms; loopback median7.45ms/max123.59ms. Serving generation and binary identity remained stable. The [evidence note](../notes/0293-2026-10-25-dashboard-asset-delivery.md) preserves scope, original failures and the repeatable private diagnostic. This bounded diagnostic packet is closed;422 remains open because the actual intermittent symptom was not reproduced. No speculative source repair or runtime effect. Documentation links, patch hygiene and exact planning audit govern integration.
