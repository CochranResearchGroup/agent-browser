# Plan 0218 | Grilling-Contract Remote View Conformance

Date: 2026-09-23

Plan version: 2

State: OPEN

Consolidation: required

Product lane: PL-PLATFORM

Lane: P218

Predecessor: [Plan 0217](0217-2026-09-22-availability-first-remote-view-reliability.md), superseded because it covered only availability and visual reliability rather than the complete accepted grilling contract

Original design authority: Codex thread `01a0b65d-47f9-7b51-a17b-791ee87769b3`, grilling exchange on 2026-09-19

Audit basis: accepted user and assistant design turns 207 through 349 in the
original design authority, reconciled against branch head `c627fd4b` on
2026-09-23

Work items: `CochranResearchGroup/agent-browser#181`, `CochranResearchGroup/agent-browser#183`, and `CochranResearchGroup/agent-browser#195`

Branch: `platform/p211-simple-cold-upgrade` with inherited P211 and P217 custody

Pull request: draft PR #191

Target: `main`

Overall effort ceiling: inherit Plan 0217's cumulative 2,000,000-token ceiling without reset; reconcile actual prior P217 usage before execution and do not begin a packet that cannot finish inside the remaining allowance

## Objective

Make Agent Browser implement the complete architecture accepted during the
September 19 grilling session. In the trusted single-user runtime, a valid
instruction to open or recover a browser must produce a usable browser and
authenticated remote view unless a concrete current resource, configuration,
authentication, or execution failure makes that impossible.

Historical uncertainty is diagnostic evidence, not permission authority.
Lease claims, fencing, custody, quarantine, cleanup obligations, retained owner
records, stale inventory, and equivalent renamed concepts must not deny an
ordinary open, reserve phantom capacity, substitute an unauthenticated profile,
or prevent creation of a separate working route. Exact uncertainty may prevent
destructive cleanup of the exact uncertain resource.

The runtime must use one provider-owned presentation authority, one
transactional SQLite authority, one durable opaque handoff contract, and one
Desktop Services control authority. It must reconstruct usable presentation
capacity after a cold start without hidden infrastructure browsers, mutable
route inventories, operator-managed display numbers, or runtime fallback to
legacy JSON.

## Current State

P211 implemented substantial SQLite, operation-journal, route-keeper,
capacity, handoff, desktop-control, and migration foundations. P217 commit
`9bdcfbe3` removed `remote_view_open` from generic profile-lease classification,
resolved an explicit ordinary profile before legacy owner evidence, and added
an architecture guard against restoring that coupling.

The complete grilling contract is not implemented or accepted. Browser and
handoff paths still consume JSON Service State, runtime route and provider
inventory environment variables remain live, the development provider keeps
the route-keeper readiness interlock hard-coded false, history budgets are
configuration without enforced compaction, verified database backup and
restore are absent, and joined cold-start plus external visual recovery
acceptance has not passed. Explicit administrative and adversarial Lease
Authority contracts remain supported, but they are not ordinary runtime
authority.

The 2026-09-23 source audit found that the branch already contains useful
provider-free implementations for SQLite defaults, forward-only migration,
operation and host-generation fencing, route-keeper supervision, capacity
growth, least-loaded placement, durable admission queueing, cooldown scale-in,
navigation and handoff recovery, and atomic Desktop Services control
publication. Those are implementation foundations, not whole-contract
acceptance.

The same audit found four classes of remaining conformance work:

1. ordinary browser, handoff, lifecycle, monitor, and desktop paths still read
   JSON Service State, so the SQLite authority boundary is incomplete;
2. the development provider still represents route-keeper runtime readiness as
   false, and provider-backed joined startup has not qualified the source
   foundations;
3. history compaction, integrity checking, verified online backup and restore,
   provider-credential access, and full disposable-profile quota mutation are
   schema or configuration foundations without the accepted operational
   behavior;
