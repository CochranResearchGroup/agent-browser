# P280: Dashboard static delivery

Status: CLOSED
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

## Qualified checkpoint

Source and isolated production-shaped candidate acceptance pass; see the owning [note](../notes/0280-2026-10-24-dashboard-static-delivery.md). Gzip reduces JS/CSS wire bytes by62.77%. All252 stream tests pass with2 existing ignored; format, strict Clippy, docs and guidance gates qualify this source. The sealed c2b6103dd candidate retains exact executable-input identity across evidence-only changes.

Production publication remains blocked by an already-active foreign forward-recovery operation. No competing install or takeover is attempted. Issue393 owns the independently reproduced2.4 to3.1s backend workspace-read delay; this packet does not claim that delay or Native first pixels fixed. The source issue391 stays open for public installed acceptance. The primary checkout and pinned candidate are retained; the owned temporary namespace is stopped.

PR392 is merged as ecd2950a3. Fresh canonical source/562-input equivalence passes. Source custody is integrated;391 remains open for installed acceptance, with no production effect performed by this lane.

## Installed closeout

Combined generation bf263f69342b is accepted. [Installed evidence](../notes/0282-2026-10-24-recovery-dashboard-manifest.md#combined-installed-acceptance-2026-10-24) separates passing public/workflow gates from advisory warnings, concurrent process changes and open393/399 work. Primary and sealed candidates are retained.
