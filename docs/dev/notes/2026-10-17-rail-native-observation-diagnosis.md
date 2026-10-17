# Rail native observation diagnosis

Product lane: PL-BUGFIX
Disposition: active-input
Owning plan or work item: [issue 301](https://github.com/CochranResearchGroup/agent-browser/issues/301), UX301 / PR302
Related lanes: P237, P240

## Control and evidence boundary

The user invoked diagnosing-bugs after the combined development candidate failed.
Existing UX301 custody and source surfaces remain primary. Continue within the
remaining cumulative 60-minute active-work allowance from the assignment-history
packet; its completed build and qualification effort are not reset. No additional
production-shaped build, production installation, provider change, ingress change
or private account work is included. At most two source repair iterations may
follow the minimized red, with focused provider-free regression and quality gates.
Installed evidence must remain explicitly distinct from source qualification.

## Reduced live feedback loop

The private rail-minimal.py uses one ordinary synthetic native handoff, one
browser and one tab, with no peer selection, keyboard input, screenshots or
handoff-preparation probe. The installed development binary is unchanged `7b46`.
Three uncached summary reads return unknown browser and tab presence in
3473, 3467 and 3642 ms. Exact task close succeeds and fresh profile process census
is empty. The prior two-tab reproduction is preserved separately.

## Ranked falsifiable hypotheses

1. Native deadlines reject healthy provider reads. Holding request and provider
   fixed while changing only the deadline should change timeout to success.
2. Synchronous process identity observation overruns async collector budgets.
   Timings inside the relevant observation namespace should account for excess
   collector latency. A direct external helper is not sufficient to reject this.
3. CDP numeric formatting or PID mismatch rejects the exact browser. Raw CDP
   process data should expose the mismatch if present.
4. Assignment or lifecycle/viewing fencing rejects current native evidence.
   The typed adapter should return a conflict rather than a valid exact window.

## Probes

The same actual inventory request times out after 103 ms at a 100 ms limit, then
succeeds in 11 ms with a two-second limit. The second timing may benefit from provider
work initiated by the first request and does not establish a cold latency bound.
Raw CDP process data contains the exact integer browser PID, rejecting hypothesis 3
for this reproduction. The direct external process observer takes 3 ms; this does
not yet reject hypothesis 2 inside the runtime namespace. Prior exact native
assignment/window readback succeeds and matches the browser PID, contradicting
hypothesis 4 for that exact observation.

A provider-free regression now uses the actual HTTP adapter, with a 550 ms
inventory and 150 ms subsequent reads. It checks prompt status return, eventual
exact native window evidence, single-flight requests and read-only operation
sequence. The baseline fails at the actual observer entry: the initial call blocks for
104.9 ms. The repaired worker passes that regression and eight existing workspace
tests. Only one native read sequence occurs while the caller polls repeatedly.
The provider deadline is two seconds per request, with a four-second batch
admission budget; the final admitted fenced window read may finish afterward.
The status caller never waits for that I/O. Native reads retain the active-only,
unique-assignment and before/after lifecycle/viewing checks.

The worker is process-local and single-flight, keyed by provider/application and
exact desktop generations. Context changes invalidate prior evidence; old worker
completion cannot publish into a new revision. Cached results expire after thirty
seconds and keep the original sample timestamp. Completed failed refreshes replace
prior positive evidence with unknown, without changing lifecycle state.

A second real-browser regression fails when a 27-second-old sample is granted a
new thirty-second confidence window. The additive observationAgeMs field now
carries elapsed sample age; the hook subtracts that age from its monotonic
confidence window without trusting client wall time. Repeated same-token reads
cannot extend the window. The browser regression passes with client year2040,
stalled reads, held diagnostics and exact process cleanup. Final extended Rust,
client/schema/UI and quality checks remain pending.

The source checkpoint may precede final validation, explicitly preserving that
pending state. One ordinary CI-profile development candidate may be built for
synthetic replay if the remaining cumulative allowance permits; this is not a
second production-shaped qualification build or production promotion.
