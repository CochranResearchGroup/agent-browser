# Runbook

## Current P219 status | 2026-09-28 G18 and G34 ledger reconciliation

[Plan 0219 version 47](docs/dev/plans/0219-2026-09-26-alice-bob-grilling-contract-completion.md)
repairs stale machine narratives for G18 and G34. G18 now cites M4-P1's
read-only integrity, WAL-aware budgets, verified two-copy online backup,
corruption preservation, manifest, and restoration-gap source proof. G34 now
cites M4-P5's exact owned-provider rebuild and PostgreSQL unrelated-row
preservation fixture. Both remain partial only for their exact frozen-candidate
installed qualification. Ledger counts remain 13/30/2/0; no runtime effect
occurred.

## Current P219 status | 2026-09-28 G39 doctor detector

[Plan 0219 version 46](docs/dev/plans/0219-2026-09-26-alice-bob-grilling-contract-completion.md)
adds the deterministic `doctorReadOnly` architecture cut at `c5f4db2a`.
Install doctor and remote-view doctor fail the cut if either directly mutates
the filesystem, reconciles or migrates runtime state, creates a backup,
persists a privileged receipt, or invokes an `--apply` effect. The negative
fixture and current zero-finding report pass. G39 remains partial for installed
no-mutation qualification and the complete explicit-repair inventory. Ledger
counts remain 13/30/2/0; no runtime effect occurred.

## Current P219 status | 2026-09-28 G30 receipt projection source-qualified

[Plan 0219 version 45](docs/dev/plans/0219-2026-09-26-alice-bob-grilling-contract-completion.md)
source-qualifies commit `7d043e45`. Privileged repair now persists a
semantically validated v2 receipt through an atomic user-private replacement.
Service status and install doctor expose the same redacted receipt state even
when Browser Runtime SQLite is unavailable, and text doctor prints that state.
The sealed plan digest and action list remain absent; missing, unreadable,
invalid, and not-ready receipts fail closed. Focused Rust, formatting, strict
Clippy, documentation, remote-view, workstation, Guacamole, and PostgreSQL
gates pass. G30 remains partial only for complete installed status/doctor
readback. Ledger counts remain 13/30/2/0; no runtime effect occurred.

## Current P219 status | 2026-09-28 G30 read-only projection

[Plan 0219 version 44](docs/dev/plans/0219-2026-09-26-alice-bob-grilling-contract-completion.md)
adds one redacted read-only SQLite reconciliation aggregate to Service status
and install doctor. It covers keeper generations, browsers per display,
handoffs, operations, queue state, control/viewer slots, recovery, migration,
storage/backup, and launch pressure without provider URLs, route users,
credentials, or Guacamole identities. The focused digest/no-backup regression
passes. G30 remains partial for privileged-repair receipt projection and
installed readback. Ledger counts remain 13/30/2/0; no runtime effect occurred.

## Current P219 status | 2026-09-28 G29 source closure

[Plan 0219 version 43](docs/dev/plans/0219-2026-09-26-alice-bob-grilling-contract-completion.md)
atomically records each real runtime-config revision and keeper-policy change
with a bounded durable history, exposes it in authoritative readback, and
preserves it across restart. Invalid and no-op patches add no event. Lowered
limits report over-target, block worsening admission, preserve existing work,
and converge through ordinary release. Focused Rust, formatting, strict Clippy,
generated-client and direct service-client checks, parity, docs, route-confusion,
coverage, workstation, and PostgreSQL fixtures pass. The umbrella service-client
command retains the inherited missing P157 oracle-source failure before the
changed lane; its downstream checks pass directly. The development skill is
current and the production skill is untouched. The ledger is 13 pass, 30
partial, two fail, and zero missing. No runtime or provider effect occurred.

## Current P219 status | 2026-09-28 G28 final-reference source closure

[Plan 0219 version 42](docs/dev/plans/0219-2026-09-26-alice-bob-grilling-contract-completion.md)
joins current Desktop Services control and unexpired authenticated viewer routes
into the final transactional scale-in check. Browsers, handoffs, operations,
presentation admission, keeper phases, and cooldown remain in the same check;
ambiguous control/viewer state fails closed. Four focused scale-in tests,
formatting, strict workspace Clippy, and patch hygiene pass. G28 remains partial
only for joined installed-provider scale-in after cooldown. The ledger remains
12 pass, 31 partial, two fail, and zero missing. No runtime or provider effect
occurred.

## Current P219 status | 2026-09-28 G10 source closure