4. no one frozen installed candidate has passed the joined cold-upgrade,
   cold-start, failure-injection, external visual, responsive-input, and fresh
   process-residue matrix.

CodeGraph was not initialized in this worktree during the audit. The findings
above therefore come from the exact branch diff, known implementation seams,
tests, and literal contract searches. M0 must produce the authoritative
machine-readable dependency closure before behavior work begins; this audit is
not a substitute for that gate.

## Grilling Contract Ledger

Every row is normative. Later implementation notes, safety language, renamed
types, or partial tests cannot weaken a row without explicit operator revision
of this plan.

| ID | Accepted decision | Required invariant |
| --- | --- | --- |
| G01 | Existing runtime host owns presentation authority | Browser Session Manager calls one typed in-process provider interface. No second daemon or provider database becomes authority. |
| G02 | No operator-maintained route inventory | The installer stores policy only. The provider deterministically owns route slots, users, credentials, Guacamole connections, and live display observations. |
| G03 | Legacy policy and runtime JSON are migration-only | One idempotent forward-only cold-upgrade migration imports valid user configuration, profile, session, handoff, and history state, records hashes and typed rejects, archives sources read-only, removes old runtime and unit authority plus exact Agent Browser-owned old processes, removes legacy variables, and never dual-writes, restores the old binary, or falls back. Provider route rows, route IDs, display numbers, leases, and connection rows are rebuilt rather than imported as authority. |
| G04 | One user-private SQLite authority | Ordinary browser, profile, session, tab, handoff, operation, presentation intent, configuration, history, cleanup-obligation, and provider-credential state have one transactional authority. JSON is limited to exports, diagnostic receipts, and migration archives outside the ordinary runtime dependency closure. |
| G05 | Stale history cannot veto work | Malformed, contradictory, ambiguous, or stale history is preserved diagnostically but cannot prevent a fresh valid browser request. |
| G06 | Presentation warms before browser launch | Startup reconstructs provider capacity. `minimumReady=1` unlocks service and `warmTarget=4` continues in the background. Readiness is false until one complete Guacamole, XRDP, and display path is usable. |
| G07 | Ordinary open waits for real capacity | A request waits up to the configured deadline, provisionally 90 seconds. Timeout or real exhaustion returns a typed failure before Chrome launches. |
| G08 | No hidden infrastructure browser | In-process protocol-level route keepers traverse Guacamole and retain XRDP sessions without Chrome, profiles, tabs, handoffs, or Browser Session Manager records. |
| G09 | Displays and browsers have deterministic placement | Prefer one browser per display while capacity can grow. At the provisional default maximum of six displays, overflow browsers may share the least-loaded healthy display up to the provisional density of four. Both limits are live configuration and remain subject to G33. |
| G10 | Capacity has bounded queuing | After display and density limits, a configurable bounded queue, provisionally 32, prioritizes active recovery, durable handoff access, then aged FIFO new opens. Duplicate browser/profile requests coalesce. |
| G11 | Desktop Services owns control | One fenced per-display Desktop Services lease governs handoff activation, focus, maximize, capture, pointer, keyboard, and future desktop automation. Opening another browser transfers control visibly; prior viewers become view-only. |
| G12 | Handoff identity is durable | One opaque authenticated `/remote-view/<handoff-id>` survives runtime, provider, Guacamole, display, browser, and tab recovery for the same logical session. It ends only by explicit close, configured retention, or a typed unrecoverable condition. |
| G13 | Handoff resolution performs bounded recovery | Resolution restores capacity, chooses a display, reuses or launches exactly one browser with the same profile, recreates the tab at its last committed URL, focuses and maximizes it, and returns the viewer. It never replays clicks, submissions, downloads, forms, or other page effects. |
| G14 | Recovery is eager only when useful | Baseline presentation capacity is eager. Active viewers recover eagerly. Dormant browsers recover lazily when their handoff or named session is accessed. |
| G15 | Recovery is bounded and singular | Active recovery retains the same handoff and retries with bounded backoff for up to the configured deadline. It proves an old browser unusable before launching at most one replacement and never creates competing replacements. |
| G16 | History is useful and bounded | Store compact structured lifecycle, navigation, display-binding, browser-replacement, recovery, and compaction events. Do not store raw logs, screenshots, bodies, heartbeats, or repeated polls. |
| G17 | Exact and summarized history coexist | Retain material lifecycle and recovery events indefinitely. Keep exact URL transitions within 64 MiB, then summarize oldest navigation by day with first URL, last URL, count, identities, incidents, and an auditable compaction event. |
| G18 | SQLite is recoverable | Use durable WAL transactions, integrity checks, one rotating verified online backup, a 96 MiB live database target, and a 128 MiB routine database, WAL, and backup budget. Preserve corrupt databases and record restoration gaps. |
| G19 | Operations are crash-consistent | Session admission, browser identity, presentation reservation, and handoff identity share one durable operation. Waiting operations do not surprise-launch after restart; begun effects reconcile or compensate; idempotent clients resume by operation ID. |
| G20 | Privilege is installed once and used only as needed | Initial install may establish one narrowly scoped noninteractive helper. Healthy cold starts make zero privileged calls. Each exact repair records resource, prior observation, action, and postcondition. Shared XRDP is never broadly restarted. |
| G21 | Disposable retention is bounded | Defaults are 24 inactive hours, 20 retained profiles, and 10 GiB, all live user settings. Oldest inactive disposable sessions expire first. Active-viewer, controlled, pending-operation, and named-profile sessions are not evicted. A disposable session has no separate pin state. |
| G22 | Configuration is typed and inspectable | Typed Service configuration and `agent-browser config get/set` own every accepted capacity, deadline, cooldown, retention, quota, history, and database-budget setting. Dashboard configuration remains read-only in this plan. Mutable environment policy is not runtime authority; environment variables are limited to immutable process bootstrap such as runtime identity and database location. |
| G23 | Valid user instructions are availability-first | Ordinary named-profile open cannot consult or be denied by legacy lease admission, historical owner proof, quarantine, cleanup obligations, or equivalent denial-first metadata. Only current concrete failures may deny it. |
| G24 | Development acceptance precedes production | Implement and validate in the isolated development runtime. Production, external ingress publication, formal release, and merge require separate authority after accepted development evidence. |
| G25 | Provider credentials follow the SQLite authority | Generated provider credentials are stored only in the user-private SQLite authority and are projected transiently into the user runtime directory or container secret mount. Transient secret files are removed on shutdown; rebuilt route-user credentials may rotate without changing handoff identity. |
| G26 | Runtime generations fence every effect | Each runtime-host start claims a monotonically increasing generation. Provider effects, browser effects, callbacks, operation completion, and recovery commits require the current generation plus operation ID; stale generations cannot commit. |
| G27 | Browser placement is stable | Prefer unused healthy displays before sharing the least-loaded healthy display. A browser keeps its display binding during ordinary operation and is not routinely migrated for load balancing; only bounded failure recovery may rebind it. |
| G28 | Extra capacity scales in safely | Only an empty, exact-reference-free display above `warmTarget` may retire after the configurable cooldown, provisionally 10 minutes. A final browser, handoff, viewer, operation, and keeper reference check precedes the effect. |
| G29 | Limit changes converge without eviction | Valid configuration updates commit transactionally, enter history, and take effect without reinstall or restart. Lowering a limit below current usage reports a visible over-target state, admits no worsening work, and converges through ordinary release without killing browsers or viewers. |
| G30 | Status and doctor expose reconciliation | Report configured versus observed capacity, keeper health, display generations, browser counts per display, pending operations, queue depth, Desktop Services control ownership, handoff recovery, migration receipt, database integrity and size, resource pressure, and privileged repair receipts. Credentials and raw provider URLs remain redacted. |
| G31 | Repeated opens are idempotent | Repeated or concurrent `remote_view_open` for the same logical session and tab returns and activates one existing handoff and coalesces under one operation ID. A genuinely new logical tab gets a distinct handoff; explicit session closure makes prior handoffs terminal. |
| G32 | Recovery URL means committed top-level navigation | Persist a URL only after Chrome reports committed top-level navigation, including operator-driven navigation. Ignore provisional redirects, subframes, and bootstrap pages; retain bounded redirect history and recover the final committed top-level URL. |
| G33 | Launch obeys live resource pressure | Even below display and density limits, each browser launch passes current memory, process, and disk admission. Pressure rejection is typed and does not disturb existing browsers, displays, or viewers. Development stress evidence must validate or lower the provisional density of four. |
| G34 | Provider route state is derived | Agent Browser-owned Guacamole database rows, users, connections, sharing profiles, route IDs, display numbers, and leases are disposable derived infrastructure. Cold upgrade rebuilds the owned namespace from SQLite policy and retained operator inputs rather than migrating provider rows. |
| G35 | Live viewer authority is observational | An authenticated remote-view client connection and its live Guacamole tunnel, with bounded heartbeat and disconnect detection, are the only authority that a viewer is active. Stored URLs, tabs, database flags, and previously opened pages are not active-viewer proof. |
| G36 | Named and disposable handoffs differ | Named-profile handoffs have no default time expiry. Disposable-profile handoffs expire after the configured inactivity period, provisionally 24 hours; expiry closes the logical session, makes its handoffs terminal, reference-checks and deletes its disposable profile, and retains compact history. Browser reclamation alone preserves either kind of recoverable handoff and profile. |
| G37 | Disposable quotas fail cleanly | Count and byte limits are enforced with oldest-inactive-first cleanup. If protected sessions prevent sufficient cleanup, reject the new disposable request with a typed quota or low-disk result. Never delete named profiles or protected disposable sessions to admit new work. |
| G38 | Disposable promotion is deferred | There is no disposable-session pinning or profile-promotion implementation in this plan. Until a future explicit promotion feature exists, every disposable profile remains subject to G36 and G37. |
| G39 | Doctor is read-only | `doctor` observes and recommends but never repairs. Startup and requests may invoke the same narrowly typed automatic reconciler, and an explicit `repair` command may request it, but neither path gains arbitrary commands or broad restart authority. |
| G40 | Waiting work does not surprise-launch | Queued operation identity and outcome persist for idempotency, but work that was only waiting becomes retryable after restart and launches nothing until the client resumes it. Effects already begun reconcile or compensate under G19 and G26. |

