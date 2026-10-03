# Runbook

Current execution record. [Preserved history through the P222 foreground repair](RUNBOOK-history-2026-10-03-through-p222-focus.md) retains prior evidence, earlier archive links, and failed attempts without alteration.

## Active Plan Locator Index

- [P12](docs/dev/plans/0012-2026-05-31-workspace-inspection-pane-app-intelligence-roadmap.md), [P78](docs/dev/plans/0078-2026-07-27-guacamole-route-fixture-recovery-interlock-plan.md), [P111](docs/dev/plans/0111-2026-08-13-multi-agent-shared-browser-profile-authority-plan.md), [P116](docs/dev/plans/0116-2026-08-15-runtime-adoption-and-transactional-upgrade-plan.md), [P144](docs/dev/plans/0144-2026-08-31-lease-authority-coordination-and-revocation-plan.md), and [P158](docs/dev/plans/0158-2026-09-02-frozen-candidate-historical-failure-stress-campaign.md)
- [P157 production identity](docs/dev/plans/0160-2026-09-06-production-profile-identity-and-operational-readiness.md), [P157 desktop slots](docs/dev/plans/0162-2026-09-10-production-desktop-slot-and-jit-viewer-allocation.md), [P157 profile reset](docs/dev/plans/0163-2026-09-10-profile-data-reset-backup-and-restore.md), [P165](docs/dev/plans/0165-2026-09-11-bill-identifier-form-drift-repair.md), [P169 Turnstile leaf](docs/dev/plans/0169-2026-09-11-cloudflare-turnstile-desktop-challenge-plan.md), and [P178](docs/dev/plans/0178-2026-09-13-browserless-runtime-lane-quiescence.md)
- [P182](docs/dev/plans/0182-2026-09-13-authentication-resume-state-reconciliation.md), [P187](docs/dev/plans/0187-2026-09-14-challenge-countermeasure-control-plane-blueprint.md), [P169 hCaptcha leaf](docs/dev/plans/0189-2026-09-13-hcaptcha-fixture-checkbox-acceptance.md), [P190](docs/dev/plans/0190-2026-09-14-advisory-candidate-build-and-promotion-orchestrator.md), [P197](docs/dev/plans/0197-2026-09-16-challenge-consumer-integration.md), and [P204](docs/dev/plans/0204-2026-09-16-ci-validation-economics-and-tiering.md)
- [P214](docs/dev/plans/0214-2026-09-17-candidate-permit-event-plan.md)
- [P222](docs/dev/plans/0222-2026-10-02-real-client-browser-delivery.md); [P221 superseded history](docs/dev/plans/0221-2026-10-01-browser-product-acceptance.md); [P220 preserved history](docs/dev/plans/0220-2026-09-28-remote-view-consumer-integration.md)

## P222 current checkpoint | 2026-10-03

Authority: execute Plan 0222; stop and checkpoint before 750000 cumulative tokens.
Begin the stop checkpoint by 730000. Counter resets do not reset effort.
Product lane: PL-PLATFORM. Plan state: OPEN; none of its five milestones accepted.
Custody: agent-browser-p220, platform/p220-remote-view-consumer, source f2957896.
Remote View custody: remote-view-rv011, retained installed provider source 28b4ec1.
Production promotion, formal release and production ingress effects are excluded.
Do not delegate this client/provider path or create a replacement acceptance goal.

| Requirement | Current evidence | Remaining gate |
| --- | --- | --- |
| Actual AuraCall Stealth/view/control/close | Actual launcher selects reviewed Stealth and navigates; last client result converging | Exact tab/window and connected viewer join; authenticated handoff, control and cleanup |
| Persistent authentication | Not accepted | Reviewed login and restart/reconnect observation |
| Shared operation | Predecessor CLI observations only | Second real client, replay and peer-preserving close |
| Recovery/capacity/mobile | Not accepted | Observed disruption, stable logical identity, focus and elastic capacity |
| Install/update/doctor/platform/integration | Isolated candidate publication only | Full reproducible-install and changed-surface qualification |

