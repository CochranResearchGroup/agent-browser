# Plan 0218 | Grilling-Contract Remote View Conformance

Date: 2026-09-23

Plan version: 1

State: OPEN

Consolidation: required

Product lane: PL-PLATFORM

Lane: P218

Predecessor: [Plan 0217](0217-2026-09-22-availability-first-remote-view-reliability.md), superseded because it covered only availability and visual reliability rather than the complete accepted grilling contract

Original design authority: Codex thread `01a0b65d-47f9-7b51-a17b-791ee87769b3`, grilling exchange on 2026-09-19

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

## Grilling Contract Ledger

Every row is normative. Later implementation notes, safety language, renamed
types, or partial tests cannot weaken a row without explicit operator revision
of this plan.

| ID | Accepted decision | Required invariant |
| --- | --- | --- |
| G01 | Existing runtime host owns presentation authority | Browser Session Manager calls one typed in-process provider interface. No second daemon or provider database becomes authority. |
| G02 | No operator-maintained route inventory | The installer stores policy only. The provider deterministically owns route slots, users, credentials, Guacamole connections, and live display observations. |
| G03 | Legacy route JSON is migration-only | One idempotent installer migration imports valid state, records hashes and typed rejects, archives sources read-only, removes legacy variables, and never dual-writes or falls back. |
| G04 | One user-private SQLite authority | Browser, profile, session, tab, handoff, operation, presentation intent, configuration, history, and cleanup obligations have one transactional authority. JSON is limited to exports and diagnostic receipts. |
| G05 | Stale history cannot veto work | Malformed, contradictory, ambiguous, or stale history is preserved diagnostically but cannot prevent a fresh valid browser request. |
| G06 | Presentation warms before browser launch | Startup reconstructs provider capacity. `minimumReady=1` unlocks service and `warmTarget=4` continues in the background. Readiness is false until one complete Guacamole, XRDP, and display path is usable. |
| G07 | Ordinary open waits for real capacity | A request waits up to the configured deadline, provisionally 90 seconds. Timeout or real exhaustion returns a typed failure before Chrome launches. |
| G08 | No hidden infrastructure browser | In-process protocol-level route keepers traverse Guacamole and retain XRDP sessions without Chrome, profiles, tabs, handoffs, or Browser Session Manager records. |
| G09 | Displays and browsers have deterministic placement | Prefer one browser per display while capacity can grow. At maximum displays, overflow browsers may share the least-loaded healthy display up to configured density. |
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
| G21 | Disposable retention is bounded | Defaults are 24 inactive hours, 20 retained profiles, and 10 GiB, all live user settings. Oldest inactive disposable sessions expire first; active, controlled, pending, named, and explicitly durable sessions are not evicted. |
| G22 | Configuration is typed and inspectable | Typed Service configuration and `agent-browser config get/set` own settings. Dashboard configuration remains read-only in this plan. Mutable environment policy is not runtime authority. |
| G23 | Valid user instructions are availability-first | Ordinary named-profile open cannot consult or be denied by legacy lease admission, historical owner proof, quarantine, cleanup obligations, or equivalent denial-first metadata. Only current concrete failures may deny it. |
| G24 | Development acceptance precedes production | Implement and validate in the isolated development runtime. Production, external ingress publication, formal release, and merge require separate authority after accepted development evidence. |

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

Keep the extracted Lease Authority crate and explicit administrative,
adversarial, and future multi-tenant contracts buildable in their own scope.
They must not be linked into the trusted single-user ordinary-open dependency
closure. Quarantined historical implementations stay outside the compiled
ordinary runtime and have an owner, reason, and deletion or archival decision.

## Consolidated Batch

- Freeze the executable ordinary-open, provider, persistence, handoff,
  recovery, and desktop-control dependency closures and install architecture
  guards for G01 through G24 before further behavior patches.
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

- every G01 through G24 invariant;
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
G-row to current symbols, state stores, tests, and evidence. Add failing
architecture checks for every currently violated prohibition and a
machine-readable G01 through G24 coverage manifest. Freeze one dependency graph
for ordinary open through handoff recovery. Exit only when omissions and
violations are mechanically visible; do not repair behavior in this milestone.

### M1 | Authority and persistence cutover | 450,000 tokens

Remove legacy denial and JSON or environment authority from the compiled
ordinary dependency closure. Complete one SQLite-backed runtime-host interface,
one forward-only migration, typed configuration, operation journaling,
integrity and backup handling, history budgets, and compaction. Preserve exact
cleanup protection outside admission. Exit when architectural guards and
provider-free authority, migration, corruption, and compaction fixtures pass.

### M2 | Provider, capacity, and launch integration | 400,000 tokens

Remove the development route-keeper interlock and join the in-process
Guacamole keeper to readiness, minimum and warm capacity, bounded queueing,
placement, density, privilege receipts, and browser launch. Prove no Chrome
launch on capacity timeout and no hidden infrastructure browser. Exit when
three isolated zero-process cold starts each produce one ready route within the
measured provisional deadline and the injected timeout launches no Chrome.

### M3 | Durable handoff, recovery, and desktop control | 350,000 tokens

Complete stable handoff resolution, last-committed-URL recovery, eager active
and lazy dormant recovery, single replacement fencing, and the shared Desktop
Services control lease. Prove runtime, provider, Guacamole, route, display,
browser, and tab failure cases preserve the required identity and never replay
page effects. Exit with provider-free failure injection plus one installed
joined recovery pass.

### M4 | Frozen visual-operational acceptance | 400,000 tokens

Freeze source, binary, installed generation, SQLite schema, provider manifest,
route IDs, displays, configuration, and acceptance denominator. Test every
frozen route from authenticated external desktop and mobile viewports using
synthetic content. Require current complete pixels, clean desktop, correct
browser and z-order, focus, pointer, keyboard, scroll, resize, control transfer,
reconnect, and every M3 restart case. Preserve first failures. Permit one
consolidated repair and one bounded repeat of affected cases.

### M5 | Qualification and integration handoff | 200,000 tokens

Run validation selected from the complete P218 diff, verify every G-row and
prohibition against the frozen candidate, update all public and governing
documentation, and perform one closed-world review limited to the contract and
repair regressions. Publish a remote checkpoint and update draft PR #191. Do
not merge, install production, release, or remove the worktree.

## Worker Assignments

The primary agent owns the critical path, contract ledger, branch, candidate,
runtime custody, evidence adjudication, and verdict. No worker is required.
If delegation becomes useful, admit at most one provider-free source worker
and one read-only acceptance-evidence reviewer with disjoint files and explicit
stop conditions. Workers cannot revise G01 through G24, mutate shared runtime,
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

Maintain one evidence table keyed by G01 through G24. Each row records source
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
