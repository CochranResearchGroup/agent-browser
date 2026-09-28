# Plan 0219 | Alice/Bob Grilling Contract Completion

Date: 2026-09-26

Plan version: 50

State: OPEN

Consolidation: required

Product lane: PL-PLATFORM

Lane: P219

Predecessor: [Plan 0218](0218-2026-09-23-grilling-contract-remote-view-conformance.md), superseded while incomplete

Work items: `CochranResearchGroup/agent-browser#181`, `CochranResearchGroup/agent-browser#183`, and `CochranResearchGroup/agent-browser#195`; issues #189 and #190 remain regression subcases of #195

Branch: `platform/p211-simple-cold-upgrade`, retaining P211, P217, and P218 custody

Pull request: [draft PR #191](https://github.com/CochranResearchGroup/agent-browser/pull/191)

Target: `main`

Execution baseline: `8bb9518eb78b37c22aaec0183348ee6c77adf0da`; successor planning began at `6b3a41e265d1fd3da27431788e4373b66177225d`

Execution owner: the primary agent assigned by the operator to this inherited lane

Authority: the operator directed execution of Plan 0219 on 2026-09-26; ordinary in-scope implementation, validation, isolated development runtime effects, and bounded repair are authorized under this plan's controls and non-goals

Active effort ceiling: 1,000,000 cumulative tokens for the operator-resumed
continuation; stop sustained implementation at 800,000 and reserve 200,000 for
reconciliation, validation, evidence, and checkpoint custody. Earlier P219
windows and every predecessor attempt, review, failure, and receipt remain
historical evidence and do not increase this ceiling.

Policy capsule: [P219 M1B through M2B](../policy-capsules/p219-m1b-m2b.md) is historical and exhausted at the M2B outcome stop; the continuation below requires a reconciled policy basis before implementation

## Objective

Complete the September 19 Alice/Bob grilling specification in the trusted
single-user runtime. A valid ordinary instruction opens or recovers a usable
browser and authenticated remote view unless a concrete current resource,
configuration, authentication, or execution failure prevents it.

Alice and Bob can share one healthy exact-profile browser while retaining
distinct sessions, tabs, targets, handoffs, activity times, and expiry. Commands
and cleanup affect the addressed session. The final active session closes the
browser. One SQLite authority, the existing runtime host, a provider-owned
presentation service, and Desktop Services own the complete workflow.

Historical uncertainty remains diagnostic and can protect an exact resource
from destructive cleanup. It cannot deny a separate valid request, reserve
phantom capacity, or substitute a different profile. Session heartbeat, live
authenticated viewer heartbeat, and Desktop Services control remain distinct.

## Specification And Authority

The original design authority is Codex thread
`01a0b65d-47f9-7b51-a17b-791ee87769b3`, accepted design turns 207 through 349.
This plan incorporates, unchanged, P218's normative sections at the source
baseline:

- [Grilling Contract Ledger, G01 through G45](0218-2026-09-23-grilling-contract-remote-view-conformance.md#grilling-contract-ledger).
- [Simple Session State Machine](0218-2026-09-23-grilling-contract-remote-view-conformance.md#simple-session-state-machine).
- [Architectural Prohibitions, P01 through P19](0218-2026-09-23-grilling-contract-remote-view-conformance.md#architectural-prohibitions).

Those sections remain the specification even though P218 is superseded. This
plan replaces P218's execution sequence, not its requirements. Any conflict
between a packet summary and a normative row resolves in favor of that row.
Changing a requirement still needs explicit operator direction.

[ROADMAP.md](../../../ROADMAP.md) owns priority,
[RUNBOOK.md](../../../RUNBOOK.md) owns current execution and cumulative control
state, and the [lane catalog](../active-lanes.yaml) records inherited custody.
P218 and the [archived runbook](../../../RUNBOOK-history-2026-09-26-through-p218.md)
preserve history. Keep new execution narratives in the runbook.

## Current State

Version 50 installs exact development generation `0.28.0-d630713aca50` and
closes G30 and G39. The first syscall audit exposed that the disposable
install-doctor Service probe inherited development runtime-host authority and
touched the shared startup-lock and SQLite descriptor namespace. Commit
`26898035` now gives that child an explicit disposable HOME, state directory,
socket directory, and disabled runtime-host authority. Its active regression,
formatting, strict workspace Clippy, workstation fixture, architecture self-test
and zero-finding cut pass. The repaired candidate passes the required
three-cycle development browser-launch smoke and development doctor with six
ready provider routes and unchanged production identity. Installed install and
remote-view doctors then complete under syscall tracing with zero write,
rename, create, or removal effects in persistent development state or the
shared runtime namespace; disposable captures leave no process or directory
residue. Service status and install doctor expose the same complete
`agent-browser.runtime-operational-status.v1` field map, including stable
unavailable privileged-receipt state. A fresh OS census finds no doctor or
smoke residue. The ledger advances to 15 pass, 28 partial, 2 fail, and 0
missing. No production, privileged, merge, push, or release effect occurred.

Version 49 closes G39's source-side explicit-repair inventory at `d5e5bab7`.
Every install-doctor `issues[].remedy` now comes from one closed typed builder
using the shared `agent-browser.doctor-repair-recommendation.v1` schema, an
exact action ID, a `read_only`, `explicit_effect`, or `manual_plan` execution
class, and `automaticExecutionAllowed=false`. A negative-tested architecture
guard rejects literal remedy objects outside that builder. The active focused
Rust contract, architecture self-test and zero-finding cut, formatting, strict
workspace Clippy, documentation build and remote-view contract, and every
selector-chosen provider-free workstation and PostgreSQL fixture pass. The
selector's legacy `workstation_payload_status` filter executes zero tests and
is not counted as proof. G39 remains partial only for complete installed
doctor no-mutation qualification. The ledger remains 13 pass, 30 partial,
2 fail, and 0 missing. No runtime, browser, provider, privileged, production,
merge, push, or release effect occurred.

Version 48 advances G39's explicit-repair inventory at `92b2155f`.
Remote-view doctor now returns every `nextCommand` through the
`agent-browser.doctor-repair-recommendation.v1` contract with an exact action
ID, a closed `read_only`, `explicit_effect`, `live_acceptance`, or
`unclassified` execution class, and `automaticExecutionAllowed=false`.
Unknown actions remain unclassified and cannot acquire effect authority from a
command string. The exhaustive focused contract, four existing recommendation
tests, formatting, strict workspace Clippy, docs build and remote-view contract,
and selector-chosen provider-free workstation and PostgreSQL fixtures pass.
G39 remains partial for equivalent typing of install-doctor issue remedies and
complete installed no-mutation qualification. The ledger remains 13 pass, 30
partial, 2 fail, and 0 missing. No runtime, browser, provider, privileged,
production, merge, push, or release effect occurred.

Version 47 reconciles two stale machine-readable ledger narratives without
changing their dispositions. G18 now records the already-qualified M4-P1
integrity, WAL-aware budgets, verified online backup rotation, corrupt-copy
preservation, manifest, and restoration-gap projection at `4ed5820d`; only the
exact frozen-candidate installed recovery proof remains. G34 now records the
M4-P5 exact owned-provider namespace rebuild and unrelated-row PostgreSQL
preservation proof at `7e34acce`; only the exact installed forward-only cold
upgrade and resulting topology proof remain. The ledger stays 13 pass, 30
partial, 2 fail, and 0 missing. This reconciliation performs no runtime,
browser, provider, privileged, production, merge, push, or release effect.

Version 46 advances G39's source proof at `c5f4db2a`. The architecture report
now has a dedicated `doctorReadOnly` cut over both install doctor and
remote-view doctor. It rejects direct filesystem mutation, runtime
reconciliation, migration, backup creation, privileged-receipt persistence,
and effect-capable `--apply` invocation from either doctor path. Its negative
fixture demonstrates failure on a receipt-persistence call, its clean fixture
passes, and the current repository reports zero findings. G39 remains partial
for installed no-mutation qualification and reconciliation of every remaining
explicit repair command to a typed narrow-effect boundary. The ledger remains
13 pass, 30 partial, 2 fail, and 0 missing. No runtime, browser, provider,
privileged, production, merge, push, or release effect occurred.

Version 45 source-qualifies G30's typed privileged-repair receipt projection at
`7d043e45`. A successful privileged adapter invocation persists only a fully
validated v2 receipt through a user-private atomic replacement. Service status
and install doctor share the same redacted readback even when Browser Runtime
SQLite is unavailable; the sealed plan digest and action list never enter the
projection. Missing, unreadable, semantically invalid, or not-ready receipts
fail closed behind stable codes. Install doctor also prints the receipt state.
Focused receipt and Service-status tests, formatting, strict workspace Clippy,
the documentation build and remote-view contract, and every selector-chosen
provider-free workstation and PostgreSQL fixture pass. G30 remains partial only
for complete installed status/doctor qualification. The ledger stays 13 pass,
30 partial, 2 fail, and 0 missing. No runtime, browser, provider, privileged,
production, merge, push, or release effect occurred.

Version 44 advances G30's source projection. Service status and install doctor
now share one read-only SQLite reconciliation aggregate covering configured
limits, keeper phases and generations, browsers per display, handoff, operation
and queue counts, current control and viewer slots, recovery phases, migration,
storage/backup integrity, and current launch pressure. The projection contains
no provider URL, route user, credential, or Guacamole connection identity. Its
focused regression proves the database digest and backup directory remain
unchanged. G30 remains partial until the typed privileged repair receipt joins
the aggregate and the complete installed readback is qualified. The ledger
remains 13 pass, 30 partial, 2 fail, and 0 missing. No runtime, browser,
provider, privileged, production, merge, push, or release effect occurred.

Version 43 closes G29. Every real runtime-config mutation now appends a bounded
128-entry change record in the same SQLite transaction as the new revision and
keeper policy. Each record contains the prior revision, committed revision,
timestamp, and exact changed fields; invalid and repeated identical patches add
nothing, and restart preserves the history. Authoritative config get/update
readback and the generated client expose the history. Existing placement logic
reports over-target before effects, rejects worsening admission without moving
or closing retained work, and returns to full or available through ordinary
release. The focused ten-test runtime-config lane, formatting, strict workspace
Clippy, generated-client drift and type checks, direct service-client checks,
service API/MCP parity, route-confusion gates, documentation build and links,
coverage validator, and selector-chosen workstation and PostgreSQL fixtures all
pass. The umbrella service-client command still stops before the changed lane
at the inherited missing P157 oracle source; every downstream check was run
directly and passed. The development skill is current; the shared production
skill remains untouched. The ledger advances to 13 pass, 30 partial, 2 fail,
and 0 missing. No runtime, browser, provider, privileged, production, merge,
push, or release effect occurred.

Version 42 closes G28's source-side final-reference gap. The immediate SQLite
scale-in reservation now joins Browser Session Manager placements, durable
handoffs, current Desktop Services control, unexpired authenticated same-boot
viewers, pending operation records, presentation admission, keeper phases, and
the configured cooldown before reserving one exact route stop. Incomplete or
cross-boot control/viewer state fails closed. Focused regressions prove viewer
expiry semantics and prove the final reservation preserves the viewed and
controlled route while selecting the unreferenced route. All four scale-in
tests, formatting, strict workspace Clippy, and patch hygiene pass. G28 remains
partial only for joined installed-provider scale-in after cooldown; ledger
counts remain 12 pass, 31 partial, 2 fail, and 0 missing. No runtime, provider,
browser, privileged, production, merge, push, or release effect occurred.

Version 41 closes G10 at source. The durable presentation queue now preserves
strict recovery, retained-handoff, then new-open priority classes; FIFO sequence
order ages requests within a class without allowing an old new-open request to
overtake recovery. The existing SQLite adapter supplies the configurable depth
with a default of 32 and coalesces matching exact operation IDs and payloads
before effects, then replays one completed result. Logical session and tab
coalescing remains at the already-accepted G31 manager and handoff boundary, so
the queue does not collapse genuinely distinct operations merely because they
share a profile. The focused 12-test queue suite, formatting, strict workspace
Clippy, coverage-ledger validator, generated-client checks, type coverage, and
service API/MCP parity pass. A row-by-row audit also repairs five stale ledger
dispositions that had not incorporated already-qualified M3-P3 and M4-P2
through M4-P4 checkpoints: G17, G20, and G33 are partial, while G37 and G38 are
pass. With G10, the authoritative ledger is now 12 pass, 31 partial, 2 fail,
and 0 missing, matching the prior checkpoint narrative. This
source-only correction changes no installed generation or provider state, and
the exact workstation cold-upgrade reconstruction remains the next G34 gate.

Version 40 publishes the final M4 source candidate to isolated development
generation `0.28.0-3e4532b3d100`, whose full binary digest is
`3e4532b3d100b25e8dc62b54de6394f2ad39794bdd46b06182c4c1892ecdc768`.
The development skill is current, every development doctor check passes, all
six provider routes remain ready, and the installer reports the production
generation and tracked production state unchanged. A fresh OS process census
records the three development service processes separately from substantial
pre-existing production and foreign browser trees; no broad cleanup occurred.

This publication proves installed candidate identity and isolated runtime
readiness only. It does not execute or prove the exact workstation cold-upgrade
provider-row reconstruction path, three zero-process starts, capacity timeout,
or wider M4 acceptance. G34 and the ledger therefore remain unchanged at 11
pass, 32 partial, 2 fail, and 0 missing. No production, privileged, merge, or
release effect occurred.

Version 39 source-qualifies the M4-P5 provider-row reconstruction primitive at
commit `7e34acce`. Cold workstation reconciliation requests an explicit rebuild
of the exact Agent Browser-owned Guacamole namespace before recreating the
header user, canonical routes, sharing profiles, parameters, permissions, and
the authoritative SQLite route-pool projection from retained inputs. The
renderer deletes only configured canonical and legacy connection names,
configured Agent Browser sharing-profile names, and the exact configured header
user inside one transaction before ordinary reconstruction.

A disposable PostgreSQL 16 fixture executes the complete generated transaction
against the packaged Guacamole schema. It proves canonical reconstruction and
exact preservation of an unrelated connection, parameter, sharing profile,
permission, entity, and user. CLI help, README, repository skill, inline source,
and docs-site guidance now describe the bounded rebuild. The focused renderer,
source-free workstation install, host provisioning, fresh-VM harness,
Guacamole assets and durability, route synchronization, documentation,
validation-selection, release-fixture, formatting, focused Rust, and strict
Clippy gates pass. G34 remains partial until the separately governed installed
cold-upgrade reconstruction succeeds, so the ledger stays 11 pass, 32 partial,
2 fail, and 0 missing. No installed runtime, browser, provider, route, display,
privileged, publication, production, merge, or release effect occurred.

Version 38 is the intermediate M4-P5 custody checkpoint at commit `8dcbfd89`.
It introduced the bounded renderer and cold-install call sequence but retained
documentation, execution-level unrelated-row preservation, full source gates,
and installed reconstruction as open requirements.

Version 37 source-qualifies M4-P4 at commit `9f6505b7`. The privilege
installer now distinguishes a healthy cold start from a repair before entering
the privileged boundary. A healthy rerun checks root-owned helper and sudoers
metadata, the helper's non-root capability report, protected lease-authority
readiness, group membership, and requested workstation dependencies without
executing `sudo`; compatible helper provenance drift remains allowed. A repair
still uses the single explicit authorization and sealed narrow action list.
The validated v2 receipt binds the host-privilege resource, prior observation,
selected action, outcome, and ready postcondition plus the exact sealed plan.

Both clean privilege-install and complete workstation-host fixtures pass and
assert that healthy reruns add zero privileged commands, including when the
AppArmor loaded-profile registry is protected. The focused Rust receipt test,
shell syntax, documentation checks and build, formatting, and strict workspace
Clippy pass. G20 advances from missing to partial pending final installed
first-install, healthy-rerun, and exact-repair qualification. The ledger is 11
pass, 32 partial, 2 fail, and 0 missing. No installed runtime, browser,
provider, route, display, privileged, publication, production, merge, or
release effect occurred. M4-P5 owns provider-row reconstruction and remaining
placement, queue, scale-in, doctor, and installed operational qualification.

Version 36 source-qualifies M4-P3B at commit `9aad8500`. Every Browser
Session Manager Chrome launch now takes a fresh cross-platform host snapshot
immediately before display access or browser effects. Admission checks
available memory, free space on the profile filesystem, current host process
count, the Linux PID ceiling when available, and managed root Chrome/Chromium
processes against configured browser capacity. Missing observations or breached
floors fail closed with typed `browser_launch_resource_pressure` reasons.
Aggregate Service status and install doctor expose the same typed observation
under `browserRuntime.launchAdmission`; an unavailable SQLite authority still
publishes the current host observation without revealing a local path. Foreign
Chrome processes do not consume the managed-process quota.

Focused admission, read-only store, and status-projection tests pass, as do
generated-client drift and type gates, API/MCP parity, documentation checks and
build, formatting, and strict workspace Clippy. A Windows cross-check stopped
before compiling this source because the WSL host lacks the MinGW C compiler;
target-platform execution therefore remains an installed-qualification gate.
G33 advances from missing to partial, and G30 and G39 remain partial until the
final installed status/doctor and full-resource qualification. The ledger is
11 pass, 31 partial, 2 fail, and 1 missing. No installed runtime, browser,
provider, route, display, privileged, publication, production, merge, or
release effect occurred. M4-P4 owns G20 narrow privilege qualification and the
remaining provider reconstruction and installed operational gates follow.

Version 35 source-qualifies M4-P3A at commit `91cb79c8`. Aggregate Service
status now includes `browserRuntime`, and install doctor includes the same
projection under `data.browserRuntime` plus concise text fields. Both surfaces
open the existing SQLite authority read-only, run no migration or backup, and
report typed configuration, migration counts and archive state, integrity,
database/WAL/history/backup bytes, budget state, backup verification, and
restoration gaps. An unavailable database reports only a stable failure code;
the local path is redacted. The read-only regression preserves the database
digest and creates no backup. Status projection remains non-mutating and the
generated Service client and response schema carry the additive field.
Focused store and status tests, generated-client drift and type gates, API/MCP
parity, documentation checks and build, formatting, and strict workspace
Clippy pass. G30 and G39 gain bounded source evidence but remain partial until
the full required-field map and whole-doctor effect audit pass. The ledger
remains 11 pass, 30 partial, 2 fail, and 2 missing. No installed, browser,
provider, route, display, privileged, publication, production, merge, or
release effect occurred. M4-P3B owns current memory/process/disk launch
admission and the remaining operational field map.

Version 34 source-qualifies M4-P2 at commit `fe46f53c`. Browser Session State
now retains exact navigation rows up to the live SQLite-owned byte limit and
then deterministically selects the oldest rows for same-transaction compaction.
Daily UTC summaries are keyed by exact profile, session, browser, tab, and
target identity and retain first and last URL, first and last timestamp, count,
and sorted incident references. Repeated compaction merges independently of
batching, exact rows remain within budget, and a bounded 512-entry event trail
records before/after bytes and affected counts. The daemon refreshes the live
limit before ordinary browser-session work. Legacy state decodes with empty
summary/event defaults; restart preserves both summaries and compaction
receipts. Bodies, screenshots, heartbeats, raw logs, and repeated polls never
enter this history surface. The complete 283-test service-model suite and the
SQLite host publication fixture pass, as do minimum-limit validation, API/MCP
parity, documentation checks and build, formatting, and strict workspace
Clippy. G17 advances from missing to partial because indefinite material
lifecycle and recovery-event retention remains unproved; G16 remains partial.
The ledger is now 11 pass, 30 partial, 2 fail, and 2 missing. No installed,
browser, provider, route, display, publication, production, merge, or release
effect occurred. M4-P3 begins with the remaining operational status, doctor,
and current-resource-pressure audit.

Version 33 source-qualifies M4-P1 at commit `4ed5820d`. The strict SQLite
runtime patch, Service schema, MCP schema, generated client, CLI help, README,
repository skill, and docs now expose the accepted 64-MiB exact-history,
96-MiB live-database, and 128-MiB routine-storage settings. Runtime-config
readback runs a read-only SQLite quick check and reports database, WAL,
exact-history, current-backup, and previous-backup bytes with typed budget and
restoration-gap state. The explicit `service runtime-config backup` repair uses
SQLite online backup, verifies the staged schema and integrity before atomic
rotation, retains at most current and previous copies, and publishes a strict
digest-and-size manifest. It never restores or deletes the live database.
Restart, rotation, missing-backup, and corrupt-copy rejection fixtures pass.
Focused runtime-config and service-config tests, contract metadata, generated
client checks, API/MCP parity, type coverage, documentation checks and build,
formatting, and strict workspace Clippy pass. G16, G17, G18, G22, and G30 gain
bounded source evidence but retain their prior ledger dispositions until URL
compaction, ordinary status/doctor joins, and final installed qualification.
The ledger remains 11 pass, 29 partial, 2 fail, and 3 missing. No installed,
browser, provider, route, display, publication, production, merge, or release
effect occurred. M4-P2 owns exact URL history compaction and daily summaries.

Version 32 starts M4-P1 from pushed checkpoint `3e2ffa5e`. The read-only M4
audit found that the v2 SQLite row already stores the accepted 64-MiB exact URL
history, 96-MiB live database, and 128-MiB routine storage defaults and validates
their ordering, but the strict update patch omits all three and no executable
consumer enforces them. Browser Runtime SQLite has WAL migration checkpointing
but no ordinary integrity surface, verified rotating online backup, database and
WAL size projection, or restoration-gap receipt. M4-P1 below owns that bounded
storage-authority foundation. The ledger remains 11 pass, 29 partial, 2 fail,
and 3 missing. No runtime or provider effect occurred in this audit.

Version 31 source-qualifies M3-P3 at commit `93a740f6`. The strict SQLite
runtime patch, Service schema, generated client, CLI help, README, repository
skill, and docs now expose the accepted 24-hour, 20-profile, and 10-GiB
disposable settings. Browser Session Host consumes the disposable inactivity
setting, projects current viewer, controller, and pending-operation protection
from one SQLite snapshot, and commits any oldest-inactive quota eviction before
admitting a replacement allocation. Count and measured regular-file byte limits
fail closed when protected capacity prevents convergence. Expiry and quota
eviction terminalize exact handoffs, compact terminal history, and perform only
idempotent direct-child deletion without following symlinks. Strict policy
decoding rejects pinning and promotion bypasses; named profiles remain outside
the candidate set. G37 and G38 advance from missing to pass; G21 and G36 remain
partial until the frozen installed candidate proves the joined lifecycle. The
ledger is now 11 pass, 29 partial, 2 fail, and 3 missing. Provider-free model,
store, host, runtime, generated-contract, documentation, formatting, and strict
Clippy gates pass. The broad browser-session filter also exposed four inherited
environment-sensitive process-census fixture failures while all 131 executed
manager, host, store, runtime, daemon, and dashboard cases in scope passed.
No installed, browser, provider, route, display, publication, production,
merge, or release effect occurred. M4 remains open and requires a fresh bounded
packet before implementation.

Version 30 starts the remaining M3 retention packet from pushed checkpoint
`762a7f10`. The read-only audit found that the v2 SQLite runtime row already
stores the accepted 24-hour disposable inactivity, 20-profile, and 10-GiB
defaults, but the typed update patch omits all three. Browser Session Host also
projects the legacy five-minute `sessionIdleTimeoutMs` into disposable session
expiry instead of the accepted `disposableInactivityMs`, while count and byte
limits are not enforced. Existing reaping reference-checks sessions and
browsers before profile deletion, but it has no current SQLite protection join
for authenticated viewers, Desktop Services control, or pending operations and
does not choose oldest inactive profiles under quota pressure. M3-P3 below owns
that exact gap. No executable or runtime state changed in this audit.

Version 29 source-qualifies M3-P2D at commit `0b728fc0`. Baseline presentation
recovery remains Route Keeper work and cannot deserialize or enter the
per-browser recovery demand model. Scheduled browser recovery now projects only
current-boot, unexpired authenticated controlling viewers through the current
SQLite handoff, session, browser, and Route Keeper authority; it deduplicates by
browser ID and revalidates that authority in the admission transaction. A fresh
exact liveness observation then enters the M3-P2C journal and generation fence,
reuses the same logical session and opaque handoff, and rebinds current desktop
control after atomic publication. Dormant named browsers remain lazy.
Provider-free model, store, host, Route Keeper, restart, formatting, strict
Clippy, documentation, and selected workstation contract gates pass. No browser,
provider, installed-runtime, publication, production, merge, or release effect
was performed. The ledger remains 9 pass, 29 partial, 2 fail, and 5 missing:
G14 gains direct provider-free scheduler proof but still requires installed
cold-start and visible active-viewer recovery. Plan 0219 remains OPEN; the next
bounded packet must address the remaining M3 retention rows rather than advance
automatically to M4.

Version 28 source-qualifies M3-P2C at commit `0522fe95`. The journaled exact-client
path now proves the selected retained browser dead before SQLite atomically
admits one per-browser recovery generation and binds it to `launch_started`.
Two operation IDs no longer stale each other, while the per-browser generation
permits only one replacement effect. A successor probes and adopts the exact
reserved launch without launching again. `observed_live` is durable and
resumable separately from a ready handoff; later navigation or presentation
failure cannot reopen launch admission. One immediate transaction fences the
operation generation and recovery generation while publishing session state,
the same opaque handoff, the committed open result, and recovery success.
Stale generations and base-state conflicts roll back the entire publication.
Focused model, manager, store, journal, concurrency, crash-recovery, formatting,
and strict workspace Clippy gates pass. The ledger remains 9 pass, 29 partial,
2 fail, and 5 missing because G15 gains source proof but still requires joined
installed recovery. Plan 0219 remains OPEN; M3-P2D is the next packet and was
not begun.

Version 27 source-qualifies M3-P2B at commit `c96b7dfd`. Browser Runtime SQLite
configuration v2 now owns retry budget, base backoff, maximum backoff, and the
existing request deadline. Version 1 rows migrate atomically and once with a
revision increment. Default startup provenance preserves the committed row;
explicit config, environment, or CLI provenance commits under the shared host
startup lock before lane construction. Browser Session Host and daemon recovery
project the same row, and ordinary daemon recovery reports `config` provenance.
The typed Service request, MCP, generated client, help, README, skill, and docs
surfaces agree. G22 advances from fail to partial; G04 remains fail and G15
remains partial. The ledger is now 9 pass, 29 partial, 2 fail, and 5 missing.
The architecture audit remains 4 pass, 4 detector-gap, and 11 unverified because
the new recovery-policy red/green detector closes only that subcase. Plan 0219
remains OPEN; M3-P2C is next and no installed, provider, or production effect is
claimed.

Version 26 was the terminal budget checkpoint for the preceding execution
window. Its pre-commit goal-service readback reported 931,884 cumulative tokens
used, with executable implementation stopped at 753,564. Documentation
checkpoint `a7f74c63` preserved the 9/28/3/5 ledger and M3-P2B restart point.
The operator subsequently opened this fresh bounded continuation; version 27
does not rewrite or borrow the earlier accounting.

Version 25 reconciles the version 24 authority correction through every older
forward-looking M3 section. Historical M3-P1 evidence remains recorded, but its
browser-keyed baseline case is explicitly non-acceptance evidence and supplies
no launch authority. Future delivery, host integration, and acceptance text now
routes baseline presentation recovery through Route Keeper and limits browser
replacement demand to an exact current viewer or exact client resume. No source
code, runtime state, ledger count, or installed claim changed.

Version 24 records the read-only M3-P2D demand-authority audit and corrects an
overbroad recovery abstraction before implementation. Normative G14 uses
"baseline capacity" to mean eager presentation capacity, which does not
identify a profile browser and cannot authorize Chrome launch. The existing
browser-keyed `BaselineCapacity` pure-model case therefore remains evidence of
mechanics only. M3-P2D must reconcile provider routes separately, derive eager
browser recovery only from current-boot, unexpired SQLite live-viewer authority
joined to the exact current handoff, session, browser, and route, and leave
dormant browsers lazy until exact handoff or named-session access. The detailed
authority and test matrix below supersedes the earlier shorthand without
changing the normative requirement, ledger counts, or executable state.

Version 23 records the read-only M3-P2C transaction audit. Recovery state and
the browser-open journal currently commit in separate SQLite transactions.
`commit_browser_open` atomically publishes operation, session state, and
handoff, but not recovery success. The open journal can durably reach
`browser_opened` before navigation, presentation, or handoff completion; a
later failure may therefore leave a proven usable replacement while recovery
remains `admitted`. Marking that attempt failed could authorize another launch,
while marking it recovered before handoff publication would weaken G13/G15.
M3-P2C must first define and persist an observed-live resumable phase, or prove
an equivalent atomic transition, before host effects are wired.

Version 22 records the read-only M3-P2B surface inventory. SQLite runtime
configuration already has atomic get/update, strict patches, generated client
types, service-request schema, CLI commands, and documentation. Recovery retry
values remain outside it: the launcher always exports resolved values plus
`default`, `config`, `env`, or `cli` provenance into daemon environment, and
`DaemonState` reconstructs its own policy. The safe compatibility rule is now
fixed: default-sourced startup values must not overwrite an existing SQLite
row; explicitly sourced values are configuration inputs committed atomically
before host construction; both daemon and host then read the committed row and
report `config` provenance. This inventory changes no runtime state or code.

Version 21 is the final checkpoint for this continuation. The goal service
reports 817,438 cumulative tokens used. Executable implementation stopped at
753,564, before the 800,000 implementation boundary; subsequent usage was
limited to read-only host discovery, requirement and prohibition audits,
documentation reconciliation, validation, and custody. The remaining 182,562
tokens under the 1,000,000 ceiling are not treated as a renewed implementation
allowance. Resume from M3-P2B only under a later continuation that explicitly
inherits this cumulative accounting and the 200,000-token closeout rule.

Version 20 records the matching P01–P19 prohibition audit. The deterministic
architecture checker reports only P09, P15, P16, and P19 as pass; P02, P03,
P05, and P12 have detector gaps; the other 11 prohibitions are unverified. All
checks report zero current findings, but zero findings without a detector is not
acceptance. Final qualification therefore remains blocked by architecture-gate
coverage independently of the open G rows.

Version 19 records the closeout completion audit against the authoritative
45-row ledger. Only G11, G23, G24, G31, G35, G41, G43, G44, and G45 pass. G03,
G04, and G22 fail; G17, G20, G33, G37, and G38 are missing; 28 rows remain
partial. Therefore P219 is not complete, M5 is ineligible, and no integration or
installed-final claim is permitted. The ordered successor work remains M3-P2B
through M3-P2D, followed by the still-open M3 retention rows and M4 operational
rows before final-candidate adjudication.

Version 18 records the read-only M3-P2 host-seam audit after the implementation
stop. The default Browser Session Host reads SQLite `BrowserRuntimeConfig`, but
retry budget and backoff still enter the separate daemon `ServiceState` recovery
path through environment-derived `BrowserRecoveryPolicyConfig`. The host config
contains no recovery policy. Journaled exact-client open performs retained
replacement after the durable `launch_started` observation, while scheduled
reap only preserves dead named records and never schedules eager replacement.
Therefore direct host wiring would create a competing policy source and would
not close G14. M3-P2 is split below into configuration-authority consolidation,
exact-client effect fencing, and eager scheduling. This audit changes execution
order without changing code, ledger status, or installed claims.

Version 17 source-qualifies M3-P2A at commit `53c66ce9`. Recovery success is
fenced to the exact admitted generation, persists as a recovered phase, resets
attempts only after exact success, and preserves monotonic generation when a
later failure starts a fresh attempt window. Stale generations cannot reset
recovery. The complete service-model suite passes 277 tests, the
browser-session-store surface passes 38 tests, and strict workspace Clippy and
formatting pass. Goal usage is 753,564 tokens. New implementation is stopped;
configuration translation and actual host effect consumption remain open. No
runtime or publication effect occurred.

Version 16 records source checkpoint `6000b9fd` and the restart-safe successor
packet M3-P2. The goal service reports 712,314 cumulative tokens used. New
implementation stops at this coherent boundary rather than risking an
incomplete host-effect cut inside the remaining implementation allowance. The
1,000,000-token ceiling and 200,000-token closeout reserve remain controlling;
this early checkpoint does not reset either counter. The worktree is clean and
the branch is 59 commits ahead of its unchanged remote. Nothing was pushed,
published, installed, merged, or applied to a runtime.

Version 15 source-qualifies the M3-P1 admission mechanics. A strict
provider-free decision contract distinguishes authenticated-active-viewer and
exact-client resume demand from lazy dormant demand; unknown old-browser
usability refuses admission. Its browser-keyed baseline case was later rejected
by version 24 as acceptance evidence and supplies no browser-launch authority.
Browser Runtime SQLite serializes the retained logical browser's
generation, attempt, deadline, and capped retry time in one immediate
transaction. Four independent connections produce one admission winner. The
complete 276-test service-model suite and 37 browser-session-store tests pass.
The ledger is now nine pass, 28 partial, three fail, and five missing. G14 moves
to partial and G15 gains bounded-retry and concurrency evidence; host effect
consumption and joined installed recovery remain open.

Version 14 starts M3 with one bounded provider-free packet, M3-P1. The packet
owns only browser recovery admission and scheduling mechanics for G14/G15:
authenticated-active-viewer demand, lazy dormant demand until an exact client
resumes, exact old-browser unusability proof, one SQLite-serialized replacement
winner, and typed bounded retry/backoff with a terminal deadline. Its historical
browser-keyed baseline fixture is not G14 acceptance evidence; baseline
presentation capacity belongs to the separate Route Keeper path.
It preserves every logical browser, session, profile, tab, target, handoff, and
navigation identity and performs no browser, provider, route, display, viewer,
publication, or production effect. The primary agent owns the service-model
contract, Browser Runtime SQLite adapter, provider-free fixtures, ledger, and
planning authorities. Passing this packet can advance G14/G15 source evidence;
it cannot close either row or qualify installed recovery without later wiring
and joined acceptance.

Version 13 records the qualified A02 ordinary-desktop SQLite cut and completes
the bounded A03–A06 reconciliation. Managed capture no longer overlays JSON Service
State, ready handoff lookup reads the Browser Runtime SQLite registry, and
interaction idempotency uses the SQLite operation journal. A one-time
fail-closed importer preserves and archives the prior JSON ledger. Compiled-path
detectors cover both retired dependencies. The adjudicated ledger is now nine
pass, 27 partial, three fail, and six missing: G15 moves from missing to partial
for its existing singular restart/adoption subproof, while bounded backoff,
old-browser unusability, and concurrent replacement remain open.

Version 12 records the qualified A01 source repair. Ordinary dispatch now
projects a receipt-bearing desktop failure as outer failure, the shared failure
classifier retains `effect_uncertain` and inspect-before-retry recourse, stream
redaction preserves the safe diagnostic receipt, and terminal job/event/outcome
surfaces agree. Provider-free validation passed 40 desktop-interaction tests,
273 service-model tests, 18 Desktop Services tests, focused dispatch,
redaction, and terminal regressions, formatting, and strict workspace Clippy.
The pre-repair retained synthetic receipt is the red observation; the active
fixtures are the deterministic green proof. A02 and the A03–A06 evidence
reconciliation remain open before M3.

Version 11 records the operator's resumed continuation with a cumulative
1,000,000-token ceiling before stop and checkpoint. Reserve 200,000 tokens for
reconciliation, validation, evidence, and custody; sustained implementation
stops at 800,000 tokens. Goal-service thread
`01a0e3f7-adbb-7312-b748-ce5a462ccd90` records this objective as active and
reported 141,869 tokens used at startup, but exposed no remaining-token field.
Use the explicit ceiling and reserve as the controlling bounds, retain the
service counter as an observed lower-bound readback, and checkpoint early if
the service cannot prove remaining budget.

Version 10 recorded the September 27 fresh-context audit and the
[continuation packet](#fresh-context-continuation-after-m2b). This revision
records work only; it does not start implementation, renew a token budget, or
authorize runtime effects. P219 remains OPEN with the same objective and
custody. Preserve the successful M2B subproofs while reopening desktop failure
reporting and reconciling evidence scope. RUNBOOK.md owns current execution.

Version 9 recorded M2B as installed and accepted on isolated development
generation `0.28.0-ac3c1daed8ab`, with implementation custody committed at
`45a76907`. The accepted binary was built from the same M2B implementation
surfaces before the documentation and service-reconcile compatibility closeout.
The root-owned privileged helper
is version 11, all development doctor checks pass, four warm provider displays
are ready, and production remains selected at
`0.28.0-b589b318c530-c0c0977896a8`. M3, M4, and M5 remain unstarted. The
M2B closeout is complete. The next bounded gate is M3 recovery and retention;
re-read the applicable policy capsule triggers before starting it.

### Prior version 8 checkpoint

Version 8 recorded source checkpoint `375cdfbe` and installed development
generation `0.28.0-57c51d356866`, with executable SHA-256
`57c51d3568663fe9f50bd14bab69bc183c71b37b819decd22d64ee62c8f35cf2`.
The optimized build, development publication, and three-iteration browser
launch smoke passed. Production remained selected at generation
`0.28.0-b589b318c530-c0c0977896a8`. The source-qualified named-profile retention
repair remains at `d7ec8d2f`: only manager-allocated disposable profiles use
the configured inactivity deadline; exact named-profile sessions and their
opaque handoffs use no default idle expiry.

The operator installed helper version 10. Fresh provider plan, stage, and
preflight then passed, but receipt `apply-1790478847005-74899.json` quarantined
after historical X locks and XRDP channel sockets exhausted displays 10 through
60. Commit `662ab5d2` added a bounded exact stale-display sweep. Receipt
`apply-1790479449908-49709.json` then proved the sweep must first resume a
retained quarantined `dev-2` stop. Commit `d4e75b70` added that SQLite-read-only
recovery gate. Receipt `apply-1790479918735-97420.json` advanced through the
retained stop but exposed a short teardown race; commit `a11be934` waits for the
exact route-user X-server PID identity to leave before reclamation.

Receipt `apply-1790480067753-19562.json` advanced through displays 10 through
19 and then failed closed at `.X20-lock`. That route-owned lock records numeric
PID 88087, which the kernel has reused as a Chrome thread under UID 1000 rather
than route UID 1004. Commit `375cdfbe` introduces helper version 11: the helper
still requires an absent XRDP session, absent display socket, exact route-owned
lock inode, mode and content, and inactive channel sockets; it accepts a live
numeric PID only when `/proc/<pid>/status` proves a different UID from the exact
route user, and repeats that proof immediately before deletion. Same-route UID
or unreadable evidence remains `rdp_route_display_lock_pid_live`.

The v11 helper, installer convergence, provider preflight capability gate,
red-then-green helper and provider regressions, focused Rust contracts,
workstation install and host-provision fixtures, fresh-VM harness, Guacamole
assets, PostgreSQL durability, route-user synchronization, formatting, and
strict workspace Clippy pass. The user-scoped candidate is installed, but the
root-owned helper still reads version 10. Interactive sudo is now the only gate
before another provider recovery. The provider remains quarantined and stopped;
all six route-keeper records are durably absent and no development XRDP route
process remains.

Authenticated operator access and real synthetic browser pixels passed through
an opaque `/remote-view/<handoff-id>` URL. At the operator's direction, the
development admin credential was aligned with the existing live credential
without recording the secret; the live auth file remained byte-identical.
Automated pointer, keyboard, and scroll effects were not established, so input
acceptance remains incomplete.

Named-profile sessions 94 through 97 in the preceding installed generation
ended with
`heartbeat_expired` about five minutes after their last activity. Source commit
`d7ec8d2f` repairs that defect and is present in the new installed candidate.
G12 and G36 therefore have source and installed-code identity evidence but
remain unaccepted because the quarantined provider prevented the installed
same-handoff retention replay. G42 still requires installed exact-session
activity refresh. G45 is now source-qualified as partial: authenticated viewer
connect and heartbeat re-observe the exact Guacamole primary, viewer authority
expires after 15 seconds with an inclusive deadline, and heartbeat or
disconnect does not refresh the logical browser session. Installed separation,
control transfer, and recovery remain unaccepted. A temporary keepalive was
removed because it masked the retention defect. One
operator report of
"Remote-view handoff was not found in the SQLite session authority" occurred
while the exact handoff row still existed and an authenticated retry resolved
successfully; retain it as diagnostic evidence, not a reproduced second root
cause.

M2A has meaningful partial evidence for provider recovery, authenticated access,
and pixels, but remains incomplete because the configured provider target is
quarantined until helper version 11 is installed. M2B remains incomplete
because input, Alice/Bob joined lifecycle, installed restart recovery, cleanup,
and named-session retention have not all passed on the frozen candidate. Do not
start M3, M4, or M5.

At succession, local HEAD and the directly queried remote branch both resolve
to the source baseline. P218 is incomplete; no installed P218 candidate has
accepted pixels, input, joined recovery, or complete grilling conformance.
The [inherited coverage ledger](../contracts/p218-grilling-contract-coverage.v1.json)
has 45 rows: two pass, 31 partial, three fail, and nine missing. Retain its
filename and row IDs as the single evidence ledger. G35 and G45 are partial
from current source and focused tests; neither has installed acceptance.

Completed work materially changes the starting point:

| Surface | Inherited evidence | Remaining qualification |
| --- | --- | --- |
| Ordinary open and handoff | `541d7346` removed the JSON ordinary-open coordinator, its acquisition repository, and the generic JSON handoff-resolution fallback. Historical acquisition/finalization helpers are under `cfg(test)` in `remote_view_handoff/legacy_json_tests.rs`. | Prove the complete current dispatch and adjacent lifecycle closure; repair only reachable defects. Do not repeat the removed coordinator cut. |
| Lease Authority | P15, P16, `serviceModelLeaseAuthority`, and `legacyAuthorityQuarantine` pass. The independent crate remains a workspace member while trusted product serialization, inventory, generated-client and dispatch exposure are removed. | Preserve the physical-quarantine gates through later batches; historical disabled tests are not product authority. |
| Manual seeding | `2b7e866e` dispatched SQLite acquire, close, and durable resolution. The current adapter calls `launch_cdp_free_from_sqlite_profile`. `0d994c7a` and `547e958e` added bounded exited-process reconciliation. | Keeper rebinding, launch-issued uncertainty, live-PID reconciliation, and installed presentation remain incomplete. Do not rediscover the helper as undispatched. |
| Alice/Bob lifecycle | The SQLite host fixture proves separate identities, command routing and heartbeat publication, Alice-first cleanup, terminal handoffs, and final-session close through provider-free effects. | Ordinary ingress parity, remaining command variants, real expiry, browser effects, and final process termination. |
| Explicit browser selection | `44961424` carries `browserId` through ordinary-open translation and journaling; unknown or inactive identities cannot launch a replacement. | Remaining supported selectors and browser-build selection, concurrency, installed behavior, and complete request-path qualification. |
| Navigation heartbeat | `1884c407` proves interrupted navigation preserves activity/expiry at `1_000/301_000`; recovery commits `2_000/302_000` once. | Other ordinary command variants, operator navigation, and installed recovery. |

The last recorded Rust evidence includes 270 model tests, 31 host tests,
formatting, and strict workspace Clippy. The 31-test receipt is
`job-20260926T151649Z-af01c876429d`; format is
`job-20260926T151723Z-b610dc2d68fd`; Clippy is
`job-20260926T151941Z-abe496c2fc6a`. These are inherited receipts, not new test
runs or installed acceptance. Preserve the earlier fixture failure and
`sccache` failure receipt `job-20260926T151732Z-4573b2171baa`.

The current architecture report has P09, P15, P16 and P19 passing, four
detector gaps (P02/P03/P05/P12), and eleven unverified prohibitions. P19 passes
only when authenticated viewer authority is bounded, tied to current boot and
controller fencing, and absent from Browser Session Manager and presentation
admission. B06 migrated
the four persistent route credentials into private Browser Runtime SQLite,
scrubs legacy keys and projects credentials transiently. Neither a detector gap
nor an unverified row passes.

Several inherited ledger descriptions and the
[M0 dependency snapshot](../architecture/p218-ordinary-open-handoff-closure.v1.json)
predate the completed cuts above. Keep the snapshot as historical evidence.
M1A reconciles current reachability and ledger descriptions before selecting
another removal. Old graph edges and old prose are not evidence that deleted
production paths still execute.

## Consolidated Batch

Deliver the complete unchanged contract through an early installed Alice/Bob
proof, then expand qualification to the remaining operational requirements.
The first useful outcome is one ordinary named-profile workflow that provides
real pixels, responsive input, separate Alice/Bob session effects, and exact
cleanup through durable authenticated handoffs.

The source and minimum provider prerequisites include transactional authority,
current generation and operation fencing, capacity before launch, SQLite
credential custody, live viewer observation, and shared Desktop Services
control. Complete those before the installed proof. Full scaling, storage
maintenance, quotas, and the failure matrix remain mandatory afterward.

Check development provider and external-observer prerequisites early, using
read-only diagnostics within the renewed scope. The September 26 handoff
reported stopped Guacamole services and failed development preflight for
ingress binding, keeper integration, XRDP, and bundle/manifest comparisons.
These are boot-specific leads, not a current diagnosis or permission to start
containers. Treat environment repair and product changes as separate causes.

## Scope And Non-Goals

Included: all G01–G45 and P01–P19 requirements; ordinary CLI, authenticated
Service, HTTP/MCP, dashboard and generated-client parity where those paths
participate; supported manual-seeding transitions; provider-free fixtures;
isolated development implementation and synthetic external acceptance after
execution is authorized; all required documentation and contract updates.

Excluded: production or staging mutation, public ingress publication, formal
release, merge, branch/worktree deletion, private-site acceptance content,
new multi-tenant admission policy, disposable-profile promotion, and recovery
that replays clicks, forms, submissions, downloads, or other page effects.
The existing branch, worktree, work items, and draft PR retain custody.

## Delivery Sequence And Budget

The latest P218 execution window was 600,000 tokens. The September 26 handoff
recorded goal thread `01a0da16-eee9-7511-99e6-ffc04d7b3cff` as `blocked` with
`tokensUsed=3,808,483` and explicitly stopped execution. This is a historical
service readback, not a fresh counter or an independently reconciled total of
all preceding windows. Do not sum overlapping counters or treat a new thread
with no goal as renewed authority.

The first Plan 0219 execution window stopped at 545,502 tokens after M1A and a
partial M1B. The operator first renewed execution with 1,500,000 tokens and
then added 500,000, establishing a 2,000,000-token active continuation ceiling.
This is a new bounded allowance, not erasure of earlier usage. Reserve 200,000
tokens for final reconciliation, validation, evidence and custody.
Implementation stops at 1,800,000 continuation tokens; the final 200,000 are
closeout-only. If current evidence no longer supports reaching M2B within that
implementation allowance, stop implementation and preserve the exact blocker.

The reasonable accomplishment for this renewal is the first installed outcome
checkpoint: complete M1B, complete the minimum M2A provider/live-viewer join,
and pass M2B on one frozen isolated-development candidate. Reaching that exact
outcome is an alternative stop criterion even when tokens remain. It must
include the ordinary authenticated Alice/Bob workflow, distinct session, tab,
target, handoff, activity and expiry identities on one exact-profile browser,
success-only command activity, Alice-first cleanup with Bob preserved, final
browser termination with fresh process census, authenticated pixels and input,
viewer connect, heartbeat, disconnect and control transfer, and bounded
restart plus durable-handoff recovery without page-effect replay. Stop after
reconciling its evidence and custody; M3, M4 and M5 remain open for a successor
window.

The [P219 capsule](../policy-capsules/p219-m1b-m2b.md) was the policy entrypoint
for the completed M2B renewal. Its historical hash verification and numeric
thresholds do not cover the continuation. That scope transition triggers
applicable policy rereads and a new scoped policy basis before implementation.

Sequence: M1A → M1B → M2A → M2B → M3/M4 → M5. Read-only environment
readiness checks may accompany M1. M3 and independent M4 work may overlap only
after shared interfaces and write ownership are fixed; both join before M5.

### M1A | Reconcile the current authority boundary

Use current source and the existing red gates to reconcile known ledger drift
for G01/G03/G04/G19/G23/G31/G41/G42 and related prohibitions. Freeze the actual
ordinary ingress, host, store, handoff, command, cleanup, and manual-seeding
dependency closure. Inspect CodeGraph freshness banners and read listed stale
files directly; do not repeat an old removal based on stale graph edges.

Exit with one bounded list of current source blockers, their G/P IDs, exact
write surfaces, existing tests, and cheapest discriminating check. Update the
same evidence ledger with completed subproofs and remaining gaps, preserving
historical failures and requirement text. Implement detector coverage as part
of the relevant source batch; inventories alone cannot satisfy prohibitions.

### M1B | Qualify the complete ordinary SQLite lifecycle

Finish only the current gaps identified by M1A: supported selectors and browser
builds, session membership, reservation and rollback, handoff publication,
exact success-only heartbeat, concurrent-open coalescing, expiry and cleanup.
Admission, browser identity, presentation reservation, and handoff publication
must share one SQLite operation. Remove reachable ordinary JSON or legacy
authority dependencies without recreating them under new names.

Exit with the ordinary authenticated request path covered by provider-free
Alice/Bob fixtures, restart/journal cases, and deterministic guards for its
authority boundaries. Include the model, host, store, ingress adapters, and
fixtures in one causal batch. Separate session liveness from viewer liveness;
the live Guacamole half of G35/G45 is an M2 prerequisite.

### M2A | Join the minimum provider and live viewer path

Join the runtime-owned keeper, Guacamole, XRDP, browser launch, SQLite
credentials with transient secret projection, authenticated viewer observation,
and Desktop Services. A complete ready route must precede Chrome launch;
minimum readiness must be observed, and warm capacity may grow afterward.
Remove the readiness interlock only after its real integration exists.

Every launch already requires current memory, process, and disk admission.
Provider effects already require exact ownership and the narrowly typed helper
where privilege is needed. M4 completes pressure, scaling, and privilege
qualification; it cannot defer these prerequisites for M2 effects.

Bound viewer heartbeat and disconnect detection. Require that observation for
viewer activity and continued desktop control, without making it browser
admission authority. Opening or activating another handoff transfers control
visibly and leaves other viewers view-only.

Exit with focused provider and request-path qualification, a validated
development environment and reviewed external observer inputs, and a frozen
candidate capable of M2B. Do not publish a candidate with known blockers in
this exercised dependency closure. Untested wider requirements remain visible.

### M2B | Prove the installed Alice/Bob workflow

Publish the coherent candidate to the isolated development runtime and record
source, binary digest, installed generation, database schema, provider manifest,
configuration, and exact route denominator. Use synthetic content and the
ordinary authenticated ingress, not a private host test API.

1. Open Alice on a named profile. Receive an opaque authenticated handoff and
   observe the intended browser with complete current pixels and responsive
   pointer, keyboard, and scroll input.
2. Open Bob on that exact profile. Prove one browser and distinct session, tab,
   target, handoff, heartbeat, and expiry identities. Reopen each session and
   repeat or concurrently issue an open without creating duplicates.
3. Navigate and issue session-addressed commands for each. Show that effects
   and successful activity refresh remain on the addressed session; a failed
   command does not extend its heartbeat. Resolve each handoff to the correct
   tab, with focus and control transferred through Desktop Services.
4. Observe an authenticated viewer connect, heartbeat, disconnect, and control
   transfer. Prove stored URLs, session activity, and controller epochs cannot
   manufacture or indefinitely retain a live viewer or control grant.
5. Close Alice and verify Bob's command path, handoff, and browser survive.
   Close Bob and verify final browser termination with a fresh OS census.
   Exercise actual expiry isolation in a separately bounded case.
6. On a separate retained synthetic pair, restart the runtime and reopen the
   same durable handoffs. Preserve logical session, profile, and handoff
   identity; attribute any replacement browser process or target correctly.
   Prove recovery without page-effect replay or competing replacement.

Record each case independently. A partial result locates the failing transition
and remains incomplete; it does not close the grilling contract. This is the
first installed outcome checkpoint, before broad stress or storage work.

### M3 | Complete recovery and retention

Extend the proven workflow across provider, Guacamole, route, display, browser,
tab, and viewer-disconnect failures. Prove bounded singular recovery, eager
baseline presentation recovery without browser launch, eager exact active-viewer
browser recovery, lazy dormant browser recovery, committed top-level
operator navigation including bounded redirects, and waiting-only restart
behavior that launches nothing until exact client resumption.

Complete named versus disposable retention, oldest-inactive quota cleanup,
protected-session refusal, no implicit pin/promotion, and the remaining
manual-seeding recovery and route-rebinding cases. Qualify their source with
provider-free failure injection, then run installed joined cases during M4's
final candidate qualification. M3's installed exit remains pending until those
cases pass; intermediate source commits do not require separate publications.
Preserve each first failure.

### M4 | Complete operational conformance and freeze final acceptance

Complete all mutable typed settings, placement and queue ordering, growth and
cooldown scale-in, live pressure admission, non-evicting limit convergence,
compact history and daily compaction, integrity checks, verified rotating
backup and restore gaps, privilege receipts, and read-only status/doctor.
Implement backup safeguards before any acceptance case migrates or restores
retained data. The early M2B fixture uses a new isolated database and synthetic
named profiles that the test owns; it does not depend on unfinished disposable
profile quota behavior or touch retained operator profiles.

Freeze the final source and installed candidate after known blocking repairs.
Prove one forward-only cold upgrade, provider-row reconstruction, three
zero-process cold starts, and an injected capacity timeout with no Chrome
launch. Validate the provisional 90-second readiness deadline and density of
four under development pressure; retain any evidence-driven lower setting.

Every frozen route must pass authenticated external desktop and mobile pixels,
clean desktop and z-order, focus, pointer, keyboard, scroll, resize, control
transfer, reconnect, and the complete recovery matrix. Use the manually
dispatched P158 external-vantage workflow when its contract applies, with its
required identity, credentials, synchronized schedule and synthetic-only
attestation; otherwise freeze the reviewed manual procedure and observers
before effects. No automatic dispatch or retry is authorized by this plan.

### M5 | Qualify the full contract and prepare integration

Re-run the Alice/Bob acceptance cases on the final frozen candidate, select
validation from the complete inherited change, and reconcile every G/P row.
Earlier candidates supply scoped development evidence, not a composite final
pass. Complete one closed-world review of accepted findings and critical
regressions. Update documentation, lane state, remote checkpoint, and draft
PR through the authorized publication workflow. Exit with an evidence-backed
integration handoff; merge and production remain separate actions.

## Requirement Mapping

Every G-row has one completion owner below. Dependencies and early subproofs
may span milestones; M5 adjudicates all 45 against the final candidate.

| Completion owner | Requirements | Main dependency |
| --- | --- | --- |
| M1 | G01, G03, G04, G05, G19, G23, G26, G31, G41, G42, G43, G44 | Current authority and ordinary lifecycle closure; installed portions qualify in M2B/M4. |
| M2 | G02, G06, G07, G08, G11, G25, G35, G45 | Provider integration, capacity-before-launch, live viewer and desktop control. |
| M3 | G12, G13, G14, G15, G21, G32, G36, G37, G38, G40 | Working handoffs and live viewer observation from M2; quota settings from M4 where needed. |
| M4 | G09, G10, G16, G17, G18, G20, G22, G27, G28, G29, G30, G33, G34, G39 | Stable provider/store interfaces; all safety prerequisites required by earlier effects stay earlier. |
| M5 | G24 | Development evidence and complete final qualification before any separately authorized production action. |

## Worker Assignments

The primary owns the critical path, all shared model/host/store interfaces,
authority and budget accounting, the evidence ledger, candidate identity,
runtime effects, and acceptance. Default concurrency is one active agent;
this planning transition assigns no worker or additional worktree.

Potential independent work after interfaces are frozen includes pure compaction
fixtures, read-only status field coverage, and documentation parity. If later
delegated, give each worker exact disjoint files, inputs, G/P IDs, one focused
verifier, and a stop on a shared-interface change or second causal defect.
Workers return evidence to the primary and do not own runtime or acceptance.
P207 retains its branch and tab-refresh feature history; this inherited lane
owns its already-assigned shared help, README, skill, installation, remote-view,
and planning prose. P214 retains desktop candidate-event source ownership.

## Controls And Stop Rules

- Carry all predecessor attempts, review findings, and cumulative usage forward.
  A successor, renamed packet, worker, or fixture cannot reset them. Reconcile
  consumed attempts before resuming an unmet criterion; there is no fresh
  blanket retry allocation here.
- Before a source batch, budget its implementation, required compilation,
  documentation and qualification, reserving at least 20 percent for closeout.
  Do not start a batch that cannot reach its stated terminal condition within
  the renewed allowance. Record the first installed-proof deadline then.
- At most one implementation attempt and one consolidated repair apply per
  inherited milestone, less attempts already consumed. At a repeated failure,
  classify product, fixture, or environment cause, then split or reframe within
  remaining authority. No automatic open-ended retry or new discovery review.
- Two consecutive checkpoints without outcome progress or 30 minutes without
  it end the current tactic. Report the affected criterion and choose a
  different evidence-backed approach within remaining bounds, or record the
  exact block. A passing unrelated fixture does not reset this clock.
- Use `cargo-signal` around `scripts/ci/cargo-safe.sh` for compiling Rust work.
  Retain raw logs and exact argv, exit status, and receipts; read compact
  diagnostics first. The documented cache opt-out may address the observed
  `sccache` failure; Cargo admission and resource limits stay enabled.
- Validate a coherent source batch once for every touched surface. Reuse
  unchanged passed gates after prose-only edits. Preserve first failures and
  record pending gates at intermediate custody commits.
- Plan one development publication for the completed M2B batch and one for
  the completed final M4 batch only if executable inputs differ. Each consumes
  the newly authorized budget. Additional publication requires a demonstrated
  defect, impact analysis, and enough allowance for the affected acceptance.
- Candidate freezes are distinct. No final row may borrow installed proof from
  another candidate. Preserve earlier valid subproofs with their original scope.
- Use only positively identified development resources. Provider effects need
  the reviewed ingress binding and supported preflight. Reuse durable opaque
  handoffs; URL presence, a process, doctor output, or unit tests do not prove
  visible pixels or responsive input.
- Keep GitHub CI operator-disabled unless separately directed. Do not publish
  ingress, mutate production/staging, merge, release, or remove custody as an
  inferred consequence of completing a source or planning packet.

## Evidence And Exit

Maintain the inherited G01–G45 ledger as the single requirement-to-evidence
surface, with its validator. Before changing evidence semantics, update the
validator and consumer metadata in the same bounded batch. Distinguish source
implementation, provider-free qualification, installed qualification, and
external user-visible proof. Each result records source/candidate identity,
test or artifact, scope, first failure, remaining gap, and primary disposition.

P219 completes only when every normative G-row passes from the final frozen
candidate, every P01–P19 prohibition has a deterministic gate, the Alice/Bob
workflow and full route/recovery matrix pass, and fresh process/resource
readback accounts for browser, daemon, keeper, XRDP, and Guacamole residue.
Exact uncertain foreign resources remain untouched without blocking separate
valid work. Required public documentation, generated contracts, local/remote
source identities, plan, roadmap, runbook, catalog, and PR must agree.

### Closeout Completion Audit At Version 19

| Evidence class | Current rows | Completion consequence |
| --- | --- | --- |
| Pass | G11, G23, G24, G31, G35, G41, G43, G44, G45 | Preserve these scoped proofs; requalify on the final candidate when their executable dependencies change. |
| Fail | G03, G04, G22 | Forward-only migration, one SQLite authority, and complete typed configuration are contradicted by current evidence and block final qualification. |
| Missing | G17, G20, G33, G37, G38 | History compaction, privilege qualification, live pressure, disposable quotas, and absence of promotion/pinning lack required evidence. |
| Partial | G01, G02, G05–G10, G12–G16, G18, G19, G21, G25–G30, G32, G34, G36, G39, G40, G42 | Source or installed subproofs exist, but their recorded remaining gaps must be closed at the owning milestone. |

The next critical dependency is G22/G04 recovery-policy consolidation in
M3-P2B because M3-P2C cannot truthfully consume the new admission fence while
ordinary host and daemon recovery read different authorities. M3-P2C then
advances G12–G15, G19, G26, G32, G40, and G42 at the exact-client boundary.
M3-P2D supplies G14 eager-demand behavior. Remaining M3 retention work owns
G21, G36, G37, and G38. M4 owns the operational and storage gaps listed in the
Requirement Mapping table. Only after those source and installed gates pass may
M5 freeze one candidate and re-adjudicate all 45 rows plus P01–P19.

The prohibition audit is 4 pass / 4 detector gap / 11 unverified. Pass rows are
P09, P15, P16, and P19. Detector gaps are P02, P03, P05, and P12. P01, P04,
P06–P08, P10, P11, P13, P14, P17, and P18 remain unverified. Every row currently
reports zero findings, which proves no violation only for the four implemented
detectors marked pass. Each owning source packet must add or identify a fixture
that makes its prohibition detector fail before M5 may count a zero-finding
result as deterministic enforcement.

The M2B milestone is an intermediate outcome. P218's cancellation is
supersession, not acceptance. A partial pass keeps this plan open after
execution starts. The completed M1B manager-open source cut is not whole-product
SQLite closure: ordinary desktop capture and interaction still consume JSON
Service State and the desktop interaction operation ledger remains JSON.
The next planned work is the continuation packet below; M3 is not started.

## Fresh-Context Continuation After M2B

Recorded: 2026-09-27, from the operator-requested read-only audit of source
checkpoint `00935477d57d60d57394d9653ae4e24ccc0d9dc5` and selected development
generation `0.28.0-ac3c1daed8ab`. The operator then requested durable recording
of the continuation plan. That request authorizes this documentation packet,
not source repair, publication, provider effects, or M3 execution.

### Audit Findings And Dispositions

These findings belong to P219 and the inherited G01–G45 ledger. They are not a
second requirement ledger, a new discovery allowance, or a replacement of the
September 19 specification. The primary performed the audit without delegation.

| Finding | Evidence and consequence | Disposition and acceptance |
| --- | --- | --- |
| A01: desktop failure reported as success | `run_configured_interaction` in `cli/src/native/desktop_interaction.rs` returns an error with a receipt as `Ok({status: failed, ...})`; the former `desktop_interact` branch in `cli/src/native/actions.rs` wrapped it in `success_response`. Retained synthetic receipt `/tmp/p219-bob-current-interact.json` reports `desktop_interaction_authority_changed`, `effect_uncertain`, and `not_verified`, but outer success and terminal `succeeded / verified_effect`. | Source repair qualified in version 12. Dispatch now emits outer failure while retaining `data`; stream persistence redacts private fields while preserving the safe receipt; the exact classifier returns uncertain effect and inspect-before-retry; terminal job, event, and outcome agree. The retained receipt establishes the inherited red observation and provider-free regressions establish green behavior. Installed requalification remains deferred to a later candidate decision. |
| A02: ordinary desktop JSON dependency | Before version 13, `run_configured_interaction` loaded `LockedServiceStateRepository::default_json()` and persisted `desktop-input/operations.json`; `ManagedDesktopStateSource::snapshot` overlaid configured JSON Service State before projecting SQLite browser and viewer authority. | Source-qualified in version 13. P219 owns this bounded cut: managed capture now begins from static configuration plus SQLite authority, handoff lookup uses the SQLite registry, and interaction replay uses the SQLite operation journal. A one-time importer archives the old ledger after idempotent import. Detector fixtures fail on either retired compiled path and pass on the replacement. G04 and wider P03/P05 product closure remain open for unrelated JSON, configuration, credential, history, and cleanup edges. |
| A03: exhausted renewal and stale policy/budget | Delivery Sequence And Budget stopped the prior renewal at M2B, and the historical capsule retained older thresholds. | Reconciled by the operator's resumed instruction and version 11. The active continuation has a 1,000,000-token cumulative ceiling, an 800,000-token implementation stop, and a 200,000-token closeout reserve. Goal thread `01a0e3f7-adbb-7312-b748-ce5a462ccd90` reported 141,869 used at startup but no remaining field, so the explicit ceiling governs and the service value remains an observed lower-bound readback. |
| A04: requirement evidence drift | G12 described older competing persistence paths; G13 omitted the narrower M2B replacement/no-effect proof; G15 had no source references despite singular restart/adoption fixtures. The M2B checkpoint proves explicit reopen cases, not eager recovery or the full failure matrix. | First reconciliation completed in version 13. G12 now records one SQLite handoff registry and retains its incomplete recovery matrix; G13 names the explicit replacement/no-effect subproof and its automatic joined-recovery gap; G15 is partial with exact journal/adoption fixtures and explicit bounded-backoff, old-browser, and concurrency gaps. The ledger is 9/27/3/6. G14 remains missing. |
| A05: shared-helper boundary | The selected production generation was unchanged, but the operator installed host-wide helper v11 at the path used by production diagnostics. | Reconciled as two facts: production binary selection remained `0.28.0-b589b318c530-c0c0977896a8`, while the shared diagnostic helper changed to v11 and may expose a compatibility warning against that binary. No production compatibility repair, publication, or attribution of unrelated warnings is claimed by P219. |
| A06: candidate and retry provenance | Installed M2B proof belongs to development generation `0.28.0-ac3c1daed8ab`, whose binary predates the later source-only service-reconcile compatibility closeout. The comprehensive Lease Authority compartment had one process-timing assertion failure, then passed immediately in isolation and passed all 118 tests on rerun without a source change for that assertion. | Reconciled under policies 0042/0072. The first failure remains retained and is classified as a process-timing flake because the exact assertion and complete compartment passed without a causal source repair; the green reruns do not erase it. Installed claims stay bound to `0.28.0-ac3c1daed8ab`; later source-only A01/A02 gates stay bound to their commits and are not projected into that installed binary. Reuse of unaffected M2B gates is allowed because A01/A02 do not change the installed manager, provider, or viewer subproofs. |

The later synthetic receipt `/tmp/p219-bob-final-current-interact.json` records
41 attempted and acknowledged effect keys, `verified_success`, and verification
passed. It supports successful input independently of A01. The restart and
handoff readbacks also support the narrower M2B subproofs. These temporary
locators are supplementary: retain the source mechanism and sanitized findings
above durably, and reproduce A01 with a hermetic fixture if its receipt is gone.
Do not copy credentials, viewer artifacts, or private runtime state into Git.

### Consolidated Batch

Keep P219, PL-PLATFORM, the inherited branch, worktree, PR #191, and work items
#181/#183/#195. A wholesale successor would duplicate the objective and is not
warranted by this audit. The bounded pre-M3 outcome is trustworthy desktop
failure reporting plus reconciled evidence, authority, and ownership of the
remaining SQLite work. It does not require replaying all M2B acceptance.

The packet excludes live failure drills, runtime publication, shared-helper
replacement, production repair, provider cleanup, pressure/density experiments,
quota cleanup, compaction, backup/restore, and deferred viewer polish. Required
SQLite work remains in the full objective; assigning it is not accepting it.

### Delivery Sequence And Budget

1. Re-anchor the existing worktree and current instruction. Read applicable
   planning, validation, documentation, branch-custody, and effect-boundary
   policies from that checkout. The exhausted M2B capsule is not M3 authority.
   Reconcile the usable continuation allowance and closeout reserve before
   sustained implementation. Carry all prior usage, failures, retry counts,
   accepted findings, and the completed discovery pass forward.
2. Reconcile A03–A06 in the canonical plan/runbook/ledger surfaces. Bind each
   retained proof to its source or installed artifact and scope. Record the
   desktop SQLite dependencies under A02 with one owner and a bounded closure
   packet. No runtime effect is needed for this reconciliation.
3. Repair A01 in one provider-free causal batch through desktop interaction,
   ordinary dispatch, terminal outcomes, and existing replay boundaries. First
   preserve a failing regression for an error carrying an uncertain-effect
   receipt, then prove truthful failure plus retained diagnostic content and
   successful-receipt compatibility. Inspect impact on session activity rather
   than assuming an outer-response repair alone establishes G42.
4. Stop at a qualified source checkpoint. Record exact validation and remaining
   gaps. Do not publish a development candidate merely for this intermediate
   repair. Any installed requalification is limited to the changed dependency
   and requires a later explicit candidate decision within execution authority.
5. After this checkpoint, derive the first M3 G14/G15 provider-free packet:
   browser admission mechanics for exact active-viewer and exact-client demand,
   lazy dormant scheduling, old-browser unusability proof, singular concurrent
   replacement, and typed bounded retry/backoff. Baseline presentation capacity
   remains a separate Route Keeper concern and supplies no browser-launch
   authority. Preserve G12/G13 handoffs, logical identities, committed URLs,
   terminal closed/expired sessions, and no page-effect replay. The closure
   owner must resolve concrete SQLite dependencies before accepting affected
   recovery behavior; unrelated cleanup and UI work stay outside this packet.

### M3-P1 Recovery Admission And Scheduling

M3-P1 is started under version 14. Reuse the existing recovery budget and
backoff defaults rather than creating a competing policy. Add one strict pure
decision contract and one Browser Runtime SQLite transaction boundary. An
eligible replacement requires an exact observation that the retained browser
is unusable. An authenticated active viewer is eager browser demand. A dormant
retained browser waits without admission until an exact handoff or named-session
client resumes. Unknown liveness fails closed. The historical baseline fixture
tests only generic admission mechanics; it cannot authorize browser replacement
and must be removed or constrained before host wiring.

The SQLite transaction must serialize competing callers for the same logical
browser and return one admission generation. A failed attempt records its
attempt count, next eligible time, and absolute deadline; exponential delay is
capped and exhaustion is terminal. Tests must prove eager and lazy decisions,
unknown-liveness refusal, exact deadline and backoff boundaries, deterministic
wire decoding, restart persistence, and one winner across independent SQLite
connections. This packet does not launch or focus a browser and does not replay
navigation or page effects. Actual host integration and installed joined
recovery remain later M3 work unless this packet's evidence exposes a smaller
causal integration cut within the remaining implementation allowance.

### M3-P1 Source Checkpoint

The packet is source-qualified without browser or provider effects. The pure
contract passes active-viewer/exact-client versus dormant demand,
unknown-liveness refusal, capped exponential backoff, deadline exhaustion, and
strict wire tests. Its baseline case is excluded from acceptance by version 24.
The SQLite adapter passes
restart persistence, generation fencing, exact retry timing, and a four-client
race with one admission winner. The complete service-model suite passes 276
tests and the browser-session-store surface passes 37 tests. Coverage validation
passes with 45 ordered unique rows and counts 9/28/3/5. Rust formatting and
strict workspace Clippy pass. Host launch/focus code
does not yet consume this fence, so G14 and G15 remain partial and no installed
recovery claim is made.

### M3-P2 Successor Packet | Host Effect Consumption

M3-P2 is planned and unstarted. Re-anchor at source checkpoint `6000b9fd`, a
clean `platform/p211-simple-cold-upgrade` worktree, Plan version 16, ledger
counts 9/28/3/5, and goal thread
`01a0e3f7-adbb-7312-b748-ce5a462ccd90`. Re-read current planning, testing,
architecture, runtime-boundary, and branch-custody policies. Verify the goal
counter before implementation; prior usage and the closeout reserve carry
forward.

The bounded outcome is one provider-free host integration. Add a typed success
transition that preserves monotonic recovery generation while resetting the
attempt window only after a proven usable replacement. Translate the existing
runtime recovery configuration into `BrowserRecoveryAdmissionPolicy`; do not
introduce another default source. At the exact retained-browser recovery seam,
derive demand from authenticated active viewer, dormant state, or exact
handoff/named-session resume. A presentation-capacity deficit never enters this
per-browser path. Bind old-browser
`ProvenUnusable` only to a fresh process/CDP observation for that exact browser.
Call `admit_browser_recovery` before any replacement effect, launch only for
`AdmitReplacement`, and fence success or failure to its generation.

Provider-free acceptance must prove: usable and unknown observations launch
nothing; dormant demand launches nothing until exact resume; two concurrent
host callers produce one replacement effect; a failed effect persists capped
backoff and cannot bypass its deadline after restart; success preserves logical
browser, profile, session, tab, target, handoff, and committed navigation
identity without replaying page effects; stale generations cannot publish or
reset recovery. Keep provider, browser, display, route, viewer, publication,
production, and installed-candidate effects excluded. If the existing host seam
cannot preserve those invariants in one causal batch, stop with the first
failing fixture and split the packet before changing broader lifecycle code.

### M3-P2A Source Checkpoint

Commit `53c66ce9` completes the first M3-P2 sub-invariant. The pure transition
requires an admitted exact generation before success, records `recovered`, and
opens a later episode at the next generation with attempt one and a fresh
deadline. Browser Runtime SQLite commits that transition atomically and retains
it across restart. Focused recovery tests pass five cases; the complete affected
suites pass 277 and 38 tests. The G15 ledger row now records the success fence
while remaining partial. The next implementation must translate existing
runtime recovery settings into the admission policy and make the host consume
admission, failure, and success fences around exactly one replacement effect.

### M3-P2 Host-Seam Audit And Ordered Successors

The current exact-client effect seam is
`BrowserSessionHost::journaled_open_with_handoff_result`. A prepared operation
first persists `launch_started`. On restart, the observed branch calls
`recover_browser_reserved`; on the original attempt it calls the manager's
reserved launch. Both reach the retained replacement and publication path. The
daemon holds the in-process host mutex, but different journaled opens and
process successors still require the SQLite per-browser admission fence.
`reconcile_liveness_current` runs from scheduled reap and calls
`reconcile_liveness_preserving_named_sessions`; it retains a dead named browser
record without admitting or launching recovery.

Execute the remaining source work in this order:

1. **M3-P2B | One recovery policy authority.** Extend the existing SQLite
   `BrowserRuntimeConfig` with retry budget, base backoff, and maximum backoff;
   use its existing request deadline as the recovery deadline. Preserve the
   current 3 / 1,000 ms / 30,000 ms / 90,000 ms values as migration defaults,
   validate nonzero budget and base, maximum at least base, and safe integer
   conversion. Project one `BrowserRecoveryAdmissionPolicy` from that row into
   `BrowserSessionHostConfig`. Reconcile the environment-derived daemon policy
   so it consumes the same SQLite values or is explicitly retired from ordinary
   manager recovery. Update every required configuration, help, README, skill,
   docs-site, and generated contract surface if these settings are exposed.
   Stop if two writable policy sources remain.
2. **M3-P2C | Exact-client replacement fence.** In the journaled-open observed
   and first-attempt branches, bind a fresh exact `browser_is_live` observation
   to `OldBrowserUsability`. Derive `ExactClientResume` only from the exact
   handoff or named-session request. Call SQLite admission before either
   reserved launch effect; only `AdmitReplacement` may call it. Record failure
   for launch, adoption, publication, target reacquisition, or handoff-ready
   failure, and record success only after the replacement state and same opaque
   handoff are atomically publishable. Prove two distinct operation IDs for the
   same browser yield one launch, stale generations cannot publish, committed
   navigation is restored without page-effect replay, and restart respects the
   persisted delay and deadline.
3. **M3-P2D | Eager scheduler.** Keep dormant named records effect-free during
   scheduled reap. Derive authenticated-active-viewer demand only from current
   SQLite viewer authority, not handoff history. Define the exact baseline
   capacity signal from current presentation/browser policy before coding; a
   route-only warm slot cannot imply which profile browser to launch. Schedule
   recovery through the same SQLite admission and host effect path, never a
   second launcher. Prove active-viewer demand recovers eagerly, dormant demand
   waits, unknown liveness launches nothing, and process restart cannot create a
   competing replacement.

Primary write ownership remains with P219 for
`browser_session_store.rs`, `browser_session_host.rs`, the recovery contract,
configuration projection, and provider-free fixtures. P214 retains Desktop
Services candidate-event source. No provider or installed-runtime effect is
authorized by these source packets. Run the complete changed-surface gates once
after the final coherent source batch; focused tests govern intermediate work.

#### M3-P2B Exact Write And Validation Inventory

| Surface | Required change | Required evidence |
| --- | --- | --- |
| `browser_session_store.rs` runtime config | Add retry budget, base backoff, and maximum backoff under a forward runtime-config schema revision; migrate v1 rows atomically to the current 3/1,000/30,000 defaults; add strict validation and patch application. Keep `requestDeadlineMs` as the single deadline. | Red v1 fixture, exact v2 migration, durable reopen, atomic patch, stale revision, invalid budget/base/max, and no-mutation failure tests. |
| Startup source reconciliation | Use launcher provenance labels to distinguish defaults from explicit config/env/CLI input. Defaults leave an existing SQLite row unchanged. Explicit values update the row before `load_default_browser_session_host`; failure prevents startup rather than falling back. | Table tests for default preservation and each explicit source; restart proves SQLite wins after bootstrap; malformed and conflicting inputs fail closed. |
| Host and daemon consumers | Add the committed policy to `BrowserSessionHostConfig`; derive `BrowserRecoveryAdmissionPolicy` with checked `u64` to `u32` conversion. Replace daemon environment reconstruction with the same committed row and project policy source as `config`. | Host/daemon equality fixture, overflow rejection, and a compiled-path detector forbidding ordinary recovery reads from the three legacy environment variables. |
| Typed request contract | Extend `BrowserRuntimeConfigPatch`, `service-request.v1.schema.json`, the service-request client generator, generated JavaScript declarations, and type coverage. Preserve strict unknown-field rejection. | Schema negative fixtures, request normalization, generated-file check, API/MCP parity, client contract, and client type checks. |
| User-facing configuration | Reconcile CLI flags and legacy environment variables as explicit bootstrap inputs or deprecate them with a typed error and migration guidance. Update `cli/src/output.rs`, README options/config sections, `skills/agent-browser/SKILL.md`, and the docs-site configuration and remote-view pages together. | Help snapshot/parser tests, documentation links/build, and exact examples proving readback through `service runtime-config get`. |
| Architecture and ledger | Extend P03/P05/P12 or the appropriate deterministic detectors so direct ordinary recovery-policy environment authority fails. Update G04, G15, and G22 only to the proof actually obtained. | Detector red/green fixtures, architecture check, coverage validator, formatting, affected store/config tests, and strict workspace Clippy. |

#### M3-P2B Source Checkpoint

Commit `c96b7dfd` completes the policy-authority packet. The v2 SQLite row,
one-time v1 migration, checked admission projection, startup provenance
reconciliation, daemon projection, strict request contracts, generated client,
operator documentation, and architecture subdetector are one coherent source
batch. Validation passes 43 store tests, 23 host tests, 4 handoff-recovery
tests, 5 navigation-recovery tests, focused config/provenance tests, strict
workspace Clippy, formatting, generated-client and type checks, API/MCP parity,
architecture red/green, coverage validation, no-launch Service contracts,
remote-view documentation checks, the docs production build, and every broad
route/workstation fixture selected for the changed surfaces. The aggregate
`pnpm test:service-client` wrapper remains red before affected tests because its
P157 oracle names the removed baseline file
`cli/src/native/service_profile_acquisition.rs`; the directly affected client
contract, type, and request-client lanes pass. The shared installed skill was
not overwritten by this source-only packet.

At the M3-P2B checkpoint, M3-P2C became the next bounded packet. Its transaction
matrix and stop rule below governed the completed source work; M3-P2B alone did
not authorize replacement effects or claim installed recovery.

Do not bump the database `PRAGMA user_version` merely for JSON-row evolution
unless table shape changes. The runtime-config value carries its own schema and
must migrate inside one immediate transaction. Do not silently treat missing
new fields as a current-schema row: a legacy schema must be recognized,
upgraded once, revisioned, and written before ordinary consumption. Do not let
startup defaults overwrite an operator-mutated row on every daemon restart.

#### M3-P2C Transaction Matrix And Stop Rule

The exact-client path has these authoritative phases:

| Open/recovery point | Current durable evidence | Required recovery behavior |
| --- | --- | --- |
| Before `launch_started` | Prepared browser-open operation | No recovery effect or admission. |
| `launch_started`, before effect | Observed open operation with reserved logical browser/desktop | Atomically reserve one per-browser recovery generation and bind it into the observation before launch. A crash here must probe only; it must not infer a second launch. |
| Launch returned, before `browser_opened` | Process effect may exist; open journal still says `launch_started` | Restart uses read-only `recover_browser_reserved`. Exact adoption advances the same generation; absent/unproven evidence records bounded failure and cleanup obligation without launching. |
| `browser_opened` through tab/navigation/presentation | Open observation contains replacement session state and may prove the browser live | Persist a typed observed-live/resumable recovery phase. Subsequent open phases may resume without launch. Navigation or presentation failure cannot convert a proven live browser into permission for another replacement. |
| `ready` commit | Session state, tab/target, opaque handoff, visibility proof, and operation result are publishable | One immediate transaction must commit open publication and recovery success together, fencing both operation generation and recovery generation. |
| Replay after committed ready | Exact committed operation and recovered generation | Return the prior result with no new observation, admission, launch, or success event. |

The `BrowserSessionPersistence` trait must expose typed admission, failure,
observed-live, and final publication operations. Product SQLite implements them;
test-only legacy stores may return an explicit unsupported result and cannot be
used as acceptance evidence. Extend `BrowserRuntimeSqliteStore` with one
combined `commit_browser_open_with_recovery_success` transaction rather than
calling `record_browser_recovery_success` and `commit_browser_open` separately.
The recovery generation must be present in the durable `launch_started`
observation so a successor cannot guess it.

Two different open operation IDs targeting one dead logical browser are the
required concurrency fixture. Exactly one may receive `AdmitReplacement`; the
other receives an in-progress/backoff result and launches nothing. After the
winner records observed-live, either request may resume only the non-effect
open phases, but one exact operation owns final handoff publication. A new
retry after a proven failed generation must use a new operation ID or an
explicitly specified journal transition; do not silently reinterpret an old
`launch_started` observation as fresh launch authority.

Stop M3-P2C implementation if the state machine cannot distinguish a proven
live replacement from a ready published handoff, if success and final open
publication are not one transaction, or if operation replay can bypass the
per-browser recovery generation.

#### M3-P2C Source Checkpoint

Commit `0522fe95` completes the exact-client replacement fence. A prepared open
has no recovery authority. A dead retained browser is observed first, then one
immediate SQLite transaction admits the recovery generation and records the
generation-bound `launch_started` observation. A crash after the external
launch resumes through read-only exact reservation recovery and cannot launch a
second process. The `browser_opened` transition atomically records
`observed_live`; subsequent non-launch phases preserve the binding. Final ready
publication commits the open result, session state, opaque handoff, and recovery
success together. Legacy persistence implementations return an explicit typed
unsupported error for these operations.

The focused validation batch passes four recovery-model tests, all 33 Browser
Session Manager integration tests, six `browser_open_` store tests, eight
`journaled_open` tests, the one-winner concurrent host fixture, the
crash-after-launch read-only recovery fixture, the durable retry and deadline
fixture, the repository focused recovery and open wrappers, Rust formatting,
diff hygiene, and strict workspace Clippy. These source-only tests launch no
browser and perform no provider, installed-runtime, publication, production,
merge, or release effect. G15 remains partial pending joined provider-free and
installed recovery qualification. M3-P2D remains unstarted.

The goal-service readback after source validation reported 689,484 cumulative
tokens used and 310,516 remaining. Implementation stopped below the 800,000
boundary, preserving more than the required 200,000 tokens for reconciliation,
validation, evidence, and checkpoint custody.

#### M3-P2D Demand Authority And Correction

M3-P2D has three distinct authorities. They share observation and admission
contracts where applicable, but they must not share an effect merely because
the word recovery appears in each path.

1. **D1 | Baseline presentation recovery.** Reconcile provider presentation
   capacity from `RouteKeeperAuthority` policy, including `minimum_ready`,
   `warm_target`, and `requested_ready_slots`, plus current provider
   observation. This path may restore route, Guacamole, and XRDP capacity. It
   must not launch Chrome, choose a profile, create a browser session, or mint a
   handoff. Qualify it first with provider-free policy fixtures and later with
   installed cold-start recovery evidence.
2. **D2 | Active-viewer browser recovery.** Add a scheduler-facing SQLite read
   that projects exact browser candidates from current `desktop_control`
   records. A candidate must be unexpired, belong to the current boot epoch,
   and join through the current handoff registry, logical session, browser,
   and Route Keeper authority. Deduplicate by logical browser ID. Before
   admission, bind `ProvenUnusable` to a fresh exact `browser_is_live`
   observation. Feed `AuthenticatedActiveViewer` through the same per-browser
   recovery admission and M3-P2C effect fence; no scheduler-specific launcher
   is allowed.
3. **D3 | Dormant browser access.** Scheduled reap continues to retain a dead
   named browser without launching it when no current active-viewer authority
   exists. An exact handoff or named-session access supplies
   `ExactClientResume` and uses the same M3-P2C admission and effect path.

Correct the service-model boundary before scheduler wiring. Remove
`BaselineCapacity` from the per-browser `BrowserRecoveryDemand` enum, or make
it structurally incapable of admitting a browser replacement. If a typed
presentation demand is useful, keep it in a separate presentation-capacity
model. A route-capacity deficit by itself must have a red fixture proving it
cannot admit or launch a browser.

Provider-free acceptance must also prove: one current authenticated viewer
causes eager recovery of its exact dead browser; expired, disconnected,
foreign-boot, stale-route, and historical-handoff records create no demand;
multiple current viewers of one browser deduplicate to one candidate and one
replacement; a dormant dead named browser launches nothing during scheduled
reap; baseline presentation slots recover without Chrome; and restart cannot
create a competing replacement. Installed acceptance remains a later candidate
gate and must separately qualify presentation cold start and visible active
viewer recovery.

#### M3-P2D Source Checkpoint

Commit `0b728fc0` completes the provider-free eager-scheduler packet. The
per-browser demand enum no longer contains baseline capacity, and a negative
serialization fixture prevents that presentation-only signal from returning.
The SQLite projection rejects expired, disconnected, foreign-boot,
historical-handoff, and stale-route authority, deduplicates multiple viewers by
logical browser ID, and admission revalidates the exact viewer and handoff under
the same immediate transaction as the recovery generation. The daemon scheduler
uses the existing Browser Session Host path. It performs a fresh exact liveness
probe, then uses the M3-P2C operation journal, reserved launch/adoption path, and
atomic publication fence. Restart observes the replacement as live and does not
create a competing process. A dead named browser without a qualifying viewer
remains retained and effect-free.

Validation passes all 278 service-model crate tests, 65 browser-session-store
tests, 26 browser-session-host tests, 37 Route Keeper tests, the focused stale
authority and scheduled restart fixtures, Rust formatting, strict workspace
Clippy, patch hygiene, documentation links, remote-view documentation checks,
the docs production build, and every source-free workstation and Guacamole
contract selected for the changed prose. The repository skill intentionally
differs from the installed shared skill because publishing user-scoped guidance
is outside this source-only packet. Goal thread
`01a0e57f-4aa3-7630-b4d7-50227d132ba5` reported 323,934 cumulative tokens used
after source validation, below both the 800,000 implementation stop and the
1,000,000 checkpoint ceiling.

This checkpoint does not change the ledger counts. G14 remains partial until
installed cold-start route recovery and visible active-viewer browser recovery
are jointly qualified. G15 remains partial pending the same installed joined
recovery proof. No installed, provider, browser, route, display, publication,
production, merge, or release effect is claimed. M4 is not eligible; the next
packet must reconcile the remaining M3 retention rows and their exact acceptance
boundary.

### M3-P3 | Disposable Retention And Quota Enforcement

M3-P3 closes the source portion of G21, G36, G37, and G38 without weakening the
named-profile retention already qualified at `d7ec8d2f`.

1. **M3-P3A | One live retention configuration.** Add
   `disposableInactivityMs`, `maximumRetainedDisposableProfiles`, and
   `maximumDisposableProfileBytes` to the strict runtime-config patch, Service
   request schema, generated client, CLI help, README, skill, and docs site.
   Project `disposableInactivityMs`, not the legacy five-minute timeout, into
   disposable session expiry and cleanup. Preserve one-time v1 to v2 migration,
   atomic validation, and exact readback.
2. **M3-P3B | Protected oldest-inactive cleanup.** Derive a scheduler and
   admission protection projection from current SQLite authenticated viewers,
   current Desktop Services control, and prepared or observed operations joined
   to exact sessions. Named profiles are structurally outside the disposable
   candidate set. Rank unprotected disposable candidates by last activity,
   then creation and stable profile ID. Enforce count and measured profile-byte
   limits before admitting a new disposable allocation and during scheduled
   reap. If protected candidates prevent convergence, return a typed count,
   byte-quota, or low-disk refusal without deleting protected or named state.
3. **M3-P3C | Terminal cleanup and no promotion.** Expiry or quota eviction
   closes the exact logical session, makes its handoffs terminal, preserves
   compact terminal history, and deletes only the reference-free direct-child
   disposable directory. External deletion is idempotent across restart and the
   resulting SQLite state commits before admission continues. Add a structural
   red fixture proving there is no disposable pin or profile-promotion field,
   command, or bypass.

Provider-free acceptance requires the 24-hour boundary, exact-session activity
refresh, deterministic oldest-first cleanup, count and byte convergence,
active-viewer/control/pending-operation protection, named-profile immunity,
typed protected-capacity refusal, terminal handoffs after expiry, direct-child
deletion only, restart-safe cleanup, and absence of pinning or promotion. Stop
and reframe if cleanup requires a second mutable authority, if filesystem size
observation can follow symlinks outside the recorded direct child, or if an
external deletion can authorize a new allocation without durable SQLite
reconciliation.

M3-P3 is source-qualified at `93a740f6`. Its provider-free acceptance passes;
installed joined retention remains part of the final M4 candidate rather than a
separate publication. The complete service-model suite passes 282 tests. The
relevant host, store, runtime, schema/client, documentation, formatting, and
strict workspace Clippy gates pass. The umbrella service-client command remains
blocked before the changed lane by the inherited missing
`cli/src/native/service_profile_acquisition.rs` P157 oracle input; the direct
service-request client, generated contract, type, parity, route-confusion, and
documentation gates pass. The installed shared user-scoped skill is unchanged
by design. Goal thread `01a0e57f-4aa3-7630-b4d7-50227d132ba5` reported 385,223
cumulative tokens used at checkpoint reconciliation.

### M4-P1 | Browser Runtime SQLite Storage Authority

M4-P1 is the first bounded M4 packet. It advances only the storage/configuration
portions of G16, G17, G18, G22, and G30 and does not claim provider, privilege,
resource-pressure, or installed acceptance.

1. **M4-P1A | Complete live storage settings.** Add
   `exactUrlHistoryMaximumBytes`, `liveDatabaseMaximumBytes`, and
   `routineStorageMaximumBytes` to the strict runtime-config patch, Service
   schema, generated client, CLI help, README, repository skill, and docs.
   Preserve atomic validation and exact readback.
2. **M4-P1B | Read-only integrity and budget projection.** Add one SQLite
   snapshot/report that runs the bounded integrity check, reports database and
   WAL bytes, exact navigation-history bytes, configured limits, and typed
   within-target or over-target state. Expose it through status/doctor without
   mutation, credential material, or provider URLs.
3. **M4-P1C | One rotating verified online backup.** Create at most one current
   and one previous Browser Runtime SQLite backup plus a compact manifest under
   the governed runtime directory. Use SQLite's online backup mechanism, verify
   the staged copy with integrity check before atomic publication, preserve a
   corrupt live database, and record typed backup or restoration gaps rather
   than silently manufacturing continuity. Backup creation belongs to the
   ordinary reconciler or an explicit typed repair path; doctor remains
   read-only.

Provider-free acceptance requires strict update/readback, relation validation,
WAL-aware byte accounting, read-only doctor behavior, successful verified
backup rotation, injected corrupt-copy rejection without replacing the prior
verified backup, and a restart readback of the same manifest. Stop and reframe
if backup requires provider state, broad filesystem cleanup, arbitrary command
execution, or a second mutable configuration authority. URL history compaction
and daily summaries remain M4-P2 after this storage foundation is source-qualified.

M4-P1 is source-qualified at `4ed5820d`. Its provider-free acceptance passes.
The read-only projection is currently exposed through runtime-config status;
joining the same storage projection into the ordinary aggregate status and
doctor outputs remains explicit M4 work and no doctor mutation is introduced.

### M4-P2 | Exact URL History Compaction

M4-P2 is the next bounded source packet. It owns only G16 and G17 history
retention: measure exact navigation history against the SQLite-owned 64-MiB
limit, summarize the oldest eligible navigation rows by UTC day with first and
last URL, count, session/browser/profile identities and incident linkage, and
record an auditable compaction event in the same transaction. Material
lifecycle and recovery events remain exact. Compaction must be deterministic,
restart-safe, idempotent, and must not retain bodies, screenshots, heartbeats,
raw logs, credential material, or repeated polls. It does not own provider,
privilege, resource-pressure, runtime publication, or installed acceptance.

M4-P2 is source-qualified at `fe46f53c`. Its provider-free model, SQLite-host,
configuration-contract, documentation, formatting, and strict Clippy gates
pass. G17 is partial rather than pass because its indefinite material-event
retention clause remains open.

### M4-P3 | Operational Status, Doctor, And Pressure Audit

M4-P3 starts read-only. Reconcile existing aggregate Service status, install
doctor, Browser Session Authority resource observations, launch admission, and
privileged-repair receipts against G30, G33, and G39. Identify which current
memory, process, disk, integrity, size, migration, keeper, display-generation,
queue, control-owner, handoff-recovery, and repair fields are already joined
and which remain absent. No live probe, provider mutation, process cleanup,
privileged command, browser launch, or installed publication is authorized by
the audit. A source packet follows only from the verified gap map.

M4-P3A is source-qualified at `91cb79c8`. It closes the Browser Runtime SQLite
status/doctor join only. The remaining packet is M4-P3B: define provider-free,
cross-platform current memory/process/disk observations at the exact browser
launch boundary, fail closed with typed non-disruptive rejection, expose the
same observation through status and doctor, and finish the G30 field map.

M4-P3B is source-qualified at `9aad8500`. The exact pre-launch boundary now
fails closed on typed current memory, profile-filesystem disk, host-process,
and managed-browser pressure and status/doctor publish the same observation.
G33 remains partial pending final target-platform and pressure qualification.
M4-P4 owns the bounded G20 narrow privilege-helper and receipt audit; it does
not authorize a privileged command or provider effect.

M4-P4 is source-qualified at `9f6505b7`. Healthy cold starts now make zero
privileged calls and exact repair receipts bind prior observation, sealed
action, and ready postcondition. G20 remains partial until installed
qualification. M4-P5 starts read-only and owns G34 provider-row reconstruction
plus the remaining operational placement, queue, scale-in, and doctor gap map.

### A01 Source Checkpoint

The repair batch is source-qualified without publication or provider effects.
Focused Cargo receipts executed one dispatch regression, one stream-redaction
regression, one terminal propagation regression, and one exact classifier
regression. The broader filtered desktop-interaction suite passed 40 tests, the
complete service-model crate passed 273 tests, and the Desktop Services crate
passed 18 tests. Strict workspace Clippy, Rust formatting, architecture
ownership, route-confusion gates, and patch hygiene pass.

The selector-recommended CDP streaming live smoke failed twice before browser
launch. A preserved debug rerun reports
`browser_runtime_database_missing:<temporary-home>/.agent-browser/service/runtime.sqlite3`
from the runtime host. The similarly named Rust filter selected zero tests and
is not counted as evidence. These receipts remain visible as an inherited
runtime-database readiness blocker; they neither invalidate the provider-free
A01 proof nor qualify CDP live streaming. No automatic retry or runtime repair
is authorized by this checkpoint.

### A02 SQLite And Ledger Checkpoint

The ordinary managed-desktop path no longer depends on JSON state or a parallel
JSON idempotency ledger. Browser, session, tab, route, display, viewer-control,
handoff, and interaction-operation authority now resolve from Browser Runtime
SQLite. Legacy `operations.json` is parsed strictly, replayed idempotently into
the SQLite journal, renamed to a read-only archive, and never used for ordinary
effects afterward. A crash between journal commit and rename is safe because
the next import must match the exact operation request and terminal result.

Provider-free validation passes 43 desktop-interaction tests, 21 desktop-capture
tests, 50 browser-session-store tests, two direct SQLite replay/import tests,
the architecture detector self-test, formatting, and strict workspace Clippy.
The Desktop Services crate remains unchanged and its 18 tests pass. This closes
A02's named dependency packet, not G04 or whole-product P03/P05. No development
candidate or provider effect was used.

The operator resumed this amended plan with a 1,000,000-token cumulative
ceiling before stop and checkpoint. Reserve 200,000 tokens for closeout and
stop sustained implementation at 800,000. The M2B outcome stop remains
historical fact. A new packet name does not reset attempts or justify automatic
replay after a second causal failure.

### Worker Assignments

One primary owns this serialized packet, findings disposition, source changes,
ledger, and validation. No worker or new worktree is assigned. Shared write
surfaces are the desktop interaction/dispatch/terminal adapters, relevant
fixtures, and P219 planning authorities. P214 retains candidate-event source
ownership; coordinate an explicit overlap before touching its crate surfaces.
Documentation or pure fixtures may be independently assignable later, only
after shared interfaces and exact file ownership are fixed.

### Evidence And Exit

The planning record is complete when this revision, RUNBOOK.md, ROADMAP.md,
and P219's catalog projection agree and documentation checks pass. That is
separate from the unstarted repair packet's exit:

- A01 has red-then-green provider-free evidence through dispatch, terminal
  classification, and replay, preserving uncertain effects and success cases.
- A02 has exact dependency evidence, a named completion packet, and explicit
  G04/P03/P05 gaps; whole-product SQLite acceptance stays open until closed.
- A03–A06 have scoped dispositions, usable execution accounting, reconciled
  evidence locators, and explicit flake/candidate attribution.
- Changed Rust passes formatting and strict workspace Clippy through
  `scripts/ci/cargo-safe.sh`, plus affected Rust/Service/client contract gates
  selected from the complete repair baseline. Run documentation, coverage,
  and architecture checks; a detector gap remains incomplete evidence.
- Verify only this accepted finding set and critical regressions in its fixes.
  Do not restart broad drift discovery or erase unaffected M2B evidence.

Use an independently bounded successor only if shared-helper production
compatibility is separately commissioned, or a proven dependency changes the
delivery outcome enough that P219 cannot contain it coherently. Such a successor
inherits controls and evidence; it does not close P219 or grant production
authority. Otherwise continue through revisions and packets of P219.

Execution checkpoint on September 26: M1B-B02 is complete. The trusted product
no longer deserializes or replays legacy runtime-owner transaction sidecars and
no longer embeds or exposes protected owner observations through Service State,
launch metadata, inventory, generated client or public documentation. P15,
P16, `serviceModelLeaseAuthority` and `legacyAuthorityQuarantine` pass; strict
workspace Clippy passes in `job-20260926T185357Z-7d9bc2827369`. G23 and G41
are promoted to pass. B05 and B06 source closure is complete. G25 is partial
and P09 passes after private SQLite custody, legacy-key scrubbing, transient
privileged-helper stdin and provider-sync replay fixtures. Installed provider
cleanup, rotation and handoff-identity evidence remain in M2A.

The preceding `0.28.0-9484cfa7ef3b` M2A checkpoint remains historical evidence:
it reached authenticated Guacamole connections 1 and 2, then lost each
runtime-owned primary after roughly 6–7 seconds. The later helper repairs and
installed generation `0.28.0-2423cfb064f2` supersede that environment diagnosis
without erasing receipt `apply-1790451432634-9571.json`.

### Bounded successor repair packet | named-profile retention

Outcome: exact named-profile sessions and their durable opaque handoffs do not
expire through the disposable-profile inactivity policy. Explicit close,
configured named retention if one is later introduced, or a typed unrecoverable
condition may still terminate them. Disposable sessions retain bounded
inactivity expiry. Handoff access and successful commands refresh only the
addressed session; viewer heartbeat remains separate.

Primary write surface: the service-model session lifecycle, the SQLite host and
runtime configuration adapter, and their focused tests. Update public contracts
or operator docs only if the externally configurable surface changes. Do not
use a keepalive, extend the global five-minute value, or special-case the P219
fixture.

Evidence and exit: first add or identify a regression that demonstrates an
exact named session and its same handoff surviving beyond the disposable idle
window while a disposable session expires. Preserve explicit-close terminality,
success-only exact-session refresh, and restart-load behavior. Then run the
focused model and host tests, formatting, and strict workspace Clippy required
by the Rust change. Stop after a source-qualified custody commit and updated
evidence. A new development build, provider mutation, or M2B replay requires a
separate candidate-freeze decision against the remaining cumulative allowance.

Source checkpoint on September 27: complete at `d7ec8d2f`. The initial named
retention regression failed in receipt
`20260927T011608Z-b158cd9e9ef2`, then passed in
`20260927T011720Z-4e968fc1e064`. The disposable-expiry regression passed in
`20260927T011744Z-d4d2d97df757`; the complete 31-test session-manager file
passed in `20260927T011818Z-d060623a7039`; all 271 Service Model tests passed in
`20260927T012640Z-743ad42f95fc`; and the 32 host/navigation tests passed in
`20260927T012656Z-65f6ac880cfc`. Restart-load normalization passed with the
documented cache-off retry in `20260927T012046Z-532cd34873bf`. Formatting passed
in `20260927T012633Z-cc299e28a1ec`, and strict workspace Clippy passed without
warnings in `job-20260927T012703Z-d135787f09f1`.

The selector-required workstation, host-provision, fresh-VM, Guacamole asset,
PostgreSQL durability, and route-user fixtures passed. The docs production
build, remote-view documentation contract, policy wiring, architecture report,
coverage-ledger validator, and diff checks passed. A broader name-filtered Rust
run retained four unrelated `browser_session_authority` failures; the changed
session-manager, host, navigation, and daemon regressions pass independently.
The development skill was synchronized during candidate publication. The
shared production skill and production runtime remained unchanged.

### Frozen candidate and provider quarantine checkpoint

Candidate publication is complete at `d26a25b9`. The optimized candidate build
passed without warnings in cargo-signal job
`job-20260927T015043Z-0ed4e4be3483`, producing SHA-256
`9502ef082e45862674821af690af2a5f033c0109f529c534664d1fbc925422b0`.
Development installation selected generation `0.28.0-9502ef082e45`, kept the
production identity unchanged, synchronized the development skill, and passed
three disposable open, URL-read, close, and residue checks.

Provider plan, stage, and preflight passed with the exact validated public
operator binding. The apply then quarantined with receipt
`apply-1790473069666-74987.json`. The retained evidence showed one live route on
`:58` and exact runtime-owned stale artifacts for absent routes. A red helper
regression reproduced survival of the primary channel socket; the green repair
is committed at `e79aa713`. Focused helper-contract Rust tests passed in
`20260927T014643Z-72914e6cb696`, formatting passed in
`20260927T014717Z-9ce8b4fcd884`, and strict workspace Clippy passed without
warnings in `job-20260927T014717Z-9bda3f5b681b`. Installer convergence is
committed at `4da14475` and `d26a25b9`.

Next gate: install the source version 10 privileged helper through an
interactive sudo path, verify the exact capability readback, then rerun the
development provider plan, stage, preflight, and one apply. Do not retry the
rejected credential file or broaden cleanup. Only after four-route readiness
may the frozen candidate proceed to installed named-retention and the remaining
M2B Alice/Bob workflow. This checkpoint promotes no additional G-row to pass.

### Live-viewer boundary and refreshed candidate checkpoint

Source checkpoint `bdf695db` makes the live-viewer TTL boundary strict. A
heartbeat at the exact expiry instant now returns
`live_viewer_lease_inactive`; both current-control validation and expiry use an
inclusive deadline. A second regression proves that live-viewer heartbeat and
disconnect leave Browser Session Manager state unchanged, while disconnect
removes current desktop control. The focused `live_viewer_` filter passed four
tests in receipt `20260927T021202Z-a0456af8a0fa`; the complete 11-test desktop
control module passed in `20260927T021442Z-dbae322f207f`. Formatting passed in
`20260927T021442Z-42874926b5c2`, and strict workspace Clippy passed without
warnings in `20260927T021455Z-db18b3da9800`.

The deterministic architecture detector now passes P19 only when the live
viewer endpoint performs exact active-connection observation, connect,
heartbeat and disconnect are present, SQLite stores the bounded viewer record,
current boot and controller authority are revalidated, the dashboard drives
heartbeat and disconnect, and neither Browser Session Manager nor presentation
admission consults viewer authority. Its red-then-green self-test and a negative
missing-inclusive-expiry fixture pass. The coverage ledger therefore changes
G35 and G45 from fail to partial and now totals two pass, 31 partial, three
fail, and nine missing.

The ignored browser-backed same-profile fixture remains diagnostic rather than
acceptance evidence. Its first retained run failed because the disposable test
runtime lacked its SQLite database in receipt
`20260927T020349Z-39f194809aba`. After a temporary fixture-only migration, it
advanced to the correct `presentation_keeper_unavailable` boundary in receipt
`20260927T020645Z-3f7e82c03caa`. The temporary change was reverted. Do not
restore the legacy JSON route inventory fallback; current browser-backed proof
requires SQLite keeper authority.

The refreshed optimized development candidate at `375cdfbe` installed as
generation `0.28.0-57c51d356866`, executable SHA-256
`57c51d3568663fe9f50bd14bab69bc183c71b37b819decd22d64ee62c8f35cf2`, and
passed all three disposable browser-launch smoke iterations. Production stayed
on its prior generation. The presentation provider remains stopped and not
ready, all six keeper records are absent, and no development XRDP route process
remains. The root-owned helper needs the interactive version 11 upgrade before
provider recovery and installed Alice/Bob acceptance.

### M2B installed acceptance checkpoint

The operator installed helper version 11. Provider plan, stage, preflight, and
apply converged with four ready warm displays. The final published candidate is
development generation `0.28.0-ac3c1daed8ab`; its doctor reports every check
green and separately confirms the production identity is unchanged.

The installed Alice/Bob workflow passed the six M2B cases:

1. Alice's authenticated opaque handoff rendered current Guacamole pixels at
   1152 by 640 during the initial input proof. Pointer, keyboard, and scroll
   effects passed.
2. Alice and Bob shared one exact-profile browser while retaining distinct
   session, tab, target, handoff, heartbeat, and expiry identities. Repeated
   and concurrent opens created no duplicate browser.
3. Successful session-addressed commands refreshed only the addressed session;
   failed commands did not refresh it. Focus and desktop effects used current
   viewer and provider-generation fences.
4. Authenticated viewer connect, heartbeat, disconnect, and control transfer
   passed. A stale Alice control attempt was rejected after Bob took control;
   Bob's current interaction completed all 41 guarded effect keys.
5. Closing Alice preserved Bob and the browser. Closing Bob terminated the
   browser, confirmed by a fresh process census. A separate disposable pair
   proved Alice expired with `heartbeat_expired` while active Bob survived and
   remained commandable.
6. Two successive runtime-host replacements preserved the exact named-profile
   sessions, logical browser ID, route slot, and original opaque handoff IDs.
   Each ordinary exact-session reopen created one replacement process and fresh
   targets. Both original handoffs resolved through authenticated public ingress
   with `operatorVisible.state=ready`, `uxState=connected`, and embedded
   Guacamole pixels. The final screenshot visibly showed the Alice fixture and
   Bob's peer tab. Both pre-restart page markers read back as `null`; navigation
   history advanced from 136 to 140 only for the four explicit reopens. The
   final Alice close preserved Bob and PID 54817; the final Bob close removed
   the browser record and process.

The dashboard route projection defect found during the restart case is repaired:
when runtime-host adoption restores a ready keeper route before the legacy
`remoteViewRoutes` projection, the Browser Session Manager now projects the
authoritative `routePool` entry as the view stream. The viewer-client acceptance
helper also recognizes the current connected viewport without requiring the
retired refresh control. The operator-directed future Guacamole UX and compact,
dismissible warning banners remain a nonblocking follow-up.

The reconciled 45-row coverage ledger now records nine pass, 26 partial, three
fail, and seven missing. M2B promotes G11, G24, G31, G35, G43, G44, and G45 to
pass; G36 advances from missing to partial because installed expiry isolation
passed while delayed profile deletion and quota cleanup remain later work.

Changed-surface validation is complete. Rust format and strict workspace Clippy
pass. The comprehensive Rust runner passed every compartment except two
deterministic service-reconcile compatibility assertions and one process-timing
Lease Authority assertion on its first attempt. Restoring explicit zero-valued
legacy viewer counters and counting orphaned route removal in the aggregate
release total repaired the two service assertions; their focused reruns and the
complete 40-test actions compartment pass. The Lease Authority assertion passed
immediately in isolation and the complete 118-test compartment passed on rerun.
All other comprehensive compartments retain their green results under policy
0042/0072. Dashboard projection, viewer-client, route-confusion, service parity,
generated-client, docs, coverage-ledger, workstation installer, host provision,
fresh-VM, Guacamole asset, PostgreSQL durability, route-user synchronization,
policy wiring, and documentation-link gates pass. Final development doctor is
green, the development skill copy is current, and a fresh OS census reports no
development managed-profile Chrome root after final cleanup.

Goal-service note: thread `01a0dfee-e1ab-79e0-b34b-b9fa80a58d0c` still exposes
a stale blocked status from an earlier gate. The operator explicitly resumed
execution, expanded the continuation ceiling to 2,000,000 tokens, and directed
this Plan 0219 continuation. The plan and current runtime evidence govern; the
stale service status is not acceptance evidence.

Deferred, nonblocking UI direction: use the sibling `../remote-view` project as
the Guacamole interaction reference, and make warning banners compact and
dismissible so they do not permanently consume viewer height. This direction
does not expand the retention repair packet.
