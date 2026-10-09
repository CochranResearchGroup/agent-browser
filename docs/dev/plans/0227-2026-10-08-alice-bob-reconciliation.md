# P227 Shared profiles, retention and desktop capacity

Date: 2026-10-08
Plan version: 2
Consolidation: required
State: OPEN
Product lane: PL-PLATFORM
Owner: primary agent
Branch: platform/alice-bob-reconciliation
Target: main
Work item: https://github.com/CochranResearchGroup/agent-browser/issues/233
Authority: operator authorized execution and bounded isolated installed validation on 2026-10-09. No production upgrade or reboot.

## Outcome

Agents can deliberately share profile data without interfering with each other's tabs. Durable work stays reconnectable under its configured retention policy. New profiles use ready vacant desktops when possible; capacity grows, wraps around and shrinks predictably.

## Settled behavior

- One session and tab per task. Tasks needing the same login and stored data may share one profile and its browser. Neither tab needs to be physically visible for tab-targeted automation. Foreground desktop input is serialized; shared login and application-state changes require coordination.
- Durable profiles retain data across tasks. Their logical handoffs default to no time expiry, with configurable retention. Explicit close remains terminal. Browser idle cleanup, session activity, link retention and native viewer authentication are separate lifetimes.
- Disposable profiles have configurable inactivity retention and safe data cleanup. Closing or expiring one task must preserve other tasks using that profile. Removing a physical browser or desktop does not itself authorize deleting profile data.
- Prefer a vacant desktop for a new independent profile. After consuming the spare, prepare another vacant desktop in the background whenever provider limits and resources permit. A launch must not wait for replenishment after it already has a usable desktop.
- At the growth limit, place new independent browsers by wrap-around across eligible existing desktops. This is the agreed overflow policy; a new queue or least-loaded policy is not part of this delivery. Same-profile sessions reuse their browser rather than advancing desktop placement.
- When at least two desktops remain unused continuously for the configured cooldown, reap excess capacity while preserving one ready vacant desktop. Initial cooldown: 15 minutes. Recheck current references before each removal; reset eligibility when use resumes. Native Remote View owns provisioning and retirement.
- Respect the configured maximum of ten total desktops and the provider's actual managed eligibility. Direct-use or foreign desktops are not automatically available to Agent Browser.

Profile durability is a data-retention choice, not a promise to keep a browser process or desktop running. A task/session owns its tab and handoff; the profile owns shared authentication and storage. A durable profile survives final-session closure. A disposable profile becomes eligible for deletion only after its configured retention and current reference checks permit it. Session names alone must not change profile durability. Prefer one profile per task unless sharing authentication or stored state is intentional.

## Consolidated batch

Deliver the four workflow outcomes below together, using existing session and native provider interfaces. First establish which behavior already works; change only demonstrated gaps. Documentation is the main sharing change unless the paired-task check fails. Profile retention and desktop capacity need their own evidence and may pass independently; one passing axis does not conceal another failing axis.

## Delivery order and acceptance

### 1. Teach and verify shared-profile operation

Update the agent and service skills and matching user documentation with the session/tab rules above. Use the existing session workflow to run one focused Alice/Bob check before changing sharing code.

Done when two sessions using one synthetic profile share one browser, address distinct background tabs, and read/write only their intended tab. Both see the intentionally shared profile data. Closing Alice leaves Bob usable; closing the final session releases the browser. Actual desktop interaction remains explicitly serialized. Repair only a demonstrated failure.

### 2. Make retention follow profile intent

Use one explicit configurable policy that distinguishes durable and disposable lifetimes. Correct the current unconditional 24-hour handoff default. Keep existing profile deletion protections; do not redesign the profile model.

Done when a controlled-clock test proves a durable link remains usable beyond the old default unless configured otherwise, a configured finite link expires correctly, and disposable inactivity eventually closes eligible work and deletes only unreferenced disposable data. Restart preserves the selected policy and logical identity. Existing expired links stay expired; upgrade treatment of existing finite links is explicit and tested rather than silently rewritten.

### 3. Deliver ready-spare growth, wrap-around and safe shrink

