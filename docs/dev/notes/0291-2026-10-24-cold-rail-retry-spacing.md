# P291 cold rail retry spacing

Product lane: PL-BUGFIX. Work item: [430](https://github.com/CochranResearchGroup/agent-browser/issues/430). Baseline: `6e3d49577e84e68679a62103b8fd426881d916a9`.

## Reproduction

The asset finding in422 remains separate:76 HTML/asset requests succeeded, with public maximum1.061seconds and loopback maximum0.057seconds. Five public Chromium150 visits had no page errors but rail readiness ranged3.041 to11.999seconds. Three loopback visits ranged2.165 to9.207seconds. Slow visits received unknown-only inventory until a later response contained live tabs.

Remove browser and public ingress from the reproducer: two read-only workspace API runs after35seconds without probe traffic each returned two unknown-only responses before15live tabs appeared. Their first live samples retained original Native observation ages1449ms and1389ms. Browser keys and all browser fields except presence were unchanged in the compared cold/ready snapshots. Ordinary warm and revalidated runs each had15 successful live-inventory reads. This qualifies an expired-observation boundary, not an authentication or asset diagnosis.

Source tracing shows Native evidence expires after30seconds. A workspace read starts background refresh, while the caller receives unknown presence until that refresh completes. The original startup policy used three250ms retries, then normal three-second polling. A reduced actual-scheduler regression with readiness at1449ms first read live evidence at3750ms. The same regression failed before repair and passes afterward at1750ms, without exceeding three follow-up requests.

## Repair and limits

Spread the same three retries with delays250,500 and1000ms after each completed read. Known evidence, empty inventory or failure ends startup catch-up. Ordinary three-second cadence, visibility suspension, read serialization, observation age and ownership remain unchanged.

This trades sampling positions within a fixed budget: evidence ready just after the second old retry can wait an additional250ms, and evidence arriving after the new retry window still waits for normal polling. It does not eliminate provider cold work or guarantee a two-second live page load.

A real Chromium150 replay used the captured unknown/live responses with readiness at1449ms. Installed assets rendered the rail at4.929seconds from navigation; rebuilt source assets rendered at3.365seconds. Both runs had no page errors and no service mutation attempts. Different shell and processing timings make this diagnostic evidence, not a causal production speedup claim. Fresh independent PID/start readback found all16 observed replay process generations absent. Three preceding browser probe cohorts likewise left zero of30 observed generations present.

## Qualification

The focused scheduler, selected-workspace context, workspace navigator and handoff documentation checks pass. Dashboard and docs builds, Rust format, strict workspace Clippy and policy wiring pass. All40 focused CLI output tests and changed-file documentation links pass. The initial cached test attempt stalled at sccache rustc-version inspection; only its exact owned child was stopped, then the cache-disabled run passed. Unrelated host-browser discovery smoke requires a pinned candidate executable and was not qualified in this source-only packet; its missing-environment refusal is retained rather than called a product failure.

The changed-surface selector recommends a broad fallback because a test script and CLI source changed. Workstation provisioning, PostgreSQL, VM and installer suites are unrelated to the help-string and scheduler changes; no deployment or shared-skill publication is performed. Raw responses, process identities, browser probes and logs remain in user-scoped state outside Git. Production remains installed9cbf948ef4af. The source fix needs a later exact-candidate installed cold-distribution acceptance before430 closes. No Native provider, profile, authentication, ingress or peer browser was changed.

Source checkpoint `33b31d49dc5a4562de8526bacf4b3494711366b5` is integrated through [PR431](https://github.com/CochranResearchGroup/agent-browser/pull/431), canonical merge `20c5f3e1bae860dc50f9a748bebccf15a5971ebb`. Installed acceptance remains open.

## First installed comparison: latency not accepted

Sealed source `feee93820644fff6a77bab2839341d025818b120`, binary `35e0e3058ecc13d66f3266c458773ee8a203885217ee410fe86f725e122a4d2b`, passed563 input hashes and exact installer/supervisor fixtures. All82 observed fixture generations were absent. The production-shaped build took1810.5seconds. Performance measurements waited for compilation and fixtures to finish.

Preserve transaction `upgrade-576cdadf-a888-4fb2-bff0-466f1130b163` completed ready. All17 pre-install browser roots and137 observed Native provider/persistent service generations survived; configuration and dashboard authentication bytes are unchanged. The broader140-unit check also included three MCP client units: two had roughly19500 restarts before installation and continued churning. Preserve that check's failure separately; no MCP client or provider repair was attempted. Doctor from the sealed path reported executable drift, while the actual installed-path doctor exits0 with three existing advisories; the supervisor process image matches its installed manifest. Fifteen public startup files match sealed input hashes.

Pinned Chrome156.0.8078.47 measured three genuinely cold visits before and after, each starting with zero live tabs and16 unknown tabs. Cold visits before were3357.3,3778.9,2542.6ms; after were3075.5,3458.5,3257.4ms. Warm visits before were914.0,1003.4,785.4,1094.4,1223.5ms; after were856.5,1059.7,1052.5,979.4,1471.1ms. Every visit rendered correctly with no page errors or service mutation attempts and no observed QA residue. This is basically unchanged performance, not installed acceptance of430.

## Standards review

No blocking documented-standard defect was accepted. Preserve Native/profile source ownership and the fixed request budget. The review uses the retained worktree's actual files; graph results from the configured canonical checkout are insufficient for this worktree's newer caller.

## Spec review

Blocking caller defect: `useSelectedWorkspaceContext` invokes `load(nextPollInterval.current ===250)`, so the new500/1000ms intervals do not explicitly revalidate the shared one-second cache. A cached unknown response can consume a follow-up without an HTTP request. This violates the documented cache-revalidated startup behavior and explains the observed request gap. The policy-only test exercises a shallower seam and did not catch it.

The existing mounted startup fixture reproduces the failure; the strengthened test also fails on the missing header. Change the caller to revalidate every startup interval below the ordinary3000ms cadence. Mounted ready, unknown, failed and empty cases then pass, with three requests in787ms for readiness and four in1796ms for the exhausted case; all18 observed green-fixture generations are absent. A corrective exact-candidate installation and repeated cold/warm comparison remain required. No claim that the initial scheduler result proves an end-to-end gain is retained.
