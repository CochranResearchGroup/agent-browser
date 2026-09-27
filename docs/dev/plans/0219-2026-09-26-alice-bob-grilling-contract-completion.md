# Plan 0219 | Alice/Bob Grilling Contract Completion

Date: 2026-09-26

Plan version: 5

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

Renewed effort ceiling: 1,000,000 additional tokens from the operator's 2026-09-26 resume direction; reserve 200,000 tokens for final reconciliation, validation, evidence, and closeout; the prior 545,502-token P219 window and every predecessor attempt, review, failure, and receipt carry forward

Policy capsule: [P219 M1B through M2B](../policy-capsules/p219-m1b-m2b.md); use this verified capsule instead of broad policy rereads until one of its explicit triggers fires

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

Version 5 records the source-qualified named-profile retention repair at
`d7ec8d2f`. The service model now applies the configured inactivity deadline
only to manager-allocated disposable profiles. Exact named-profile sessions
and their opaque handoffs use no default idle expiry, and the SQLite host
normalizes and persists legacy finite named-session deadlines on restart.
Explicit close and typed unrecoverable termination remain unchanged.

The installed September 26 result still runs development generation
`0.28.0-2423cfb064f2` and binds executable SHA-256
`2423cfb064f2f3ce4a110f197b5202b64535cfc6c674aabf6484027047fbd2e2`.
The exact stale-display and XRDP-channel reclamation repairs are committed at
`5cf1c10f` and `67a17375`; privileged helper contract version 9 is installed.
The focused helper, installer, Rust, route, formatting, and strict workspace
Clippy checks recorded for that batch pass. Production was not changed.

The repairs recovered three development RDP routes on displays `:58`, `:59`,
and `:60`. The configured fourth warm route remains unavailable because older
display artifacts outside the exact repair custody still occupy the allocation
range. Current read-only status therefore reports three ready displays and a
blocking presentation-provider readiness result for the four-route target.
This is sufficient to retain the minimum two-party evidence already observed,
but it is not full provider readiness.

Authenticated operator access and real synthetic browser pixels passed through
an opaque `/remote-view/<handoff-id>` URL. At the operator's direction, the
development admin credential was aligned with the existing live credential
without recording the secret; the live auth file remained byte-identical.
Automated pointer, keyboard, and scroll effects were not established, so input
acceptance remains incomplete.

Named-profile sessions 94 through 97 in the installed generation ended with
`heartbeat_expired` about five minutes after their last activity. Source commit
`d7ec8d2f` repairs that defect without changing the installed generation. G12
and G36 therefore have source and provider-free regression evidence but remain
unaccepted at the installed boundary. G42 and G45 still require installed
exact-session activity refresh and separation of session heartbeat from viewer
heartbeat. A temporary keepalive was removed because it masked the defect. One
operator report of
"Remote-view handoff was not found in the SQLite session authority" occurred
while the exact handoff row still existed and an authenticated retry resolved
successfully; retain it as diagnostic evidence, not a reproduced second root
cause.

M2A has meaningful partial evidence for provider recovery, authenticated access,
and pixels, but remains incomplete because the configured provider target is
not ready. M2B remains incomplete because input, Alice/Bob joined lifecycle,
installed restart recovery, cleanup, and named-session retention have not all
passed on one frozen candidate. Do not start M3, M4, or M5.

At succession, local HEAD and the directly queried remote branch both resolve
to the source baseline. P218 is incomplete; no installed P218 candidate has
accepted pixels, input, joined recovery, or complete grilling conformance.
The [inherited coverage ledger](../contracts/p218-grilling-contract-coverage.v1.json)
has 45 rows: two pass, 29 partial, five fail, and nine missing. Retain its
filename and row IDs as the single evidence ledger. Succession promotes no row.

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

The current architecture report has P09, P15 and P16 passing, five detector
gaps (P02/P03/P05/P12/P19), and eleven unverified prohibitions. B06 migrated
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
partial M1B. The operator renewed execution with 1,000,000 additional tokens.
This is a new bounded allowance, not erasure of the earlier usage. Reserve
200,000 tokens for final reconciliation, validation, evidence and custody.
At 700,000 renewed tokens, require direct evidence that the source is candidate
capable and that remaining environment work can reach M2B within the 100,000
implementation tokens still available before the reserve. If not, stop
implementation and preserve the exact blocker. In all cases, implementation
stops at 800,000 renewed tokens and the final 200,000 are closeout-only.

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

Use the verified [P219 capsule](../policy-capsules/p219-m1b-m2b.md) as the
policy entrypoint. Its hashes match the adopted policy sources at renewal. Do
not reread the broad policy set unless the capsule's scope, effect class,
branch, worktree, owner, validation contract or policy hash changes, or a
failure exposes an ambiguity the capsule does not resolve.

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
baseline/active-viewer recovery and lazy dormant recovery, committed top-level
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

The M2B milestone is an intermediate outcome. P218's cancellation is
supersession, not acceptance. A partial pass keeps this plan open after
execution starts. M1B source closure is complete; the current action is the
minimum M2A provider/live-viewer join followed by a frozen M2B candidate.

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
The repository skill intentionally differs from the shared installed skill
until a separately authorized development candidate publication.

Next gate: decide whether to freeze and publish a new development candidate for
installed retention and M2B replay. This source packet does not authorize that
runtime effect and promotes no G-row to installed pass.

Goal-service note: thread `01a0dfee-e1ab-79e0-b34b-b9fa80a58d0c` is `active`
with objective `contimue plan 211 with an additional 1 mm token cap.` The
objective names the cancelled predecessor, so execution follows its unmet
outcome through active successor Plan 0219 rather than reopening Plan 0211.
The latest control readback at this checkpoint reported `tokensUsed=218445`
and `timeUsedSeconds=1163`; those usage counters continue to advance.

Deferred, nonblocking UI direction: use the sibling `../remote-view` project as
the Guacamole interaction reference, and make warning banners compact and
dismissible so they do not permanently consume viewer height. This direction
does not expand the retention repair packet.