Qualified source repairs: post-transport clock, scoped grant-key joins, exact
terminal pending replay/one renewal, opaque grant/revoke/release route IDs,
authenticated presentation redirect, and current owned foreground-window check.
The redirect regression failed before repair. The foreground regression failed
when focus returned success for another PID. Corrected checks pass: redirect,
five model record tests, sixteen pool tests, workspace formatting and strict
Clippy. Source f2957896 is not yet installed. No new candidate build is running.
Do not rebuild until remaining known readiness work is incorporated.

Installed task generation: 0.28.0-a96de24ab02e; SHA256
`a96de24ab02e69802834a209aae7176173c168d84804e33e697a6354677a827e`.
Task wrapper: `/home/ecochran76/.local/bin/agent-browser-dev-p221`.
Task state: `/home/ecochran76/.local/share/agent-browser-dev-p221`.
Installed generations: `/home/ecochran76/.local/lib/agent-browser-dev-p221`.
Both production and default-development unchanged publication checks passed.
Private evidence: task `evidence/p222-opaque-route-install.json` and
`evidence/p222-auracall-opaque-route.log`; the actual client uses no injected runner.
Original pending provider requests and orphan host tokens remain preserved.

Fresh census after foreground source repair: zero task browsers/sessions and
zero Chrome processes with the exact task profile argument. The expired session's
handoff preserves a live_resource issuance; issuance is not visible/control proof.
Task runtime host was last observed at PID 392054 under the installed generation.
Include .local/lib executable identity when checking hosts, not only .local/share.
Provider control owner 2667915 listens on 19103, exec handle 71812. Task gateway
2666588 listens on 19102 and auth 19104, exec handle 58208. Inspect identities before restart.
Retained task desktop and Guacamole containers are intentional provider resources.
Do not kill unrelated processes, rewrite private ledgers or reset profile state.

Remaining implementation gap: configured managed open/handoff resolution emits
converging/pending unconditionally. Focus now fences both desktop generations and
requires one active positive-size window for the owned PID, but viewer connectivity
and the authenticated operator join still need proof. Never map grant_issued to
operatorVisible ready or give a raw provider URL as an operator handoff.

Fresh ingress discovery: cooper-webservices has agent-browser-dev on 4948 and
production remote-view on 19092. Neither is the task dashboard 5048 or gateway
19102. There is no p221 ingress inventory and no task provider manifest binding.
The task config's production Remote View external origin must not be borrowed.
The pending endpoint question requests the intended isolated authenticated HTTPS
dashboard and Remote View origins. No shared ingress inventory was changed.
Private routing proposal now exists at task evidence/p222-ingress-proposal/services.
Proposed service names are agent-browser-dev-p221 and remote-view-dev-p221,
with dashboard 5048, gateway 19102 and private ingress auth 19104. Both pass the
existing Cooper inventory validator in strict mode. These are draft service
identities, not published routes, operator handoffs or reviewed deployment IDs.
No shared inventory, Traefik or bastion state changed. Task dashboard is now
running through its installed entry point: frontend 2650450 on 5048, backend
2650449 on 5049. Dashboard auth status reports unauthenticated; provider gateway
and forward-auth reject missing identity with 403. Task endpoint names remain the
proposed defaults pending any operator correction.

Supported Remote View apply qualified a public_origin/auth-only change with no
conflicts or interruptions, then completed with no pending reconciliation.
Config hash: 2c325acbbeccccff94aace9cf1c107b82b2e3b7819c33cac21d2f42423fc4cac.
Receipt: task remote-view/receipts/65cf04b8d5237-0a1b50d36adec8f152a90ff874a9a77b-apply.json;
rollback backup is retained under the matching task backups directory.
Task provider now names https://remote-view-dev-p221.ecochran.dyndns.org and a
new mode-0600 task secret outside its runtime home. It no longer references the
production ingress secret or auth-recheck URL. Initial draft validation rejected
a secret inside provider runtime home; corrected placement passes. Initial secret
preparation failed on a missing directory; corrected before endpoint restart.
Apply replaced the task executable inode; exact task process/config readback
qualified stopping the two old deleted-inode endpoints. New endpoints use the
retained control home and consumer policy; no private ledger was reset.
Next: complete the viewer-readiness join and dashboard/auth listeners, then render
and publish the task-only ingress with exact deployment identity, followed by one
source-bound candidate and actual AuraCall acceptance. Do not publish a route
whose required backend is absent or use inventory validation as live acceptance.

