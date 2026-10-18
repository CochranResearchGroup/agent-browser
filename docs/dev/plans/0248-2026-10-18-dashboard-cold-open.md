# P248: Selected desktop cold-open latency

Status: OPEN
Lane: P248
Product lane: PL-BUGFIX
Work item: CochranResearchGroup/agent-browser#312
Pull request: https://github.com/CochranResearchGroup/agent-browser/pull/313
Branch: fix/dashboard-cold-open-248
Baseline: 90770e535dc4ea13042d259d6ee74b36518efde2
Owner: ecochran76

## Current State

A real authenticated production dashboard opens selected synthetic Native pixels in 7089 ms. The trace contains 3153 ms between first inventory and focus, followed by 665 ms focus, 991 ms handoff and 1242 ms Native connection. Another open failed on an exclusive retirement_effect lock. Follow-up captured unknown-to-live across the three-second poll. The selection now requests authoritative exact handoff proof immediately, preserving unknown rail state, and removes the redundant pre-focus. A new repository/provider-boundary test reproduced foreground exclusion during read-only retirement observation. Provider reads now precede acquisition of the current-state effect guard; a controller introduced during the read must still veto claims and effects. Source tests and installed qualification remain in progress.

## Outcome and scope

Reduce waiting and failed opens for an existing selected browser. Finish targeted diagnosis, then repair confirmed cold presence delivery, duplicate focus, and retirement contention without weakening exact identity, current live proof, maximize, latest-selection ordering or cleanup fences. No viewer, authentication, TTL, quota or ownership redesign. Reuse the existing primary checkout and retain prior sealed candidate pins; foreign P240 and canonical checkouts remain untouched.

## Delivery and seams

Serialized critical path: measurement, one red/green slice at a time, review, isolated validation and integration. No delegated tracks. Primary writer owns browser workspace observation, NativeTaskViewport, relevant retirement adapter/store and their tests. P240 is completed; preserve its resource policy and historical evidence. UX301 is integrated; consume its canonical Native seam.

- Public read-only workspace projection: selected current unknown presence can request exact authoritative verification immediately while the rail stays unknown; catches cold poll waiting, not physical viewer rendering.
- Real mounted Native task viewport with controlled HTTP: one handoff selection retains maximize and latest-selection ordering; catches duplicate requests, not provider execution.
- Retirement guard/repository boundary: current activity still vetoes effects while unrelated foreground reads remain bounded; catches lock scope, not foreign OS activity.
- Real isolated installed dashboard with one synthetic task: monotonic navigation-to-connected nonuniform Native pixels, per-stage timing, same owned process and exact cleanup.

## Acceptance and definition of done

Record missing evidence before selecting the presence and lock repairs. Preserve red commands and green receipts. Run relevant browser, Rust and client tests plus Rust format and strict workspace Clippy for touched Rust, dashboard build and selected documentation checks. Review Standards and Spec separately against the pinned baseline. Qualify real Native pixels, exact selection and owned-process cleanup; report remaining latency honestly. Source integration uses an issue-linked fork PR. Installed effects remain source-bound and separately evidenced; no formal release or production claim from tests alone.

## Checkpoint: source repair

Baseline follow-up: first workspace response at 844 ms reports browser/tab unknown; at 3926 ms both are live, followed by focus at 3937 ms. Full unmodified open took 10084 ms. These samples are diagnostic, not a latency distribution. A separate 60-second lock sample saw only two 53/75 ms holds; it does not reproduce the earlier one-second hold.

Red: `pnpm test:dashboard-managed-workspace` rejects immediate authoritative selection for current unknown presence. `AGENT_BROWSER_TEST_CHROME=/opt/google/chrome/chrome pnpm test:native-task-selection` observes redundant view_focus before handoff. `AGENT_BROWSER_CARGO_CACHE=off scripts/ci/rust-tests.sh --focused automatic_retirement_publishes_custody_before_native_completion` fails with a foreground snapshot lock timeout inside provider observation.

