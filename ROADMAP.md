# Roadmap

Updated: 2026-10-17

## Current direction

[The product contract](docs/dev/policies/0054-browser-product-contract.md) owns the value proposition: complete browser work, hand it to a person, and continue afterward with less effort and fewer failures.

Use an existing client profile, a managed persistent profile, or a disposable profile as the task requires. Remote View owns desktop operation and the native viewer; Agent Browser owns browser operation and their integration. The operator handoff opens the native desktop directly through existing Remote View authentication, including production Authelia. Prefer an Agent Browser-owned desktop chooser.

Prioritize less plumbing, useful continuity, seamless human assistance, predictable operation and clear results. Each substantial change names its concrete workflow improvement and validates it. Current implementation gaps belong in RUNBOOK rather than becoming permanent product rituals.

## Current work and boundaries

[P245](docs/dev/plans/0245-2026-10-16-hosted-auth-consumer.md) qualifies the bounded existing-host QBO then BILL account/company journeys for #290. [RUNBOOK](RUNBOOK.md) and the [nine-item evidence map](docs/dev/receipts/p245/issue-290-acceptance.json) preserve source/install/native boundaries and the final publication gate. This does not complete every authentication, password-manager, MFA or API workflow.

[P237](docs/dev/plans/0237-2026-10-11-native-dashboard-handoffs.md) completed the approved 30-story native desktop dashboard and independent handoff lifetime outcome for #261. The [installed acceptance map](docs/dev/notes/0237-2026-10-16-installed-acceptance.md) binds dedicated development/public-native workflows to exact source and retained checks. PR293 owns the final repairs; Remote View PR441/442 and Cooper PR13 supply canonical prerequisites. Production, formal release and wider #260/#105/#68 work remain separate.

[P238](docs/dev/plans/0238-2026-10-11-workflow-mcp.md) delivers the small workflow MCP, task-appropriate profiles, native handoff/continuation and supported Codex client guidance at the isolated installed seam. RUNBOOK and PR267 bind current qualification; production promotion remains separate.

[P236](docs/dev/plans/0236-2026-10-11-incremental-upgrade-progress.md) owns the bounded incremental installer observation repair for #181, including its reviewed candidate entry point. Source qualification remains separate from production-shaped installation.

[P227](docs/dev/plans/0227-2026-10-08-alice-bob-reconciliation.md) completed the bounded shared-profile, retention and desktop-capacity outcome: intentional profile sharing with independent task tabs, configured durable/disposable retention, one ready spare, wrap-around overflow and safe shrink after the configured cooldown. Controlled-clock and isolated installed workflows, cleanup and PR 234 qualify this bounded outcome. RUNBOOK preserves failures and limits; production promotion remains separate.

[P223 version 6](docs/dev/plans/0223-2026-10-04-finish-agent-browser-acceptance.md) completed its bounded browser-workflow outcome on 2026-10-06. Consumer source entered canonical main through PR 212, and the provider dependency through Remote View PR 310. RUNBOOK owns the retained acceptance evidence and limitations. Production promotion, formal release and broader qualification remain separate work; this closeout adds no execution authority for them.

The four-profile governance MCP policy pilot landed through PR #206. Use its compact cited packets and freshness checks instead of bulk policy ingestion.

## Historical material

[Legacy planning archive](docs/history/2026-10-05-legacy-planning/README.md) preserves earlier roadmaps, runbooks, plans, identifiers, and evidence. It is reference-only and does not define current direction. Archival does not close unrelated issues or PRs. Do not reinstate old requirements from an archived OPEN state.
