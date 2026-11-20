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
and p95 interval bounds. StableTime service UTC bounds at the measurement
instant are used conservatively; guest time is not extrapolated. Clock round
trips are included, so short intervals may be unresolved.

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
conservative millisecond intervals:

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
