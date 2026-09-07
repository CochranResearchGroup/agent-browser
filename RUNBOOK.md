# Runbook

This is the sole current execution status. Plans own acceptance and strategy;
linked archives preserve historical evidence. Keep this file at or below 200
lines under [policy 0043](docs/dev/policies/0043-roadmap-runbook-governance.md).

## Turn 214 | 2026-09-07

Current task: adopt LitScout outcome-first policy and runbook compaction.
Scope is repository documentation; this task does not resume browser tests.
Policy adoption is independent of production runtime acceptance.

### Plan 0160: OPEN, production operational acceptance incomplete

- Authority and acceptance: [Plan 0160](docs/dev/plans/0160-2026-09-06-production-profile-identity-and-operational-readiness.md),
  lane P157. Profile ownership and identity proof remain the highest priority.
- Last recorded installed candidate source: `0cf07a43`, generation
  `0.28.0-86d7b29c81fe-5f4b8dee3613`. This is recorded evidence, not a fresh
  runtime census during this documentation task.
- Original consumer identity/rejoin acceptance remains unproven. The first
  installed cold operator-view attempt failed with session-tabs HTTP 504 and
  no iframe. A later warm, no-input diagnostic displayed the iframe; it does
  not erase the cold failure or establish full operator acceptance.
- Evidence remains in the operator-owned campaign directory
  `~/.local/state/agent-browser/campaigns/p160/publication-0cf07a43/`:
  `activation-receipt.json`, `viewer-first-attempt.log`,
  `local-viewers-qGlp5N/events.json`, and `diagnostic-viewer.log`.
  These are private local evidence, not published durable acceptance receipts.
- Cumulative effort: the operator reports approximately 12 hours in the current
  repair/verification run. Exact aggregate accounting was not established.
  This adoption does not reset effort or manufacture a new remaining budget.
- Progress classification for production during this policy task: `no_progress`.
- Next runtime decision: reconcile cumulative effort and establish a finite
  remaining ceiling in this entry before sustained work; identify an
  evidence-backed route to a named unmet acceptance milestone before another
  costly replay. Passing local regressions alone is insufficient.

### Retained execution boundaries

- Production is not declared fully operational; all five acceptance rows remain
  subject to Plan 0160. Do not infer acceptance from this policy closeout.
- Do not replay completed staging or activation to collect another receipt.
- Preserve browser ownership and original consumer identity. Do not borrow
  foreign capabilities or treat synthetic diagnostics as consumer acceptance.
- Maintenance timer restoration and unattended-cycle acceptance remain pending.
- Plan 0158 external-vantage work retains its explicit dispatch and no-retry
  restrictions. Policy adoption does not authorize an external attempt.
- Historical stops remain effective for their scoped lanes unless a later
  approved plan supersedes them. Consult the relevant entry before resuming an
  older lane; no archived turn is an instruction to restart work.

## History index

- [Complete previous runbook, through Turn 213](RUNBOOK-history-through-2026-09-02.md)
  is preserved byte-for-byte beside this file, retaining relative-link behavior.
- [Plan 0160 historical checkpoints](docs/dev/plans/0160-2026-09-06-production-profile-identity-and-operational-readiness.md#planning-checkpoint)
  remain in place; future current status belongs here.
- [Policy adoption and validation](docs/dev/notes/2026-09-07-outcome-first-policy-adoption.md).

Read only the historical entries relevant to the task, not the whole archive
at startup. Historical plan and runbook claims do not override this current
acceptance status.