Progress classification: blocker_reduction; product acceptance remains incomplete.
Active tracker read: 199684; prior tracker floor: 255647; combined floor: 455331.
Use the conservative combined count against the original ceiling and refresh it.
Reassess after two no-outcome checkpoints or 30 active minutes; verified blocker
removal does not reset cumulative no-outcome time. Do not spend cycles on isolated
passing subsets when no evidence-backed path advances actual client acceptance.
Memory disposition: not_durable; this is unfinished transient execution evidence.

## Historical anchor compatibility

<a id="p222-continuation--2026-10-03--presentation-path-qualification"></a>[P222 continuation | 2026-10-03 | Presentation path qualification](RUNBOOK-history-2026-10-03-through-p222-focus.md#p222-continuation--2026-10-03--presentation-path-qualification)
<a id="turn-439--2026-10-02--p222-execution-resumed"></a>[Turn 439 | 2026-10-02 | P222 execution resumed](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-439--2026-10-02--p222-execution-resumed)
<a id="turn-438--2026-10-02--product-delivery-successor"></a>[Turn 438 | 2026-10-02 | Product delivery successor](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-438--2026-10-02--product-delivery-successor)
<a id="turn-437--2026-10-02--installed-browser-selection-and-shared-session-workflow"></a>[Turn 437 | 2026-10-02 | Installed browser selection and shared-session workflow](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-437--2026-10-02--installed-browser-selection-and-shared-session-workflow)
<a id="turn-436--2026-10-02--requested-pre-500k-stop-checkpoint"></a>[Turn 436 | 2026-10-02 | Requested pre-500k stop checkpoint](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-436--2026-10-02--requested-pre-500k-stop-checkpoint)
<a id="turn-435--2026-10-02--installed-view-attempt-exposes-browser-id-collision"></a>[Turn 435 | 2026-10-02 | Installed view attempt exposes browser ID collision](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-435--2026-10-02--installed-view-attempt-exposes-browser-id-collision)
<a id="turn-434--2026-10-02--installed-ordinary-reads-work-viewing-setup-ready"></a>[Turn 434 | 2026-10-02 | Installed ordinary reads work; viewing setup ready](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-434--2026-10-02--installed-ordinary-reads-work-viewing-setup-ready)
<a id="turn-433--2026-10-02--ordinary-reuse-routing-repaired-task-gateway-setup"></a>[Turn 433 | 2026-10-02 | Ordinary reuse routing repaired; task gateway setup](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-433--2026-10-02--ordinary-reuse-routing-repaired-task-gateway-setup)
<a id="turn-432--2026-10-02--installed-navigation-works-client-presentation-incomplete"></a>[Turn 432 | 2026-10-02 | Installed navigation works; client presentation incomplete](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-432--2026-10-02--installed-navigation-works-client-presentation-incomplete)
<a id="turn-431--2026-10-02--actual-client-launches-chrome-navigation-join-repair"></a>[Turn 431 | 2026-10-02 | Actual client launches Chrome; navigation join repair](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-431--2026-10-02--actual-client-launches-chrome-navigation-join-repair)
<a id="turn-430--2026-10-02--desktop-automation-no-longer-depends-on-gateway"></a>[Turn 430 | 2026-10-02 | Desktop automation no longer depends on gateway](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-430--2026-10-02--desktop-automation-no-longer-depends-on-gateway)
<a id="turn-429--2026-10-02--installed-client-advances-to-desktop-observation"></a>[Turn 429 | 2026-10-02 | Installed client advances to desktop observation](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-429--2026-10-02--installed-client-advances-to-desktop-observation)
<a id="turn-428--2026-10-02--pending-acquisition-recovery-implemented"></a>[Turn 428 | 2026-10-02 | Pending acquisition recovery implemented](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-428--2026-10-02--pending-acquisition-recovery-implemented)
<a id="turn-427--2026-10-02--exact-provider-recovery-unblocks-acquisition"></a>[Turn 427 | 2026-10-02 | Exact provider recovery unblocks acquisition](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-427--2026-10-02--exact-provider-recovery-unblocks-acquisition)
<a id="turn-426--2026-10-02--real-client-exposes-placement-and-acquisition-gaps"></a>[Turn 426 | 2026-10-02 | Real client exposes placement and acquisition gaps](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-426--2026-10-02--real-client-exposes-placement-and-acquisition-gaps)
<a id="turn-425--2026-10-02--current-candidate-installed-for-client-workflow"></a>[Turn 425 | 2026-10-02 | Current candidate installed for client workflow](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-425--2026-10-02--current-candidate-installed-for-client-workflow)
<a id="turn-424--2026-10-02--product-delivery-correction"></a>[Turn 424 | 2026-10-02 | Product delivery correction](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-424--2026-10-02--product-delivery-correction)
<a id="turn-423--2026-10-01--simple-product-acceptance-successor"></a>[Turn 423 | 2026-10-01 | Simple product acceptance successor](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-423--2026-10-01--simple-product-acceptance-successor)
<a id="turn-422--2026-10-01--requested-resumption-checkpoint"></a>[Turn 422 | 2026-10-01 | Requested resumption checkpoint](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-422--2026-10-01--requested-resumption-checkpoint)
<a id="turn-421--2026-10-01"></a>[Turn 421 | 2026-10-01](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-421--2026-10-01)
<a id="turn-420--2026-10-01"></a>[Turn 420 | 2026-10-01](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-420--2026-10-01)
<a id="turn-419--2026-10-01"></a>[Turn 419 | 2026-10-01](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-419--2026-10-01)
<a id="turn-418--2026-10-01"></a>[Turn 418 | 2026-10-01](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-418--2026-10-01)
<a id="turn-417--2026-10-01"></a>[Turn 417 | 2026-10-01](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-417--2026-10-01)
<a id="turn-416--2026-10-01"></a>[Turn 416 | 2026-10-01](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-416--2026-10-01)
<a id="turn-415--2026-10-01"></a>[Turn 415 | 2026-10-01](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-415--2026-10-01)
<a id="turn-414--2026-10-01"></a>[Turn 414 | 2026-10-01](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-414--2026-10-01)
<a id="turn-413--2026-10-01"></a>[Turn 413 | 2026-10-01](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-413--2026-10-01)
<a id="turn-412--2026-10-01"></a>[Turn 412 | 2026-10-01](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-412--2026-10-01)
<a id="turn-411--2026-10-01"></a>[Turn 411 | 2026-10-01](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-411--2026-10-01)
<a id="turn-410--2026-10-01"></a>[Turn 410 | 2026-10-01](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-410--2026-10-01)
<a id="turn-409--2026-10-01"></a>[Turn 409 | 2026-10-01](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-409--2026-10-01)
<a id="turn-408--2026-10-01"></a>[Turn 408 | 2026-10-01](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-408--2026-10-01)
<a id="turn-407--2026-10-01"></a>[Turn 407 | 2026-10-01](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-407--2026-10-01)
<a id="turn-406--2026-09-29"></a>[Turn 406 | 2026-09-29](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-406--2026-09-29)
<a id="turn-405--2026-09-29"></a>[Turn 405 | 2026-09-29](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-405--2026-09-29)
<a id="turn-404--2026-09-29"></a>[Turn 404 | 2026-09-29](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-404--2026-09-29)
<a id="turn-403--2026-09-29"></a>[Turn 403 | 2026-09-29](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-403--2026-09-29)
<a id="turn-402--2026-09-29"></a>[Turn 402 | 2026-09-29](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-402--2026-09-29)
<a id="turn-401--2026-09-29"></a>[Turn 401 | 2026-09-29](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-401--2026-09-29)
<a id="turn-400--2026-09-29"></a>[Turn 400 | 2026-09-29](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-400--2026-09-29)
<a id="turn-399--2026-09-29"></a>[Turn 399 | 2026-09-29](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-399--2026-09-29)
<a id="turn-398--2026-09-29"></a>[Turn 398 | 2026-09-29](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-398--2026-09-29)
<a id="turn-397--2026-09-29"></a>[Turn 397 | 2026-09-29](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-397--2026-09-29)
<a id="turn-396--2026-09-29"></a>[Turn 396 | 2026-09-29](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-396--2026-09-29)
<a id="turn-395--2026-09-29"></a>[Turn 395 | 2026-09-29](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-395--2026-09-29)
<a id="turn-394--2026-09-28"></a>[Turn 394 | 2026-09-28](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-394--2026-09-28)
<a id="turn-393--2026-09-28"></a>[Turn 393 | 2026-09-28](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-393--2026-09-28)
<a id="turn-392--2026-09-28"></a>[Turn 392 | 2026-09-28](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-392--2026-09-28)
<a id="turn-391--2026-09-28"></a>[Turn 391 | 2026-09-28](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-391--2026-09-28)
<a id="turn-390--2026-09-28"></a>[Turn 390 | 2026-09-28](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-390--2026-09-28)
<a id="turn-389--2026-09-28"></a>[Turn 389 | 2026-09-28](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-389--2026-09-28)
<a id="turn-388--2026-09-28"></a>[Turn 388 | 2026-09-28](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-388--2026-09-28)
<a id="turn-387--2026-09-17"></a>[Turn 387 | 2026-09-17](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-387--2026-09-17)
<a id="turn-386--2026-09-17"></a>[Turn 386 | 2026-09-17](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-386--2026-09-17)
<a id="turn-385--2026-09-17"></a>[Turn 385 | 2026-09-17](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-385--2026-09-17)
<a id="turn-384--2026-09-17"></a>[Turn 384 | 2026-09-17](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-384--2026-09-17)
<a id="turn-383--2026-09-17"></a>[Turn 383 | 2026-09-17](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-383--2026-09-17)
<a id="turn-382--2026-09-17"></a>[Turn 382 | 2026-09-17](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-382--2026-09-17)
<a id="turn-381--2026-09-17"></a>[Turn 381 | 2026-09-17](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-381--2026-09-17)
<a id="turn-380--2026-09-17"></a>[Turn 380 | 2026-09-17](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-380--2026-09-17)
<a id="turn-380--2026-09-17-1"></a>[Turn 380 | 2026-09-17](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-380--2026-09-17-1)
<a id="turn-379--2026-09-17"></a>[Turn 379 | 2026-09-17](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-379--2026-09-17)
<a id="turn-378--2026-09-17"></a>[Turn 378 | 2026-09-17](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-378--2026-09-17)
<a id="turn-377--2026-09-17"></a>[Turn 377 | 2026-09-17](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-377--2026-09-17)
<a id="turn-376--2026-09-17"></a>[Turn 376 | 2026-09-17](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-376--2026-09-17)
<a id="turn-375--2026-09-17"></a>[Turn 375 | 2026-09-17](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-375--2026-09-17)
<a id="turn-374--2026-09-17"></a>[Turn 374 | 2026-09-17](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-374--2026-09-17)
<a id="turn-373--2026-09-17"></a>[Turn 373 | 2026-09-17](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-373--2026-09-17)
<a id="turn-376--2026-09-17-1"></a>[Turn 376 | 2026-09-17](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-376--2026-09-17-1)
<a id="turn-375--2026-09-17-1"></a>[Turn 375 | 2026-09-17](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-375--2026-09-17-1)
<a id="turn-373--2026-09-17-1"></a>[Turn 373 | 2026-09-17](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-373--2026-09-17-1)
<a id="turn-374--2026-09-17-1"></a>[Turn 374 | 2026-09-17](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-374--2026-09-17-1)
<a id="turn-373--2026-09-17-2"></a>[Turn 373 | 2026-09-17](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-373--2026-09-17-2)
<a id="turn-372--2026-09-16"></a>[Turn 372 | 2026-09-16](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-372--2026-09-16)
<a id="turn-371--2026-09-16"></a>[Turn 371 | 2026-09-16](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-371--2026-09-16)
<a id="turn-370--2026-09-16"></a>[Turn 370 | 2026-09-16](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-370--2026-09-16)
<a id="turn-369--2026-09-16"></a>[Turn 369 | 2026-09-16](RUNBOOK-history-2026-10-03-through-p222-focus.md#turn-369--2026-09-16)
