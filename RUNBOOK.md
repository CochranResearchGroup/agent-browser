# Runbook

This is the sole current execution status. Plans own acceptance and strategy;
linked archives preserve historical evidence. Keep this file at or below 200
lines under [policy 0043](docs/dev/policies/0043-roadmap-runbook-governance.md).

## Turn 214 | 2026-09-07

Current task: execute the Plan 0160 consumer-first strategy amendment.
The operator authorized amendment and execution after the reassessment.
Additional round starts 2026-09-07T14:04:08+00:00 and ends
2026-09-07T14:34:08+00:00 (30 minutes total, including governance).
Controller: primary agent. Milestone: original consumer recovery or one
reproduced causal defect. Existing approximately 12-hour effort is retained.
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
- Current state: read-only installed diagnosis completed; original-consumer
  execution and attestation are pending. Fresh lease explanation still offers
  `rejoin_owned_browser` for missing owner/principal binding. The existing
  consumer capability is active; rotation is forbidden by active work.
- Evidence: `consumer-first-20260907/lease-explain.json` and
  `capability-status.json` under the same private P160 campaign root.
  Installed generation matches the recorded candidate above.
- Backend source uses the same exact-owner eligibility check when advertising
  and executing rejoin. No new backend defect has been reproduced in this round.
- Next action: original consumer executes its supported rejoin with its existing
  private capability, then obtains attestation through its original handle.
  A prepared `consumer-first-20260907/consumer-rejoin-once.py` refreshes the
  revision, checks recorded identity, submits once, and checks custody readback.
  Syntax checked; not executed here. It does not supply consumer attestation.
- This repository session has requested that consumer-side result from the
  operator. It has not borrowed the capability, changed its binding, submitted
  page input, or started a replacement browser. No runtime retry is scheduled.
- Production progress classification: `no_progress`; the verified diagnosis
  narrows the next action but does not advance A1 acceptance or reset the clock.

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
