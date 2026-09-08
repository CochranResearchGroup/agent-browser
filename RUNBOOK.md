# Runbook

This is the sole current execution status. Plans own acceptance and strategy;
Git checkpoints and linked archives preserve history. Keep this file at or below
200 lines under [policy 0043](docs/dev/policies/0043-roadmap-runbook-governance.md).

## Turn 270 | 2026-09-08

Authority: [Plan 0160](docs/dev/plans/0160-2026-09-06-production-profile-identity-and-operational-readiness.md).
Installed bf4de8aa after one 605-second release build and five exact-release
qualification cases. The alias, explicit Retry and bounded ownership-read repairs
passed 21 focused Rust tests, dashboard gates, fmt and clippy. Qualification
covered independent headless/headed clients, causal refusals, Windows Downloads
and private relative aliases; cleanup zero. Evidence: publication-bf4de8aa/.
Production replacement preserved five original browsers, recovered storage and
33 tab records. Only the synthetic handle's connectionInstanceId changed.
Original-handle attestation remains complete. Maintenance removed nothing;
doctor exits 0. Eleven rollback holds remain and the timer is inactive.
Two authenticated viewers, synthetic pixels, same-link reconnect and trusted
mouse/keyboard input passed in local-viewers-txWCsl. Counters advanced 14 to 15;
no explicit Retry was needed and no new primary terminal event was observed.
Original data-origin CSV failed. HTTP-origin CSV now passes in a temporary owned
tab of the same retained production browser: 24 verified bytes, original handle
attestation preserved, temporary tab closed, five production identities intact.
Evidence: publication-bf4de8aa/normal-origin-check/acceptance.json. Context policy
unchanged; tab/origin restriction and original consumer export remain unproven.
Instrumented capture now records Chrome receiving the attempt and dropping it
before creating a DownloadItem: Download.Counts buckets 28 and 31 each rose by
one, with no download events or handler exception. Chromium defines bucket 31
as a content-setting/request-limiter drop. This narrows the cause; it does not
identify the exact effective setting. Trace button cleanup passed.
The source classifier now preserves download codes without falsely labeling
missing/canceled events as ownership failures; explicit identity failures retain
lifecycle_owner. All 16 focused tests, fmt and clippy pass. This fix is not installed.
Source fix pushed at 028597ea. Cold CLI reopen passes headless (Bk7rtr) and
headed with fill/click (KTEIFz); same profile and exact process exit verified.
Fresh doctor exits 0 after one manual reconciliation refreshed stale monitoring;
no generations removed, five browsers preserved, timer inactive. Evidence:
current-readiness-20260908/. Original consumer handle remains disconnected;
its own retry was requested. Consumer recovery and A1/A3/AX before A4 stay open.

### Current installed identity

- Source: `bf4de8aa14b91bfb1596da89f545fb79516a8961`; host PID32075.
- Generation: `0.28.0-b69c4e5a4a60-31faac73c7ec`.
- Binary SHA256: `b69c4e5a4a600ebc75be8869d662fc8fc7fd5633169380e804c52754d4bf7b18`.
- Support SHA256: `31faac73c7ecd0d6f0b2f0b79485446a23f5b78f879edcb383093148124ae5a7`.
- Shared skill matches installed source; separate Service skill unchanged.
- Immediate rollback `0.28.0-18b4c398fabd-051204a349a5` is held. Eleven holds remain.

### A1: installed identity repairs and remaining boundary

Earlier rejoin, missing-binding and MCP routing repairs remain in the installed
candidate. Git `d6f01b00` preserves the detailed history and scoped older proofs.
Prior installed proof is indexed in `installed-15cd8e5b-a1/acceptance-matrix.json`: exact
binary, fixture driver and ledger hashes, distinct cases, cleanup and explicit
gaps. These are disposable test-owned clients, not consumer impersonation.

Seven current lease findings span six rows: two legacy-principal warnings,
three missing owner bindings on Default, and an owner-generation/session-authority
pair on stealthcdp-default. All four affected profiles have active shared-local
policies. These maintenance warnings alone do not establish an ordinary request
refusal. The original synthetic profile has complete attestation and successful
control despite its legacy warning. Earlier exact-release fixture `retained-unit-sim-NdcCp4`
passed ordinary control before rejoin, sealed rejoin with its original credential,
active lease, complete attestation, two original handles and retained storage.
Cleanup left zero owned processes. Private `ownership-dispositions.json` assigns
all six records; combined identity recovery now has isolated exact-binary proof. Consumer capabilities were not borrowed to
apply ownership changes or claim their workflows accepted.

The stored consumer `owner_connection_still_active` refusal was reproduced after
abrupt disposable-host death. Source `3dfd4d09` binds new connection IDs to boot,
PID namespace, PID and start evidence. Retained handles recover only with positive
host-death proof and unchanged subjects/permissions. Legacy repair subsequently
used exact pre-publication custody plus a fresh producer census to disconnect
one proven dead transport. Consumer permissions, owner/lease authority and all
five browsers were preserved. See `legacy-connection-repair/` and Git `4c3d9156`
for the detailed history. Actual consumer reconnect remains unverified.

