# Contacts task recovery

Status: OPEN
Product lane: PL-BUGFIX
Owner: Contacts recovery session
Work item: #629 observation refusal; restart lifecycle scope to be reconciled.
Branch: fix/contacts-task-recovery

## Current State

Reused clean existing Contacts repair checkout at current origin/main2817e3bae; no new checkout. Operator requests clean stuck-task restart and continuation after explicit browser termination. Existing scoped browser-close refuses when peer tasks exist. Restarted acquisition opens and evaluates, then navigation fails browser_task_changed_during_observation. Existing issue629 describes the same observation error; root causality remains unproven until deterministic measurement. Graphiti read timed out; governance packet unavailable, canonical policies read directly.

## Scope and acceptance

Reproduce observation refusal at real SQLite seam; distinguish harmless peer changes from changed task/process. Repair only the demonstrated cause and preserve changed physical target rejection. Investigate an explicit scoped restart path that honors operator browser-wide authority without deleting profile data. Local validation only; no hosted CI. Installed acceptance and production deployment remain separate. Private tenant source facts remain outside this repository.

## Work ownership

Serialized sole writer of browser_session_store observation regression and bounded correction. Reuse prior closed Contacts lane; preserve other checkouts and retained artifacts. No subagents or new runtime created.


## Reproduction and findings

Current production reproduces `mode_operation_tab_missing` when preparing a restart from an existing task with no selected tab. The same browser has a live verified process and no competing mode-operation owner. Creating one recovery tab with the existing task-scoped `browser_tab new` and stable `tabKey` makes the same preparation succeed. That preparation was then cancelled without relaunch. This establishes the missing-tab prerequisite, not an obsolete-browser or pending-operation cause.

One supported same-mode CDP relaunch from an existing tab completed with one graceful close and one replacement launch, retaining logical tasks. The first continuation required intervention; a later continuation and read succeeded. Read-only page metadata subsequently confirms the retained contact profile is loaded, not blank and not a login form. This does not establish account freshness, a latency improvement, or completed enrichment. The original uncertain navigation was not replayed.

Issue629's observation failure is independently reproduced by a real two-connection SQLite regression changing only `active_session_ids` during inspection. Command: `scripts/ci/cargo-safe.sh test -p agent-browser --bin agent-browser browser_mode_inspection_allows_peer_session_membership_change -- --nocapture`. Baseline failed with `browser_task_changed_during_observation` in0.14s; native build took492.224s including dependency compilation. This proves that peer membership can cause the reported code; the exact changed field in the historical live navigation was not captured and remains unproven.

Ranked hypotheses were membership comparison, changed task/process identity, and a broader store race. The narrow fix ignores only peer membership in the browser observation comparison, retaining browser/profile/PID/CDP-endpoint/desktop identity and original task-incarnation checks. Focused mode tests pass8/8; removed-task and replaced-process rejection remain covered. Close-policy tests pass7/7. Changed error messages now name supported recovery; README, help, skill/reference and documentation site describe same-mode relaunch and task recovery tabs.

## Local validation and limitations

Base:2817e3bae. No commit, push, source integration or candidate installation in this slice. Local docs links and11policy checks pass; strict workspace Clippy and final formatting check pass. Hosted CI excluded under operator policy. Build retries retain the initial incorrect library-target selection, a thread-exhaustion compiler failure, and stopped cache-blocked attempts. Final compilation uses the existing wrapper with2jobs,2CPUs, a10GiB claim,12GiB cap and unchanged16GiB workstation reserve; shared compiler-cache service remains untouched. Builds use a minimal environment; raw error logs are private. Production still runs the pre-fix observation guard. No temporary source logging or disposable browsers were created; the operator's persistent browser and recovered task are retained deliberately.


Final source qualification: focused8 GREEN (206.513s build/check wall), close-policy7 GREEN (2.443s), strict workspace Clippy GREEN (233.480s), formatter check GREEN (228.959s including admission wait). No source debug instrumentation was introduced. Local patch is ready for review; integration, sealed production build, installation and changed-binary live acceptance remain unexecuted. Memory disposition queued separately; queue acceptance is not persistence proof.


## Review and delivery 2026-11-16

Operator explicitly requested review, merge and deployment. Fixed review baseline2817e3bae17512a8b90e80d1c7391ce744dae224, current origin/main at review. Standards axis: no blocking findings; the two small repeated SQLite setups match nearby tests and do not justify a new fixture abstraction in this bounded repair. Spec axis: no blocking findings against629 and operator recovery scope. Physical task/process/endpoint/desktop comparisons remain; only peer membership is excluded. Recovery remains explicit and original uncertain actions are not replayed. No claim that the historical live trigger was captured or that a complete latency repair is proven. Deployment will use a sealed production-shaped candidate and preserving workstation installation, then installed acquisition acceptance.