Green so far: managed workspace adapter, real mounted unknown-task selection including maximize, latest selection and denied readiness without iframe; selected workspace context, shared read coordinator, handoff policy refresh, native rail, handoff docs, dashboard build and docs build. All 14 automatic-retirement tests, strict workspace Clippy and Rust format passed; broader affected checks and installed proof pending.

The lock repair moves initial idle/readback observations before global effect exclusion and rechecks current local authority before any claim or Native mutation. Native prepare/reconciliation and physical effects retain their existing exclusion and fences. It does not promise that every retirement phase is lock-free or that historical lock timeouts are fully eliminated.

## Review against 90770e535

Standards: primary review checks the complete patch against AGENTS.md, domain CONTEXT.md, testing discipline and existing ownership boundaries. Existing Native viewer and authority paths are reused. No new polling, cache confidence, viewer, authentication ceremony or cleanup policy is introduced. No blocking standards findings remain.

Spec: the current owned unknown selection requests the existing authoritative handoff operation without claiming rail liveness; absent, stale, inactive and mismatched owners remain blocked. Maximize stays on the queued handoff operation. The mounted HTTP fixture covers the held earlier selection and denied live verification. Retirement preflight reads occur outside exclusion and the new controller-race test proves revalidation precedes Native mutation. Existing fences and operation replay tests pass. Installed latency and physical Native behavior remain acceptance gates, not inferred from these source results.

## Installed checkpoint and changed diagnostic tactic

Outcome progress: CI-profile source `ccd296fd2`, binary `ae8805d80ebb2ccd2706d4b095c7c8d9a350b12b131d7457981a182e7659d74e`, passed development publication and install doctor with production preserved. Actual authenticated first unknown presence now begins the single handoff at 895 ms. Its real Native iframe connects with pixels. First/warm navigation-to-pixels are 7593/6630 ms, exceeding the provisional five-second diagnostic threshold. That threshold is not a user SLA. The source mechanisms are qualified; latency acceptance remains incomplete.

Do not replay the whole acceptance suite to learn the remaining stage. One temporary timing build (`a0e6e55bdef2`, development only) measured task preparation plus exact focus at 544 ms against a 4262 ms HTTP request. Native assignment/registration reads were 1-4 ms, windows 27-109 ms. Gateway backend round trips were 1279-1670 ms. Initial stderr capture missed daemon logs because the existing shared runtime discards stderr without its debug setting; a temporary development-only debug drop-in enabled one targeted API measurement. A temporary logging-format check failed, was corrected, and strict Clippy/format passed. All tagged source instrumentation and the drop-in are removed; restore the qualified development binary before further acceptance. These probe results are diagnostic and must not be promoted.

The named task HTTP port lookup calls full `discover_sessions`, including every managed extension manifest and foreign Chrome/CDP probes. P248 now repairs this routing cost using an exact named-owner metadata lookup with the same current runtime-process predicate. A provider-free test blocks an unrelated extension manifest and requires the selected route to stay available. This is a bounded change of tactic addressing the measured HTTP overhead; Native rendering and authorization checks are unchanged. Prior cumulative evidence and failed timing verdicts remain retained.


## Named-owner routing regression

The actual `session_port_for_name` seam failed with an unrelated extension manifest held open: the selected route did not return within one second. Two earlier fixture compile errors were corrected before this behavioral red; they are not regression evidence. The repair reads only the named owned session port and preserves the existing runtime-process predicate. Empty names, path traversal, zero/malformed ports and absent ownership return unavailable. Full inventory discovery remains available for inventory callers. The obsolete array-filter helper and its implementation-only test are removed.

Standards follow-up: the internal performance repair adds no user-facing command or contract. It retains routing, focus and Native readiness authority. Spec follow-up: the deterministic blocked-unrelated-manifest fixture covers the measured dependency directly; real installed timing remains pending. Temporary probe logging and its development drop-in are removed, and the previously qualified development binary was restored with production unchanged.

Green: the selected-owner regression completes in 0.01 s; all ten discovery tests and six dashboard request routing tests pass. Strict workspace Clippy and Rust format checks pass for the routing repair. The prior 107-test browser-session consumer run and 14 automatic-retirement tests remain qualified for unchanged source.


