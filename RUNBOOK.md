# Runbook

Current execution and stop-state index. Detailed checkpoints through Turn 312
are preserved in [the September 13 archive](RUNBOOK-history-2026-09-13-through-turn312.md).
[Turn 313 is preserved separately](RUNBOOK-history-2026-09-13-turn313.md).
[Turns 314 through 320 are archived](RUNBOOK-history-2026-09-14-turn314-through-turn320.md).
[The P169 challenge history through its Turn 319 is archived separately](RUNBOOK-history-2026-09-14-p169-through-turn319.md).
[Superseded Plan 0186 checkpoints through Turn 328 are preserved separately](RUNBOOK-history-2026-09-14-p186-through-turn328.md).
Keep this file at or below 200 lines under policy 0043.

## Active Plan Locator Index

Current open, planned, or blocked plan locators retained outside the recent
turn summaries:

- [P12](docs/dev/plans/0012-2026-05-31-workspace-inspection-pane-app-intelligence-roadmap.md), [P78](docs/dev/plans/0078-2026-07-27-guacamole-route-fixture-recovery-interlock-plan.md), [P111](docs/dev/plans/0111-2026-08-13-multi-agent-shared-browser-profile-authority-plan.md), and [P116](docs/dev/plans/0116-2026-08-15-runtime-adoption-and-transactional-upgrade-plan.md)
- [P144](docs/dev/plans/0144-2026-08-31-lease-authority-coordination-and-revocation-plan.md), [P158](docs/dev/plans/0158-2026-09-02-frozen-candidate-historical-failure-stress-campaign.md), [P157 production identity](docs/dev/plans/0160-2026-09-06-production-profile-identity-and-operational-readiness.md), and [P157 desktop slots](docs/dev/plans/0162-2026-09-10-production-desktop-slot-and-jit-viewer-allocation.md)
- [P157 profile reset](docs/dev/plans/0163-2026-09-10-profile-data-reset-backup-and-restore.md), [P165](docs/dev/plans/0165-2026-09-11-bill-identifier-form-drift-repair.md), [P169 Turnstile leaf](docs/dev/plans/0169-2026-09-11-cloudflare-turnstile-desktop-challenge-plan.md), and [P178](docs/dev/plans/0178-2026-09-13-browserless-runtime-lane-quiescence.md)
- [P180](docs/dev/plans/0180-2026-09-13-pre-drain-browserless-lane-quiescence-repair.md), [P182](docs/dev/plans/0182-2026-09-13-authentication-resume-state-reconciliation.md), [P183](docs/dev/plans/0183-2026-09-13-terminal-owner-browser-missing-migration-repair.md), and [P184](docs/dev/plans/0184-2026-09-14-resumable-candidate-generation-retention.md)
- [P185](docs/dev/plans/0185-2026-09-14-route-viewer-runtime-profile-identity.md), [P187](docs/dev/plans/0187-2026-09-14-challenge-countermeasure-control-plane-blueprint.md), and [P169 hCaptcha leaf](docs/dev/plans/0189-2026-09-13-hcaptcha-fixture-checkbox-acceptance.md)

## Turn 329 | 2026-09-14

[Plan 0187](docs/dev/plans/0187-2026-09-14-challenge-countermeasure-control-plane-blueprint.md)
is OPEN and entered W0 custody normalization. Published feature checkpoint
`2ae7a68332b7a505c74bbd1e987d8a82fb52409d` remains preserved while
`challenge/p169-control-plane` joined current-main checkpoint
`994ed7b5f5c1deda5a968fa94a6ae3a82f04e2fc` through merge checkpoint
`99c85c683e40331d4d3f592c76f7ca3725d73ea7` without rewriting the old branch.
Current main supplies the extracted lease-authority crate that W2 must consume.

