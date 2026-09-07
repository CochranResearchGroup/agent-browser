# Runbook

This is the sole current execution status. Plans own acceptance and strategy;
Git checkpoints and linked archives preserve history. Keep this file at or below
200 lines under [policy 0043](docs/dev/policies/0043-roadmap-runbook-governance.md).

## Turn 226 | 2026-09-07

Authority: [Plan 0160](docs/dev/plans/0160-2026-09-06-production-profile-identity-and-operational-readiness.md).
Lane P157; branch `plan/profile-permissions-and-request-provenance`.
Progress: production inventory admission exposed a product contract gap.
The installed loader accepts development inventory only and explicitly rejects
production. No inventory was applied and no environment guard was weakened.
Provider database route3 uses agent-browser-rdp-c (UID1010); its Xorg PID60951
runs on :14, agreeing with the retained browser allocation. Runtime host maps
only UID1000, so display proof must retain its namespace-aware observation path.
Evidence: `publication-dfbcd2aa/production-inventory-admission.json`.
Plan0160 now requires explicit production inventory support, preservation of
orphaned incumbent custody, and separation of recovery capacity from visible
presentation readiness. Next implement and qualify that bounded contract.
Full A1–A4/AX remains OPEN. Consumer workflows have not been impersonated or accepted.
Unrelated modified note0156 and untracked note0159 remain preserved.

### Current installed identity

- Source: `dfbcd2aac277365532c70f9d7722fe8796e367af`.
- Generation: `0.28.0-d0eb66dc19fd-d7d81d2f5bc4`.
- Binary SHA256: `d0eb66dc19fdc0b6812cc10ddcb1f22a5f349fa6a9c4aeb5bc02fe92cf49002b`.
- Support SHA256: `d7d81d2f5bc4f72bcef58a8e8815c248320423bfd5e4f749b0502c9e984a284f`.
- Host PID83902; activation preserved five exact browser identities, retained
  private storage and all tab custody. Admission found no active jobs.
- Installed manifest and original synthetic handle pass; attestation complete,
  page marker intact, input counters 9/9. This was readback, not new input proof.
- Doctor exits 1; full operational readiness remains unaccepted.
- Main shared skill synced with backup; separate Service skill unchanged.
- Immediate rollback generation and 32 verified backup files remain available.
  No formal release, GC or external notification occurred.

### A1: installed identity repairs and remaining boundary

The earlier installed `dc570e5b` repair fixed `no_safe_reconciliation_transition`
when the exact lease already authorized rejoin. The planner seals one
`rejoin_owned_browser` transition and apply reuses unique, uncontested owner,
session and tab guards. Earlier exact-release proof retained both original
handles across host interruption, restored active lease and complete attestation,
and correlated the returned receipt with the persisted plan and lease.

Earlier storage/rejoin and handle-only MCP routing qualification is preserved at
`c40e583a`. The installed `d6d99e43` routes original handles correctly and its
isolated reconnect, foreign denial, exact release and peer-preservation checks
passed. This does not establish current consumer acceptance.

Seven current lease findings span six rows: two legacy-principal warnings,
three missing owner bindings on Default, and an owner-generation/session-authority
pair on stealthcdp-default. All four affected profiles have active shared-local
policies. These maintenance warnings alone do not establish an ordinary request
refusal. The original synthetic profile has complete attestation and successful
control despite its legacy warning. Exact installed binary `retained-unit-sim-NdcCp4`
passes ordinary control before rejoin, sealed rejoin with its original credential,
active lease, complete attestation, two original handles and retained storage.
Cleanup left zero owned processes. Private `ownership-dispositions.json` assigns
all six records; combined identity recovery now has isolated exact-binary proof. Consumer capabilities were not borrowed to
apply ownership changes or claim their workflows accepted.

