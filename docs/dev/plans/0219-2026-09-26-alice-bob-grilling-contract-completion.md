# Plan 0219 | Alice/Bob Grilling Contract Completion

Date: 2026-09-26

Plan version: 22

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

Version 15 source-qualifies M3-P1. A strict provider-free decision contract now
distinguishes eager baseline, authenticated-active-viewer, and exact-client
resume demand from lazy dormant demand; unknown old-browser usability refuses
admission. Browser Runtime SQLite serializes the retained logical browser's
generation, attempt, deadline, and capped retry time in one immediate
transaction. Four independent connections produce one admission winner. The
complete 276-test service-model suite and 37 browser-session-store tests pass.
The ledger is now nine pass, 28 partial, three fail, and five missing. G14 moves
to partial and G15 gains bounded-retry and concurrency evidence; host effect
consumption and joined installed recovery remain open.

Version 14 starts M3 with one bounded provider-free packet, M3-P1. The packet
owns only recovery admission and scheduling semantics for G14/G15: eager
baseline or authenticated-active-viewer demand, lazy dormant demand until an
exact client resumes, exact old-browser unusability proof, one SQLite-serialized
replacement winner, and typed bounded retry/backoff with a terminal deadline.
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
   eager baseline/active-viewer versus lazy dormant scheduling, old-browser
   unusability proof, singular concurrent replacement, and typed bounded
   retry/backoff. Preserve G12/G13 handoffs, logical identities, committed URLs,
   terminal closed/expired sessions, and no page-effect replay. The closure
   owner must resolve concrete SQLite dependencies before accepting affected
   recovery behavior; unrelated cleanup and UI work stay outside this packet.

### M3-P1 Recovery Admission And Scheduling

M3-P1 is started under version 14. Reuse the existing recovery budget and
backoff defaults rather than creating a competing policy. Add one strict pure
decision contract and one Browser Runtime SQLite transaction boundary. An
eligible replacement requires an exact observation that the retained browser
is unusable. Baseline-capacity deficit and an authenticated active viewer are
eager demand. A dormant retained browser waits without admission until an exact
handoff or named-session client resumes. Unknown liveness fails closed.

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
contract passes eager/lazy demand, unknown-liveness refusal, capped exponential
backoff, deadline exhaustion, and strict wire tests. The SQLite adapter passes
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
derive demand from baseline deficit, authenticated active viewer, dormant
state, or exact handoff/named-session resume. Bind old-browser
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

Do not bump the database `PRAGMA user_version` merely for JSON-row evolution
unless table shape changes. The runtime-config value carries its own schema and
must migrate inside one immediate transaction. Do not silently treat missing
new fields as a current-schema row: a legacy schema must be recognized,
upgraded once, revisioned, and written before ordinary consumption. Do not let
startup defaults overwrite an operator-mutated row on every daemon restart.

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