Use native provider lifecycle contracts and the existing browser placement path. Keep the spare replenishment off the foreground launch path and prevent concurrent requests from creating duplicate spares. Persist wrap-around placement sufficiently for repeat/restart behavior to be predictable; exclude ineligible desktops and preserve existing assignments.

Done when a small deterministic pool demonstrates all of these:

- A new profile consumes an already-ready vacant desktop, then background replenishment restores one ready spare while below the limit.
- At the configured limit, successive independent profiles wrap around eligible desktops without relocating existing browsers or disturbing peers.
- With two unused desktops, no removal occurs before cooldown; after cooldown excess capacity is retired, leaving one ready spare. Resumed use prevents retirement.
- Session, browser, handoff, viewer, control and pending-operation references are checked according to actual current bindings. Stale history alone cannot permanently reserve phantom capacity. Retained work recovers if its physical resources were legitimately reclaimed.
- Regrowth produces a usable native handoff without manual viewer mapping. Current resource pressure or no eligible destination produces a concrete typed failure while preserving existing work.

### 4. Verify availability and finish the installed workflow

At the same ordinary-open boundary, inject unrelated stale history alongside an available valid destination. Prove it cannot veto the request. An exact current unsafe acquisition remains protected and returns an actionable error. If the existing path already satisfies this, record the result and move on.

After source checks pass, run a bounded isolated installed demonstration covering shared-profile tabs, direct native handoff and automation return, ready-spare placement, wrap-around, shrink and regrowth. Use synthetic data and task-owned resources. Capture fresh process/resource readbacks after cleanup. Record failures with the exact behavior they invalidate; reuse unaffected evidence.

## Provider dependency and stopping rule