The stored consumer `owner_connection_still_active` refusal has matching
self-declared subjects and both permissions. Fresh readback finds the identical
historical owner-connection hash still marked active after host replacement.
The isolated `retained-unit-sim-Q2Hc69` reproduction held an authenticated socket
open, killed only its disposable host, preserved Chrome, then restarted the host.
The original handle was refused with that same typed error because the dead
connection remained active. Normal connection teardown passes; abrupt host death
bypasses its disconnect guard. Source `3dfd4d09` now mints opaque connection IDs
with boot, PID namespace, PID and process start evidence. Reconnect uses only
persisted custody and positive host-death evidence; subject and permissions
remain enforced. Legacy IDs and unreadable evidence never authorize takeover.
`retained-unit-sim-8rJXkf` proves live-owner protection, host-kill recovery,
complete attestation, original-handle evaluation, same Chrome and foreign denial
without page effects. An initial attempt `retained-unit-sim-Xpwqqx` recovered
custody but correctly refused control with conflicting fixture profile hints;
correcting those hints before launch passed. The failure remains preserved.
The consumer legacy access record exactly matches the pre-publication custody
snapshot and remains active. Fresh census finds three production processes, all
started after that snapshot, and six development processes with separate homes.
The reviewed legacy transition affects one tab; no jobs were active. Native
transactional repair is exact-release qualified; production preview passes.
Production application succeeded for that one tab: the dead connection is now
disconnected, with permissions, target and owner/lease authority unchanged. All
five browsers survived. Consumer reconnect and artifact transport remain open.

### Storage recovery and prevention

Source `bee768ed` supplies the installed private, mode-0700 state-backed `/tmp`
and `/var/tmp` mounts. Thirteen supervisor tests and a real Chrome synthetic CSV
passed after parent retirement. Recovery repaired six deleted temporary mounts
across three exact namespaces while preserving five original browser processes.
Both publications preserved those mounts and tab custody. New-host mount readback
confirms private state backing. Detailed recovery history remains at Git
checkpoint `dd94dfe7` and in the private ownership-bound recovery receipts.

Keep `runtime-tmp/recovered` backing directories while retained namespaces
reference them. The new template does not itself repair already-deleted mounts.
The historical Playwright Node process no longer exists; its artifact directory
was not recreated. Actual consumer CSV, new-client artifact transport and shared
browser-context download policy remain unaccepted. The private recovery helper
still needs a reusable product diagnosis/recovery surface.

### A2–A4 and AX remain open

- A2: the prior dc570e5b journey connected two authenticated viewers, denied an
  anonymous viewer, matched synthetic pixels and obtained operator focus. Mouse
  acknowledgement then failed and page counters stayed 9/9. Keyboard and reconnect
  were not attempted. Cause remains unproven. No unchanged viewer retry was run;
  neither publication has a matching complete operator-journey acceptance receipt.
- A3: current installed doctor exits 1. Findings cover presentation and upgrade
  readiness, retained terminal history, the seven lease findings above, stale
  monitoring and unknown pressure ownership. Exact report is retained privately.
- A4: maintenance timer remains linked, inactive and disabled for automatic
  startup. No scheduled-cycle acceptance is claimed. Restore it only after
  supported rollback retention and A1–A3 readiness are established.
- AX: the dashboard's fractional elapsed milliseconds caused HTTP 400 and
  blocked its observation queue. Source `d523aaeb`, now installed, rounds only
  the wire value. A fractional-clock regression and real receiver/journal
  correlation passed. Its duplicate legacy-owner projection fix also passed
  all 26 focused lease tests and retained-handle simulation. Genuine unresolved
  bindings remain visible. Source `a9830cfa` preserves daemon `id`, `failure`,
  and `terminalOutcome` through CLI decoding and MCP formatting. The existing
  regression proves field retention and compatibility with older responses.
  In `retained-unit-sim-gSifp9`, both actual MCP denials returned request/job
  correlation and the exact persisted terminal outcome. Original-handle
  reconnect, foreign denial without effects and exact release also passed.
  This repair is now installed; full installed AX acceptance remains open.

### Retention and next work

