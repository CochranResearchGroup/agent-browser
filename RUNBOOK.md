# Runbook

This is the sole current execution status. Plans own acceptance and strategy;
Git checkpoints and linked archives preserve history. Keep this file at or below
200 lines under [policy 0043](docs/dev/policies/0043-roadmap-runbook-governance.md).

## Turn 215 | 2026-09-07

Authority: [Plan 0160](docs/dev/plans/0160-2026-09-06-production-profile-identity-and-operational-readiness.md),
lane P157, branch `plan/profile-permissions-and-request-provenance`.
The identity-first batch runs from 2026-09-07T15:58:19Z through 16:58:19Z.
Controller: primary. Prior approximately 12-hour effort, the completed
30-minute round and the preceding 60-minute round remain cumulative.
The preceding recommendation-only turn made no outcome progress; this turn
amended the strategy and executed a causal identity repair. Full readiness
remains OPEN. No consumer capability was borrowed or production consumer lease
reassigned. Unrelated note 0156 and untracked note 0159 remain preserved.

### Installed repair and current acceptance

- Installed source: `dc570e5b4f70722e56cb3ec14b62420ef4baefc2`.
  Selected generation: `0.28.0-73428f5db89f-de1a24242899`.
  Binary SHA256: `73428f5db89f5331d5cf340a38f97a0281cbfb9bdec2650b43f2cd7fcd0aae28`.
  Support SHA256: `de1a24242899b4dff891f7277019ef600d749c4bbd1aa9b2bf04c2c6ff0542e3`.
- A1 advanced and remains OPEN. The installed predecessor reproduced
  `no_safe_reconciliation_transition` despite the lease authorizing rejoin.
  The planner now seals one `rejoin_owned_browser` transition and apply reuses
  the direct operation's unique, uncontested owner/session/tab checks.
  The exact release passed the same retained-consumer simulation: original
  capability and both handles survived host interruption; ordinary tab control
  worked before repair; plan/apply restored an active lease and complete
  attestation without replacing browser or targets. Its returned receipt joined
  to the persisted plan ID and lease ID. Foreign changed custody was denied
  without mutation in the focused regression; replay was idempotent.
- Six exact-release cases passed, including the new reconciliation case,
  cold owner, headed/headless retained reopen and headed/headless session-only
  cases. Every fixture cleanup passed. Activation admitted with no active jobs
  and preserved five exact live browser process identities and 70 tab-custody
  records. Authenticated manifest and retained synthetic-handle readback passed
  after activation; mouse/keyboard counters remained 9/9.
- A2 is OPEN on this candidate. Its new generation invalidated the prior bound
  presentation receipt, so one bounded local synthetic journey was run.
  Both authenticated viewers connected, anonymous access was denied, synthetic
  pixels matched and operator focus succeeded. Mouse acknowledgement then failed.
  Direct page readback confirmed counters still 9/9; the click did not reach the
  page. Keyboard and reconnect steps were not attempted. The client browser
  closed. No unchanged retry followed. Cause is unproven; do not attribute this
  failure to the lease repair or claim the earlier pass covers this binary.
  Prior A2 success on c36ce2b0 remains historical evidence.
- A3 remains OPEN. Supported doctor still exits 1. After viewer connection its
  presentation and upgrade-readiness findings cleared, despite the later input
  failure. That narrower receipt does not prove full A2 acceptance. Remaining
  findings concern terminal history, legacy principals, missing owner bindings,
  generation/session proof, stale monitoring and unknown process ownership under
  pressure. Exact current report is retained privately.
- A4 remains OPEN. The maintenance timer is linked and inactive, with automatic
  startup disabled. No scheduled-cycle or additional restart acceptance is claimed.
- AX advanced but remains OPEN. The new reconciliation receipt correlates to its
  plan and lease. Investigation also confirmed a dashboard logging wire mismatch,
  repaired in source below. Missing mouse-input delivery remains untraced.

### Follow-up source batch, not installed

Two concrete diagnostic defects are repaired in source commit `d523aaeb`:

