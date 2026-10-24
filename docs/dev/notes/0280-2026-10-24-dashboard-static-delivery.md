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

## Sealed candidate and isolated acceptance

Clean source c2b6103dd8b8654fd8412f76816ee1b9a69657a8 produced production-shaped candidate candidate-26183b85f1b6a078-8532c10293f13e64. Its binary SHA256 is 8532c10293f13e646222819ab8e4b48f16c387a1d39b18cc41d7f15ddac933c8. The full release build completed in 16m34s, and sealed payload validation passes. All 562 executable inputs match the retained checkout. Subsequent evidence and guidance edits do not change these executable inputs.

Isolated namespace s391 runs this exact binary. Fourteen exported JavaScript/CSS assets total 2,529,563 identity bytes and 941,845 wire bytes, a 62.77% reduction. Every negotiated representation decodes to the original, Content-Length matches actual wire bytes, Vary separates encodings, q=0 returns identity and hashed assets carry immutable caching. HTML and auth API boundary checks pass. Authenticated dashboard and Service APIs pass, as do three disposable browser open/read/close cycles. Development doctor has only its unconfigured local hostname-ingress check false; direct loopback identity and application checks pass. This is not a public development ingress claim.

Five fresh contexts with one owned headless fixture yield shell median797.70ms and populated rail median3616.93ms, range3206.37 to3790.83ms. Workspace Service reads take median2703.77ms. No page errors or status serialization failures occur. The fixture closes, all16 retained probe-browser generations are absent and the three exact namespace units are stopped/disabled with MainPID0. Fresh process readback finds no owned namespace processes; inaccessible foreign/kernel process metadata is not claimed as scanned. The candidate, stopped namespace files and private evidence are retained.

## Remaining gates

Issue393 owns the separate producer-side inventory delay. With zero browsers and tabs, explicit fresh workspace reads take2598 to2960ms. Direct backend reads take2378 to3079ms, almost entirely before headers for approximately14KB bodies. The HTTP workspace projection directly calls the read-only owner snapshot/enrichment path. The precise wait inside that producer is not established. These diagnostic timings do not create a new SLA.

An existing production install, started outside this lane, remains in forward-recovery state and previously returned502 during a public sign-in probe. No production install, recovery, rollback, restart or ingress effect is performed by this lane. Public post-fix latency and installed acceptance remain open under391 until that operation is resolved. Failed preflights and the interrupted public probe remain private evidence; they are not counted as successful acceptance.

## Canonical integration

PR392 merges as ecd2950a3bb0b524b1e9060bb84d9ec7615d882f. Fresh canonical readback proves the sealed source commit is an ancestor and all562 executable inputs still match. Source custody is integrated; the primary checkout, exact candidate and stopped development namespace remain retained for installed acceptance. The entry guidance is9960 bytes and the exact sealed executable passes the public MCP discovery/refusal guidance gate.

Final readback still reports the pre-existing production forward-completion gate. The source fix is merged, but this lane performs zero production installs or recovery attempts, and391 remains open. Issue393 is ready for the distinct producer diagnosis; it is not a hard source dependency for static delivery.