The earlier GC preview listed 22 candidates and `previousHealthyGenerationId=null`.
That preview is historical after two additional publications. No GC or workstation
reconcile was applied. Original generations and verified private rollback copies
remain. Each publication backed up its immediate predecessor with 32 matching
file hashes. The immediate predecessor is `0.28.0-7991b404eafe-8362848f8b0b`.

Controlled publication receipts remain outside the supported upgrade ledger.
Repair the supported publication/retention path before enabling maintenance;
never fabricate accepted transactions or green monitor receipts to protect a
rollback target. Preserve original recovery storage through any future change.

Next implement the explicit production presentation-inventory contract;
actual consumer reconnect after the legacy repair remains pending. Complete
consumer artifact transport
and remaining A1 dispositions, preserving ordinary self-identification.
For Guacamole input, check refresh/reconnect, effective settings, and scoped
browser/provider recovery in a disposable session before code diagnosis.
Consolidate repairs before another production candidate. After A1–A3 pass,
observe three scheduled cycles, a controlled restart and its next cycle using
original handles and the durable handoff URL. Plan0158 external-vantage dispatch
boundaries remain unchanged.

### Evidence index

Private evidence root: `~/.local/state/agent-browser/campaigns/p160/`.

- `legacy-connection-repair/`: exact release qualification, production native
  transaction, backup and readback preserving permissions and five browsers.
- `publication-dfbcd2aa/`: sealed generation, rollback backup, activation,
  original synthetic-handle acceptance, nonzero doctor and skill-sync evidence.
- `retained-unit-sim-YtpFEw/`: exact release legacy repair with live-owner and
  changed-evidence denial, restored original handle and zero remaining processes.
- `retained-unit-sim-8rJXkf/`: 3dfd4d09 abrupt-host recovery and live/foreign
  ownership protection, zero-residue cleanup, plus legacy reconciliation evidence,
  writer census and exact one-tab transition review.
- `retained-unit-sim-gSifp9/`: optimized a9830cfa real MCP error correlation,
  exact persisted outcomes, ordinary reconnect/control, and zero-residue cleanup;
  private `consumer-stale-connection-readback.json` anchors historical comparison.
- `publication-d6d99e43/`: frozen source, exact release, qualification, sealed
  generation, activation/custody/storage receipts, installed original-handle
  readback, doctor, rollback backup, skill sync and the next AX finding.
- `retained-unit-sim-Djjk8o`: installed MCP handle-only routing failure.
- `retained-unit-sim-VCGCs6`: optimized MCP repair, reconnect and denial proof.
- `retained-unit-sim-VBuQI4`: exact-release MCP proof including physical release
  and peer preservation; cleanup complete.
- `publication-dd94dfe7/`: first publication, storage/handle proof, identity
  dispositions, current-at-that-point doctor and verified dc570e5b backup.
- `retained-unit-sim-7QAzvP`: exact dd94dfe7 capability/rejoin/storage proof.
- `retained-unit-sim-O4DdJy`: self-identified HTTP retained-handle proof.
- `temporary-storage-recovery/`: frozen recovery plan/helper, negative checks,
  three production operation ledgers, final five-browser readback and receipt.
- `deleted-tmp-recovery-81654n_8`, `retained-download-6xfjyhb6`: disposable
  recovery and same-Chrome synthetic CSV proofs with cleanup.
- `publication-dc570e5b/`: previous exact-release six-case qualification and
  `local-viewers-1I12nr/events.json`, the failed mouse journey.
- `reconcile-rejoin/`, `missing-binding-sim-UOeTwk`,
  `primary-endpoint-sim-Xrh9tr`: lease and diagnostic-wire source/receiver proof.
- `publication-c36ce2b0/`: historical successful operator journey and backup.

## History index

- [Complete previous runbook, through Turn 213](RUNBOOK-history-through-2026-09-02.md)
- Git checkpoint `50b05156` preserves Turn214's installed cold-viewer acceptance.
- Git checkpoint `dd94dfe7` preserves Turn216's live storage recovery.
- Git checkpoint `c06ea5e8` preserves the first publication in this hour.
