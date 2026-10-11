# Incremental workstation install observations

Product lane: PL-BUGFIX
Disposition: accepted-evidence
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

## Final source qualification

Frozen executable source is 8cd1921891f6ed113d9a6acde2e97c4d6a74c10d. Its immutable debug CLI SHA256 is 544f69bc5f19316dde5e94e74a6ac5addffa10a32e671e6509b411dae802fa38. The user waived the previous host reserve; compilation passed with an 8 GiB reserve, 4 GiB hard cap, no swap and one job. This did not change shared admission defaults or take custody of foreign work.

| Requirement | Evidence | Qualification |
| --- | --- | --- |
| Reviewed candidate early output | 68 observations; first after 75 ms; apply 66,296 ms | Actual public adapter with synthetic sealed fixture inputs |
| Workstation early output | 57 observations; first after 100 ms; apply 55,775 ms | Actual public isolated apply |
| Failure termination | Both injected units-staged failures returned exit 1 within the fixture bound | Existing terminal error semantics preserved |
| Exact cleanup | Both independently observed cleanup receipts found zero residue | No fixture-owned host retirement required |
| Candidate mode and parser | All 11 candidate Rust tests passed at frozen source | Full CLI test binary, including the new regression |
| Workspace quality | Strict workspace Clippy passed with warnings denied | Final frozen source, 4 GiB hard cap |
| Installer compatibility | Earlier complete no-progress fixture passed; two exact hosts retired, zero residue | Reused because the adapter correction adds an optional observer and delegates the unchanged disabled-mode installation body; final workstation tests cover the helper interactions |
| Documentation and format | Final feature docs build and format check passed | Retained unchanged-input gates; closeout prose receives links and planning checks |
| Installed qualification | Unverified | No publication; synthetic seals are not production-shaped build evidence |

All 192 final workstation Rust tests passed in 656.52 seconds, including durable-write failure, heartbeat truthfulness, blocked-output completion and installer recovery. All 39 help-output tests passed. These final-source gates replace the pending adapter qualification; integration remains a separate boundary.

Wire logs and exact fixture roots intentionally remain in the operator-local temporary evidence directory. The tracked commands `pnpm test:candidate-progress-fixture` and `pnpm test:workstation-progress-fixture`, with `AGENT_BROWSER_FIXTURE_BIN` set to an immutable binary built from the source above, reproduce the qualification. This note preserves the durable identity, measured outcomes and limits without copying runtime state into the repository. Earlier scoped OOMs, cancelled waiters and older-binary results remain preserved as distinct evidence.

[PR256](https://github.com/CochranResearchGroup/agent-browser/pull/256) integrated this bounded repair at aefeb390be9e30224f618d15901e54d47165c14f. The frozen tested executable source is an ancestor of that merge; subsequent changes are qualification and integration prose. Candidate evidence remains retained outside the worktree, separately from installed runtime identity. The lane catalog now records integrated custody; governed checkout removal follows the final clean metadata integration.