[Plan 0219 version 41](docs/dev/plans/0219-2026-09-26-alice-bob-grilling-contract-completion.md)
closes G10 at source. Presentation admission preserves strict recovery,
retained-handoff, then FIFO new-open priority; the durable configured queue
defaults to 32, exact duplicate operations coalesce and replay once, and
logical session/tab reuse remains at the G31 manager boundary. The focused
queue suite, formatting, strict workspace Clippy, coverage validator,
generated-client checks, type coverage, and service API/MCP parity pass. The
row-by-row audit also repairs stale machine dispositions for G17, G20, G33,
G37, and G38 from their already-qualified M3-P3 and M4-P2 through M4-P4
checkpoints. The authoritative JSON ledger is now 12 pass, 31 partial, two
fail, and zero missing, matching the prior checkpoint narrative. No installed
candidate or provider state changed. Exact workstation cold-upgrade
reconstruction remains the G34 gate.

## Current P219 status | 2026-09-28 final M4 candidate published to development

[Plan 0219 version 40](docs/dev/plans/0219-2026-09-26-alice-bob-grilling-contract-completion.md)
publishes source-qualified commit `7e34acce` through custody checkpoint
`7d840169` as isolated development generation `0.28.0-3e4532b3d100` with full
binary digest `3e4532b3d100b25e8dc62b54de6394f2ad39794bdd46b06182c4c1892ecdc768`.
The development skill is current, doctor is fully green, all six provider routes
remain ready, and production identity and tracked state are unchanged.

A fresh OS process census records the three selected development service
processes and preserves all pre-existing production and foreign browser trees.
No cleanup was attempted. This publication does not execute the exact
workstation cold-upgrade reconstruction path, so G34 and the ledger remain 11
pass, 32 partial, 2 fail, and 0 missing. The next gate is a bounded installed
cold-reconstruction acceptance procedure that does not borrow proof from the
provider-free PostgreSQL fixture or from ordinary development publication.

## Current P219 status | 2026-09-28 M4-P5 source-qualified

[Plan 0219 version 39](docs/dev/plans/0219-2026-09-26-alice-bob-grilling-contract-completion.md)
source-qualifies the bounded Agent Browser-owned Guacamole namespace rebuild at
commit `7e34acce`. A disposable PostgreSQL 16 fixture executes the generated
transaction against the packaged schema and proves that an unrelated
connection, parameter, sharing profile, permission, entity, and user survive
exactly while the configured routes are reconstructed. Required CLI, README,
skill, inline, and docs-site guidance is aligned. The complete selected source
gates pass, including formatting, strict workspace Clippy, focused Rust,
workstation and Guacamole fixtures, docs build and links, and validation
selection.

G34 remains partial and the ledger stays 11 pass, 32 partial, 2 fail, and 0
missing until a separately governed installed cold-upgrade reconstruction
proves the retained provider database and resulting route topology. No installed
runtime, browser, provider, route, display, privileged, publication, production,
merge, or release effect occurred in this source packet.

## Current P219 status | 2026-09-27 M4-P5 intermediate custody

[Plan 0219 version 38](docs/dev/plans/0219-2026-09-26-alice-bob-grilling-contract-completion.md)
is OPEN. Commit `8dcbfd89` adds an exact Agent Browser-owned Guacamole
namespace rebuild primitive to cold workstation reconciliation. Focused
route-user and source-free workstation fixtures, shell syntax, Python
compilation, and patch hygiene pass. This is not M4-P5 source qualification:
documentation, full Rust gates, unrelated-row preservation proof, and installed
provider reconstruction remain open. The ledger stays 11 pass, 32 partial, 2
fail, and 0 missing. No provider or other runtime effect occurred.

## Current P219 status | 2026-09-27 M4-P4 source-qualified

[Plan 0219 version 37](docs/dev/plans/0219-2026-09-26-alice-bob-grilling-contract-completion.md)
is OPEN. Commit `9f6505b7` makes healthy privilege-installer reruns use only
unprivileged metadata and capability checks and add zero privileged commands.
Repairs retain the one explicit authorization and sealed narrow action list;
the validated v2 receipt records resource, prior observation, action, outcome,
and ready postcondition.

The clean privilege and complete workstation-host fixtures, focused Rust
receipt test, shell syntax, documentation checks and build, formatting, and
strict workspace Clippy pass. G20 moves from missing to partial pending final
installed qualification; the ledger is 11 pass, 32 partial, 2 fail, and 0
missing. No installed runtime, browser, provider, privileged, publication,
production, merge, or release effect occurred. M4-P5 owns provider-row
reconstruction and the remaining operational gap map.

## Current P219 status | 2026-09-27 M4-P3B source-qualified

[Plan 0219 version 36](docs/dev/plans/0219-2026-09-26-alice-bob-grilling-contract-completion.md)
is OPEN. Commit `9aad8500` adds a cross-platform current-resource snapshot at
the exact managed Chrome launch boundary. It checks memory, profile-filesystem
disk, host process capacity, and managed root browser count before display or
browser effects and fails closed with typed pressure reasons. Service status
and install doctor expose the same `browserRuntime.launchAdmission` projection.

