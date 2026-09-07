# Runbook

This is the sole current execution status. Plans own acceptance and strategy;
linked archives preserve historical evidence. Keep this file at or below 200
lines under [policy 0043](docs/dev/policies/0043-roadmap-runbook-governance.md).

## Turn 214 | 2026-09-07

Current task: finish the AX logging batch and diagnose the A2 cold-view failure.
The operator explicitly authorized review of the approach and up to 60 additional
minutes. This round starts 2026-09-07T14:49:48+00:00 and ends
2026-09-07T15:49:48+00:00. Prior approximately 12-hour effort and the
completed 30-minute round (14:04:08–14:34:08 UTC) remain cumulative history.
Controller: primary agent. First finish pending integration checks; then isolate
which cold-view layer stalls using a local reproducer. Batch a causal repair and
its logging before publication. Do not turn a warm pass or logging-only fix into
operator acceptance, or interrupt the original consumer for diagnosis.
Reassess after two checkpoints or 30 active minutes without acceptance progress;
no tactic or successor resets the overall deadline. No automatic live retry.
Approach review: consumer-only diagnosis was an unnecessary dependency; the
initial fresh-launch simulation was incomplete; passing intermediate checks did
not justify stopping the broader task. The closer retained-handle simulation
now covers that historical condition, while A2's cause remains unproven.

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
- Follow-up historical-case simulation also passed after a host interruption:
  the same retained browser accepted the original handle with stale `shared`
  lease metadata while the current session was `exclusive`, and proved managed
  launch custody with no handoff receipt. The original test credential then
  rejoined successfully; both handles and all process/target identities survived.
- The recorded consumer failure predates these attestation repairs. Current
  read-only owner evidence shows no pending transfer or transfer history, so an
  older generation number alone does not imply missing transfer custody.
  This simulation covers the reported stale-handle and absent-handoff conditions;
  it does not claim a fresh attestation from the production consumer.
- Evidence: private P160 campaigns `missing-binding-sim-ThA0Um/` (initial case)
  and `missing-binding-sim-aAMdDU/` (retained historical case), each containing
  `probe-source.mjs`, `ledger.jsonl`, `acceptance.json`, and `cleanup.json`.
  Both completed with zero remaining owned fixture processes.
- Earlier harness setup failures are retained in `missing-binding-sim-IDfJUT`
  and `missing-binding-sim-IvUQPa`. The latter fixture's three remaining owned
  processes were terminated with PID/start/executable checks after correcting
  the cleanup interpreter; final residue was zero. No failed rejoin was retried.
- The simulation removes the assumed need to interrupt the original consumer
  for mechanism testing. The pending consumer request is no longer a dependency
  for investigation. Production remains untouched by these experiments.
- Next active issue: A2 cold operator view. Recorded tab read took approximately
  28 seconds and returned HTTP 504; primary-claim response was still absent at
  the viewer cutoff. A later warm diagnostic passed. Source inspection shows
  session-tabs forwards backend responses or emits 502 on fallback failure,
  so the observed 504 alone does not locate the failing layer.
- AX gap: the retained client failure has an observation ID but no backend port,
  elapsed time, or joined primary-owner terminal event. No server causal failure
  for that cold request appears in the retained failure journal. Do not claim
  session-tabs caused the missing iframe or rerun an unchanged viewer test.
  The next bounded repair must join request timing and gateway/primary evidence
  or reproduce the cold condition locally before changing runtime behavior.
- AX repair batch started: dashboard fetch instrumentation now fills the existing
  `elapsedMs` field when fetch settles, including failures delivered after later
  recovery. The regression failed before the fix and passed afterward; it proves
  queued delivery delay is excluded and existing privacy/no-retry checks pass.
  `pnpm test:service-failure-journal`, dashboard build/TypeScript, Rust format
  and workspace Clippy passed for that batch. It is not installed.
- Cold-frame diagnosis reproduced 11,818 ms startup from an immediately ready
  in-memory provider: 128 blob acknowledgements caused 154 full authority reads
  against a private copy of current Service State. Each optimized check took
  66–84 ms. This identifies a startup delay mechanism, not the historical 504
  source or the exact production provider message pattern.
- The repair batches acknowledgements with fresh authority before each write,
  moves synchronous admission and guard reads off async workers, and prevents
  overdue periodic checks from accumulating. The same optimized simulation
  passed in 295 ms with five checks and all 128 acknowledgements retained.
  The scheduler regression failed before the repair and passed afterward.
  Seventeen focused Rust tests and the dashboard sharing contract passed,
  including exact owner-change rejection and retained-owner/cancellation tests.
  Batch-wide format/Clippy and optimized candidate build passed.
  The isolated authenticated dashboard endpoint passed with two concurrent
  requests sharing one provider connection, all 128 acknowledgements delivered,
  and one returned terminal occurrence joined to exactly one owner failure
  record. The failed owner remained sticky; no second provider connection
  started. Backend and mock provider cleanup completed. The fixture uses the
  existing backend-only mode and its own generated dashboard credential.
  Evidence: P160 `primary-endpoint-sim-ZwpewN/`. Earlier fixture setup failures
  occurred before viewer requests; `primary-endpoint-sim-nHoqlD/` retains the
  missing-host bootstrap failure and completed cleanup.
- At the 30-minute reassessment, the reproducible startup bottleneck is removed
  and the real endpoint contract passes in isolation. Production A2 is still
  open. The remaining allowance prepares an exact production candidate and
  checks admission for publication while preserving active work. No original
  consumer credential or live profile was used by the endpoint fixture.
  Temporary measurement tests and private copied state are kept outside Git;
  evidence is in P160 `primary-guard-measurement/`. Production is unchanged.
- The preceding 30-minute round ended with the AX batch incomplete. The new
  operator-authorized round above resumes its required checks and A2 diagnosis.
- Progress classification: `outcome_progress` for isolated retained-handle and
  custody acceptance within A1; full A1, A2, A3, A4 and AX remain open.

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
