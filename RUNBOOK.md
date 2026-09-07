# Runbook

This is the sole current execution status. Plans own acceptance and strategy;
Git checkpoints and linked archives preserve history. Keep this file at or below
200 lines under [policy 0043](docs/dev/policies/0043-roadmap-runbook-governance.md).

## Turn 217 | 2026-09-07

Authority: [Plan 0160](docs/dev/plans/0160-2026-09-06-production-profile-identity-and-operational-readiness.md),
lane P157, branch `plan/profile-permissions-and-request-provenance`.
The candidate batch runs 17:17:48Z–17:47:48Z with one publication decision.
All prior effort remains cumulative, including the prior hour and live storage
recovery. Previous goal turn: verified wait on build session30120. Current
progress: exact candidate installed with retained browser, handle and storage
proof. Full readiness remains OPEN. No consumer capability was borrowed, profile
lease reassigned or consumer browser restarted. Unrelated note0156 and untracked
note0159 remain preserved. Build, qualification and activation have completed.

### Installed repair and current acceptance

- Installed source: `dd94dfe7e00cb3a098aa8d58b84cabf552e31856`.
  Selected generation: `0.28.0-13d426fb15c9-bb2f5ae53042`.
  Binary SHA256: `13d426fb15c92a5e21b7744f45cd45b6dfe7942f8c2d6e3f58348ad7ff6938a6`.
  Support SHA256: `bb2f5ae530427178dac9237495902fd02e84581ec633f9ee2ed8ac5dc99a6e7d`.
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
- The predecessor passed six exact-release cases. The new exact release passed
  a combined retained-handle reconciliation and persistent-storage simulation.
  Two fixture defects preceded that pass: default group shutdown stopped Chrome;
  recreating a still-loaded transient unit failed. The corrected fixture uses
  main-process-only retirement and restarts the existing unit. Both failures and
  all cleanup receipts remain preserved; no candidate source retry occurred. Activation admitted with no active jobs
  and preserved five exact live browser process identities and 70 tab-custody
  records. Authenticated manifest and retained synthetic-handle readback passed
  after activation; mouse/keyboard counters remained 9/9.
- A2 is OPEN. The previous dc570e5b candidate had one bounded local synthetic
  journey. The new generation has no matching presentation acceptance receipt.
  Both authenticated viewers connected, anonymous access was denied, synthetic
  pixels matched and operator focus succeeded. Mouse acknowledgement then failed.
  Direct page readback confirmed counters still 9/9; the click did not reach the
  page. Keyboard and reconnect steps were not attempted. The client browser
  closed. No unchanged retry followed. Cause is unproven; do not attribute this
  failure to the lease repair or claim the earlier pass covers this binary.
  Prior A2 success on c36ce2b0 remains historical evidence.
- A3 remains OPEN. Current installed doctor exits 1 for presentation and upgrade
  readiness, retained terminal history, two legacy-principal rows, three missing
  owner bindings, one owner-generation/session-authority pair, stale monitoring
  and unknown pressure ownership. Current report is retained privately. The
  original synthetic handle has complete attestation despite its maintenance
  warning; ordinary operation and maintenance evidence remain separate.
- A4 remains OPEN. The maintenance timer is linked and inactive, with automatic
  startup disabled. No scheduled-cycle or additional restart acceptance is claimed.
- AX advanced but remains OPEN. The new reconciliation receipt correlates to its
  plan and lease. Investigation also confirmed a dashboard logging wire mismatch,
  repaired in source below. Missing mouse-input delivery remains untraced.

### Diagnostic batch, now installed

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
fixture processes. These fixes are now installed in the
selected dd94dfe7 candidate, and its main shared skill matches repository source.
The separate Service skill and its lock-diagnostic guidance were preserved.

### Existing storage recovered; prevention now installed

Source `bee768ed` now supplies the installed preventive unit. Thirteen supervisor
tests and a real Chrome synthetic CSV passed after parent retirement. Production
recovery repaired six deleted temporary mounts across three exact namespaces,
preserving all five original browser processes. Candidate activation preserved
those mounts and 70 tab-custody records. New host storage is private, unit-scoped
state-backed storage. Original synthetic handle readback retains complete
attestation and unchanged mouse/keyboard counters 9/9. Recovery details remain
in Git checkpoint dd94dfe7 and the private operation receipts.

The report's old Playwright Node PID no longer exists. Its artifact directory
was not recreated or bridged. Consumer CSV, current client artifact transport,
shared-context download policy and typed product diagnosis remain unaccepted.
Do not revive stale artifact paths or change shared download settings by guesswork.
Recovery backing directories under user state runtime-tmp/recovered must remain
while retained namespaces reference them. The private recovery receipt binds
all three operation ledgers and the final five-browser readback.

### Current identity repair, not installed

The exact installed binary passed self-identified HTTP control across disposable
host interruption with both original handles and no registered capabilities.
An MCP stdio simulation then reproduced a distinct routing refusal: after
opening a tab successfully, a handle-only diagnostics request was sent to the
MCP default session instead of the handle's session. This is not proof that the
historical consumer's connection-active denial has the same cause.
Source now derives a missing MCP route from the canonical handle fields while
preserving explicit routes and daemon custody checks. Focused route and desktop
regressions, formatting, Clippy and API/MCP parity pass. The optimized candidate
build is running. Exact MCP
reconnection and foreign-owner denial remain pending. The identity follow-up
ends by 18:17:48Z, including this batch's 17:17:48Z start in the hour allowance;
no second production publication is part of this follow-up.

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

The diagnostic and preventive-template candidate is installed. Next resolve the
highest-impact remaining identity refusal, distinguishing an actual request
blocker from a maintenance warning. Complete identity dispositions and scoped
consumer artifact transport before claiming A1.
The recovery helper still needs a reusable product diagnostic/recovery surface;
its private receipt must not be represented as a supported upgrade transaction.
Before another operator attempt, isolate mouse delivery with an observable input
path; pixel readiness and operator-focus success alone proved insufficient.
Preserve the failed attempt and its unchanged page counters. Do not spend an
unchanged viewer retry. Consolidate remaining repairs before
another production candidate. Reconcile every A1/A3
finding with operation-specific evidence; only after A1–A3 pass restore A4's
three real scheduled cycles, controlled restart and next scheduled cycle.

### Private evidence

Evidence stays under `~/.local/state/agent-browser/campaigns/p160/`:

- `publication-dd94dfe7/`: exact release, corrected combined simulation, one
  activation, original handle readback, storage preservation, current doctor,
  verified 32-file dc570e5b rollback backup and shared skill synchronization.
- `temporary-storage-recovery/`: frozen plan/helper, negative checks, three
  production apply ledgers, pilot handle proof, final readback and recovery receipt.
- `deleted-tmp-recovery-81654n_8`: failed direct bind, successful beneath/reveal
  recovery, same-Chrome CSV and cleanup.
- `retained-download-6xfjyhb6`: real Chrome post-retirement CSV, exact
  frame/GUID and byte readback, cleanup complete.
- `reconcile-rejoin/`: installed collection read, validation record, optimized
  candidate qualification, and fractional-wire endpoint driver.
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