Earlier download, tab-scope and fixture-oracle evidence is retained at Git
`bf4de8aa:RUNBOOK.md` and its private fixture links. Historical wrong-tab cause
and independent post-close census remain unproven; do not erase these obligations.

### Storage recovery and prevention

Source `bee768ed` supplies the installed private, mode-0700 state-backed `/tmp`
and `/var/tmp` mounts. Thirteen supervisor tests and a real Chrome synthetic CSV
passed after parent retirement. Recovery repaired six deleted temporary mounts
across three exact namespaces while preserving five original browser processes.
Publications preserved those mounts and tab custody; new-host backing is verified. Detailed recovery history remains at Git
checkpoint `dd94dfe7` and in the private ownership-bound recovery receipts.

Keep `runtime-tmp/recovered` backing directories while retained namespaces
reference them. The template does not repair already-deleted mounts.
The historical Playwright Node process no longer exists; its artifact directory
was not recreated. Synthetic retained-browser native transport now passes while
preserving context policy; actual consumer CSV is unverified. The private recovery helper
still needs a reusable product diagnosis/recovery surface.

### A2 local acceptance and remaining A3/A4/AX work

- A2: current txWCsl passes two viewers, pixels, same-link reconnect and trusted
  input on bf4de8aa. Earlier 0LgorK/L4sk1e failures remain historical evidence.
  Original target/attestation survive. Plan0158 external vantage remains separate.
- A3: doctor exits 0 after one supported maintenance pass. Nothing was removed.
  Seven advisory lease findings remain; consumer recovery is not established by
  a green doctor. Keep their existing evidence-backed dispositions visible.
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

Earlier disposable cleanup corrected false zero-residue receipts and reduced
observed RSS below 6 GiB while preserving all five production browsers. Detailed
history is retained at `e17214e6` and private cleanup receipts.

The earlier maintenance pass removed 47 closed-tab history rows and 27 reviewed,
backed-up generations. No browser processes, profiles, sessions or displays were
removed. Fresh monitoring is healthy; unknown-pressure readiness cleared.
Doctor then exposed a contract mismatch: GC ignores terminal failed transaction
payload references, while rollback readiness still requires its old generation.
Restored that exact 32-file payload from verified backup and applied a supported
retention hold. Doctor then exited 0; its GC preview retained three generations and had
zero candidates. Net removed generations: 26. No accepted transaction was forged.
Keep all eleven holds and original recovery storage. The installed GC repair now
retains the latest transaction rollback dependency. Current generation GC preview
has zero candidates; unrelated older history stays reclaimable.

Pending acquisition and orphaned route capacity retention now pass in source,
isolated release and installed durable-link checkout. Actual consumer reconnect
and artifact transport remain pending; preserve ordinary self-identification.
For Guacamole input, check refresh/reconnect, effective settings, and scoped
browser/provider recovery in a disposable session before code diagnosis.
Do not rebuild without a new demonstrated source defect. After A1–A3 pass,
observe three scheduled cycles, a controlled restart and its next cycle using
original handles and the durable handoff URL. Plan0158 external-vantage dispatch
boundaries remain unchanged.

### Evidence index

Private evidence root: `~/.local/state/agent-browser/campaigns/p160/`.
- `publication-bf4de8aa/`: build, five-case qualification, activation, doctor0,
  input acceptance, failed installed CSV, click probe and headed comparison.
- `primary-lock-diagnosis/`: bounded read-only lock telemetry and causal limits.
- `installed-15cd8e5b-a1/`: acceptance matrix plus cold-lifecycle disposition;
  three failed setup attempts, their zero-residue proof and the scoped AX join.
- `session-cold-lifecycle-wOiz0e/dashboard-readback-receipt.json`: authenticated
  journal lookup across dashboard restarts; original failure and job preserved.
- `publication-15cd8e5b/`: release/activation, supported holds, verified backups,
  maintenance result/preservation, historical rollback restoration, final doctor,
  zero-candidate GC preview and `viewer-failure-disposition.json`.
- `publication-700ffa8a/`: installed acceptance, preservation, maintenance and doctor.
- `a2-operator-journey-r2/access-grant-attempt/local-viewers-ZP6oBA/`: full local journey.
- `publication-ac277ea0/` and `local-viewers-sJv4NQ/`: previous release and full
  local two-viewer, reconnect and trusted Guacamole input evidence.

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

## History index

- [Complete previous runbook, through Turn 213](RUNBOOK-history-through-2026-09-02.md)
- Git checkpoint `50b05156` preserves Turn214's installed cold-viewer acceptance.
- Git checkpoint `dd94dfe7` preserves Turn216's live storage recovery.
- Git checkpoint `c06ea5e8` preserves the first publication in this hour.