Focused admission, read-only store, and status tests, generated-client drift
and types, API/MCP parity, documentation checks and build, formatting, and
strict workspace Clippy pass. The attempted Windows cross-check was blocked
before source compilation by the WSL host's missing MinGW compiler. G33 moves
from missing to partial; the ledger is 11 pass, 31 partial, 2 fail, and 1
missing. No installed runtime, browser, provider, privileged, publication,
production, merge, or release effect occurred. M4-P4 owns the G20 narrow
privilege-helper and receipt audit.

## Current P219 status | 2026-09-27 M4-P3A source-qualified

[Plan 0219 version 35](docs/dev/plans/0219-2026-09-26-alice-bob-grilling-contract-completion.md)
is OPEN. Commit `91cb79c8` joins read-only Browser Runtime SQLite state into
aggregate Service status and install doctor. Both report typed configuration,
migration, integrity, WAL-aware sizes, history budget, backup verification,
and restoration gaps without migration or backup effects. Unavailable state
redacts the local path to a stable code. The regression preserves the database
digest and creates no backup.

Focused store/status tests, generated-client drift and types, API/MCP parity,
documentation checks and build, formatting, and strict workspace Clippy pass.
G30 and G39 gain source evidence but remain partial; the ledger remains 11
pass, 30 partial, 2 fail, and 2 missing. No installed runtime, browser,
provider, privileged, publication, production, merge, or release effect
occurred. M4-P3B owns current memory/process/disk launch admission and the
remaining operational status map.

## Current P219 status | 2026-09-27 M4-P2 source-qualified

[Plan 0219 version 34](docs/dev/plans/0219-2026-09-26-alice-bob-grilling-contract-completion.md)
is OPEN. Commit `fe46f53c` completes deterministic exact-navigation history
compaction. The daemon consumes the live SQLite byte limit. Oldest exact rows
compact before atomic publication into UTC-day summaries keyed by exact
profile, session, browser, tab, and target identity, with first/last URL and
time, count, sorted incident references, and a bounded compaction audit trail.
Restart and repeated compaction preserve deterministic results without adding
bodies, screenshots, heartbeats, raw logs, or repeated polls.

The complete 283-test service-model suite and the SQLite host publication test
pass, along with runtime-limit validation, API/MCP parity, documentation checks
and build, formatting, and strict workspace Clippy. G17 moves from missing to
partial because indefinite material lifecycle and recovery-event retention is
still open; G16 remains partial. The ledger is 11 pass, 30 partial, 2 fail, and
2 missing. No installed runtime, browser, provider, publication, production,
merge, or release effect occurred. M4-P3 begins with a read-only audit of
aggregate status, doctor, and current-resource-pressure gaps.

## Current P219 status | 2026-09-27 M4-P1 source-qualified

[Plan 0219 version 33](docs/dev/plans/0219-2026-09-26-alice-bob-grilling-contract-completion.md)
is OPEN. Commit `4ed5820d` completes the provider-free Browser Runtime SQLite
storage-authority packet. Typed configuration now owns the exact-history,
live-database, and routine-storage limits. Read-only runtime-config status
reports SQLite integrity, database and WAL bytes, exact-history bytes, backup
bytes, budget state, and restoration gaps. The explicit backup action creates
an online SQLite copy, verifies it before atomic rotation, retains current and
previous copies with a strict digest-and-size manifest, and never restores or
deletes the live database.

Focused storage, runtime-config, service-config, and contract tests pass, as do
generated-client drift and type checks, API/MCP parity, remote-view docs,
production docs build, formatting, and strict workspace Clippy. G16, G17, G18,
G22, and G30 gain bounded source evidence without changing their ledger
dispositions. The ledger remains 11 pass, 29 partial, 2 fail, and 3 missing.
No installed runtime, browser, provider, publication, production, merge, or
release effect occurred. M4-P2 owns exact URL history compaction and daily
summaries; the aggregate status/doctor join remains later M4 work.

## Current P219 status | 2026-09-27 M4-P1 storage audit

[Plan 0219 version 32](docs/dev/plans/0219-2026-09-26-alice-bob-grilling-contract-completion.md)
is OPEN at pushed checkpoint `3e2ffa5e`. The v2 SQLite row already carries the
accepted exact-history, live-database, and routine-storage byte defaults, but
the strict update contract omits them and no executable path enforces or reports
their budgets. Ordinary Browser Runtime SQLite also lacks an integrity report,
verified rotating online backup, and restoration-gap receipt. M4-P1 owns only
that configuration and storage-authority foundation. URL compaction is deferred
to M4-P2; privilege, pressure admission, provider reconstruction, and installed
qualification remain later packets. The ledger remains 11 pass, 29 partial,
2 fail, and 3 missing. No runtime or provider effect occurred during the audit.

## Current P219 status | 2026-09-27 M3-P3 source-qualified

