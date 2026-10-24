# Dashboard static delivery, P280 / issue391

Product lane: PL-BUGFIX
Disposition: accepted-evidence
Owning plan or work item: P280 / CochranResearchGroup/agent-browser#391
Related lanes: none

## Reproduced workflow

Fresh authenticated browser contexts reproduce a slow populated rail on installed generation de9fa872987c. Initial public five-context median is 4.90s; an instrumented three-context public median is 5.41s, versus loopback 0.97s. Startup assets total 2,482,025 encoded body bytes. A sampled JavaScript response lacks compression and cache headers. Public asset completion takes 2.0 to 4.4s, compared with 106 to 119ms locally.

Browser-only local asset fulfillment retains public document, authentication and APIs and lowers median rail population to 1.82s. One 5.50s sample still needs a second inventory poll. The comparison proves a material asset-delivery contribution, not the sole cause or a product SLA. Native first pixels and that remaining inventory tail are separate. No foreign desktop was selected or controlled. Private raw probes remain outside Git.

## Source repair

Both native dashboard HTTP listeners use one static delivery helper. Successful embedded Next static text assets negotiate gzip, with explicit q=0 overriding wildcard acceptance. Content-Length describes the actual wire bytes, and Vary separates identity from gzip. A fixed embedded-file registry bounds lazy generation-local compressed representations. Small or incompressible files use identity. Only successful non-HTML Next assets with a 16 to 64 digit hexadecimal filename component gain one-year immutable caching. HTML, missing assets, auth and dynamic API routes do not gain compression or immutable caching.

## Validation and custody

The original transfer regression records the absent Content-Encoding. Initial four focused transfer tests pass, including both actual listener sockets. Final source adds an unconditional synthetic gzip round trip so a Rust-only placeholder-dashboard build still tests encoding; built-asset checks exercise actual exported bytes when present. The complete stream compartment passes 252 tests with two existing ignored tests, including negotiation, both listeners, HTML/API/error boundaries, refresh/auth behavior and protected operation cases. Format and strict workspace Clippy pass. Candidate/runtime qualification remains the next checkpoint. The docs build, handoff documentation and policy wiring pass.

Source integration, isolated candidate qualification and production publication remain distinct checkpoints. CI stays disabled. Foreign dirty auth work, PR362 and retained enrollment/profile candidates remain outside this repair. No ingress configuration, Native ownership, account operation or formal release is included.
