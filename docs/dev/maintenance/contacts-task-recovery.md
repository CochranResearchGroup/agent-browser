# Contacts task recovery

Status: CLOSED
Product lane: PL-BUGFIX
Owner: Contacts recovery session
Work item: #629 observation refusal; restart lifecycle scope to be reconciled.
Branch: fix/contacts-task-recovery

## Current State

PR718 is merged as bd89d29d66e89c4fd0b1b2d3517119d3f1d739d5 and deployed. Sealed binary3e1f3f85d4dfb1550f820fc928a55d0d11e0e989a97aeb440f5c7f65eec89185 matches the running production host. Exact-binary installer fixture, preserving installation and installed doctor pass. Six recorded original browser processes survive unchanged. Fresh installed Contacts LinkedIn acquisition completes in61.448s with eight successful calls; profile URL, current role, employment history and education are observed. Network remains incomplete. No failing-navigation blocker recurred in this run; broader reliability and controlled latency acceptance remain separate.

## Scope and acceptance

Reproduce observation refusal at real SQLite seam; distinguish harmless peer changes from changed task/process. Repair only the demonstrated cause and preserve changed physical target rejection. Investigate an explicit scoped restart path that honors operator browser-wide authority without deleting profile data. Local validation only; no hosted CI. Installed acceptance and production deployment remain separate. Private tenant source facts remain outside this repository.

## Work ownership

Serialized sole writer of browser_session_store observation regression and bounded correction. Reuse prior closed Contacts lane; preserve other checkouts and retained artifacts. No subagents or new runtime created.


## Reproduction and findings

Current production reproduces `mode_operation_tab_missing` when preparing a restart from an existing task with no selected tab. The same browser has a live verified process and no competing mode-operation owner. Creating one recovery tab with the existing task-scoped `browser_tab new` and stable `tabKey` makes the same preparation succeed. That preparation was then cancelled without relaunch. This establishes the missing-tab prerequisite, not an obsolete-browser or pending-operation cause.

One supported same-mode CDP relaunch from an existing tab completed with one graceful close and one replacement launch, retaining logical tasks. The first continuation required intervention; a later continuation and read succeeded. Read-only page metadata subsequently confirms the retained contact profile is loaded, not blank and not a login form. This does not establish account freshness, a latency improvement, or completed enrichment. The original uncertain navigation was not replayed.

Issue629's observation failure is independently reproduced by a real two-connection SQLite regression changing only `active_session_ids` during inspection. Command: `scripts/ci/cargo-safe.sh test -p agent-browser --bin agent-browser browser_mode_inspection_allows_peer_session_membership_change -- --nocapture`. Baseline failed with `browser_task_changed_during_observation` in0.14s; native build took492.224s including dependency compilation. This proves that peer membership can cause the reported code; the exact changed field in the historical live navigation was not captured and remains unproven.

Ranked hypotheses were membership comparison, changed task/process identity, and a broader store race. The narrow fix ignores only peer membership in the browser observation comparison, retaining browser/profile/PID/CDP-endpoint/desktop identity and original task-incarnation checks. Focused mode tests pass8/8; removed-task and replaced-process rejection remain covered. Close-policy tests pass7/7. Changed error messages now name supported recovery; README, help, skill/reference and documentation site describe same-mode relaunch and task recovery tabs.

## Pre-delivery source qualification

Base:2817e3bae. No commit, push, source integration or candidate installation in this slice. Local docs links and11policy checks pass; strict workspace Clippy and final formatting check pass. Hosted CI excluded under operator policy. Build retries retain the initial incorrect library-target selection, a thread-exhaustion compiler failure, and stopped cache-blocked attempts. Final compilation uses the existing wrapper with2jobs,2CPUs, a10GiB claim,12GiB cap and unchanged16GiB workstation reserve; shared compiler-cache service remains untouched. Builds use a minimal environment; raw error logs are private. Production still runs the pre-fix observation guard. No temporary source logging or disposable browsers were created; the operator's persistent browser and recovered task are retained deliberately.


Final source qualification: focused8 GREEN (206.513s build/check wall), close-policy7 GREEN (2.443s), strict workspace Clippy GREEN (233.480s), formatter check GREEN (228.959s including admission wait). No source debug instrumentation was introduced. Local patch is ready for review; integration, sealed production build, installation and changed-binary live acceptance remain unexecuted. Memory disposition queued separately; queue acceptance is not persistence proof.


## Review and delivery 2026-11-16

Operator explicitly requested review, merge and deployment. Fixed review baseline2817e3bae17512a8b90e80d1c7391ce744dae224, current origin/main at review. Standards axis: no blocking findings; the two small repeated SQLite setups match nearby tests and do not justify a new fixture abstraction in this bounded repair. Spec axis: no blocking findings against629 and operator recovery scope. Physical task/process/endpoint/desktop comparisons remain; only peer membership is excluded. Recovery remains explicit and original uncertain actions are not replayed. No claim that the historical live trigger was captured or that a complete latency repair is proven. Deployment will use a sealed production-shaped candidate and preserving workstation installation, then installed acquisition acceptance.


## Installed acceptance 2026-11-16

Standards and Spec reviews have zero accepted blocking findings. PR718 squash integration retains the exact validated Rust bytes from fb08d285f. Production-shaped release/fat-LTO build completed in2171.982s, sealed as candidate-f353361cc2fff3b8-3e1f3f85d4dfb155. Exact-binary workstation installer fixture passes in33.090s. Supported install transaction upgrade-fbb8afc4-b5e2-4578-ae61-bbaa8c629252 reports ready; generation and running host SHA match the sealed artifact. Installed doctor exits0; no independent public-viewer acceptance is claimed. Fresh OS start-time readback preserves all6 original recorded browser processes. Persistent profile data was not reset.

Fresh installed acquisition:61.448s overall,8returned browser calls, observed profile URL/current role/history/education, incomplete network. This is one successful acquisition, not complete network coverage, a live UI-button/graph-application result, or proof of a causal latency improvement. The previous94.669s saved call total is an uncontrolled historical sample. Source facts and raw results remain exclusively in tenant storage; project receipts retain metadata only. Test/browser processes from the installer fixture remain fixture-owned and cleaned by that passing fixture; existing authenticated browsers are intentionally retained.
