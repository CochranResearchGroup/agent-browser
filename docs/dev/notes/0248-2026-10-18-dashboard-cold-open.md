# Dashboard cold-open diagnosis and repair

Product lane: PL-BUGFIX
Disposition: accepted-evidence
Owning plan or work item: [P248](../plans/0248-2026-10-18-dashboard-cold-open.md), [#312](https://github.com/CochranResearchGroup/agent-browser/issues/312)
Related lanes: UX301, P240

## Observed workflow

An actual public dashboard navigation to a synthetic owned tab took 7089 ms to connected nonuniform Native pixels. Another sample took 10084 ms. The first current workspace response reported unknown presence; selection waited roughly three seconds for the next poll before signed focus and handoff. These are diagnostic samples, not a latency distribution.

## Repairs and source evidence

Current exact browser/session/tab bindings may request authoritative handoff verification while descriptive rail presence stays unknown. Stale, absent, inactive and mismatched bindings remain blocked. Selection uses one display-ordered handoff with maximize, retaining generation fences and Native readiness before embedding. The mounted HTTP fixture covers held earlier selection, latest selection and denied verification without an iframe.

Initial retirement provider observations occur before global state exclusion, followed by current-authority revalidation before any claim or Native mutation. A controlled provider read previously caused a foreground snapshot lock timeout; the regression now admits the foreground read and rejects retirement after a newly published controller. Existing CAS, replay and physical effect fences remain.

Named HTTP port lookup reads only the selected owner's metadata with the existing runtime-process predicate. The actual route regression blocks an unrelated extension manifest: red before repair, green in 0.01 s afterward. Missing owners, traversal and invalid ports remain unavailable.

Gateway phase timing then isolated 1681 ms in owner-lane preparation and 1421 ms in relay. A ready existing-tab retain/focus request unnecessarily spawned profile reconfiguration. Reuse now requires both the named lane and HTTP listener ready; acquisition/navigation and missing-listener recovery retain profile setup. Signed focus and Native visibility remain downstream checks.

## Installed checkpoints and limitations

Initial three-fix CI candidate reached real Native pixels in 7593/6630 ms, first/warm. Named-port candidate reached pixels in 7213 ms; the temporary phase probe reached pixels in 6040 ms. All miss a provisional five-second diagnostic threshold, which is not a user SLA. These failures remain retained and are not promoted into an accepted latency claim.

The named-port sample begins handoff at 953 ms from current unknown presence, without the three-second poll wait. Three compact workspace reads take 37-56 ms. One Native iframe connects with pixels, no CSP failure and no legacy diagnostic polling. The sample does not establish the port scan as the dominant live cost.

Temporary probe source and development debug drop-in are removed. Qualified source/binary identities, sanitized phase timings and private acceptance artifacts are retained outside Git. Production is unchanged by P248. The clean ready-lane CI candidate is source `180ac29f6`, binary `2464b40d1564a251d7012f4477c28b34039256c3b09f2217e2e8d6b050f22ed1`. Publication and install doctor pass. Its fresh selected task reaches connected pixels in 9875 ms; first workspace starts at 2752 ms, handoff begins at 3009 ms and takes 4885 ms. Workspace reads remain 52-96 ms. This is another failed timing verdict, so repeatable latency improvement remains open. The synthetic task closes and its transport process is gone. No competing test/build ran during this measurement; a later host census shows load averages 20.22/17.50/17.29, which does not establish a cause.

Stop repeated full-open probes after these three failed installed/probe verdicts. Retain source and functional delivery qualification separately from the unresolved latency gate. The next diagnostic question is where time accrues outside task-local preparation within relay and whether the ready-lane gate is exercised as expected under representative workload. Do not promote CI-profile bytes to production or claim a five-second result.

Source validation includes the deterministic blocked-discovery regression, ten discovery and six dashboard request tests, ready-lane and stale-port recovery tests, six relay-routing tests, 107 browser-session consumer tests (one additional provider-dependent fixture ignored), 14 retirement tests, mounted dashboard selection, dashboard/docs builds, formatting and strict workspace Clippy. A legacy acquisition fixture initially omitted originKind; it is corrected without changing runtime enforcement. Its corrected result is recorded in the plan.


## Accepted direct-owner development checkpoint

The earlier unchanged result was not acceptance. A built-in monotonic sample takes 5285 ms HTTP with only 876 ms execution and 316 ms queue accounting. A private boundary trace isolates approximately 2125 ms creating a legacy lane and 1177 ms persisting its scheduler job, versus 608 ms actual handoff and less than 1 ms host lock wait. An early wall-clock discontinuity makes summing that trace invalid; end-to-end HTTP and browser metrics use monotonic clocks.

Source `3fb7afa7a`, clean development CI binary `154434c8234cf139cfa6d4d622e4d51286be8993aa1ffcab9281a355df63f27b`, sends an authenticated existing-tab selection to the shared browser owner directly. It retains signed focus, per-display ordering and revalidation, current target ownership and Native readiness. Acquisition/navigation/dry-run keep their original path; a missing runtime still recovers. The earlier ready-HTTP-lane special case is removed because this cold task has no HTTP lane.

Actual fresh public dashboard navigation reaches connected nonuniform Native pixels in 4762 ms. Its handoff takes 1303 ms and starts from current unknown presence; a new page/context repeat on the retained task reaches pixels in 3722 ms. A follow-up direct handoff takes 1068 ms. Both full opens pass the provisional five-second diagnostic budget, without turning that budget into a user SLA or a latency distribution. The original comparison samples are retained above.

Installed proof shows the same browser PID and process birth, Native-ready response, no new logical HTTP stream file and no launch job for the selected request. The first auxiliary readback looked in the historical JSON mirror and failed before navigation; it was corrected to the current SQLite owner document without replaying browser creation. No browser/task was substituted and no viewer document was replaced.

The real authenticated connection regression fails before the fix for unnecessary job/lane creation and passes after, while invalid signed focus remains refused before either effect. All 13 daemon tests, selected routing exclusions, acquisition recovery, daemon readiness, client helpers, docs build, formatting and strict Clippy pass. Temporary instrumentation is removed. Development install doctor passes; production-shaped build, exact-byte development qualification and production activation remain separate gates.
