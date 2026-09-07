# Runbook

This is the sole current execution status. Plans own acceptance and strategy;
linked archives and Git checkpoints preserve history. Keep this file at or below
200 lines under [policy 0043](docs/dev/policies/0043-roadmap-runbook-governance.md).

## Turn 215 | 2026-09-07

The active identity repair batch runs from 2026-09-07T15:58:19Z through
16:58:19Z under the amended Plan 0160 strategy and standing execution authority.
Prior approximately 12-hour effort, the completed 30-minute round and the
completed 60-minute cold-viewer round remain cumulative. Controller: primary.
The full operational-readiness objective remains open.

The installed candidate reproduced `no_safe_reconciliation_transition` in an
isolated retained-consumer fixture even though its exact lease authorized rejoin.
The planner omitted that supported operation. Source now seals a single guarded
`rejoin_owned_browser` transition; apply uses existing exact custody checks.
The regression failed before repair. All 26 focused lease tests now pass,
including changed foreign-tab custody refusal with no state mutation and
idempotent replay. A fixture setup omission was corrected after the first green
attempt exposed a missing test tab; that attempt is not a product regression.
Client contract checks, formatting, Clippy and docs build passed. Optimized
candidate qualification and production publication are pending. No consumer
capability was borrowed and no production lease repair was applied.

### Installed candidate and acceptance

- Authority: [Plan 0160](docs/dev/plans/0160-2026-09-06-production-profile-identity-and-operational-readiness.md),
  lane P157. Profile ownership and identity proof remain the highest priority.
- Installed source: `c36ce2b04ed23ba3c4db51612b1b86686db52eac`.
  Selected generation: `0.28.0-b5c77d59b0de-5413901f4e26`.
  The authenticated runtime manifest matched the release binary after activation.
- Activation admitted with no active jobs and preserved five exact browser
  process identities and all 70 tab-custody records. Evidence includes before
  and after process, ingress and tab records plus the activation receipt.
- A1 remains OPEN. Exact-candidate isolated qualification passed retained-owner,
  headed, headless and session-only cases. The closer consumer simulation used
  its original capability and stale shared handle after host interruption;
  managed-launch custody proved identity without a handoff receipt. Ordinary
  tab control returned 42 BEFORE explicit rejoin. Public rejoin then made the
  lease active and preserved both original handles, process and targets.
  Cleanup ended with no fixture residue. This proves those mechanisms without
  interrupting the production consumer; it does not invent its fresh result or
  resolve every outstanding profile-owner/maintenance binding.
- A2 PASSED on this installed candidate, in the first cold attempt. Two separate
  authenticated viewers displayed the synthetic page; unauthenticated access
  was denied. Trusted mouse and keyboard input passed, as did reconnection via
  the same durable handoff while the second viewer remained present.
  Retained-handle readback proved complete attestation and exactly one additional
  trusted event per input class, with counters moving from 8/8 to 9/9.
  Primary readiness requests took 2,770 ms cold, 86 ms for the second viewer,
  and 157 ms after reconnect. This is local operator acceptance, not resumed
  Plan 0158 external-vantage work.
- A3 remains OPEN. Post-viewer supported doctor exited 1. The operator-journey
  and upgrade-readiness findings cleared; remaining findings include terminal
  transaction history, legacy principals, missing owner/principal bindings,
  generation/session proof, stale runtime monitoring and unknown process
  ownership under RSS pressure. Distinguish profile-owner maintenance authority
  from ordinary authorized tab control; the latter passed before rejoin.
- A4 remains OPEN. The timer was inactive but still enabled for startup. It is
  now linked and inactive, with automatic startup disabled, preventing GC of
  the unprotected rollback generation on the next login or boot. Restore its
  schedule only after A1–A3 acceptance.
  No scheduled-cycle or additional production-restart acceptance is claimed.
- AX advanced but remains OPEN as a complete matrix. Fetch-failure elapsed time
  now measures fetch settlement, excluding deferred journal delivery. A forced
  provider failure through the isolated real dashboard endpoint returned a
  terminal occurrence that joined to exactly one owner journal record. The
  failed owner remained sticky, with no replacement provider connection.

### Causal repair and validation

The primary transport performed full Service State authority reads on async
workers and before every individual image acknowledgement. On a private copy
of current state, each optimized guard took 66–84 ms. An immediately available
in-memory provider delivering 128 blobs took 11,818 ms to become ready and
caused 154 guards. Bounded acknowledgement batches and blocking-pool authority
reads reduced the same case to 295 ms and five guards. Every actual write
still checks fresh exact ownership; no successful authority result is cached.
Periodic checks no longer accumulate overdue ticks.