Safe shrink depends on native Remote View proving current desktop inactivity and accepting an exact, guarded consumer release. Track this work in [Remote View issue 339](https://github.com/CochranResearchGroup/remote-view/issues/339) and its Plan 0069. Agent Browser returns unused assignments; Remote View owns the physical cooldown, viewer checks and desktop retirement. Keep one ready spare across that seam rather than building a second desktop lifecycle manager.

A viewer reconnect or renewed work must prevent retirement. A definite refusal leaves capacity available; an ambiguous release retains its exact pending obligation until reconciled. Never count an uncertain release as free capacity. Demonstrate retained logical work recovering after a confirmed physical release.

Do not declare this batch complete with documentation, fixtures or provider-only tests. Completion requires the installed synthetic workflow in step 4. If resource admission or the execution bound prevents that demonstration, checkpoint the exact passed evidence, failed behavior and remaining gate; leave the plan OPEN.

## Validation and work boundary

Primary automated seam: ordinary managed-session requests with disposable SQLite state and the public native-provider transport. Use a controlled clock and small pool to exercise limits and 15-minute behavior cheaply. Add regression tests only for changed or demonstrated failing behavior. Run the repository's required changed-surface checks, including format and strict workspace Clippy for Rust changes and contract/client checks where applicable.

Update all required help, README, skills, docs and inline comments when flags, configuration or behavior change. Configuration names and upgrade mechanics are implementation details to settle against existing interfaces, not reasons to reopen agreed product behavior.

One owner delivers the bounded batch. Shared-profile guidance/verification, retention and capacity can be prepared independently; the installed demonstration follows their source acceptance. The false-blocker check is independent. No subagents or new worktrees are required by this plan. Preserve unrelated dirty work and pinned candidate artifacts. Integrate through the normal work-item-linked PR path.

Done means the behavior above is demonstrated and the guidance matches it. It does not mean every historical ledger row is requalified. No legacy RDP/helper restoration, second viewer, new access ceremony, ten-browser stress campaign, production upgrade, reboot or formal release is included. No further breakdown approval or new planning round is required by this plan before carrying out subsequently authorized implementation.

## Delivery sequence and budget

Run the paired-task baseline first, then retention and capacity repairs, then one installed synthetic demonstration against the resulting candidate. Retention fixtures and guidance can be prepared independently; provider idle/release support precedes shrink acceptance. Reuse unaffected evidence. Bound the pool demonstration to three desktops with a shortened configured cooldown, and separately verify the initial 15-minute configuration. Do not add a stress campaign, abstraction extraction or broad historical requalification to finish this batch. Honor any separately authorized execution bound; planning does not create or renew one.

## Worker assignments

The primary owner maintains this plan and delivers the Agent Browser changes and acceptance. Remote View owns native provisioning, idle observation and physical retirement; its dependency must land before consumer integration. No delegation or additional checkout is needed. Preserve existing work and assign a single writer to any shared contract surface.

## Evidence and exit

Keep one compact acceptance record: workflow, candidate identity, observed result, and remaining limitation. Record shared storage plus independent tab actions and peer closure; durable and disposable retention across restart; spare growth, wrap-around, cooldown/reset, shrink and regrowth; and native human input followed by automation in the same browser. Include a fresh process/resource readback after task-owned cleanup.

Close only when those outcomes pass and the matching guidance is integrated through the work-item-linked PR path. A ready URL, fixture test count, new schema or finished document alone is insufficient. If a workflow fails, preserve its failure and checkpoint the exact remaining gate rather than broadening the project or declaring completion.

## Evidence and current state

[Prior reconciliation](../notes/2026-10-08-alice-bob-contract-reconciliation.md) is background evidence, not the delivery checklist. Later operator decisions settle wrap-around, one ready spare, configurable 15-minute shrink cooldown, configurable handoff longevity and durable/disposable distinction. Current source already distinguishes named/disposable profile data and restricts disposable deletion, but logical handoff creation still applies the same finite default. P226's bounded separate-desktop acceptance is reusable for its original scope, not proof of this batch.

Planning validation: local links resolve and diff hygiene passes. No code or runtime changes were made by writing this plan.

## Current State

Execution started 2026-10-09 03:16 UTC at c4054ed6. Checkpoint by 06:06 UTC, before the three-hour bound at 06:16 UTC, or before two million tokens, whichever comes first. First packet: shared-profile behavior verification and profile-aware retention through existing consumer tests. Reuse p204; preserve all unrelated checkouts and prior candidates.

Installed checkpoint, 2026-10-09 04:44 UTC: isolated p227 demonstrated shared profile storage and separate Alice/Bob tab actions, native handoff readiness, and background spare growth. After adopting the repaired provider only into p227r, the empty spare assignment was released and its native desktop removed, leaving one ready desktop. A fresh live-resource idle observation returned idle for that remaining empty desktop; attaching a task-owned windowless, non-dumpable X11 client made observation refuse retirement evidence. The probe was closed. Provider adoption completed without pending reconciliation.

Ordinary Alice continuation after browser inactivity failed with `browser_session_recovery_absence_unproven`; preserve this installed failure and its unproven transient cause. A fresh profile/process census followed by one retry recovered the original task URL. Closing Alice then preserved Bob's browser, original task URL and shared storage. Independent Carol and Dave profiles grew the pool to three desktops; Eve wrapped onto Bob's desktop with a distinct browser/profile. All four task URLs remained correct. Closing Carol and Dave closed only their browsers. Full native human-input/automation-return acceptance, cooldown/reset qualification and final cleanup/integration remain open. The private synthetic hostname is not externally published. Source gates do not substitute for these installed checks; the plan remains OPEN.

Native viewer checkpoint: the same retained Bob handoff resolved from the private Agent Browser HTTPS origin into the native desktop viewer, using isolated dashboard authentication and a synthetic native authentication fixture. The connected WebSocket displayed the intended Chrome tab. Viewer mouse and keyboard input entered a synthetic value in the page; ordinary Bob automation read that exact value and the shared storage afterward. No extra viewer grant or dashboard navigation was needed for the handoff. This qualifies the private installed flow, not public ingress or production Authelia. The first harness attempt failed Node-side DNS resolution; its exact observer was closed and confirmed absent. A separate handoff attempt during authentication configuration reload failed native window readback; after completed service adoption the normal handoff succeeded. Preserve both failed attempts. Provider full serial tests and current format check passed. Cooldown/reset qualification, final cleanup and source integration remain open.
