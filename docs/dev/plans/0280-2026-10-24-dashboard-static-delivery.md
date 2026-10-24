# P280: Dashboard static delivery

Status: IN_PROGRESS
Product lane: PL-BUGFIX
Work item: CochranResearchGroup/agent-browser#391
Branch: fix/dashboard-static-delivery
Accountable owner: ecochran76
Baseline: 611f9beef111ab3eeb27a6886516f5742f317e88

## Outcome

Reduce cold browser-rail startup effort through negotiated gzip for embedded static text and immutable caching only for successful hashed Next static assets. Share delivery across both dashboard listeners. Keep dynamic responses and HTML outside compression and immutable caching.

## Evidence and bounds

Installed diagnosis reproduces public median5.41s versus loopback0.97s;2.48MB of startup assets lacks compression. Browser-only local asset fulfillment lowers median to1.82s, with one unexplained second-poll tail. This is a diagnostic comparison, not an SLA or Native first-pixel qualification. Private probes and raw timing artifacts remain user-scoped.

This lane owns native HTTP static delivery, focused transfer tests and matching docs. Reuses the retained dashboard primary checkout after current-main reconciliation. Foreign dirty auth source, PR362, enrollment candidate and profile-proof checkout remain preserved. Native provider and ingress configuration are untouched. P279 and its deployed candidate remain pinned.

Validation: decoding equality and Content-Length; q=0 and wildcard negotiation; successful hashed-cache restriction; HTML/API/error boundaries; both listeners; safe Rust format, strict Clippy and focused native tests. Build one development candidate and qualify HTTP transfer plus cold rail startup before canonical integration and any governed deployment. Preserve all existing browser process/profile/task identities.