The scheduler regression failed before the repair and passed after it.
Seventeen focused Rust tests passed, including changed-owner rejection,
retained ownership, cancellation, provider identity and failure correlation.
Dashboard sharing contracts, failure-journal tests, dashboard build/TypeScript,
Rust formatting and workspace Clippy passed for their changed surfaces.
The optimized development candidate passed the authenticated endpoint fixture:
two concurrent requests shared one provider and all 128 acknowledgements arrived.
The final release binary then passed all five established isolated qualification
cases, each with successful cleanup, before production publication.

Production Agent Browser skill guidance now matches installed source c36ce2b0.
The existing additional Service skill lock-diagnostics guidance was preserved.
Source is committed and pushed on `plan/profile-permissions-and-request-provenance`.
No formal release was created.

### Maintenance blocker and next work

- Service process-GC preview found zero candidates. Generation-GC preview found
  21 candidates, INCLUDING the immediately previous production generation
  `0.28.0-86d7b29c81fe-5f4b8dee3613`. No GC or workstation reconcile was applied.
- The controlled activation receipt is outside the product transaction ledger.
  `workstation_install.rs::generation_retention_plan` builds retention from
  supported upgrade transactions, selected generation, live processes and
  supervisor references. It does not consume this campaign's activation receipt;
  the preview reports no previous healthy generation. Thus the current rollback
  target is not protected from unattended generation GC.
- A private backup of that entire sealed generation was created and all 32 file
  hashes were verified. The original generation remains in place. The backup
  protects recovery material; it does not manufacture supported retention or
  an accepted upgrade transaction.
- The supported installer dry-run returned a non-mutating planned result with
  `ready=false`; it did not establish a retention reference or acceptance.
- Next: resolve A1 ownership failures with operation-specific evidence and an
  isolated consumer reproduction. The active bounded batch starts at
  2026-09-07T15:58:19Z and ends by 16:58:19Z, including validation and governance.
  Reassess after 30 minutes without A1 outcome progress. Prior effort remains
  cumulative. The preceding recommendation-only turn made no outcome progress;
  execution resumes under the amended Plan 0160 strategy.
- Then repair the supported controlled-publication/retention path so an actual
  prior rollback generation remains protected until operational acceptance.
  Do not create a fake accepted transaction, green monitor receipt, supervisor
  reference or live-process placeholder to influence GC.
- Reconcile remaining profile-owner findings with operation-specific impact and
  current evidence. Preserve valid own-tab control while maintaining exact
  authority requirements for owner recovery, transfer and cleanup.
- Then run one reviewed supported maintenance pass, require doctor zero and
  complete A1/A3 dispositions, and only then resume A4's real scheduled cycles.
  Do not replace scheduled cycles with manual invocations or another warm viewer
  pass. No additional production activation or viewer retry is queued.

### Evidence and custody

Private operator-owned evidence is under
`~/.local/state/agent-browser/campaigns/p160/`:

- `publication-c36ce2b0/`: qualification, staging and activation receipts;
  `local-viewers-3RC8R1/events.json`; before/after synthetic input readback;
  doctor results; GC previews; installer dry-run; rollback backup manifest;
  production skill-sync receipt. Original browser and provider data stay private.
- `primary-guard-measurement/`: exact before/after measurement sources and
  receipts. Temporary measurement tests and copied state are outside Git.
- `primary-endpoint-sim-ZwpewN/`: real-binary endpoint proof and zero-residue
  readback. Earlier setup failure `primary-endpoint-sim-nHoqlD/` records missing
  host bootstrap and completed cleanup; the corrected fixture used the existing
  backend-only boundary and its own generated dashboard credential.
- `missing-binding-sim-G1aDln/`: exact installed candidate, ordinary control
  before rejoin, original credential/handle continuity and completed cleanup.
- Earlier retained-handle simulations: `missing-binding-sim-ThA0Um/` and
  `missing-binding-sim-aAMdDU/`. Earlier setup failures and cleanup remain retained.
- The original cold failure remains in `publication-0cf07a43/`, including
  `local-viewers-qGlp5N/events.json`. Its later warm diagnostic did not erase it.
  This round's first cold pass follows a measured causal repair.

The unrelated consumer contribution in note 0156 and untracked note 0159 were
preserved. Tenant data, capabilities and raw runtime evidence were not committed.
Plan 0158's manual external-vantage dispatch and no-retry boundaries remain in force.

## History index

- [Complete previous runbook, through Turn 213](RUNBOOK-history-through-2026-09-02.md)
  remains byte-for-byte beside this file with relative links preserved.
- [Plan 0160 historical checkpoints](docs/dev/plans/0160-2026-09-06-production-profile-identity-and-operational-readiness.md#planning-checkpoint).
- [Policy adoption and validation](docs/dev/notes/2026-09-07-outcome-first-policy-adoption.md).
- Earlier Turn 214 status is retained in Git checkpoints `05434b3c` and `c36ce2b0`.

Read only history relevant to the task. Archived status does not override this
current acceptance record or authorize restarting a previously stopped lane.
