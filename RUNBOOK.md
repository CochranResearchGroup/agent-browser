# Runbook

This is the sole current execution status. Plans own acceptance and strategy;
linked archives preserve historical evidence. Keep this file at or below 200
lines under [policy 0043](docs/dev/policies/0043-roadmap-runbook-governance.md).

## Turn 214 | 2026-09-07

Current task: execute the Plan 0160 consumer-first strategy amendment.
The operator authorized amendment and execution after the reassessment.
Additional round starts 2026-09-07T14:04:08+00:00 and ends
2026-09-07T14:34:08+00:00 (30 minutes total, including governance).
Controller: primary agent. The operator subsequently directed isolated
simulation instead of interrupting the consumer. Existing approximately
12-hour effort and the current round deadline are retained.
Prior turn classification: progress, because evidence established that the
named-profile mismatch was no longer the recorded blocker and selected the
consumer's supported rejoin path as the next investigation.

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
- Current state: isolated missing-binding recovery passed on the exact installed
  binary. The original test credential rejoined through the public CLI; the
  lease became active with no blocking identity axes. Both original handles
  remained usable, PID/start token and target set were preserved, and final
  attestation was complete. No product repair or rebuild was required.
- Scope limit: this fixture naturally produced missing principal binding on a
  fresh managed launch. Attestation was already complete before rejoin through
  managed-launch custody. It does not reproduce the old consumer's historical
  missing `profile_lease` and `handoff_receipt` proof combination or establish
  consumer acceptance. A lease warning alone is not proof of unusable control.
- Evidence: private P160 campaign `missing-binding-sim-ThA0Um/`, containing
  `probe-source.mjs`, `ledger.jsonl`, `acceptance.json`, and `cleanup.json`.
  Cleanup found zero remaining owned fixture processes.
- Earlier harness setup failures are retained in `missing-binding-sim-IDfJUT`
  and `missing-binding-sim-IvUQPa`. The latter fixture's three remaining owned
  processes were terminated with PID/start/executable checks after correcting
  the cleanup interpreter; final residue was zero. No failed rejoin was retried.
- The simulation removes the assumed need to interrupt the original consumer
  for mechanism testing. The pending consumer request is no longer a dependency
  for investigation. Production remains untouched by these experiments.
- Next diagnosis: reproduce the older retained-owner custody condition, including
  its handoff/launch evidence, before attributing its historical attestation
  failure to principal binding. Do not repeat this passing clean-launch case
  or resume costly operator-view retries as a substitute.
- Progress classification: `blocker_reduction`; isolated rejoin is proven,
  original-consumer and complete A1 acceptance remain unproven.

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
