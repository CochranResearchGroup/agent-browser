# P298: retirement progress and truthful tab accounting

Product lane: PL-BUGFIX
Work item: #498, bounded source repair under #488.
Owner: ecochran76. Branch: fix/ttl-reconciliation.
Baseline: c20cbb24e79e8d9b0de56d2c03e7d27a5e1981ea.

## Outcome and scope

Allow current idle-browser retirement without stale historical desktop peers
blocking qualification. Preserve original-process identity, uncertain outcomes,
active peer work and assignment boundaries. Mark orphan legacy tabs non-live
without physical effects. Retain bounded background reaper outcomes outside
stderr. No TTL-default changes, production effects or deployment.

## Stable seams and acceptance

1. Public RemoteViewSessionEffects retirement admission: historical idle-closed
   peers without identity and positively ended peers from another assignment do
   not block the current assignment. Live or uncertain peers remain protected.
   This proves provider-neutral retirement selection, not actual browser closure.
2. Service reconciliation: Ready tabs whose owner browser is missing become
   Closed while their records remain. Existing owned live targets remain Ready.
   This proves record truthfulness, not cleanup of physical targets.
3. Background maintenance receipt: success and failure survive discarded stderr,
   include observation time and bounded per-browser deferrals, and cannot change
   cleanup authority. Synthetic temporary storage proves persistence/readback;
   production sampling and installation are separate.

Use one failing test and minimal fix per slice. Record exact local commands,
results and source identity. Run Rust format, workspace clippy and affected
compartments, plus docs checks for user-visible diagnostic documentation. Hosted
CI is excluded under operator direction; local Linux/WSL does not prove other
platforms. Review Standards and Spec independently against the baseline.

## Custody and overlap

Reuse the clean canonical checkout serially; no additional worktree. Preserve
fix/auth-retention-20261030 (#497) and its dirty files. Primary writes retirement
effects, custody tests, service reconciliation, daemon receipt/reporting, and
associated docs only. Only forward the host reaper result in its existing reap functions; preserve
its retention-command host hunks, session manager and MCP changes.
Canonical source integration uses a linked PR after local validation.

## Completion

Source outcome complete. PR #499 carries integration and closes #498. The
[acceptance receipt](../notes/0298-2026-10-30-ttl-reconciliation.md) records
red/green failures, local passing gates, composite consumer evidence, the
baseline planning-audit failure, one Standards/Spec review and exact fixture
cleanup. #488 installed acceptance remains separate and open.