[Plan 0219 version 31](docs/dev/plans/0219-2026-09-26-alice-bob-grilling-contract-completion.md)
is OPEN. Commit `93a740f6` completes the provider-free disposable-retention
packet. SQLite owns live inactivity, count, and byte settings. Cleanup ranks
the oldest inactive disposable allocation, protects current viewer, controller,
and pending-operation sessions from one SQLite snapshot, commits eviction
before replacement admission, terminalizes handoffs, compacts history, and
deletes only an idempotent direct-child directory without following symlinks.
Named profiles cannot enter the candidate set, and strict policy decoding
rejects pinning or promotion fields.

G37 and G38 advance from missing to pass. G21 and G36 remain partial until the
frozen installed candidate proves joined retention. The ledger is 11 pass,
29 partial, 2 fail, and 3 missing. The complete 282-test service-model suite,
focused host/store/runtime tests, strict Clippy, formatting, generated-client
contracts, route-confusion gates, documentation checks, and docs build pass.
The broad browser-session filter retained four unrelated environment-sensitive
process-census fixture failures; its other 131 executed cases passed. No
installed runtime, browser, provider, publication, production, merge, or
release effect occurred. M4 requires a bounded successor packet before source
or runtime effects.

## Current P219 status | 2026-09-27 M3-P3 retention audit

[Plan 0219 version 30](docs/dev/plans/0219-2026-09-26-alice-bob-grilling-contract-completion.md)
is OPEN at pushed checkpoint `762a7f10`. The M3-P3 audit confirms that SQLite
already stores the accepted disposable inactivity, count, and byte defaults,
but typed updates omit them, disposable sessions still consume the legacy
five-minute timeout, and no count/byte quota engine joins viewer, control, and
pending-operation protection. Existing cleanup is direct-child and
reference-checked but is not oldest-inactive quota enforcement. M3-P3 now owns
the bounded source repair and provider-free proof for G21/G36/G37/G38. The
ledger remains 9 pass, 29 partial, 2 fail, and 5 missing until executable
evidence lands. No runtime or provider effect occurred during the audit.

## Current P219 status | 2026-09-27 M3-P2D source-qualified

[Plan 0219 version 29](docs/dev/plans/0219-2026-09-26-alice-bob-grilling-contract-completion.md)
is OPEN. Commit `0b728fc0` completes the provider-free eager-scheduler packet.
Baseline presentation capacity remains Route Keeper authority and cannot enter
the per-browser recovery demand model. Scheduled browser recovery requires a
current-boot, unexpired authenticated controlling viewer joined through the
current SQLite handoff, session, browser, and Route Keeper authority. Candidates
deduplicate by browser ID, revalidate at admission, receive a fresh exact
liveness observation, and use the existing M3-P2C journal, generation fence,
reserved launch/adoption path, and atomic publication. The same logical session
and opaque handoff survive replacement, current desktop control is rebound, and
restart cannot create a competing replacement. Dormant named browsers remain
lazy.

All 278 service-model crate tests, 65 store tests, 26 host tests, 37 Route
Keeper tests, focused stale-authority and restart fixtures, formatting, strict
workspace Clippy, documentation checks and build, and selected source-free
workstation/Guacamole contracts pass. The installed shared skill remains
unchanged by design. No browser, provider, installed-runtime, publication,
production, merge, or release effect occurred. The ledger remains 9 pass,
29 partial, 2 fail, and 5 missing. G14 and G15 remain partial until installed
cold-start and visible joined recovery are qualified. M4 remains ineligible;
the next bounded packet owns the remaining M3 retention rows.

Goal thread `01a0e57f-4aa3-7630-b4d7-50227d132ba5` reported 323,934 cumulative
tokens used after source validation, below the 800,000 implementation stop and
the 1,000,000 checkpoint ceiling.

## Current P219 status | 2026-09-27 M3-P2C source-qualified

[Plan 0219 version 28](docs/dev/plans/0219-2026-09-26-alice-bob-grilling-contract-completion.md)
is OPEN. Commit `0522fe95` completes M3-P2C. The journaled exact-client path
observes the selected retained browser before one immediate SQLite transaction
admits a per-browser recovery generation and binds it to `launch_started`.
Distinct open operation IDs retain independent operation fencing, while only
one receives replacement authority. A successor probes and adopts the exact
reserved launch without a second effect. The durable `observed_live` phase is
separate from handoff readiness, and final session, handoff, open result, and
recovery success publication is one generation-fenced transaction.

Focused recovery-model, manager, store, journal, concurrency, crash-recovery,
retry/deadline, formatting, and strict workspace Clippy gates pass. Stale
recovery generations and base-state conflicts publish nothing. The source-only
packet performs no browser, provider, installed-runtime, publication,
production, merge, or release effect. The ledger remains 9 pass, 29 partial,
2 fail, and 5 missing. G15 gains direct host-path evidence but remains partial
until joined installed recovery is qualified. M3-P2D is next and was not begun;
later M3/M4 work remains open.
The post-source-validation goal readback was 689,484 tokens used and 310,516
remaining, so implementation stopped below 800,000 with the required closeout
reserve intact.

