# Command latency attribution and warm reuse

Product lane: PL-PLATFORM
Status: OPEN
Work item: #753 under spec749
Branch: platform/spec749-latency
Base: b7bbc5692

## Source outcome

Warm authenticated compatible runtime reuse no longer waits for another lane's
startup-admission lock. Executable, version and authentication checks remain
fresh. Only demand for a new host takes the startup lock, then rechecks for an
owner another caller may have published. Existing startup publication waiting,
legacy shutdown grace and stale-runtime handoff behavior remain intact.

The focused regression uses the real connection boundary and retained owner
metadata. It requires a warm read to return while another startup lock remains
held, preserving the holder's token. Its one-second receive bound detects
coordination blocking; it is not an elapsed-time performance claim.

## Measurement

The initial diagnostic used the existing smoke context and stdio client,
launched one disposable headless browser and evaluated the same synthetic page
value through the CLI and exact-target CDP. Alternating pairs avoid favoring one
order. It reports cold launch separately, binary digest, sample count, median
and p95 service-bracket bounds. StableTime service UTC bounds at the measurement
instant are used conservatively; guest time is not extrapolated. Clock round
trips are included, so short intervals may be unresolved. A positive lower
service-bracket bound is not a lower bound on the browser operation. Without
delivery accounting the operation itself is bounded only from zero to the
service-bracket upper bound.

Pinned inputs are AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD,
AGENT_BROWSER_SMOKE_BROWSER_EXECUTABLE and
AGENT_BROWSER_LATENCY_STABLE_TIME_CLIENT. Optional sample count is
AGENT_BROWSER_LATENCY_SAMPLES, from3 to20. The existing StableTime executable is
used by default; AGENT_BROWSER_LATENCY_STABLE_TIME_BINARY can select it explicitly.
Run the diagnostic with Node. Its successful completion means measurement
finished, not that every ticket acceptance row passed.

The measured baseline uses binary72d2b4d4c2985435a5c5137f8f016bfdb8d649bc4d735b742075106d4ea06af0,
whose installed source qualification is recorded by the LinkedIn command-state
latency acceptance. Five same-page pairs in the final baseline gave these
conservative millisecond service-observation brackets, including delivery:

| Workload | Median interval | p95 interval |
| --- | --- | --- |
| Cold launch, n1 | 12831.9 to13324.4 | 12831.9 to13324.4 |
| Direct exact-target CDP, n5 | 0 to342.5 | 0 to361.6 |
| Agent Browser CLI, n5 | 232.6 to897.0 | 309.2 to1015.5 |

These bounds overlap. They do not qualify a precise overhead ratio or an
improvement. An earlier same-binary run had a cold interval4221.7 to4680.8ms and
warm CLI median0 to457.7ms, demonstrating run variability rather than a second
source improvement. The fresh synthetic state is not the historical8MB retained
state and this CLI path is not the workflow MCP path.

The installed public coordination probe completed only after the unrelated
startup lock was released. All synthetic runs ended with a fresh OS census of
zero remaining owned processes. Initial attempts used an incorrectly short
idle timeout and failed with socket absence; the diagnostic now disables idle
shutdown and retains those failures as diagnostic evidence.

Existing daemon timing output is exposed separately as unqualified guest-clock
diagnostics. It does not supply reliable full phase attribution. Routing,
individual state reads, readiness, CDP versus other action time and persistence
remain unmeasured separately.

The comparison now targets the actual addressed workflow MCP path. It reuses
the existing public workflow smoke bootstrap: create a named Chrome profile in
the disposable home, open it through browser_open in existing/automation mode,
then evaluate through browser_command with the returned exact task ID and fresh
request IDs. No private process is adopted and no owner metadata is fabricated.
The same task briefing target is independently matched to the direct CDP page,
and the actual browser executable, selected profile and process start identity
are verified. All reads return the same fixed synthetic value with matching
evaluation parameters.

Persistent addressed MCP is the primary warm measurement. A fresh CLI MCP
facade performs that same addressed evaluation as a separate startup diagnostic;
it cannot substitute a shorter unscoped CLI path. Cold managed opening is
reported separately. The earlier CLI-only baseline numbers above remain
historical diagnostic evidence and do not claim workflow MCP latency. This
updated path awaits the coordinator's exact-source candidate and is deliberately
not rerun against an unchanged installed binary merely to collect more samples.

## Validation and remaining gate

Installed baseline and startup-lock RED: diagnostic completed with
warmReuseCompletedBeforeStartupLockRelease=false. Node syntax and patch hygiene
checks pass. Source regression compilation was queued through cargo-safe with
one job,6GiB claim,8GiB cap and unchanged16GiB reserve, but memory-pressure
admission never dispatched the compiler. Both queued invocations were cancelled
by their exact session; no source test result is claimed.