The clean routing candidate `46f1483b2`, CI binary `a6005459aeae47b2d4b56af5794e595807bac92e745c83b5e59d6911472fc93b`, passes development installation and install doctor with production unchanged. Actual fresh owned-task selection begins at 953 ms from current unknown presence; its handoff still takes 4352 ms and connected Native pixels arrive at 7213 ms. This misses the provisional five-second budget. Three compact workspace reads take 37-56 ms, with no diagnostic polling or CSP failure. The task is closed. The unrelated discovery dependency is fixed, but the live result falsifies the hypothesis that it dominates this sample's remaining latency. One bounded gateway phase probe now separates preparation from relay instead of repeating the full suite or weakening the diagnostic threshold.


## Gateway preparation diagnosis

The temporary gateway phase probe records: classification 97 ms, state loaded 165 ms, normalization 229 ms, named port ready 1910 ms, relay done 3331 ms. The real Native pixel sample is 6040 ms. This isolates 1681 ms in lane preparation and 1421 ms in relay; the prior task-local 544 ms measurement does not cover all relay work. The probe source is removed, the synthetic task is closed, and qualified `a6005459aeae` is restored.

The existing readiness gate forces profile reconfiguration whenever normalized commands carry profile fields, including an existing-tab handoff whose handler only retains and focuses an already-owned tab. P248 narrows this rule for that existing-tab operation only: reuse requires both the named lane and its HTTP listener ready; absent listeners and acquisition/navigation still recover using the original profile arguments. Signed operator proof, target binding, Native observation and focus remain downstream gates. A ready lane is transport availability, not proof of visibility or permission.

The ready-existing-tab regression is red before the change and green afterward. Three existing daemon-readiness/recovery tests pass, including stale-port rejection and mode scrubbing; strict workspace Clippy passes. Standards and Spec follow-up find no blocker: only the existing-tab retain/focus path reuses an already-ready transport; acquisition, navigation and recovery retain profile setup, and the task handler retains signed focus and Native readiness checks.


## Ready-lane installed result and bounded stop

Source `180ac29f6`, clean CI binary `2464b40d1564a251d7012f4477c28b34039256c3b09f2217e2e8d6b050f22ed1`, passes isolated publication and install doctor. The fresh synthetic selection again starts from current unknown presence without waiting for a poll, with one queued handoff and a real connected Native iframe. This sample takes 9875 ms: first workspace starts at 2752 ms, handoff at 3009 ms, handoff duration 4885 ms, first Native WebSocket frame at 8488 ms. Three workspace reads take 52-96 ms. No CSP failure or diagnostic polling occurs. This is a failed latency verdict; do not claim the ready-lane repair improved installed speed or that the five-second diagnostic budget passed. The task is closed and no production promotion is selected.

Fresh host readback reports load averages 20.22/17.50/17.29 with substantial available memory; foreign workloads are preserved. This is context, not causal proof of the latency variance. After three recent installed/probe verdicts still miss the threshold, stop repeating full opens. Source behavior and functional Native delivery are qualified separately; remaining relay latency and repeatable end-to-end improvement stay open under #312. Source integration may retain these bounded dependency/request-count improvements without closing latency acceptance.

The expanded acquisition regression initially fails before the readiness gate because its legacy fixture omits required originKind. Add only the missing synthetic agent origin metadata, preserving runtime enforcement, then re-run the acquisition regression. Retain that failed check alongside the final verdicts.

Final source checks: corrected authenticated cold acquisition regression passes, six relay-routing tests pass, formatting and strict workspace Clippy pass after the fixture-only correction. Development launch/URL/close/residue smoke passes all three repetitions. Documentation links and 11 policy-wiring tests pass. All task fixtures and their transports are closed; the final OS/runtime readback is retained privately. The installed binary remains bound to `180ac29f6`; subsequent changes are documentation and one cfg(test) originKind fixture only, without a production-artifact equivalence claim. Draft review is appropriate; latency acceptance and #312 remain OPEN.