## Current P219 status | 2026-09-27 terminal budget checkpoint

[Plan 0219 version 26](docs/dev/plans/0219-2026-09-26-alice-bob-grilling-contract-completion.md#fresh-context-continuation-after-m2b)
is OPEN. The operator resumed the amended continuation with a cumulative
1,000,000-token ceiling before stop and checkpoint, including a 200,000-token
closeout reserve and an 800,000-token implementation stop. Goal-service thread
`01a0e3f7-adbb-7312-b748-ce5a462ccd90` reported 141,869 tokens used at startup
but no remaining-token field. The read-only audit preserved successful M2B subproofs and found a
desktop error receipt reported as outer success, remaining ordinary desktop
JSON dependencies, and policy/budget, ledger, shared-helper, candidate and retry
provenance gaps. The linked continuation owns findings A01–A06 and their exits.
A01 is source-qualified: failed desktop receipts now produce truthful outer,
job, event, and terminal failures with uncertain-effect recourse and redacted
diagnostic preservation. A02 is also source-qualified: managed capture,
ready-handoff lookup, and interaction replay now use Browser Runtime SQLite,
with one-time fail-closed import and archival of the former JSON ledger. A03–A06
are reconciled with the current budget, scoped helper/binary identities, ledger
rows, and retained first-failure disposition. M3-P1 is now the active bounded
provider-free packet: pure eager/lazy recovery decisions plus SQLite-serialized
singular admission, exact old-browser unusability proof, and bounded retry.
It has no browser, provider, route, display, publication, or production effect.
M3-P1 is source-qualified: 276 service-model and 37 browser-session-store tests
pass, including a four-connection race with one SQLite admission winner.
Source checkpoint `6000b9fd` is clean. The goal service reports 712,314 tokens
used. M3-P2 is planned but unstarted: wire the SQLite admission fence into the
provider-free host recovery effect, add a fenced success/reset transition, and
inject values from the existing recovery configuration. No push, publication,
install, merge, or runtime effect occurred.
M3-P2A is source-qualified at `53c66ce9`: exact success is generation-fenced,
restart-durable, resets attempts only after success, and advances generation for
the next episode. The affected complete suites pass 277 and 38 tests; strict
Clippy and formatting pass. Goal usage is 753,564 and implementation is stopped.
Configuration translation and actual host fence consumption remain unstarted.
The read-only host audit proves those cannot safely be one unordered edit:
ordinary host configuration comes from SQLite, the older daemon retry policy is
environment-derived, exact-client replacement occurs after `launch_started`,
and scheduled reap only retains dead named records. M3-P2B must first establish
one SQLite policy authority; M3-P2C then fences exact-client replacement; M3-P2D
adds eager active-viewer/baseline scheduling without surprise-launching dormant
browsers.
The completion audit confirms only nine rows pass. Three fail, five are missing,
and 28 remain partial. P219 stays OPEN and M5 is ineligible. The critical path
is M3-P2B policy consolidation, M3-P2C exact-client fencing, M3-P2D eager
scheduling, remaining M3 retention, M4 operational closure, then one final
candidate adjudication.
The architecture audit independently reports 4 pass, 4 detector gaps, and 11
unverified prohibitions. Zero findings on an unimplemented detector is not
acceptance. M5 therefore remains ineligible on both requirement and prohibition
coverage.
The version 21 checkpoint readback was 817,438. Executable work stopped at
753,564; later usage was closeout-only. Remaining capacity under the ceiling
does not renew implementation. Resume at M3-P2B with all counters and evidence
carried forward.
The read-only M3-P2B inventory identifies every required store, startup,
consumer, schema, generated-client, CLI/help, README, skill, docs-site,
architecture, and ledger surface. Startup defaults must preserve existing
SQLite configuration; explicit config/env/CLI values become atomic bootstrap
updates before host construction; daemon and host then consume the same row.
The M3-P2C audit found that recovery and open publication currently have
separate transactions. The open journal can prove `browser_opened` before later
navigation/presentation failure, leaving a usable browser while recovery stays
`admitted`. Add an observed-live resumable phase and commit final recovery
success with open/session/handoff publication in one SQLite transaction before
wiring any replacement effect.
The M3-P2D audit corrects the eager-demand boundary. G14 baseline capacity is
presentation route capacity and cannot select or launch a profile browser.
Route Keeper policy owns eager route, Guacamole, and XRDP reconciliation.
Eager browser recovery requires a new current-boot, unexpired SQLite projection
from live viewer control through the exact current handoff, session, browser,
and route, deduplicated by browser ID and passed through the M3-P2C admission
and effect fence. Dormant dead named browsers remain effect-free until exact
handoff or named-session access. The browser-keyed `BaselineCapacity` model
case must be removed or constrained before scheduler wiring.
Version 25 reconciles this correction through the older M3-P1, M3 delivery,
and M3-P2 host-integration text. The historical baseline fixture remains a test
of generic mechanics only and is explicitly excluded from G14 acceptance and
all future browser-launch authority.
The goal-service pre-commit checkpoint readback was 931,884 cumulative tokens
used, 68,116 below the 1,000,000-token ceiling. Executable work stopped at
753,564. Plan 0219 remains OPEN; this window stops at the durable checkpoint
without converting unused ceiling space into implementation authority.

A01 validation passed 40 desktop-interaction tests, 273 service-model tests,
18 Desktop Services tests, the focused dispatch/redaction/terminal regressions,
formatting, strict workspace Clippy, architecture ownership, route-confusion
gates, and diff hygiene. The selected CDP live smoke remains unqualified: two
attempts failed before browser launch and a preserved debug receipt reports a
missing temporary-home runtime SQLite database. The Rust command selected zero
tests and is not evidence. No candidate was published and no provider effect
was attempted.

A02 validation passes 43 desktop-interaction tests, 21 desktop-capture tests,
50 browser-session-store tests, 18 Desktop Services tests, architecture detector
self-tests, formatting, and strict workspace Clippy. The reconciled coverage
ledger is nine pass, 28 partial, three fail, and five missing. G14 is partial for
provider-free eager/lazy scheduling, and G15 adds exact old-browser proof,
bounded retry/backoff, restart persistence, and one concurrent admission winner.
Actual host effect consumption remains open. Whole-product G04/P03/P05 closure
remains open outside the desktop cut.
M3-P1 can improve source evidence for G14/G15 but cannot accept installed or
joined recovery by itself.

Audit baseline: clean `00935477`, 55 commits ahead of the unchanged remote
`8bb9518e`; development binary digest `ac3c1daed8ab` matches its recorded full
identity. The prior audit coverage readback was 9/26/3/7; architecture reports four pass, four
detector gaps, eleven unverified. No production repair, runtime publication,
push, merge, or release is part of this documentation continuation.
Documentation validation: policy wiring, all three changed Markdown files'
local links, selector, and diff hygiene pass. Planning audit reports 287
findings on other plan files, none on P219; no repository-wide clean claim.

## Prior P219 status | 2026-09-27 M2B accepted

[Plan 0219](docs/dev/plans/0219-2026-09-26-alice-bob-grilling-contract-completion.md)
version 9 is `OPEN`: M2B is installed and accepted, while M3 through M5 remain
unstarted. Custody remains on `platform/p211-simple-cold-upgrade`, draft PR
#191, and work items #181/#183/#195. Production, merge, release, and worktree
removal remain excluded.

The selected isolated development generation is `0.28.0-ac3c1daed8ab`, built
from the working tree based on `1d940440eeb3e0aae93faba739ed6c89e0af5f96`.
Every development doctor check passes, helper version 11 is installed, four
warm provider displays are ready, and production remains
`0.28.0-b589b318c530-c0c0977896a8`.

M2B passed shared-profile Alice/Bob identity isolation, exact addressed-command
activity, failed-command non-refresh, authenticated viewer lifecycle and
control transfer, stale-controller rejection, guarded desktop input, Alice-first
cleanup, Bob-final process termination, and a separate disposable expiry case.
Two runtime-host replacement cycles preserved the same logical browser,
sessions, route slot, and opaque handoff IDs while launching only one replacement
browser each time. Both original public handoffs resolved to their fresh exact
targets with `operatorVisible.state=ready`, `uxState=connected`, and visible
Guacamole pixels. Pre-restart page markers were absent after recovery, and the
navigation ledger advanced only for four explicit reopens. Final cleanup left no
profile browser process.

The 45-row grilling ledger now totals nine pass, 26 partial, three fail, and
seven missing. G11, G24, G31, G35, G43, G44, and G45 are newly accepted; G36
is partial pending delayed disposable-profile deletion and quota cleanup.

M2B closeout validation is complete. Rust format, strict Clippy, every retained
green comprehensive compartment, the repaired 40-test actions compartment, the
118-test Lease Authority rerun, dashboard and viewer contracts, docs builds,
service parity and types, route-confusion gates, workstation/provider fixtures,
coverage validation, policy wiring, documentation links, diff hygiene, final
doctor, skill sync, and fresh development process census pass. M3 recovery and
retention is the next bounded milestone and is not started. The goal service still shows a stale blocked row
from the former helper gate; the operator explicitly resumed this successor
plan and set the active continuation ceiling to 2,000,000 tokens.

Nonblocking UI direction remains unchanged: adopt the Guacamole interaction
approach from `../remote-view` and replace the large persistent yellow banners
with compact dismissible notices.

## Prior P219 checkpoint | 2026-09-27

[Plan 0219](docs/dev/plans/0219-2026-09-26-alice-bob-grilling-contract-completion.md)
version 8 is `OPEN`. The operator expanded the active continuation ceiling to
2,000,000 tokens; production, staging, public ingress, merge, release, and worktree
removal remain excluded. Custody stays on
`platform/p211-simple-cold-upgrade`, draft PR #191, work items #181/#183/#195.
The current source checkpoint is `375cdfbe`; the named-retention repair is
`d7ec8d2f`. The branch is 52 commits ahead and zero behind its remote.

Current installed development identity is generation
`0.28.0-57c51d356866`, executable SHA-256
`57c51d3568663fe9f50bd14bab69bc183c71b37b819decd22d64ee62c8f35cf2`.
The optimized build, development installation, and the three-iteration
browser-launch smoke passed.
Production remained unchanged and the development skill is current.

The operator installed helper version 10, including exact primary channel
socket reclamation. Four subsequent applies failed closed and retained
production identity: `apply-1790478847005-74899.json` exposed the historical
display backlog; `apply-1790479449908-49709.json` exposed a quarantined `dev-2`
stop that had to resume before sweeping; `apply-1790479918735-97420.json`
exposed the short X-server teardown race; and
`apply-1790480067753-19562.json` advanced through displays 10 through 19 before
`.X20-lock` named a live numeric PID.

The `:20` PID is not the old route X server. Kernel readback proves PID 88087
is now a Chrome thread under UID 1000, while the exact lock inode is owned by
route UID 1004. Commits `662ab5d2`, `d4e75b70`, and `a11be934` add the bounded
sweep, retained-stop recovery, and exact PID teardown wait. Commit `375cdfbe`
adds helper version 11, which accepts numeric PID reuse only when the kernel
proves a different UID from the route user and rechecks that fact immediately
before exact inode deletion. Same-route UID, unreadable identity, a live XRDP
session, a live display socket, or an active channel socket still fails closed.

The v11 helper and provider regressions, focused Rust contracts, formatting,
strict workspace Clippy, source-free installer, host provisioning, fresh-VM,
Guacamole asset, PostgreSQL durability, and route-user synchronization fixtures
pass. The new user-scoped candidate is installed. The root-owned helper remains
version 10, so provider preflight now fails the explicit
`privileged-helper-foreign-pid-reuse` capability gate until one interactive
sudo installation. The provider is quarantined and stopped; all six keeper
records are absent and no development XRDP route process remains.

Authenticated access through an opaque `/remote-view/<handoff-id>` URL and real
synthetic browser pixels passed. The development admin credential was aligned
with the existing live credential at the operator's direction without recording
the secret; the live auth file stayed byte-identical. Automated pointer,
keyboard, and scroll effects were not established.

The named-session retention defect is repaired and source-qualified at
`d7ec8d2f`. The manager now applies `sessionIdleTimeoutMs` only to
manager-allocated disposable profiles. Exact named-profile sessions and their
opaque handoffs have no default idle expiry, while explicit close remains
terminal. Host restart normalizes and persists legacy finite named-session
deadlines. The installed generation contains this repair, but G12/G36 are not
accepted at the installed boundary because provider quarantine prevents the
same-handoff retention replay. G42 still requires installed exact-session
refresh. G45 is source-qualified as partial: the current authenticated
Guacamole primary is re-observed on connect and heartbeat, authority expires
after 15 seconds with an inclusive boundary, and heartbeat or disconnect does
not refresh the logical session. A single operator report that the handoff was
absent from SQLite was not reproduced: the exact row existed and authenticated
retry succeeded.

Acceptance remains partial:

| Requirement | Source or installed evidence | Current state | Missing proof |
| --- | --- | --- | --- |
| M1B authority closure | Source cuts and focused gates through `bdf695db`; candidate `0.28.0-57c51d356866` installed | complete for this checkpoint | Installed lifecycle acceptance remains in M2B. |
| M2A provider and viewer | Historical authenticated Guacamole, opaque handoff, and real pixels; four exact recovery receipts retained | partial | Install helper version 11, recover four routes, and revalidate pixels and input. |
| M2B Alice/Bob | Named profile and durable handoff exercised | incomplete | Correct retention, joined Alice/Bob identities and effects, input, cleanup, restart recovery, and fresh process census on one candidate. |
| G12/G36 retention | Source regressions prove retained named handoffs, bounded disposable expiry, and restart normalization at `d7ec8d2f` | partial | Installed retention and same-handoff replay on a frozen candidate. |
| G35/G45 viewer authority | Exact-primary observation, bounded viewer TTL, inclusive expiry, disconnect, current-control revalidation, and session/viewer separation pass at `bdf695db` | partial | Installed viewer lifecycle, control transfer, eager recovery, and session retention after provider recovery. |
| Goal control row | Thread `01a0dfee-e1ab-79e0-b34b-b9fa80a58d0c` is `active` | current | Its Plan 211 wording routes through active successor Plan 0219; the operator set a 2,000,000-token continuation cap. |

The bounded retention repair is complete. Its red regression and green focused
receipts, the complete 271-test Service Model lane, 32 host/navigation tests,
formatting, and strict workspace Clippy are recorded in Plan 0219. The
selector-required workstation and Guacamole fixtures, docs build,
remote-view documentation contract, architecture report, and coverage-ledger
validator also pass. Four unrelated `browser_session_authority` failures remain
in a broader name-filtered Rust run and do not invalidate the independently
green changed surfaces. The development skill is synchronized with the
candidate; the shared production skill and production runtime remain unchanged.

The live-viewer boundary repair is committed at `bdf695db`. A heartbeat at the
exact expiry instant is rejected as `live_viewer_lease_inactive`, and viewer
heartbeat and disconnect leave Browser Session Manager state unchanged. Four
focused live-viewer tests passed in receipt
`20260927T021202Z-a0456af8a0fa`; all 11 desktop-control tests passed in
`20260927T021442Z-dbae322f207f`; formatting passed in
`20260927T021442Z-42874926b5c2`; and strict workspace Clippy passed without
warnings in `20260927T021455Z-db18b3da9800`. P19 now passes its deterministic
architecture gate. The ledger totals two pass, 31 partial, three fail, and nine
missing.

The ignored same-profile browser fixture is stale diagnostic evidence. It
first failed on a missing disposable SQLite database in receipt
`20260927T020349Z-39f194809aba`; a temporary migration advanced it to the
correct `presentation_keeper_unavailable` boundary in
`20260927T020645Z-3f7e82c03caa`. The temporary fixture change was reverted. Do
not restore a JSON route fallback.

Next gate: install the version 11 privileged helper through interactive sudo,
verify `acceptsProvablyForeignPidReuse=true`, and rerun the exact development
provider plan, stage, preflight, and one deferred-ingress recovery apply. Do not
retry the rejected credential file or broaden cleanup. After four-route readiness,
replay installed named retention
and the remaining M2B Alice/Bob workflow. Do not start M3, M4, or M5.

Nonblocking future UI direction: adopt the Guacamole interaction approach from
the sibling `../remote-view` project and make warning banners compact and
dismissible. This does not expand the retention repair packet.

Progress classification: refreshed frozen development candidate, exact provider
blocker removal in source, and source-qualified live-viewer separation. M2A and
M2B remain incomplete; no G-row is promoted to installed pass by this
checkpoint.

## Active Plan Locator Index

- [P12](docs/dev/plans/0012-2026-05-31-workspace-inspection-pane-app-intelligence-roadmap.md), [P78](docs/dev/plans/0078-2026-07-27-guacamole-route-fixture-recovery-interlock-plan.md), [P111](docs/dev/plans/0111-2026-08-13-multi-agent-shared-browser-profile-authority-plan.md), [P116](docs/dev/plans/0116-2026-08-15-runtime-adoption-and-transactional-upgrade-plan.md), [P144](docs/dev/plans/0144-2026-08-31-lease-authority-coordination-and-revocation-plan.md), and [P158](docs/dev/plans/0158-2026-09-02-frozen-candidate-historical-failure-stress-campaign.md)
- [P157 production identity](docs/dev/plans/0160-2026-09-06-production-profile-identity-and-operational-readiness.md), [P157 desktop slots](docs/dev/plans/0162-2026-09-10-production-desktop-slot-and-jit-viewer-allocation.md), [P157 profile reset](docs/dev/plans/0163-2026-09-10-profile-data-reset-backup-and-restore.md), [P165](docs/dev/plans/0165-2026-09-11-bill-identifier-form-drift-repair.md), [P169 Turnstile leaf](docs/dev/plans/0169-2026-09-11-cloudflare-turnstile-desktop-challenge-plan.md), and [P178](docs/dev/plans/0178-2026-09-13-browserless-runtime-lane-quiescence.md)
- [P182](docs/dev/plans/0182-2026-09-13-authentication-resume-state-reconciliation.md), [P187](docs/dev/plans/0187-2026-09-14-challenge-countermeasure-control-plane-blueprint.md), [P169 hCaptcha leaf](docs/dev/plans/0189-2026-09-13-hcaptcha-fixture-checkbox-acceptance.md), [P190](docs/dev/plans/0190-2026-09-14-advisory-candidate-build-and-promotion-orchestrator.md), [P197](docs/dev/plans/0197-2026-09-16-challenge-consumer-integration.md), and [P204](docs/dev/plans/0204-2026-09-16-ci-validation-economics-and-tiering.md)
- [P219](docs/dev/plans/0219-2026-09-26-alice-bob-grilling-contract-completion.md) and [P214](docs/dev/plans/0214-2026-09-17-candidate-permit-event-plan.md)

## History

[The P218 archive](RUNBOOK-history-2026-09-26-through-p218.md) preserves the
previous runbook, Turns 418 through 450, P218 source checkpoints, failures,
prior stop instructions, and links to earlier archives. Plan 0219 version 4
preserves the superseded September 26 M2A checkpoint and its receipt.