1. The dashboard sent fractional `performance.now()` durations to a receiver
   requiring unsigned integer milliseconds. HTTP 400 retained the same invalid
   observation at the front of its delivery queue. The client now rounds only
   the transmitted duration, preserving measurement at fetch settlement.
   A fractional-clock regression failed before repair and passed after.
   An isolated real installed-binary endpoint rejected 17.25 with HTTP 400;
   the repaired client delivered 17 with HTTP 202 and correlated journal readback.
   This also explains repeated observation rejections in the prior successful
   viewer journey. It does not explain the missing mouse input.
2. Owner-derived legacy projection failed to exclude profiles already covered
   by canonical claims or registered-capability leases. Extended existing tests
   with actual owner/browser records reproduced two rows instead of one.
   Projection now excludes the duplicate; genuine capability binding warnings
   and legacy owners without a covering lease remain visible.

All 26 focused lease tests pass. Failure-journal/client-observation checks and
Dashboard build/TypeScript, formatting and workspace Clippy with warnings denied
pass. The optimized candidate also passed the retained-consumer reconciliation
fixture with one lease row, both original handles usable and no remaining
fixture processes. These source fixes have not replaced the
installed dc570e5b binary or its shared skill. Do not report them as live.

### Retention and next work

Generation-GC preview lists 22 candidates, including the immediately previous
production generation `0.28.0-b5c77d59b0de-5413901f4e26` and its predecessor.
It reports `previousHealthyGenerationId=null`. No GC or workstation reconcile
was applied. Verified private backups preserve both prior sealed generations;
the latest backup has 32 matching file hashes. Original generations remain.

The controlled publication receipt is outside the product upgrade ledger.
Retention consumes supported transactions, selected generation, live processes
and supervisor references. Repair a supported publication/retention path before
maintenance resumes. Never fabricate accepted transactions or green monitor
receipts to protect a rollback target.

Next hour: spend at most ten minutes mapping the remaining identity findings
to concrete failures, then up to 35 minutes repairing the highest-impact case
in isolation, and reserve 15 minutes for focused validation and custody. Keep
identity proof first; a warning inventory alone is not acceptance progress.
Before another operator attempt, isolate mouse delivery with an observable input
path; pixel readiness and operator-focus success alone proved insufficient.
Preserve the failed attempt and its unchanged page counters. Do not spend an
unchanged viewer retry. Then consolidate the remaining identity, logging and
retention repairs before another production candidate. Reconcile every A1/A3
finding with operation-specific evidence; only after A1–A3 pass restore A4's
three real scheduled cycles, controlled restart and next scheduled cycle.

### Private evidence

Evidence stays under `~/.local/state/agent-browser/campaigns/p160/`:

- `reconcile-rejoin/`: installed collection read, validation record, optimized
  candidate qualification, and fractional-wire endpoint driver.
- `missing-binding-sim-fy12OQ`: installed-predecessor planner failure and cleanup.
- `missing-binding-sim-jhmudE`: optimized repaired planner, original handles and
  targets preserved, cleanup complete.
- `publication-dc570e5b/`: exact-release six-case qualification, sealed generation,
  activation/custody receipts, pre/post manifest and synthetic-handle readback,
  current doctor, GC preview, verified rollback backup and installed skill sync.
  `local-viewers-1I12nr/events.json` retains the failed mouse journey.
- `missing-binding-sim-4c1esh`: exact release reconciliation and persisted receipt.
- `missing-binding-sim-UOeTwk`: d523aaeb optimized follow-up qualification,
  duplicate-free lease projection and retained handles, cleanup complete.
- `primary-endpoint-sim-Xrh9tr`: fractional rejection, corrected client delivery,
  journal correlation and terminated disposable backend/provider.
- `publication-c36ce2b0/`: prior cold-viewer pass and previous-generation backup.
  Historical cold-stream delay repair and measurements remain in Git checkpoint
  `50b05156` and `primary-guard-measurement/`.

Plan 0158's manual external-vantage dispatch boundaries remain unchanged.
No formal release was created and no external notifications were sent.

## History index

- [Complete previous runbook, through Turn 213](RUNBOOK-history-through-2026-09-02.md)
- Git checkpoint `50b05156` preserves Turn 214's installed cold-viewer acceptance.