Coordinator must run connection tests, required Rust format/strict Clippy and an
exact-source candidate comparison after integration. A compiled candidate must
prove warmReuseCompletedBeforeStartupLockRelease=true. Full reliable phase
attribution and measured candidate improvement remain open. This ticket is not
ready to close from this source packet alone. No installed runtime was changed.

## Selected-phase attribution

The coordinator-approved continuation adds opt-in sampling of one named phase
per request, selected by AGENT_BROWSER_LATENCY_PHASE. A disabled phase performs
no time-client requests. Selected phases use two observations from the supported
installed StableTime MCP client, retaining its private HTTPS/credential handling.
This is not an application-specific host-counter bridge.

The capture returns a measured or unavailable diagnostic independently from
browser success/failure. Readiness and startup admission have connection-owned
capture sites. Routing, state reads, CDP, persistence and queue call sites use the
same capture at their existing owner boundaries, with response wiring owned by
the coordinator. A local capture can cross an await without global/thread-local
state, and finishing it returns a value for the addressed response.

StableTime contract2.0 and source policy document the service measurement
counter/frequency, epoch and conditional rate envelope. Two qualified service
observations from one epoch/frequency bound their service-to-service elapsed
span using reciprocal fast/slow frequency extremes and outward tick rounding.
UTC interval coherence and the declared finite horizon are checked. No guest
clock or source precision cutoff is substituted. Transport and client startup
remain inside the bracket, so actual phase elapsed has only a zero-to-upper
bound. This is useful localization with explicit measurement overhead, not an
invented exact per-phase duration.

The documented supported client has no local elapsed-clock adapter. Source
inspection found only time/health/deadline operations. Consequently the capture
does not extrapolate service UTC using a WSL timer or start a private QPC reader.

Client execution now has a five-second ordinary OS availability deadline and a
128KiB stdout cap enforced during nonblocking pipe reads. Timeout, overflow and
I/O failure kill/reap only the exact helper spawned for that observation and
return an unavailable measurement; browser work is not vetoed. The bounded pipe
transport is supported on Unix; other platforms explicitly report unsupported
sampling transport rather than risking an unbounded wait. This does not describe
an auxiliary-client failure as a StableTime service outage. Focused process tests
cover a hung helper, oversized output, exact child reaping and successful input/
output exchange using the existing connection test harness.

## Exact-source addressed candidate measurement

The ordinary debug candidate built from30bd33677 has SHA256463a3221cf58e2d658949c39c4fe3752715ec515c53f9d7b84d95127d7a36501. The corrected public benchmark completed three alternating pairs on one exact managed synthetic target, separately measuring fresh CLI facades. Service-observation median/p95 upper bounds in milliseconds were: direct CDP459.45/462.98, persistent addressed MCP701.58/714.21 and fresh CLI MCP facade829.97/840.22. Every operation elapsed interval starts at zero because clock delivery is not independently bounded. These intervals overlap and do not establish a precise overhead ratio. The cold-open service-observation bracket was separately49141.12to50125.71ms; the actual operation remains bounded only by0to50125.71ms. This is not an optimized installed-runtime claim.

The warm evaluate read completed before the unrelated startup lock was released. That path uses existing task transport and does not call startup admission, so this probe cannot establish the startup-reuse change. A separate cdp-selected run completed with qualified optional phase samples and verified that an identical tracked request returned replayed=true/dispatchAttempted=false. CDP samples bracket supported service measurement counters under the service-declared conditional rate policy and include timing delivery. No guest-clock extrapolation occurs. Both runs ended with zero owned processes. Original failed benchmark supplied an unsupported params.timeoutMs and is retained as diagnostic evidence, not passing measurement. The corrected public arguments use script only and retain equivalent renderer budgets. Raw receipts remain outside Git at /tmp/spec749-latency-candidate-corrected.json and /tmp/spec749-latency-cdp-phase.json. Other phase source routes are wired; separate phase qualification and cumulative Rust gates remain.


## Controlled affected-path comparison

The common source is `afb1b86920b4029feb34466cceb640be6aa13084`. The former-order diagnostic changes only pre-lock warm reuse, patch SHA256 `899a1ff7ef20b3bf0dddf1b83d917a4206a96097d5c160d56f432213dd7e51b3`. Its ordinary debug binary SHA256 is `e8a644eb6007fa1c6b2e6fe7c80a5ec7d650d35518f38b6d6a4795835f973742`; the normal binary is `0f752de6ac3f9ef0f8620d0773842df7e94b77c789f20ede266575374f3851de`. Both builds froze source and dashboard assets; neither binary was installed. Identity and raw synthetic receipts remain outside Git under `/home/ecochran76/.local/state/agent-browser/validation/spec749/latency-controlled-afb1/summary.json` and the associated candidate directories.

