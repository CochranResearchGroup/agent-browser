# Runbook

Current index. [The September 13 archive](RUNBOOK-history-2026-09-13-through-turn312.md) preserves checkpoints through Turn 312; [Turn 313](RUNBOOK-history-2026-09-13-turn313.md), [Turns 314 through 320](RUNBOOK-history-2026-09-14-turn314-through-turn320.md), [P169 history through Turn 319](RUNBOOK-history-2026-09-14-p169-through-turn319.md), [superseded P186 checkpoints through Turn 328](RUNBOOK-history-2026-09-14-p186-through-turn328.md), and [P194 through P196 checkpoints from Turns 340 through 343](RUNBOOK-history-2026-09-15-p194-through-turn343.md) remain separately preserved.
## Active Plan Locator Index

- [P12](docs/dev/plans/0012-2026-05-31-workspace-inspection-pane-app-intelligence-roadmap.md), [P78](docs/dev/plans/0078-2026-07-27-guacamole-route-fixture-recovery-interlock-plan.md), [P111](docs/dev/plans/0111-2026-08-13-multi-agent-shared-browser-profile-authority-plan.md), [P116](docs/dev/plans/0116-2026-08-15-runtime-adoption-and-transactional-upgrade-plan.md), [P144](docs/dev/plans/0144-2026-08-31-lease-authority-coordination-and-revocation-plan.md), and [P158](docs/dev/plans/0158-2026-09-02-frozen-candidate-historical-failure-stress-campaign.md)
- [P157 production identity](docs/dev/plans/0160-2026-09-06-production-profile-identity-and-operational-readiness.md), [P157 desktop slots](docs/dev/plans/0162-2026-09-10-production-desktop-slot-and-jit-viewer-allocation.md), [P157 profile reset](docs/dev/plans/0163-2026-09-10-profile-data-reset-backup-and-restore.md), [P165](docs/dev/plans/0165-2026-09-11-bill-identifier-form-drift-repair.md), [P169 Turnstile leaf](docs/dev/plans/0169-2026-09-11-cloudflare-turnstile-desktop-challenge-plan.md), and [P178](docs/dev/plans/0178-2026-09-13-browserless-runtime-lane-quiescence.md)
- [P182](docs/dev/plans/0182-2026-09-13-authentication-resume-state-reconciliation.md), [P187](docs/dev/plans/0187-2026-09-14-challenge-countermeasure-control-plane-blueprint.md), [P169 hCaptcha leaf](docs/dev/plans/0189-2026-09-13-hcaptcha-fixture-checkbox-acceptance.md), and [P190](docs/dev/plans/0190-2026-09-14-advisory-candidate-build-and-promotion-orchestrator.md)

## Turn 355 | 2026-09-16