The merge preserves the first-request Guacamole header fix, current-main
runtime-profile and shared-host isolation, and both sides' runbook evidence.
The challenge packets are renumbered to Plan 0188 and Plan 0189. Parent
[issue #127](https://github.com/CochranResearchGroup/agent-browser/issues/127)
is open and in progress; issue #66 remains the blocked fixture leaf. W0 still
requires merge validation and published branch custody. No hCaptcha retry,
browser input, provider mutation, production mutation, or release is authorized.

W0 source and custody validation passes at published activation checkpoint
`342bb8fa`: version sync, lease-crate architecture, presentation-provider
fixture, formatting, strict Clippy, 108 lease-authority tests, 7 hCaptcha tests,
32 desktop-interaction tests, route-confusion gates, the source-free workstation
fixture, and planning audit. The active-lane catalog change is proposed on the
topic branch and becomes canonical only through protected-main integration.

W1 source implementation now recaptures and revalidates the selected target
after motion and before button-down, while the existing guarded-event fence
revalidates authority, surface, process, display, route, geometry, and provider
generation. Freshness starts at capture completion before locator work. The
provider-free long-motion, stale-refresh, moved-target, changed-geometry, and
effect-phase controller-change cases pass with the existing effect taxonomy.
Formatting, strict Clippy, 37 desktop-interaction tests, 7 hCaptcha tests, and
2 controlled-X11-provider tests pass. No live hCaptcha or browser effect ran.

W2 source extraction now places the provider-neutral transaction kernel and
process-local route coordinator in `agent-browser-desktop-services`. CLI-owned
dispatch, Service State projection, durable filesystem persistence, capture,
OCR, X11 input, and external route fencing remain adapters. The architecture
guard is green; formatting, strict Clippy, 3 independent crate tests, 37 CLI
desktop-interaction tests, 7 hCaptcha tests, 108 lease-authority tests, 3 CDP
tests, and remote-view documentation checks pass. No public behavior or live
runtime state changed.

Plan 0186 is CLOSED through PR #129 and main checkpoint `f84ae098`. Exact
candidate SHA-256 `66ac712c6a91a26395d3369fb40ec111e5e46a101a736e35c7c0acce999e7e76`
passed its source-free fixture and preserving install. Transaction
`upgrade-bacc8671-6067-4820-bb0f-7d264a446615` finalized routes A, B, and C;
their validation processes reached terminal cleanup with locks released. Final
doctor reports one runtime host, one dashboard, zero legacy daemons, no drain,
a healthy monitor, and 43 healthy profile leases. No tenant workflow retried.

## Turn 328 | 2026-09-14

[Plan 0186](docs/dev/plans/0186-2026-09-14-route-viewer-admission-drain-recovery.md)
is CLOSED. The repair batch merged through PR #129 as `f84ae098`; exact
candidate SHA-256 `66ac712c6a91a26395d3369fb40ec111e5e46a101a736e35c7c0acce999e7e76`
passed the source-free fixture and a default preserving workstation install.
Transaction `upgrade-bacc8671-6067-4820-bb0f-7d264a446615` accepted the exact
candidate and finalized A, B, and C as separate route-viewer handoff lanes.
Their validation processes later reached terminal cleanup with profile locks
released. Final doctor passes: one selected runtime host, one dashboard
process, zero legacy daemons, no admission drain, healthy runtime monitor, and
43 healthy profile leases. Last30Days owner generation 90 is unchanged.
Dashboard operator-journey evidence remains a nonblocking separate follow-up.
No tenant workflow was retried.

## Turn 324 | 2026-09-14

[Plan 0181](docs/dev/plans/0181-2026-09-13-lease-authority-kernel-crate-extraction.md) is CLOSED through PR #106, which merged exact validated head `91aa3204` into
`main` as `b5a78faf`. The new crate owns the canonical kernel and protected
stack; the old owner is deleted and the CLI retains one private adapter.

Full CI run 34846719359 is terminal. macOS ARM compiled the extracted crate and
then failed in inherited CLI-only code; Windows also reached the extracted
crate before fail-fast cancellation. Native E2E passed 42 tests, then retained
one navigation fixture browser and cascaded to 14 failures. Browser repair is
outside P181 authority and does not invalidate its provider-free source proof.
P6 measured a 91.95 percent focused-loop improvement without the stricter
promotion claim. Focused run 34857911397 passes Linux, both macOS targets, and
Windows after `b6aa71dd` fixed Unix-only fixture paths; ordinary CI run
34857911400 is green. Final pre-join head `8a57dce5` also passes focused run
34861501476 and ordinary run 34861501499. The second P186 repair merged to
`main` afterward and was joined cleanly. Exact focused run 34864731916 passes
Linux, both macOS targets, and Windows. Ordinary run 34864731908 passes every
fast gate, including comprehensive Rust and no-launch smokes. No runtime,
browser, profile, provider, installation, production, release, or Plan 0144
acceptance claim is made. No P181 execution remains.

## Turn 323 | 2026-09-14

P184 merged through PR #109 as `3b7e8411`; its exact integrated binary digest
is `a2899457`. Fresh preview had zero protected removals, changes, or removals.
The changed-source apply preserved the external browser, committed Service
State with no changes, and selected generation
`0.28.0-a28994570dd3-9d43d7f4e826`. Transaction
`upgrade-8c858bc3-4507-48a0-8eea-c85cd3326fbf` is forward-only at revision 17
with admission drained and exact resume as its only completion action.

P185's managed-profile repair merged through PR #111 as `ffc6e510`, and exact
integrated candidate `bce36a4c` built successfully. Reboot then removed all
route viewers. A transaction-bound attempt to recreate route A failed before
effect as `runtime_admission_draining`: the drain permits claimed Service
reconcile but not the launch, headers, navigation, and cleanup that reconcile
requires when no viewer survives. [Plan 0186](docs/dev/plans/0186-2026-09-14-route-viewer-admission-drain-recovery.md)
and [issue #112](https://github.com/CochranResearchGroup/agent-browser/issues/112)
own a canonical-viewer-only admission repair. Ordinary and tenant profiles
must remain blocked. PR #113 merged the first repair as `3c7d29da`; exact
candidate `34318d21` admitted route A launch, but `set headers` lacked the
global runtime profile at claim attachment and its generated launch failed
before effect. The temporary host was terminated with no display left. Next:
integrate exact session-profile shaping, complete the revision-bound forward
resume, then install one exact integrated generation. Do not retry any tenant
browser workflow.
