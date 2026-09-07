# Runbook

This is the sole current execution status. Plans own acceptance and strategy;
Git checkpoints and linked archives preserve history. Keep this file at or below
200 lines under [policy 0043](docs/dev/policies/0043-roadmap-runbook-governance.md).

## Turn 218 | 2026-09-07

Authority: [Plan 0160](docs/dev/plans/0160-2026-09-06-production-profile-identity-and-operational-readiness.md),
lane P157, branch `plan/profile-permissions-and-request-provenance`.
Previous publication history is preserved at `f269ce64`. This successor began
21:27Z with a 30-minute ceiling through 21:57Z; all previous effort is cumulative.
Recent status replies were no progress. This turn advanced AX with real MCP
failure-to-job correlation and reproduced an A1 stale-connection refusal after
abrupt host interruption. No production publication in this batch.
Source `a9830cfa` is qualified in isolation; production remains on `d6d99e43`.
The expanded existing regression failed before repair, then all 129 MCP tests,
format, workspace Clippy and API parity passed. Optimized build passed.
No build or fixture remains running. Full A1–A4/AX readiness remains OPEN.
No consumer capability was borrowed, profile lease reassigned or consumer browser
restarted. Unrelated modified note0156 and untracked note0159 remain preserved.

### Current installed identity

- Source: `d6d99e4387412ce731b145b6ef950c6779dbfe55`.
- Generation: `0.28.0-7991b404eafe-8362848f8b0b`.
- Binary SHA256: `7991b404eafe0e7a81ac14f6eff6065b6162791195dda1252a15f2f9dc7cf6fb`.
- Support SHA256: `8362848f8b0bbd0e4d9eb17cbe35cbc0ac41ba2fd814756adafdd608c4488e11`.
- Activation observed new host PID14552 at 18:06Z, preserving five exact live
  browser process identities, all six recovered namespace mounts and 70
  tab-custody records. No jobs were active at either admission check.
- Authenticated installed-manifest and original synthetic-handle readback pass.
  Attestation is complete; synthetic mouse/keyboard counters remain 9/9.
- The main shared skill matches repository source. The separate Service skill
  and its lock-diagnostic guidance were preserved. No formal release or external
  notification occurred.

### A1: installed identity repairs and remaining boundary

The earlier installed `dc570e5b` repair fixed `no_safe_reconciliation_transition`
when the exact lease already authorized rejoin. The planner seals one
`rejoin_owned_browser` transition and apply reuses unique, uncontested owner,
session and tab guards. Earlier exact-release proof retained both original
handles across host interruption, restored active lease and complete attestation,
and correlated the returned receipt with the persisted plan and lease.

The first candidate this hour, `dd94dfe7`, combined that repair with diagnostic
and private-storage changes. Its exact release passed original-capability,
two-handle reconciliation across a disposable systemd host interruption, with
unchanged browser/targets and persistent temporary storage. Self-identified HTTP
control also passed without a registered capability. Two initial systemd fixture
errors were corrected: default group shutdown terminated Chrome, and recreating
a still-loaded transient unit failed. Their evidence and cleanup are retained.

MCP then reproduced a different real refusal on that installed binary: opening a
tab succeeded, but a subsequent request supplying its valid handle alone was
routed to the MCP default session. The daemon rejected the browser/session
mismatch. Source `d6d99e43` now derives missing MCP route hints from the original
handle, retaining explicit selectors and all daemon custody/permission checks.

The final exact release passed MCP self-identified handle-only diagnostics,
original-handle control after client closure/reconnection, foreign-subject denial
with no page change, explicit conflicting-route denial, and exact owned-target
release with peer tabs and browser process preserved. The earlier optimized
candidate passed the same identity/control checks. The expanded existing routing
regression, existing desktop-routing regression, format, workspace Clippy and
API/MCP parity all pass. One test invocation selected zero tests and was not
counted; the corrected invocation executed the regression. Fixture request-shape
errors are retained separately from the red product routing failure and green
candidate result. All fixture process cleanup checks passed.

Seven current lease findings span six rows: two legacy-principal warnings,
three missing owner bindings on Default, and an owner-generation/session-authority
pair on stealthcdp-default. All four affected profiles have active shared-local
policies. These maintenance warnings alone do not establish an ordinary request
refusal. The original synthetic profile has complete attestation and successful
control despite its legacy warning. Consumer capabilities were not borrowed to
apply ownership changes or claim their workflows accepted.

The stored consumer `owner_connection_still_active` refusal has matching
self-declared subjects and both permissions. Fresh readback finds the identical
historical owner-connection hash still marked active after host replacement.
The isolated `retained-unit-sim-Q2Hc69` reproduction held an authenticated socket
open, killed only its disposable host, preserved Chrome, then restarted the host.
The original handle was refused with that same typed error because the dead
connection remained active. Normal connection teardown passes; abrupt host death
bypasses its disconnect guard. This reproduces the failure mode, while the exact
historical interruption remains unproven. Repair needs positive host-lifetime
proof so dead connections can be retired without taking over live owners.
Consumer acceptance and actual consumer artifact transport remain incomplete.

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
  This repair is not installed in production yet. Full AX remains open.

### Retention and next work

The earlier GC preview listed 22 candidates and `previousHealthyGenerationId=null`.
That preview is historical after two additional publications. No GC or workstation
reconcile was applied. Original generations and verified private rollback copies
remain. Each publication backed up its immediate predecessor with 32 matching
file hashes. The final predecessor is `0.28.0-13d426fb15c9-bb2f5ae53042`.

Controlled publication receipts remain outside the supported upgrade ledger.
Repair the supported publication/retention path before enabling maintenance;
never fabricate accepted transactions or green monitor receipts to protect a
rollback target. Preserve original recovery storage through any future change.

Next repair dead-host connection lifecycle using the reproduced A1 failure,
preserving denial for live foreign connections. Keep self-identification as the
ordinary default. Complete consumer artifact transport and A1 dispositions.
For Guacamole input, check refresh/reconnect, effective settings, and scoped
browser/provider recovery in a disposable session before code diagnosis.
Consolidate repairs before another production candidate. After A1–A3 pass,
observe three scheduled cycles, a controlled restart and its next cycle using
original handles and the durable handoff URL. Plan0158 external-vantage dispatch
boundaries remain unchanged.

### Evidence index

Private evidence root: `~/.local/state/agent-browser/campaigns/p160/`.

- `retained-unit-sim-gSifp9/`: optimized a9830cfa real MCP error correlation,
  exact persisted outcomes, ordinary reconnect/control, and zero-residue cleanup;
  private `consumer-stale-connection-readback.json` anchors historical comparison.
- `retained-unit-sim-Q2Hc69/`: abrupt host death with active socket reproduces
  stale active ownership and original-handle refusal; original Chrome preserved;
  cleanup terminated three fixture-owned processes, zero remaining.
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