## Architectural Prohibitions

The following are compile-time or deterministic source-contract failures:

1. `remote_view_open` depends on legacy profile-lease admission, Lease Authority,
   historical runtime-owner proof, quarantine, or cleanup-obligation modules.
2. Browser Session Manager reads route inventory, display numbers, provider
   inventory, or policy from environment variables or generated JSON files.
3. Ordinary runtime code falls back to legacy JSON after the SQLite authority
   exists, including when SQLite is corrupt.
4. A hidden Chrome process is used to establish or retain XRDP or Guacamole
   presentation capacity.
5. A second provider database, daemon, scheduler, controller, or desktop-input
   lease competes with the runtime host, SQLite authority, presentation
   provider, or Desktop Services.
6. Status reports ready before `minimumReady` complete provider paths are live.
7. Browser launch can precede capacity reservation and readiness proof.
8. Safety, custody, reconciliation, or ownership terminology is renamed while
   preserving an ordinary-path veto.
9. Provider credentials become authoritative in environment variables,
   editable JSON, generated units, or persistent secret files outside SQLite.
10. `doctor` performs mutation, or any repair path can execute arbitrary
    privileged commands or broadly restart shared XRDP.
11. Routine balancing moves a healthy browser between displays, lowering a
    configured limit kills existing work, or scale-in skips the final exact
    reference check.
