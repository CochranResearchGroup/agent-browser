# P248: Selected desktop cold-open latency

State: OPEN
Product lane: PL-BUGFIX
Work item: CochranResearchGroup/agent-browser#312
Branch: fix/dashboard-cold-open-248
Baseline: 90770e535dc4ea13042d259d6ee74b36518efde2
Owner: primary session in agent-browser-dashboard

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
