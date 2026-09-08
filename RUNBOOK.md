# Runbook

This is the sole current execution status. Plans own acceptance and strategy;
Git checkpoints and linked archives preserve history. Keep this file at or below
200 lines under [policy 0043](docs/dev/policies/0043-roadmap-runbook-governance.md).

## Turn 249 | 2026-09-07

Authority: [Plan 0160](docs/dev/plans/0160-2026-09-06-production-profile-identity-and-operational-readiness.md).
Blocker reduction: isolated replay reproduced the retained-route selector dropping
an orphaned route with a pending display. The existing acquisition planner already
supports same-owner recovery; the earlier selector prevented it from seeing :14.
Source now retains that route, including an available pool entry naming its display,
only when current-boot browser/display/route custody matches. Five foreign or stale
cases refuse unchanged. The real planner selects the retained display and still
requires physical visibility proof; selection never marks pending state ready.
Regression failed before repair; 169 remote-view tests, fmt, clippy and docs pass.
Two optimized test builds took 253/255s; the passing group took 0.11s.
Repair is not installed. Private `retained-route-selection-repair.json` indexes proof.
Next: qualified candidate build/publication, then original durable-link recovery.
Production remains `7b00d6a0`: original handle control and attestation pass, but
remote-view recovery refuses wrong display :12. Current doctor is not accepted.
No live retry occurred during this source repair. Actual consumer recovery and
A1–A4/AX remain OPEN; preserve five browsers and keep the maintenance timer disabled.
Prior installation, rollback and cumulative effort remain at `92be586c`.

### Current installed identity

- Source: `7b00d6a07b98f4ebd8ac1c1c35b25cfd2c17626c`.
- Generation: `0.28.0-b5c3fb971ec8-016a21729553`.
- Binary SHA256: `b5c3fb971ec88747fd026b3a171e575becccfe4076aa8d8f026e3cff56495343`.
- Support SHA256: `016a217295535207df181119e8b19bbde2a64e4cb89af8184b875bb0340ed134`.
- Host PID61036; activation and rollback preserved five exact browser identities
  and 30 open-tab custody records. The selector uses the supported relative path.
- Original synthetic handle retains complete attestation and successful control.
- General shared skill synced to source6140e35b with verified prior-version backup.
- Immediate rollback `0.28.0-8194a64b02ef-c65cd6d981c3` has 32 verified backup files.
- Four rollback holds remain, including the three preceding generations already
  retained at `bd668544`. Maintenance timer stays disabled.

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

Current source `ac277ea0` also passed native synthetic CSV capture and client
artifact retrieval after disposable host retirement (`retained-unit-sim-xNVafa`).
An immediate Blob revocation did not prevent the download. A second fixture
(`retained-unit-sim-vfstsf`) emitted a peer download first; both artifacts existed
separately and the primary request returned its own bytes. This does not prove
shared default-context policy ownership or ordinary Playwright namespace transfer.
The three-tab fixture proved attach/detach and physical release of only the
intended target. Its next assertion wrongly assumed top-level targetId was
unsupported; the current schema supports it and the response named the requested
target. `retained-unit-sim-3dOUr1/disposition.json` records that limit. Historical
wrong-tab cause and independent post-close census remain unproven. Two earlier
fixture setup errors are preserved. MeOiTT cleanup falsely reported zero because
Chrome flattened argv and erased HOME. Corrected cleanup now proves no residue.

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
was not recreated. Actual consumer CSV, new-client artifact transport and shared
browser-context download policy remain unaccepted. The private recovery helper
still needs a reusable product diagnosis/recovery surface.

### A2–A4 and AX remain open

- A2: prior `local-viewers-PVUWLr` passed viewers and same-link refresh, then stopped
  on moving input geometry. The next attempt exposed completed acquisition with
  pending display and orphaned route. Stale reconciliation is now repaired.
  Current `local-viewers-osNeRt` returned `route_display_owner_unproven` for :12;
  the retained browser is on :14. Request correlation and compensation are recorded.
  Current operator journey and input remain unaccepted. Do not reuse earlier passes.
- A3: current doctor exits 1 for operator journey and stale monitoring. Original
  handle control is proven separately. Seven lease warnings remain advisory.
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
Keep all four holds and original recovery storage. The installed GC repair now
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
- `installed-15cd8e5b-a1/`: acceptance matrix plus cold-lifecycle disposition;
  three failed setup attempts, their zero-residue proof and the scoped AX join.
- `session-cold-lifecycle-wOiz0e/dashboard-readback-receipt.json`: authenticated
  journal lookup across dashboard restarts; original failure and job preserved.
- `publication-15cd8e5b/`: release/activation, supported holds, verified backups,
  maintenance result/preservation, historical rollback restoration, final doctor,
  zero-candidate GC preview and `viewer-failure-disposition.json`.
- `a2-operator-journey-r2/access-grant-attempt/local-viewers-YpB5BS/`:
  current view/reconnect proof and preserved keyboard crop failure.
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