12. A stored page, tab, URL, or database flag substitutes for a live
    authenticated viewer connection.
13. A merely queued pre-restart request launches a browser without client
    resumption.
14. A disposable handoff survives its configured expiry or quota cleanup by
    acquiring an undeclared pin or promotion state.

Keep the extracted Lease Authority crate and explicit administrative,
adversarial, and future multi-tenant contracts buildable in their own scope.
They must not be linked into the trusted single-user ordinary-open dependency
closure. Quarantined historical implementations stay outside the compiled
ordinary runtime and have an owner, reason, and deletion or archival decision.

## Consolidated Batch

- Freeze the executable ordinary-open, provider, persistence, handoff,
  recovery, and desktop-control dependency closures and install architecture
  guards for G01 through G40 before further behavior patches.
- Remove runtime JSON and environment authority through one forward-only
  migration, then join Browser Session Manager and presentation provider to one
  SQLite-backed runtime-host interface.
- Complete route-keeper startup, readiness, capacity, queueing, browser launch,
  durable handoff recovery, Desktop Services control, history compaction, and
  database recovery as one coherent runtime outcome.
- Qualify one source candidate provider-free, publish it once to the isolated
  development runtime, and run the complete cold-start, failure-injection, and
  external visual-operational acceptance matrix.
- Reconcile PR #191, roadmap, runbook, active lane, user documentation, and
  generated contracts only from the frozen accepted candidate.

