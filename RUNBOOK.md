# Runbook

This is the sole current execution status. Plans own acceptance and strategy;
Git checkpoints and linked archives preserve history. Keep this file at or below
200 lines under [policy 0043](docs/dev/policies/0043-roadmap-runbook-governance.md).

## Turn 216 | 2026-09-07

Authority: [Plan 0160](docs/dev/plans/0160-2026-09-06-production-profile-identity-and-operational-readiness.md),
lane P157, branch `plan/profile-permissions-and-request-provenance`.
The previous hour closed within 15:58:19Z–16:58:19Z. Goal continuation authorized
this bounded recovery follow-up, 16:58:51Z–17:18:51Z, primary-controlled.
This follow-up is checkpointed with no running repair, fixture or build process.
All prior effort remains cumulative. Progress: recovered existing production
storage after isolated proof. Full readiness remains OPEN. No consumer capability
was borrowed, profile lease reassigned, browser restarted or browser input sent.
Unrelated note0156 and untracked note0159 remain preserved.

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

### Existing production storage recovered; prevention not installed

Source `bee768ed` prevents future host retirement from deleting retained-browser
temporary storage. It mounts private, unit-scoped disk-backed state directories
at `/tmp` and `/var/tmp`. All 13 supervisor tests, formatting and workspace Clippy
passed. The rendered unit and real Chrome synthetic CSV download passed after
parent retirement. This preventive template is NOT INSTALLED.

Read-only production observation at 16:57:35Z found all five retained browsers
holding unlinked `/tmp` and `/var/tmp`, across three mount namespaces. Existing
PID/tab preservation had not established usable storage. The mechanism reproduced
in a disposable systemd service; the historical deleting stop remains unproven.

A direct bind over a deleted mount failed ENOENT. A bounded disposable recovery
then used Linux MOVE_MOUNT_BENEATH to stage private storage under the deleted
mount before revealing it. The same browser PID/start and mount namespace survived,
and Chrome completed an exact frame/GUID-correlated synthetic CSV download with
matching bytes. Fixture processes, units and storage were cleaned up.

A frozen production plan bound five Ready owners, process start/executable/UID,
three exact namespaces, deleted mount inode/device, permitted namespace members,
mount propagation, boot and expiry. Three negative checks rejected changed start,
unproven executable and changed mount identity. The first pilot stopped before
any filesystem or mount effect because the ambient Python lacked pidfd_open;
the tested /usr/bin/python3 supplied it. That failure is preserved.

Recovery applied first to the owned synthetic operator namespace. Its original
service handle retained complete attestation and unchanged 9/9 input counters.
The two remaining namespaces then passed the same guarded repair. All six mounts
now have positive link counts, private mode 0700 and verified write/readback;
original nosuid/nodev/noexec restrictions were retained where present. Five browser
PIDs, start identities and mount namespaces remained unchanged. Global `/tmp`
remained unchanged. No browser restart, page action, download-policy change or
consumer capability use occurred. This is a LIVE storage repair, not installation
of the new template or full browser/download acceptance.

Post-repair install doctor still exits 1 for lease identity, monitoring, unknown
pressure ownership and retained upgrade history. Storage repair does not clear
those findings.

The report's old Playwright Node PID no longer exists. Its artifact directory
was not recreated or bridged. Consumer CSV, current client artifact transport,
shared-context download policy and typed product diagnosis remain unaccepted.
Do not revive stale artifact paths or change shared download settings by guesswork.
Recovery backing directories under user state runtime-tmp/recovered must remain
while retained namespaces reference them. The private recovery receipt binds
all three operation ledgers and the final five-browser readback.

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

Next consolidate the already-tested diagnostic and preventive-template fixes
into one installed candidate, preserving the recovered storage. Complete remaining
identity dispositions and scoped consumer artifact transport before claiming A1.
The recovery helper still needs a reusable product diagnostic/recovery surface;
its private receipt must not be represented as a supported upgrade transaction.
Before another operator attempt, isolate mouse delivery with an observable input
path; pixel readiness and operator-focus success alone proved insufficient.
Preserve the failed attempt and its unchanged page counters. Do not spend an
unchanged viewer retry. Then consolidate the remaining identity, logging and
retention repairs before another production candidate. Reconcile every A1/A3
finding with operation-specific evidence; only after A1–A3 pass restore A4's
three real scheduled cycles, controlled restart and next scheduled cycle.

### Private evidence

Evidence stays under `~/.local/state/agent-browser/campaigns/p160/`:

- `temporary-storage-recovery/`: frozen plan/helper, negative checks, three
  production apply ledgers, pilot handle proof, final readback and recovery receipt.
- `deleted-tmp-recovery-81654n_8`: failed direct bind, successful beneath/reveal
  recovery, same-Chrome CSV and cleanup.
- `retained-download-6xfjyhb6`: real Chrome post-retirement CSV, exact
  frame/GUID and byte readback, cleanup complete.
- `private-tmp-lifecycle-oswt5o1o`: disposable deletion reproducer.
- `rendered-tmp-lifecycle-qn2_650f`: preventive unit configuration proof and cleanup.
- `state-tmp-lifecycle-u1gpfuc3`: disk-backed private storage lifetime proof.
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