[Plan 0201](docs/dev/plans/0201-2026-09-16-x-display-live-occupancy.md)
is closed. [PR #166](https://github.com/CochranResearchGroup/agent-browser/pull/166)
merged source head `72c7a859` into `main` as `a23764a1`; issue #159 closed.
Exact-head CI run `35110970667` passed the complete Rust suite, no-launch
service smokes, strict Clippy, formatting, workstation fixtures, service client,
dashboard, and version sync. The active-lane projection no longer includes
P201. No browser, X server, installed-runtime, provider, Service State,
retained-profile, production, or release effect occurred.

## Turn 354 | 2026-09-16

[Plan 0201](docs/dev/plans/0201-2026-09-16-x-display-live-occupancy.md)
is source-complete at `42beec54` through draft PR #166. The original focused
run failed because an ordinary file and a stale socket inode were both treated
as `ActiveSocket`. The repair uses exact filesystem and abstract addresses in
`/proc/net/unix`, keeps matching X processes and unknown observation reserved,
revalidates before removing exact stale paths, and reports bounded per-display
classifications on exhaustion. All 11 focused regressions, formatting, strict
workspace Clippy, patch hygiene, planning audit, and selector-required
no-launch route-confusion gates pass. Exact-head CI, review, integration, and
closeout remain. No live or installed-runtime effect occurred.

## Turn 353 | 2026-09-16

[Plan 0201](docs/dev/plans/0201-2026-09-16-x-display-live-occupancy.md)
is admitted from `main@b11a7227` for issue #159. The no-live packet will first
prove ordinary files and stale filesystem socket inodes are not live X display
evidence while live filesystem and abstract listeners remain reserved. It will
then add exact stale-residue reclamation and a bounded exhaustion summary.
P201 solely owns `cli/src/native/cdp/chrome.rs`; P190 and P197 are disjoint. No
browser, X server, installed-runtime, provider, Service State, retained-profile,
production, or release effect is authorized.

## Turn 352 | 2026-09-16

[Plan 0200](docs/dev/plans/0200-2026-09-16-cargo-scope-descendant-lifetime.md)
is closed. PR #163 merged source head
`2978594e` into `main` as `c98da4cc`; issue #102 closed and its stale
`state/in-progress` label was removed. The clean P200 worktree and merged local
and remote branches were removed. Exact-head CI run `35098782749` and
merge-commit CI run `35099042490` pass every selected gate, including Rust,
no-launch service smokes, and workstation fixtures. No installed
runtime, browser, provider, Service State, retained-profile, production, or
foreign-process effect occurred. Live GitHub readback found no branch protection
or repository ruleset enforcing the documented gate; issue #164 tracks that
separate governance defect without opening another implementation lane. The
stale integrated P169 custody record is removed from the active-lane catalog;
its merged history remains in Plan 0187 and `main`, while P197 owns the active
consumer-integration work and issue #66 retains its separate live gate. P200 is
also removed from the active-lane catalog after closure.

## Turn 351 | 2026-09-16

[Plan 0200](docs/dev/plans/0200-2026-09-16-cargo-scope-descendant-lifetime.md)
is locally source-complete at `fc2a3359` through draft PR #163. Shellcheck,
Node syntax, deterministic success and exit-23 cleanup, dead-wrapper
active-scope accounting, unavailable-systemd behavior, profile-residue and
foreign-process controls, real user-systemd success and exit-23 cleanup,
planning audit, diff hygiene, and changed-surface selection pass. Fresh
readback finds no P200 unit or process residue. The aggregate WSL-entrypoint
command has the same pre-existing
`scripts/test-lease-authority-crate-architecture.js:46:raw_compiling_cargo`
failure on canonical `main@3863106e`; it is not counted as P200 evidence.
Protected exact-head CI, integration, and closeout remain.

## Turn 350 | 2026-09-16

[Plan 0200](docs/dev/plans/0200-2026-09-16-cargo-scope-descendant-lifetime.md)
has source checkpoint `ac862e80`. The red provider-free fixture proved the
wrapper returned while its browser-like descendant remained alive. The repair
assigns an exact scope identity, retains a dead-wrapper claim while its scope
is active or unobservable, and stops only that scope before releasing the
claim. Success, exit-23 failure, lingering-scope accounting,
unavailable-systemd, profile-residue, and foreign-process controls pass. One
disposable real user-systemd scope also passes with no P200 unit or process
residue. Shellcheck passes; complete changed-surface validation and protected
integration remain. The unrelated live P190 Cargo claim and dirty worktree are
unchanged.

## Turn 349 | 2026-09-16

[Plan 0200](docs/dev/plans/0200-2026-09-16-cargo-scope-descendant-lifetime.md)
is admitted from `main@3863106e` for issue #102. The provider-free packet will
first prove that an exact user-systemd scope can retain a browser-like
descendant after its Cargo parent exits while its admission claim disappears,
then bind accountability and bounded teardown to scope emptiness. P200 owns
`scripts/ci/cargo-safe.sh` and its new fixture. P190 remains the writer for its
candidate orchestration and current test-runner changes. No installed runtime,
browser, provider, Service State, production, or foreign-process effect is
authorized.

## Turn 348 | 2026-09-16

[Plan 0199](docs/dev/plans/0199-2026-09-16-compatible-access-profile-selection.md)
is CLOSED. PR #160 merged exact rebased branch head `01c05d0d` into `main` as
`e2e81e38`; source-head CI run `35092326047` and merge-commit CI run
`35092355450` pass. Issue #67 is closed. Access planning now prefers a
positively compatible retained profile before browser work and returns typed
no-effect recourse for an explicitly requested incompatible profile without
producing an executable request. No browser, provider, profile, Service State,
installation, shared-runtime, production, or release effect occurred.

## Turn 347 | 2026-09-16

[Plan 0199](docs/dev/plans/0199-2026-09-16-compatible-access-profile-selection.md)
is source-complete at checkpoint `15dd3b58` through PR #160. The original
provider-free fixture selected `a-incompatible-retained` ahead of a compatible
candidate. The repair joins exact browser capability compatibility before the
access plan emits an executable request, deterministically prefers a compatible
candidate, and returns typed no-effect recourse for an explicitly requested
incompatible retained profile. All 47 focused access-plan tests, formatting,
strict workspace Clippy, API/MCP parity, generated-client checks, the complete
service-client suite, the docs build, and handoff-doc checks pass. The legacy
no-launch shell fixture has the same pre-existing Google readiness mismatch
against canonical `main` and the P199 binary before reaching this contract. No
browser, provider, profile, Service State, installation, or runtime effect
occurred. P198 source and closeout are integrated at `main@7db8310f` without
P199 editing its lifecycle-owner surface. Protected integration and refreshed
exact-head CI remain.

## Turn 346 | 2026-09-16

[Plan 0198](docs/dev/plans/0198-2026-09-16-retained-owner-inventory-coherence.md)
is CLOSED. PR #158 merged exact branch head `38344ecf` into `main` as `c2ade1d1`; CI run `35086940883` passes and issue #143 closed.
The root defect was browser inventory applying current-PID authority validation
to structurally valid owner history retained after a terminal browser cleared
its PID. The repair preserves that history as observational, keeps nonterminal
PID mismatches fail-closed, retains typed launch-recovery recourse, and marks
read-only collection failures `no_effect`. The red-to-green fixture, focused
owner and recourse tests, 99 service-health tests, formatting, and strict Clippy
pass. No live or installed-runtime effect occurred.

## Turn 344 | 2026-09-15

[Plan 0196](docs/dev/plans/0196-2026-09-15-foreground-launch-stale-revision-acceptance.md) is CLOSED. PRs #150, #153, and #154 merged the #87 race proof and exact installed-acceptance repairs into `main@279b2228`. Final generation
`0.28.0-15f0f3576657-30788a166073`, SHA-256
`15f0f3576657da6cfc9a59bdb23b0b3d94fb37d73a4c11d3c1e08a6d4fcf8634`, is accepted under transaction `upgrade-d0e7c98f-d6b7-4e48-bc60-aa570e5dc329` revision 13. Installed doctor passes with one runtime host, one dashboard, healthy monitor, and exact supervisor identity. A fresh explicit filesystem profile opened `about:blank`, returned the same URL, closed, moved its profile to trash, and left zero exact process holders. A targeted Guacamole web recreate loaded the sealed extension at `0555` and `0444`, returned HTTP 200, and left PostgreSQL and guacd unchanged. Issues #87, #131, and #151 are closed; #143 remains open for the broader retained-owner inventory defect. The dashboard operator-journey warning remains nonblocking and no tenant workflow was run.

## Turn 339 | 2026-09-15

P169 W5 is integrated and [Plan 0193](docs/dev/plans/0193-2026-09-15-challenge-task-service-orchestration.md)
is CLOSED. PR #142 merged exact source head `709641e9` into `main` as
`81de07cf`. Source-head CI run 34993316164 passes. Merge-commit CI run
34997057075 passes on attempt 2 after one unchanged control-plane timing test
failed once and passed on the failed-job-only rerun. W6 is next but unstarted
and not admitted; Plan 0187 and issue #127 remain open. No browser, CAPTCHA,
provider, credential, installed-runtime, production, or release effect
occurred.

## Turn 337 | 2026-09-15

P169 W5 is admitted through [Plan 0193](docs/dev/plans/0193-2026-09-15-challenge-task-service-orchestration.md)
on `challenge/p169-task-orchestration` from merged-main checkpoint `85b4ee92`.
The packet will add one durable challenge-aware Service task, reuse established
Service State and task-custody seams, drive it first through registered
provider-free fixtures, and project one receipt plus bounded status and
resource summaries. No browser, CAPTCHA, credential, provider, installation,
shared-runtime, production, or release effect is authorized.

## Turn 336 | 2026-09-15

P169 W0 through W4 are integrated. PR #128 merged source checkpoint
`d7c59be2` into `main` as `e2bd73ff`. Exact-head PR CI passed after one bounded
rerun of a diagnosed timing-threshold flake. Merge-commit CI run 34977471104
and Lease Authority run 34977470855 pass; the latter covers Linux, Windows,
and both macOS targets. The source branch remains durably published while its
clean checkout is retired after this reconciliation lands. Plan 0187 and issue
#127 remain OPEN because W5 through W8 are not delivered; W5 is unstarted and
not admitted. Issue #66 remains the separately live-gated leaf. No browser,
CAPTCHA, provider, installed-runtime, production, or release effect occurred.

## Turn 335 | 2026-09-14

P169 remains the valid antibot worktree and is ready for protected-main
integration after exact-head CI. Plan 0187 joined current `main` checkpoint
`58349195` without rewriting challenge history. Published source checkpoint
`406323bb` completes W0 through W4: phase-bound interaction freshness,
`agent-browser-desktop-services`, the pure `agent-browser-challenge-control`
crate, and a two-profile Turnstile and hCaptcha provider-free Service slice.
Both profiles use one generic five-outcome evaluator and the no-launch action
always reports `emittedEffects=false`. Focused crate, CLI, schema, generated
client, architecture, documentation, formatting, and strict-Clippy gates pass.
W5 is unstarted and not admitted. No browser, challenge, provider,
installed-runtime, production, or release effect occurred.

## Turn 334 | 2026-09-14

[Plan 0191](docs/dev/plans/0191-2026-09-14-repository-worktree-and-lane-reconciliation.md) is CLOSED through issue #139 and PR #140 after reconciling repository custody.
Eight P181 wake lifecycle files are preserved in a user-scoped archive with
SHA-256 `a7249285b59ce1644b2401406f9f3519e4627af4f2b8c4fddd843e0aaf2e0b32`;
both detached benchmark diffs have standalone patches. Five integrated or
disposable auxiliaries are removed, leaving canonical P191 and challenge P169.
P180 and P183 through P185 close from merged source plus P186 installed
acceptance. P182 remains BLOCKED without source custody because its same-run
acceptance was not exercised. P169 now points to Plan 0187, issues #127 and
#66, draft PR #128, and remote-equal `fec7fd87`; its conflict and failed Rust
gate remain. [P190](docs/dev/plans/0190-2026-09-14-advisory-candidate-build-and-promotion-orchestrator.md) stays PLANNED. No runtime or external effect occurred.

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