## Scope And Non-Goals

Included:

- every G01 through G40 invariant;
- removal or compile-time quarantine of conflicting ordinary-path code;
- typed runtime, provider, persistence, configuration, handoff, capacity,
  recovery, desktop-control, history, and migration interfaces;
- provider-free fixtures, isolated development effects, and synthetic external
  desktop and mobile evidence;
- required help, README, skill, docs-site, inline comments, generated clients,
  plan, roadmap, runbook, and active-lane parity.

Excluded:

- production or staging mutation, public ingress publication, formal release,
  merge, or upstream contribution;
- new multi-tenant admission policy or adversarial user model;
- replay of page interactions beyond the last committed top-level URL;
- dashboard configuration editing;
- durable-profile promotion for disposable profiles;
- private-site content or credentials in acceptance artifacts.

## Delivery Sequence And Budget

### M0 | Closed-world conformance map and red architecture gates | 200,000 tokens

Reconcile inherited P217 usage, branch and installed identities, then map every
G01 through G40 row to current symbols, state stores, tests, and evidence. Add
failing architecture checks for every currently violated prohibition and a
machine-readable G01 through G40 coverage manifest. Freeze one dependency graph
for ordinary open through handoff recovery. Exit only when omissions and
violations are mechanically visible; do not repair behavior in this milestone.

### M1 | Authority and persistence cutover | 450,000 tokens

Remove legacy denial and JSON or environment authority from the compiled
ordinary dependency closure. Complete one SQLite-backed runtime-host interface,
one forward-only migration, typed configuration, operation journaling,
provider-credential custody, integrity and backup handling, restore-gap
reporting, history budgets, and compaction. Expose every accepted mutable
setting, not only presentation capacity fields. Preserve exact cleanup
protection outside admission. Exit when architectural guards and provider-free
authority, migration, corruption, backup, restore, configuration, quota, and
compaction fixtures pass.

### M2 | Provider, capacity, and launch integration | 400,000 tokens

Remove the development route-keeper interlock and join the in-process
Guacamole keeper to readiness, minimum and warm capacity, bounded queueing,
placement, stable bindings, density, live resource-pressure admission,
over-target convergence, safe scale-in, privilege receipts, and browser launch.
Prove no Chrome launch on capacity timeout and no hidden infrastructure browser. Exit when
three isolated zero-process cold starts each produce one ready route within the
measured provisional deadline and the injected timeout launches no Chrome.

### M3 | Durable handoff, recovery, and desktop control | 350,000 tokens

