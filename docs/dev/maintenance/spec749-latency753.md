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

The small diagnostic uses the existing smoke context and stdio client, launches
one disposable headless browser and evaluates the same synthetic page value
through the CLI and exact-target CDP. Alternating pairs avoid always favoring one
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