The existing benchmark now pairs addressed `browser_continue` with exact-target direct `Runtime.evaluate('location.href')`, alternating three pairs and verifying returned output and process identity after the trailing service observation on both paths. The original steady evaluate pairs remain separate. A corrected coordination probe snapshots completion before releasing its held startup lock. Former ordering completed continuation only after lock release; current ordering completed while it remained held. This is causal evidence for removing unrelated startup admission from a healthy continuation, not an elapsed-time guarantee.

Former/current continuation median service-bracket upper bounds were492.42/231.15ms and p95 upper bounds494.47/242.29ms. Direct URL-read median upper bounds were398.35/138.67ms. Actual-operation intervals all start at zero; service delivery, uncertainty and host-load differences preclude a precise speedup ratio. Cold opening is reported separately. Both runs passed original-request no-dispatch replay and exact retained task/browser/tab/target identity. Owned residue lists were empty; fresh OS readback found both measured Chrome PIDs absent and no benchmark-owned user-data process.

Five separate selected-phase runs on the earlier pinned `beaec2731` candidate qualified routing, state read, CDP, persistence and cold startup-lock samples. Their aggregate is `/home/ecochran76/.local/state/agent-browser/validation/spec749/latency-capacity-phases/aggregate.json`. They are phase localization with sampling overhead, not a controlled before/after result or evidence for later code.

## Startup ownership repair, ticket 768

The shared TDD RED run on source `1046932d0` executed the existing connection regression `aged_startup_marker_cannot_reclaim_a_live_owner`. The old implementation replaced a marker held by a live process because its fixture mtime was old. The selected test failed. Its raw synthetic receipt remains outside Git at `/home/ecochran76/.local/state/agent-browser/validation/spec749/tdd-red-1046932d0/03.log`.

Current writers use a permanent `runtime-host.startup.v2.lock` inode with the standard library exclusive file lock. The guard retains its non-inherited file descriptor until startup leaves the admission scope. It never unlinks the guard, including after failure or owner exit. Metadata age is not ownership evidence. Lock contention uses an ordinary bounded OS wait; no StableTime client or guest-time ownership expiry is introduced. See the standard library [File::try_lock](https://doc.rust-lang.org/std/fs/struct.File.html#method.try_lock) and [File::unlock](https://doc.rust-lang.org/std/fs/struct.File.html#method.unlock) contracts.

A legacy `runtime-host.startup.lock` is inspected only when present. Its old timestamp is format only. Positive kernel absence of its recorded PID permits current startup while retaining the original bytes. A currently live PID or unqualified marker/observation defers cold startup with an explicit owner diagnostic. A live PID is not asserted to prove the exact historical process identity. Fresh namespaces require no speculative cohort census, and verified compatible warm reuse precedes both guard admission and legacy inspection. Uncontrolled mixed-generation cold writers remain unsupported: older binaries ignore the kernel lock and can apply their own unsafe age-based policy.

The four new diagnostics distinguish actual kernel guard I/O, current lock contention, a currently live legacy PID, and unqualified legacy evidence:

- `runtime_host_startup_guard_unavailable`
- `runtime_host_startup_guard_busy`
- `runtime_host_legacy_startup_owner_live`
- `runtime_host_legacy_startup_owner_unqualified`

Existing installation diagnosis and the selected-runtime preserve plan provide recourse. Start with `agent-browser --json install doctor` and `agent-browser install workstation --dry-run --runtime-replacement-policy preserve --json`; inspect and continue the original installation transaction under its existing authority. These diagnostics do not authorize deleting a marker, terminating a foreign process, replacing a browser, or replaying an uncertain operation. Independent compatible warm work remains available.

The accepted improvement budget is functional: healthy exact warm continuation must complete while unrelated current-protocol startup admission remains held. The repair adds no default time-client dependency. The controlled `afb1b8692` pair above establishes that functional improvement, while its overlapping operation intervals establish no precise speedup ratio or numeric SLA.

The existing warm-reuse unit now holds a real kernel lock. Focused ownership checks exercise contention through release on the same persistent inode despite past/future fixture mtimes, positive dead-owner admission with unchanged legacy bytes, and release of v2 admission after an unqualified legacy inspection. The existing benchmark uses an owned `/usr/bin/flock` process running the current Node executable with a fixed readiness/release handshake. It snapshots completion before release and verifies that the guard inode remains. Its isolated exclusion/release helper check passed; this is not full browser acceptance. The earlier legacy-marker receipts remain historical evidence for their exact source and script. A current-protocol candidate requires its own pinned browser run. The coordinator owns the combined Rust GREEN run and final runtime acceptance.
