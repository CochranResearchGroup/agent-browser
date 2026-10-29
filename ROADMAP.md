# Roadmap

[#449 standalone Auth integration](docs/dev/contracts/standalone-auth-integration.v1.md), PL-AUTH: migrate browser authentication helpers from deprecated Bots wiring to standalone Auth Receipts. The core specification consumes P279 source and current provider contracts; implementation and installed acceptance remain open.

[P279 governed authentication journeys](docs/dev/plans/0279-2026-10-24-auth-receipts-journeys.md), Auth #81/#79 under PL-AUTH: preserve verified provider steps while requiring the installed credential authority and deterministic MFA delivery. Source guidance only.

[P270 hosted continuity](docs/dev/plans/0270-2026-10-22-hosted-continuity.md) retains bounded Native14 warm/function evidence and P277 actual restart/reopen proof. #355 remains open for the unknown Native8 launch outcome and same-pair combined coordination/cold evidence. [P278 diagnosis](docs/dev/plans/0278-2026-10-23-reopen-launch-diagnosis355.md) owns the bounded causal probe and secret-free phase diagnostics. Native578 relocation remains explicitly deferred; no production effects.

[P268 attachment](docs/dev/plans/0268-2026-10-22-hosted-attachment.md) and [P267/#350 takeover](docs/dev/plans/0267-2026-10-22-hosted-takeover.md) close as integrated source outcomes. [Acceptance](docs/dev/notes/0268-2026-10-22-hosted-attachment-control.md) records actual Native04 workflow and corrected cleanup; Native05 cookie continuity and independent cleanup also pass. No deployment; wider #347 remains open.


[P266](docs/dev/plans/0266-2026-10-22-public-origin-env.md), #358, repairs standard MCP service-origin bootstrap under PL-BUGFIX; [RUNBOOK](RUNBOOK.md) owns current validation and integration evidence.

[P265 hosted launch](docs/dev/plans/0265-2026-10-22-hosted-launch.md), #348 under #347, is source-qualified with actual Native viewer pixels and standalone continuity. #349/#350 are integrated source outcomes; no production publication.

[P264 missing browser selection](docs/dev/plans/0264-2026-10-21-missing-browser-selection.md), #344 under #68, keeps missing explicit browser evidence separate from peer workspaces.

[P263 installed download acceptance](docs/dev/plans/0263-2026-10-21-installed-download-acceptance.md), #284, qualifies original-file retrieval on installed 9c8edd62a16f. [Acceptance](docs/dev/notes/0263-2026-10-21-installed-download-acceptance.md) records three public MCP fixture passes with exact process continuity and zero residue; no new deployment or private-account run.

[P261 workspace export URLs](docs/dev/plans/0261-2026-10-21-workspace-export-urls.md), #341 under #68, qualifies URL redaction for diagnostic/chat exports. [Source acceptance](docs/dev/notes/0261-2026-10-21-workspace-export-urls.md) records real red-green and unchanged runtime values; production is unchanged.

[P260 selected-tab evidence](docs/dev/plans/0260-2026-10-21-selected-tab-evidence.md), #339 under #68, qualifies exact selected-tab page evidence. [Source acceptance](docs/dev/notes/0260-2026-10-21-selected-tab-evidence.md) records the reproduced wrong-page projection and passing repair gates; production is unchanged.

[P259 handoff retention reconciliation](docs/dev/plans/0259-2026-10-20-handoff-retention-reconciliation.md), #260, qualifies existing independent link/tab lifecycle behavior. [Acceptance map](docs/dev/notes/0259-2026-10-20-handoff-retention-reconciliation.md) records long-duration owner clocks and current regressions; source only.

[P258 bounded interlock logging](docs/dev/plans/0258-2026-10-20-interlock-logging.md), #294, qualifies bounded routine logs and preserved explicit diagnostics. [Source acceptance](docs/dev/notes/0258-2026-10-20-interlock-logging.md) records red/green and byte counts; the installed interlock is unchanged.

[P257 retirement lock diagnostics](docs/dev/plans/0257-2026-10-20-retirement-lock.md), #307, is deployed as sealed `9c8edd62a16f`. [Installed acceptance](docs/dev/notes/0257-2026-10-20-retirement-lock-installed.md) records the qualified dashboard checkpoint, retained browser/profile identities and continuity limits.

[P256 absent-browser workflow recovery](docs/dev/plans/0256-2026-10-20-absent-browser-workflow.md), #327, qualifies an executable preserve-mode recovery path for browserless handoff preparation and retains uncertain handoff evidence. [Source acceptance](docs/dev/notes/0256-2026-10-20-absent-browser-workflow.md) records focused and broad checks plus the interrupted installer fixture. Production publication remains separate.

[P254 compact workspace fixture](docs/dev/plans/0254-2026-10-20-compact-workspace-fixture.md), #330, restores the full mounted gate to current Properties and combined Native selection. [Acceptance](docs/dev/notes/0254-2026-10-20-compact-workspace-fixture.md) retains all four late-response cases, privacy/mobile/peer checks and zero fixture residue. Test maintenance only; production is unchanged.

P253 [Native viewer failures](docs/dev/plans/0253-2026-10-20-native-viewer-errors.md), #329, qualifies distinct preparation, access and page failure guidance plus typed client refusal preservation. [Source acceptance](docs/dev/notes/0253-2026-10-20-native-viewer-errors.md) separates passing mounted/client/build gates, the old #330 fixture failure and pending production publication.

P252 [compact workflow skill](docs/dev/plans/0252-2026-10-20-workflow-skill-budget.md), #325, restores the unchanged parity gate with a 9,163-byte entry point, preserved references and verified development guidance publication. [Acceptance](docs/dev/notes/0252-2026-10-20-workflow-skill-budget.md) records the documentation-only boundary.

P251 [Properties and profile durability](docs/dev/plans/0251-2026-10-19-dashboard-profile-properties.md), #323, is installed after PR324. Operators can inspect profile retention, rename profiles and promote throwaway profiles in Properties. [Installed acceptance](docs/dev/notes/0251-2026-10-19-dashboard-profile-properties-installed.md) records qualification and limits.

P250 [dashboard subsystem health](docs/dev/plans/0250-2026-10-19-dashboard-subsystem-health.md), #317, is integrated through PR318 and installed as sealed `81eea74b6d6d` after development qualification. The [installed receipt](docs/dev/notes/0250-2026-10-19-dashboard-subsystem-health-installed.md) binds actual public drawer checks, both doctors, three browser cycles and preserved identities. The drawer exposes remaining Native warnings; a pre-existing Docker/WSL regression is mitigated by a verifier-only bridge.


Updated: 2026-10-19

## Current direction

[The product contract](docs/dev/policies/0054-browser-product-contract.md) owns the value proposition: complete browser work, hand it to a person, and continue afterward with less effort and fewer failures.

Use an existing client profile, a managed persistent profile, or a disposable profile as the task requires. Remote View owns desktop operation and the native viewer; Agent Browser owns browser operation and their integration. The operator handoff opens the native desktop directly through existing Remote View authentication, including production Authelia. Prefer an Agent Browser-owned desktop chooser.

Prioritize less plumbing, useful continuity, seamless human assistance, predictable operation and clear results. Each substantial change names its concrete workflow improvement and validates it. Current implementation gaps belong in RUNBOOK rather than becoming permanent product rituals.

## Current work and boundaries

[P249](docs/dev/plans/0249-2026-10-19-stale-session-host.md), #320, owns the bounded stale session host recovery defect; source qualification passes 128 Rust tests with 1 explicit real-viewer fixture ignored; PR321 is integrated and included in the P250 installed candidate; receiving-account acceptance remains separate.

[P248](docs/dev/plans/0248-2026-10-18-dashboard-cold-open.md) completes the bounded selected-desktop cold-open repair under #312, including the Native-owned lazy audio dependency. Production first/repeat Native pixels take 4503/3083 ms without duplicate handoffs. RUNBOOK and the acceptance note preserve exact installed identities and remaining runtime limits.

[P245](docs/dev/plans/0245-2026-10-16-hosted-auth-consumer.md) qualifies the bounded existing-host QBO then BILL account/company journeys for #290. [RUNBOOK](RUNBOOK.md) and the [nine-item evidence map](docs/dev/receipts/p245/issue-290-acceptance.json) preserve source/install/native boundaries and accepted canonical delivery through PR297. This does not complete every authentication, password-manager, MFA or API workflow.

[P237](docs/dev/plans/0237-2026-10-11-native-dashboard-handoffs.md) completed the approved 30-story native desktop dashboard and independent handoff lifetime outcome for #261. The [installed acceptance map](docs/dev/notes/0237-2026-10-16-installed-acceptance.md) binds dedicated development/public-native workflows to exact source and retained checks. PR293 owns the final repairs; Remote View PR441/442 and Cooper PR13 supply canonical prerequisites. Production, formal release and wider #260/#105/#68 work remain separate.

[P238](docs/dev/plans/0238-2026-10-11-workflow-mcp.md) delivers the small workflow MCP, task-appropriate profiles, native handoff/continuation and supported Codex client guidance at the isolated installed seam. RUNBOOK and PR267 bind current qualification; production promotion remains separate.

[P236](docs/dev/plans/0236-2026-10-11-incremental-upgrade-progress.md) owns the bounded incremental installer observation repair for #181, including its reviewed candidate entry point. Source qualification remains separate from production-shaped installation.

[P227](docs/dev/plans/0227-2026-10-08-alice-bob-reconciliation.md) completed the bounded shared-profile, retention and desktop-capacity outcome: intentional profile sharing with independent task tabs, configured durable/disposable retention, one ready spare, wrap-around overflow and safe shrink after the configured cooldown. Controlled-clock and isolated installed workflows, cleanup and PR 234 qualify this bounded outcome. RUNBOOK preserves failures and limits; production promotion remains separate.

[P223 version 6](docs/dev/plans/0223-2026-10-04-finish-agent-browser-acceptance.md) completed its bounded browser-workflow outcome on 2026-10-06. Consumer source entered canonical main through PR 212, and the provider dependency through Remote View PR 310. RUNBOOK owns the retained acceptance evidence and limitations. Production promotion, formal release and broader qualification remain separate work; this closeout adds no execution authority for them.

The four-profile governance MCP policy pilot landed through PR #206. Use its compact cited packets and freshness checks instead of bulk policy ingestion.

[P247](docs/dev/plans/0247-2026-10-17-planning-completeness.md) repairs selected planning discoverability checks under #299.

## Historical material

[Legacy planning archive](docs/history/2026-10-05-legacy-planning/README.md) preserves earlier roadmaps, runbooks, plans, identifiers, and evidence. It is reference-only and does not define current direction. Archival does not close unrelated issues or PRs. Do not reinstate old requirements from an archived OPEN state.