Complete stable handoff resolution, last-committed-URL recovery, eager active
and lazy dormant recovery, single replacement fencing, and the shared Desktop
Services control lease. Add repeated-open coalescing, authenticated live-viewer
authority, named versus disposable expiry, quota cleanup, and restart-safe
waiting-operation behavior. Prove runtime, provider, Guacamole, route, display,
browser, tab, viewer-disconnect, quota, and restart failure cases preserve the
required identity and never replay page effects. Exit with provider-free
failure injection plus one installed joined recovery pass.

### M4 | Frozen visual-operational acceptance | 400,000 tokens

Freeze source, binary, installed generation, SQLite schema, provider manifest,
route IDs, displays, configuration, and acceptance denominator. Test every
frozen route from authenticated external desktop and mobile viewports using
synthetic content. Require current complete pixels, clean desktop, correct
browser and z-order, focus, pointer, keyboard, scroll, resize, control transfer,
reconnect, one real forward-only cold upgrade, and every M3 restart case.
Measure the provisional 90-second readiness deadline and density of four under
development pressure. Preserve first failures. Permit one consolidated repair
and one bounded repeat of affected cases.

### M5 | Qualification and integration handoff | 200,000 tokens

Run validation selected from the complete P218 diff, verify every G01 through
G40 row and prohibition against the frozen candidate, update all public and governing
documentation, and perform one closed-world review limited to the contract and
repair regressions. Publish a remote checkpoint and update draft PR #191. Do
not merge, install production, release, or remove the worktree.

## Worker Assignments

The primary agent owns the critical path, contract ledger, branch, candidate,
runtime custody, evidence adjudication, and verdict. No worker is required.
If delegation becomes useful, admit at most one provider-free source worker
and one read-only acceptance-evidence reviewer with disjoint files and explicit
stop conditions. Workers cannot revise G01 through G40, mutate shared runtime,
or declare acceptance.

## Controls And Stop Rules

- Start execution only after current cumulative P217 usage is reconciled
  against the inherited ceiling.
- One implementation attempt and one consolidated repair are allowed per
  milestone. A filename, exception, renamed authority, or passing narrow test
  does not reset an attempt.
- A second related defect triggers batch reconciliation before another build or
  installed candidate.
- No runtime candidate is built until all known source and architecture
  violations for its milestone are repaired and cheaper checks pass.
- Use one development candidate publication for the completed implementation
  batch. Another publication requires a demonstrated source defect and enough
  remaining allowance for full acceptance.
- Do not weaken an architectural prohibition to make existing code pass.
- Do not use protocol status, process presence, stored URLs, doctor output, or
  unit tests as substitutes for visible pixels and responsive input.
- Do not treat preservation of an uncertain exact resource as permission to
  reserve capacity or refuse a separate working route.
- Stop production, staging, ingress, release, merge, destructive profile
  cleanup, and private-site effects unless separately authorized.

## Evidence And Exit

Maintain one evidence table keyed by G01 through G40. Each row records source
commit, test or artifact, installed generation when applicable, result,
failure preservation, and reviewer disposition. Evidence from different
candidates cannot be combined for final acceptance.

P218 completes only when:

1. every G-row has current evidence from one frozen candidate;
2. every architectural prohibition is enforced by a deterministic gate;
3. the ordinary-open dependency closure contains no denial-first authority;
4. three cold starts and the injected no-Chrome timeout pass;
5. every frozen route passes desktop and mobile pixels, input, reconnect, and
   recovery;
6. fresh OS process and resource readback shows no unexplained browser,
   daemon, keeper, XRDP, or Guacamole residue;
7. exact uncertain resources remain untouched and diagnostic without blocking
   service;
8. changed-surface validation and closed-world review pass;
9. local and remote source identity, plan, roadmap, runbook, lane catalog, and
   draft PR agree.

A partial pass remains OPEN with the exact failed G-row. Completion does not
authorize merge, production installation, release, ingress publication,
profile deletion, branch deletion, or worktree removal.
