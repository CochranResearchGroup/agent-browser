# Plan 0219 | Alice/Bob Grilling Contract Completion

Date: 2026-09-26

Plan version: 9

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

Renewed effort ceiling: 2,000,000 tokens for the active continuation after the operator's 1,500,000-token renewal and later 500,000-token addition; reserve 200,000 tokens for final reconciliation, validation, evidence, and closeout; the prior 545,502-token P219 window and every predecessor attempt, review, failure, and receipt carry forward

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

Version 9 records M2B as installed and accepted on isolated development
generation `0.28.0-ac3c1daed8ab`, built from the working tree based on
`1d940440eeb3e0aae93faba739ed6c89e0af5f96`. The root-owned privileged helper
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
