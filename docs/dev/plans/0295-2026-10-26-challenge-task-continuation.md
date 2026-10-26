# P295: Continue browser tasks through challenge handling

Status: PLANNED
Lane: P295
Product lane: PL-CHALLENGE
Related lanes: PL-PLATFORM, PL-RECIPES
Owner: ecochran76
Work item: https://github.com/CochranResearchGroup/agent-browser/issues/453
Branch: challenge/stage1-continuation-plan
Target: main
Baseline: c5063a3f84712b53b00448e182c1ced4b1db32e3
Consolidation: required
Custody: planning-only remote branch and draft PR; no implementation checkout

## Current State

The operator-approved [Stage 1 specification](https://github.com/CochranResearchGroup/agent-browser/issues/447#issuecomment-6407474751) requires task briefings, original-operation recovery, challenge classification, qualified passive/checkbox automation and native human assistance with verified return. Implementation has not started. This document is the bounded implementation handoff, not delivered behavior or runtime authority.

Current source already has durable browser/task identity, human takeover/return, exact target binding, challenge lifecycle/model contracts, two checkbox detector profiles and extensive synthetic fixtures. The service challenge-task start path still requires a fixture scenario; its provider-free completion does not establish a live detector, current Native click or website admission. No accepted current Turnstile solve is inferred from historical work. See [CAPTCHA recovery evidence](https://github.com/CochranResearchGroup/agent-browser/issues/442) and [teaching/continuation evidence](https://github.com/CochranResearchGroup/agent-browser/issues/444).

The primary checkout has foreign uncommitted Auth work. This planning branch is constructed from the fetched baseline without switching its branch, touching its index or admitting another worktree. Five documentation/governance files are the only planned changes in this PR. Source implementation requires fresh checkout/overlap admission when that task actually starts.

## Bounded outcome

An agent inspecting a retained task understands what was completed and what remains uncertain, recognizes a supported challenge, uses one bounded eligible passive/checkbox path or the same native human handoff, independently checks protected-site admission, and resumes the original task without repeating an uncertain website action. Human return, agent replacement and one isolated service restart exercise that same workflow and preserve peer work.

Expected improvement: fewer reconstruction turns and bespoke recovery commands when a browser task hits a challenge or needs human help. Compare the existing public workflow against the candidate using the same frozen scenario; record agent operations needed, missing context and failure/result transitions. Record a baseline before expanding the acceptance matrix. Do not invent target latency or claim an improvement from test count alone.

## Consolidated batch

1. Read-first compact task briefing and result projection, joining current exact task/profile/browser/target, goal attribution, last verified outcome, artifact references, pending assistance, control state and original uncertain operation IDs. Label stale, supplied or unavailable fields; omit credential/private page material. Inspecting must not launch, navigate, focus, acquire control, acknowledge human use or dispatch another effect.
2. Current-generation challenge observation adapter that distinguishes passive verification, registered Turnstile/hCaptcha checkbox, image puzzle, ordinary authentication need, access denial, loading and unknown. Return evidence age and classification limits. Reuse existing detector rules where applicable and existing Native capture/semantics; DOM evidence may supplement CDP mode. No-CDP operation must not silently attach a debugger. Ambiguous/unsupported evidence becomes intervention, not a guessed click.
3. Compose the observed need with existing durable challenge control and exactly scoped Native input. Passive waiting has a deadline and observed completion. An eligible registered checkbox gets at most the existing one policy-authorized attempt, with current target/control evidence before effect, durable operation identity and uncertainty retained after dispatch. Adapt the live path rather than send fake fixture IDs or promote fixture outcomes to real evidence.
4. Continue through existing native assistance and explicit return: retain the same opaque handoff/task identity, freshly observe the returned browser, separately verify challenge clearance and intended protected-site access, then resume the original playbook. Failed/expired/moved/unknown cases return a concrete next action and preserve uncertain submissions. A changed generation requires an explicit fresh join; never substitute a peer or claim unsaved state survived.

One critical-path owner serializes the common projection, lifecycle and adapter contracts. These are one browser workflow, not a generic workflow engine, second event store, new authority or refactor program.

## Source seams and ownership

| Existing surface | Reuse or bounded change | Ownership constraint |
| --- | --- | --- |
| `cli/src/native/browser_host_control.rs`, existing native input dispatch | Existing human ownership, epochs, in-flight effects and observed return | PL-PLATFORM contract provider; do not duplicate input/viewer authority |
| `cli/src/native/service_challenge_task.rs` | Real observation-to-task composition alongside explicitly provider-free fixture behavior | PL-CHALLENGE; keep current fixture behavior honest and compatible |
| `crates/agent-browser-service-model/src/service_challenge_task.rs` | Durable challenge state, typed intervention, original operation and independent downstream admission | Shared schema/model changes have one writer before clients |
| `crates/agent-browser-challenge-control/` | Existing lifecycle, budgets, registered checkbox profiles and pure decision rules | No image provider/solver implementation in Stage 1 |
| `cli/src/native/desktop_locator.rs`, `desktop_interaction.rs`, shared Native capture/input adapters | Reuse OCR/geometry and guarded interaction requirements through current Remote View | Old X11/presentation assumptions are evidence, not a provider to revive |
| Existing MCP workflow inspection/action and service request adapters | Additive briefing/result integration on supported task seams | Keep MCP, HTTP, generated client and schemas aligned if changed |
| `packages/client/src/playbooks.js`, existing playbook continuation | Consume the same observation/result and retained task | Preserve Auth integration writer; no new credential operations |
| Existing handoff/control public fixtures and captcha lab | Public workflow red/green, actual Native synthetic effects, independent task-admission oracle | No private pages or production challenges to create a reproducer |

Paths identify inspected source; exact public wire names are delivery details. During implementation, use graph discovery against the then-current baseline and choose the narrow supported task interface, not a new top-level tool per internal helper. Schema additions and the shared provider contracts land before dependent client changes.

## Dependencies and overlaps

The separately assigned [standalone Auth integration](https://github.com/CochranResearchGroup/agent-browser/issues/449) owns all private input/MFA/account authentication work. Its current dirty adapter and guidance are foreign. Stage 1 can use synthetic already-authenticated pages for independent challenge/continuation proof; real authenticated acceptance consumes the qualified Auth outcome when needed.

The browser mode/profile program is [source-qualified](https://github.com/CochranResearchGroup/agent-browser/issues/400); verify the exact current Native capture/input capability used by this workflow rather than assume an entire installed matrix. Preserve the dashboard command writer and [resource convergence](https://github.com/CochranResearchGroup/agent-browser/issues/105) lane. Do not absorb their unrelated defects, broadly restart services or take their checkout/candidate custody.

[Historical Turnstile work](https://github.com/CochranResearchGroup/agent-browser/issues/66) stays paused. This is a new current-generation plan with a read-only evidence dependency. Product-lanes prose naming an old active feature worktree is not assignment authority. Do not mark the old lane resumed/closed or reuse its old live-effect authorization.

Required Native observation, input or exact-operation readback gaps must be named against their owning provider and evidenced by one discriminating public check. They block only the affected acceptance row. Do not make whole Remote View, Auth, speech, upgrade, CI or resource programs prerequisites.

## Delivery sequence and budget

### A. Freeze public baseline and result contract

Use a disposable local HTTPS application and existing task/control seams. Establish one exact task, its peer and one uncertain synthetic submission. Capture existing inspection/reconnect behavior before changing it. Define the additive briefing and action/result semantics using existing operation identities. No new mutable effect source of truth. Freeze the externally observable oracle before implementing the first change.

### B. Briefing and classification

Implement the smallest read-only projection and current observation adapter. Create fixed positive and confounding negative examples for every declared class, including both registered checkbox profiles, passive completion, delayed loading, ordinary login, denied access, ambiguous layout and unknown/stale evidence. Capture corpus/version and results. A corpus pass is bounded detection evidence, not universal CAPTCHA recognition. Keep image puzzles detectable but unautomated.

### C. Bounded Native action and same-task assistance

Wire one eligible registered checkbox through current Native capture/input/control and the existing durable task. Passive wait, checkbox attempt and human assistance share verified-return/result semantics. The first public current-Native reproducer must establish whether the historical observation-age defect still exists before any timing change. Preserve original unknown outcomes and never manufacture site admission from widget state.

### D. Interruption and installed qualification

Exercise fresh agent/client inspection, actual human takeover/return and one isolated service restart, plus lost acknowledgement, changed target/control and peer preservation. Stop a failing scenario and retain its original operation before deciding the next action. Do not rerun the full matrix to seek a preferred result. Reuse passed unchanged gates and expand only for a new failure, change or acceptance gap.

Use one broad Standards/Spec review of the frozen completed batch, one consolidated remediation pass and closed-world verification. One current generation is the ordinary qualification target; a rebuild or retry needs a concrete source change or unresolved failure. One planner/primary is responsible; no subagents or independent checkout are assigned by this document. No token budget or runtime deadline is invented here.

## Worker assignments

Current primary: ecochran76's planning session owns this plan, issue and five planning projections only; implementation is unassigned/not started. The operator's other agent retains the standalone Auth work. Future Stage 1 implementation primary owns the single critical path and shared service/schema/documentation arbitration before any effect.

Low-conflict future tracks may prepare a fixed synthetic classification corpus and a public interruption oracle after shared input/result contracts are frozen. They return to the same primary and do not acquire independent Git/runtime custody. Provider/model/source contracts are serialized before consumer adapters. A later execution request must record any actual worker assignment and current writable files; this plan is not a delegation event.

## Acceptance and evidence matrix

| Row | Observable outcome | Required evidence |
| --- | --- | --- |
| Briefing | Useful task goal/context/outcome/artifact/assistance/uncertainty with exact identity and provenance | Public inspect regression; no browser mutation/launch/focus; private-field redaction |
| Classification | Declared corpus distinguishes checks, image puzzle, denied access, login, loading and unknown | Versioned examples, false-positive/negative observations and explicit limits |
| Passive/checkbox | Qualified wait or one guarded Native attempt produces a verified outcome or typed intervention | Actual Native synthetic controls and independent HTTPS server/application oracle |
| Assistance | Same durable native link, human control, fresh return and original task continuation | Actual viewer input/display qualification where claimed; peer unchanged |
| Interrupted continuation | New agent/client and isolated service restart recover original task/operation evidence | Same logical identity, honest changed physical generation, no refill/resubmit/relaunch inferred from a lost reply |
| Uncertainty and race | Lost acknowledgement, stale frame, changed window/control, expiry and unknown submission preserve effect history | Public negative/race cases; independent server effect counts prove no duplicate |
| Cleanup | Only fixture-owned resources are retired and peers survive | Fresh OS process/resource census in addition to modeled status |
| Installed/provider | Exact installed candidate supports the workflow; any real provider claim has its own attributable receipt | Isolated publication plus separately authorized public/provider acceptance; fixture/test token is not production clearance |

Completion requires all applicable promised rows, including installed real-browser qualification; source-only work may be integration-ready but must leave this plan's wider acceptance explicit. Live provider attempts need exact bounded authority, not merely this document or an open issue. Do not silently weaken acceptance to close the plan.

## Validation

Planning PR: patch hygiene, changed local links, exact P295 planning/catalog audit, general active planning audit with unchanged baseline findings named, policy wiring and documentation-site build. No application, Rust, browser or provider suite is needed for this five-file planning-only change. CI remains disabled and must not be dispatched.

Future implementation: use the changed-surface selector against the whole batch baseline. Run meaningful public contract/classification tests, focused Rust compartments and relevant generated-client/service parity. Any Rust source change requires cargo-safe workspace format and strict Clippy. Update CLI help, README, repository agent skill, docs-site pages and inline contracts for user-facing changes. Use the isolated development candidate and existing real Native synthetic harness, with task-owned profiles and exact cleanup; do not replace production binaries. Only current scoped effects may be executed, and source tests/installation/provider proof stay separate.

## Non-goals

No hCaptcha image solver or other image/audio solving; no video/transcription/teaching implementation; no replacement credential, viewer, desktop or general orchestration authority; no new broad event stream, extraction, XHR mode, backup/reset, profile migration, resource soak or release. Do not mutate user accounts, solve a live challenge, send messages, access private recordings, restart a shared service or clean foreign resources in the planning task.

## Evidence and exit

Use one compact acceptance table in the eventual RUNBOOK checkpoint with exact baseline/candidate, commands, results, source/install/provider scope, artifact locators, retained failures and remaining obligations. Keep private page/media/credential/runtime evidence outside Git. No screenshot, successful dispatcher call or old plan status substitutes for the promised public browser outcome.

Planning task exits when this bounded plan and its roadmap/runbook/index/catalog projection have a pushed reviewable draft PR with verified checks and preserved foreign work. Keep Status PLANNED and the implementation issue open. No implementation checkout, source edit, live run, deployment or old-lane resumption is implied by that closeout.

Implementation exits when the acceptance rows pass at their declared seams, the primary completes the single review/remediation bound, canonical source enters through its PR and fixture/runtime cleanup is proven. Keep unavailable provider/account acceptance open with a named dependency and preserved evidence; do not claim the entire stage delivered from source qualification alone.
