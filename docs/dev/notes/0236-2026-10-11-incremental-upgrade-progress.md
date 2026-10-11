# Incremental workstation install observations

Product lane: PL-BUGFIX
Disposition: active-input
Owning plan or work item: P236 / #181
Related lanes: none
Baseline: da19973fa5b60a58cfe26a3143df3c6d35d87ff1

The current-source public isolated installer succeeded after 50.8 seconds but emitted only its terminal JSON report. The incremental-output assertion failed. No unit shim ran; the elapsed time was actual payload staging, not an injected delay. Exact fixture cleanup found no hosts or residue. A parser regression subsequently failed on the unknown progress option after successful compilation. The earlier compilation OOM is retained as a resource failure, not a behavioral red.

The opt-in progress-json option requires JSON apply. It emits versioned NDJSON observations on stderr while stdout retains one terminal report. One current snapshot bounds observer storage. Durable snapshots are published only after successful transaction persistence. Heartbeats describe observer liveness; they cannot advance recorded work, transaction revisions or validation deadlines. Private runtime paths and executable identities are omitted.

Observer completion has a 250-millisecond delivery budget. A blocked sink returns a telemetry backpressure receipt without holding installer completion. The worker is joined only when already finished; CLI process exit retires a blocked worker, including existing failure paths. Apply timing excludes compilation. The final emission receipt qualifies delivery, not installed browser or provider acceptance.

All 192 workstation Rust tests pass, including parser restrictions, fixed-clock truthfulness, a failed durable-write boundary and an actually blocked output worker. The original public loop now produces 54 observations across a 52.5-second apply, with the first observation after 93 milliseconds and one terminal stdout JSON report. Exact cleanup again finds zero residue. The unsealed debug source binary SHA256 is a8335f1efc5126a1f64677c57b91727b27e5840edb444289197e1d357fa3238c.

The tracked public fixture requires an explicitly selected candidate and isolated HOME, XDG and workstation roots. It preserves timestamped wire evidence, checks success and injected failure, and performs exact executable/nonce/home cleanup before deleting its own temporary root. Systemctl and sudo sentinels fail if the isolated path unexpectedly attempts unit effects.

CLI help, README, the repository agent skill, installation documentation and source comments describe the option. Provider-free workstation provisioning, VM harness, Guacamole assets, Postgres durability, route-user synchronization and handoff documentation checks pass. The docs build and format check pass. The complete installer fixture also passes and retires two exactly owned hosts with zero residue. These gates qualify the initial workstation repair. Strict workspace Clippy remains pending; the 2 GiB attempt ended with a scoped OOM, not a lint result. The adapter correction below invalidates whole-slice readiness until its affected gates run.

This is isolated source qualification. No production or development runtime was published, provider restored, private authentication performed, foreign enrollment source changed, or reboot performed. Wider issue 181 remains open for production-shaped upgrade acceptance and separately qualified build/apply receipts. CI remains disabled.

## Reviewed candidate adapter correction

The original issue reports candidate install. The initial workstation repair did not expose its option there: a public zero-effect invocation returned Unknown candidate install argument before reading any artifact. The exact parser, mode helper, types and source regression were extracted into a small standard-library-only Rust fixture. It reproduced that rejection and passes after repair, including dry-run exclusion. This is actual parser evidence, not a full CLI or sealed-install gate. The full CLI red-test waiter never compiled and was cancelled; it is not a behavioral result.

Candidate apply now passes its explicit progress choice into the reviewed adapter. Global JSON mode remains required. The adapter starts the same observer before sealed document review and binary hashing, then transfers it to the existing workstation apply function. Artifact verification, coordination custody and install authority are unchanged. Whole CLI compilation, affected Rust tests and the corrected candidate public wire remain pending under host resource admission. No merge readiness is asserted.
