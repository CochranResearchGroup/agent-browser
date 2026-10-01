# Plan 0220 | Remote View Consumer Integration

Date: 2026-09-28

Plan version: 26

State: OPEN

Consolidation: required

Product lane: PL-PLATFORM

Lane: P220

Predecessor: Agent Browser Issue #195 and the linked P219 supersession audit, cancelled as superseded while incomplete

Work item: [CochranResearchGroup/agent-browser#202](https://github.com/CochranResearchGroup/agent-browser/issues/202)

Provider evidence: closed [CochranResearchGroup/remote-view#70](https://github.com/CochranResearchGroup/remote-view/issues/70), J2/J3, corrective Plan 0028, closed RV-011 revision 3 [issue #62](https://github.com/CochranResearchGroup/remote-view/issues/62), and closed installed join [CochranResearchGroup/remote-view#146](https://github.com/CochranResearchGroup/remote-view/issues/146)

Target: `main`

Baseline: `origin/main` at `a3848e16`

Branch: `platform/p220-remote-view-consumer`

Owner: primary P220 implementation owner

Integration: merge through the protected pull-request workflow after J2/J3 consumer replay, retained-domain reconciliation, and changed-surface validation

Authority: the operator directed creation and execution preparation on 2026-09-28. This plan authorizes repository planning, provider-free fixtures, and ordinary source implementation after branch admission. Installed, privileged, public-ingress, production, merge, release, destructive cleanup, and Remote View runtime effects remain separately gated.

## Objective

Make Agent Browser a consumer of Remote View desktop and presentation
contracts in the trusted single-user WSL proof of concept. An ordinary request
resolves one logical Agent Browser session and tab, selects an Agent Browser
reserved Remote View desktop, launches or recovers the addressed browser there,
and returns only the durable opaque Agent Browser operator handoff. Resolving
that handoff locates the browser's current desktop and obtains the current
Remote View presentation; it does not bind the handoff permanently to one
desktop or provider route.

Agent Browser continues to own browser profiles, browser processes, CDP,
logical sessions, tabs, targets, browser-to-desktop affinity, activity,
recovery, foreground arbitration, and final-session cleanup. Remote View owns
the Agent Browser-reserved desktop pool, desktop identity, lifecycle, health,
application and window observation, viewing routes, viewer admission,
presentation, OS-level input, diagnostics, and exact desktop cleanup. Agent
Browser launches its browsers into the exact generation-bound desktop
environment. It does not use Remote View `application place` or `application
stop` for those browsers.

Cleanup evidence checkpoint: the external release adapter now requires a private
permit generated from current Browser Runtime references and complete, clear
scoped recovery, foreground and cleanup inventories. The permit binds the full
release target. The ordinary host now compares its last persisted aggregate against current
SQLite state before publishing, preventing stale snapshot replacement. It also
checks the baseline before ordinary Session Manager operations, rejecting
existing durable drift before those effects. Neither check closes the concurrent
read-to-effect window. Runtime
collection, pre-launch association fencing, transactional final release and
physical residue readback remain unimplemented. This source-only
increment does not qualify installed cleanup or the broader Alice/Bob acceptance.

Launch custody source work now models a pending process intent with exact
profile and active assignment identity, plus observed browser PID and subsequent
session publication. Unknown outcomes remain obligations. The SQLite adapter
now admits exact intents, excludes unresolved profile peers and commits session
and custody publication together. Runtime effect wiring and release-claim
exclusion are still pending.
The model contains no environment or provider process ownership.

Release admission source work checks current durable session references and
unpublished launch intents under the custody transaction, then retains an exact
assignment fence. Launch admission and ordinary session publication reject that
fence. Retirement completion source work retains exact joined outcomes and allows
a fresh assignment after completion, including unchanged lifecycle generation.
Recovery and foreground owner admission, runtime effect wiring and physical
absence proof remain pending. Four focused SQLite custody tests qualify
retirement completion and full-aggregate publication. Strict workspace Clippy,
format, architecture and documentation checks pass for this source increment. Independent
cleanup inventories supplied by a caller are not fresh owner proof by themselves.

The next source increment connects durable launch admission to fresh private
environment acquisition and an injectable process effect. Existing intent requires
readback rather than another process. Exact returned process identity is recorded
before handing a launch to the session owner. The BrowserManager runtime now
implements that process boundary, consuming the complete fresh environment with
independent viewing validation and exact assignment lifecycle identity. Private
launches make one Chrome attempt, suppress persisted stderr and redact failures
before the failure journal. Runtime browser-sequence overflow is checked before
process effects. The ordinary host now accepts an observed adapter intent for
atomic custody-and-session publication, advances its baseline and acknowledges
only after success. Generic publication cannot bypass an unpublished matching
launch. The ordinary consumer effect composition now joins host baseline,
coordinator admission, fresh process ingress and publication intent. Real SQLite
composition tests prove first open, healthy reuse, co-location, restart reuse and
failure without duplicate process effects. Native HTTP transport and ordinary
runtime construction now join explicitly configured application pools. New
browser demand prepares the desired capacity through durable acquisition
requests, then joins live assignment and window observations. Construction,
status and healthy reuse allocate no provider resources. Interrupted acquisition
remains pending across restart even when provider inventory shows an assignment;
completed request history advances only with exact active or retired evidence.
Managed command dispatch passes through the same composed owner. Stable
tab-specific handoffs and the complete ordinary request journey remain before
milestone one is complete. Automatic capacity shrink remains in the everyday
lifecycle work. Installed acceptance remains pending.

## Current State

### October 1 provider reconciliation and amendment review

Remote View canonical `main` at `da540f22a6ff851272c5e9b91d7e3117a28bf6cd`
closes Plan 0015 revision 3 and issues #62 and #146. Its current
[application integration guide](https://github.com/CochranResearchGroup/remote-view/blob/da540f22a6ff851272c5e9b91d7e3117a28bf6cd/docs/consumer-integration-guide.md),
[single-user contract](https://github.com/CochranResearchGroup/remote-view/blob/da540f22a6ff851272c5e9b91d7e3117a28bf6cd/docs/design/0004-single-user-application-pools.md),
and [canonical closeout](https://github.com/CochranResearchGroup/remote-view/blob/da540f22a6ff851272c5e9b91d7e3117a28bf6cd/docs/dev/notes/0011-2026-10-01-rv011-canonical-closeout.md)
supersede the earlier provider-gap statements preserved below. The installed
source receipt identifies `2dc51d776e0d92709d3ce0469c03e793b5de891d`, not the
later documentation-only canonical head. Provider acceptance includes an
external client launching and stopping its own xterm, exact window activation,
rendered viewing, input, embedding, lifetime and release. These are reviewed
provider receipts, not live checks or Agent Browser acceptance performed here.

The operator explicitly resumed implementation on October 1. Revision 25
continues the provider-free external-mode adapter packet under the existing
source authority. Runtime effects remain separately gated. Revision 24 was a
documentation-only review. The primary amendments are:

- Consume the shared `remote_view_consumer` application envelope through a
  provider-neutral adapter configured for external lifecycle. A configured
  application selector is resource context, not a credential or principal.
  Existing Authelia ingress remains separate; introduce no Remote View account,
  delegated client token, role binding, or second login.
- Retain exact assignment and lifecycle generation. `observe_assignment`
  supplies an independent viewing generation. Require both for
  `launch_environment`, window inventory, activation and view issuance; refresh
  at effect boundaries and reject stale or missing observations. Carry the
  authoritative full launch environment privately, rather than assuming a
  cached display-only vector is sufficient.
- Use current public operations for inventory/acquire, assignment observation,
  launch environment, windows/activate/events, view issuance/resolution/revoke,
  and acknowledged release. Freeze actual request/response fixtures against
  revision 3; the older strict J1/J3 fixtures are historical compatibility
  evidence, not a current installed-wire contract. Preserve idempotency and
  ambiguous-outcome reconciliation without automatic duplicate effects.
- Emit cleanup schema version 1 for the exact assignment and lifecycle generation
  only after `applicationReferencesClear`, `pendingRecoveryClear`,
  `foregroundLeasesClear`, and `cleanupTasksClear` are all proved. Retained peer
  counting alone is insufficient: include every current browser association and
  unresolved obligation. Remote View separately preflights its own records.
  Pool return and physical scale-in remain separate.
- Keep Remote View view-route expiry/revocation distinct from Agent Browser
  logical session activity and durable handoff identity. Reissue provider views
  when needed without replacing the tab-bound operator handoff. Embedding is
  available but remains optional for this delivery.

Next bounded packet after explicit implementation continuation: one primary
owner reconciles the revision 3 external-mode wire model and implements an
injectable adapter with provider-free request/response replay. Cover independent
stale generations, missing live-resource readiness, environment privacy,
external mode without placement, idempotent requests, and false or mismatched
cleanup acknowledgements. Do not wire live transport effects in that packet.
Exit when focused contract tests, architecture guards, Rust formatting/strict
Clippy and applicable client/schema checks pass. Then connect fresh runtime
observations, transactional detach and reference-safe final release before
handoff/focus/recovery acceptance. Existing objective, ownership, co-location,
operator-stop and installed Alice/Bob gates remain unchanged.

Revision 25 implementation starts with the focused `remote_view_application`
service-model module and the source-bound synthetic
[revision 3 request fixture](../contracts/remote-view-application-r3.v1.fixture.json).
The fixture covers all 12 external-mode public request operations and exact
assignment/environment, inventory, window/action, event, view/grant/revocation
and joined-release response shapes. It excludes managed placement/stop and
principal setup. The injectable launch adapter observes the assignment, fetches
the private full environment with both generations, and observes again before
returning a non-serializable, non-cloneable, debug-redacted launch value. Every
field of the target join must match. Runtime callers must refresh observations
again at actual launch consumption; a stored value is not freshness evidence.

| Revision 3 adapter requirement | Current provider-free evidence | Remaining proof and next action |
| --- | --- | --- |
| External wire shapes | All 12 request operations and public response records round-trip the source-bound synthetic fixture; all five mutation transcripts are injectable; managed verbs and obsolete fields rejected | Actual public transport and runtime integration |
| Assignment join | Exact acquisition and independent desktop/generation join validated; missing or record-only readiness rejected | Wire runtime ingress to fresh observation |
| Private launch environment | Full synthetic map preserved; malformed/incomplete/stale inputs rejected; debug redaction and compile-fail serialization check | Consume through actual browser launch adapter |
| Launch freshness | Injectable observe/environment/observe transcript; changed target invalidates environment; unknown outcomes return without retry | Wire actual launch consumption and authoritative mutation readback |
| Mutation custody | Atomic Browser Runtime SQLite claim precedes transport; restart persistence and competing-connection admission pass; unknown or inexact replies retain pending request; completed cached acquisition is checked against current inventory | Authoritative reconciliation of pending provider outcomes and runtime operation identity |
| Read-side integration | Injectable inventory, windows, events and retained view resolution consume exact request/response transcripts; event payloads reject private environment fields, binding drift and invalid cursors; grants reject expiry, revocation and wrong target/path | Connect runtime consumers and authoritative pending-outcome reconciliation; issuance/resolution does not prove pixels or input readiness |
| Cleanup acknowledgement | Every false/missing assertion and mismatched assignment/generation rejected | Exact provider release response identity and complete retirement sets pass provider-free validation; still construct acknowledgement from all current Agent Browser obligations and qualify installed release |

This is an intermediate source checkpoint, not the completed adapter packet,
installed acceptance, or merge readiness. The [mutation custody contract](../contracts/p220-remote-view-mutation-custody.v1.md)
qualifies injectable submission, exact release response validation and durable
SQLite pending/completed records. Authoritative reconciliation of ambiguous
provider outcomes remains pending. Acquisition keys are absent from public
inventory and client replay keys are absent from assignment events. These
records cannot correlate an unknown mutation by timing or target similarity;
retain the exact unresolved request until authoritative operation readback or
explicit exact-key owner replay qualifies its outcome. Runtime evidence must construct
cleanup acknowledgements; the wire validator does not establish actual
reference or process absence.

Historical checkpoints below retain their original evidence bounds. The full
service-model replay preceded the final wire-compatibility and detached-conflict
refinements; focused retention tests and strict Clippy passed afterward. Do not
claim that comprehensive replay qualified the final exact implementation head.


Remote View Plan 0017 is closed from corrected canonical evidence. J2
integrated at `7298ae7f76626c61ba5086eb4f934699251009ce`; J3 integrated at
`018d3d752f9d99805242f99c00034f0caabc53d1`. Its final provider-free Agent
Browser fixture proves two simultaneous desktop assignments, one external
browser association per desktop, independent desktop and mobile viewer
sessions, private-state exclusion, truthful reacquisition, and exact joined
cleanup. RV-014 and Remote View Issue #70 are closed.

The initial Plan 0017 closure was later reopened because its complete operation
matrix was reachable only through in-memory adapter-shaped fixtures and its
closure evidence lacked release and documentation validation plus a fresh
resource census. Corrective Plan 0028 added real production CLI,
authenticated HTTP, and MCP reachability through one `ControlRuntime`, repaired
the legacy capacity-routing regression found during canonical validation, and
reclosed Plan 0017 from source `b7caa1f209f2d64cd400b1766c50389ab4dc5cb3`
with governance evidence at
`eefddd2c862ce97bcf4387b2cc43dbbb2d9274eb`. Remote View `origin/main` at
`30f3e37` reconciles the closure narrative. This is authoritative provider
source and provider-free validation evidence, not Agent Browser installed
acceptance or authority for a live effect.

P220 source checkpoint `9a34e292` establishes the Agent Browser-owned F0
consumer boundary and distinct two-desktop browser/profile association
invariant. The current follow-up consumes the complete published foundation
observation, including command, resource states, and effect discipline. Remote
View J1 source checkpoint `f674518e34fea346002c72c4adc3966b628d0b78`
publishes lifecycle observations, durable operation status, exact generation
fencing, and release semantics. The now-published J2 and J3 contracts add
application registration, bounded pools, exact assignment and placement,
opaque viewing-route identity, viewer admission and layered live status,
generation-safe reacquisition, placement stop, and joined release that retires
the exact route and viewer sessions.

Retained-domain extraction is active through `ca3709e1`. The extracted
presentation-neutral spine owns named browser sessions and tabs, independent
durable SQLite persistence, one-time archived JSON migration, verified
rotating backups, restart reattachment, idle reaping, focus, and addressed
ordinary-command routing. It now also bounds disposable profile count and
regular-file bytes with oldest-first protected-session-aware eviction, rejects
pin or promotion bypass fields, and compacts exact URL history into
restart-safe daily identity summaries under a fixed byte ceiling. Desktop
selection consumes Remote View UUID, route label, generation, readiness,
pool-assignment, and application/window observations instead of the legacy
Agent Browser route inventory. The earlier
`remote_view_application_placement_contract_unavailable` boundary and the
request recorded on Remote View issue #62 were based on an incorrect ownership
model. Remote View's managed application-placement verbs are valid for
consumers that delegate launch and stop effects, but they are not an Agent
Browser dependency. Agent Browser owns browser launch, process identity,
profile, CDP, readiness, termination, and recovery. Remote View neither
authorizes nor inspects CDP.

The runtime join now targets Remote View's existing reserved-pool assignment,
generation-bound desktop environment, live application/window inventory,
durable events, authenticated viewing route, and exact window-raise surfaces.
One Remote View assignment reserves a desktop for the Agent Browser service;
Agent Browser may associate multiple browsers with that desktop. Empty
desktops are preferred, not required. Assignment release returns an empty
desktop to the Agent Browser-reserved pool, while a distinct clean scale-in
operation may return it to general Remote View capacity. A bounded integration
spike against Remote View `origin/main` at
`0a42bff5202858a452848a781376caac66667a2f` confirms that installed dynamic
desktop lifecycle observations now supply exact live launch resources. The
record joins `desktopId`, `generation`, `resources.display`,
`resources.vncPort`, and `readinessScope: live_resource`, so launch does not
require application placement. The same spike found one narrower public join
gap: installed-control lifecycle uses a random desktop UUID while
`GatewayDesktopProjection` independently derives a fixed UUID and the public
window operations remain slot-addressed. Agent Browser cannot infer that join
from friendly route, display, capacity order, or local configuration. Remote
View issue #146 records the exact generation-bound viewing and window-action
need without requesting browser or CDP ownership.

Remote View's opaque `routeId` is a provider route identity, not by itself the
Agent Browser operator-facing `/remote-view/<handoff-id>` URL. P220 will keep
those identities distinct and add one Agent Browser handoff adapter over the
public route contract. The Agent Browser handoff identifies one logical session
and tab, resolves the browser's current desktop at use time, activates the exact
CDP target, and then raises the browser's current top-level window through
Remote View. It must not expose a route ID or provider URL as though it were the
durable operator handoff.

P220 checkpoint `552c8162` consumes the strict public J3 registration,
assignment, placement, viewing-route, viewer-session status, and joined-release
records in the provider-free service-model boundary. The source-bound fixture
proves two exact desktops, one ready placement and opaque route per desktop,
independent desktop/mobile viewer sessions, and release that retires the exact
route and both sessions. Unknown browser-private fields, partial retirement,
identity mismatch, and generation mismatch fail closed. This completes the
J3 wire-consumption tracer. Its placement-shaped records remain compatibility
evidence for Remote View, but P220 must adapt the runtime join and retained
binding so Agent Browser browsers depend on service-level desktop assignment
and observation rather than managed application placement.

Checkpoint `d7c43915` exposes the P220-owned SQLite backup boundary through
`service runtime-backup status|create` and the generic HTTP and MCP
`service_request` transport. Status verifies without mutation; create performs
one online SQLite backup, validates integrity and digest, and rotates at most
one previous copy. Neither surface restores data or includes the separate
Service State store. The generated client publishes typed status and manifest
responses. Disposable smoke cleanup now accommodates the intentionally
read-only migration archives.

Checkpoint `b47f7a0c` maps the validated Remote View public registration, pool,
assignment, placement, desktop generation, opaque route, and viewer-session
identities into an Agent Browser-owned retention record keyed by the exact
browser. The binding is idempotent across viewer-list ordering, survives a
SQLite restart, and rejects duplicate public identity across active browsers.
That one-placement-per-browser mapping is now an adaptation target: the Remote
View assignment belongs to Agent Browser's use of the desktop and may be
referenced by multiple Agent Browser browsers. Release changes state only when
the desktop reference count reaches zero and the released assignment, desktop
generation, route, and complete viewer-session set match exactly. The retained
record has no provider URL, display number, credential, or provider-private
state.
Follow-up `fd4264ec` makes bind and release single `BEGIN IMMEDIATE`
transactions over the complete Browser Session aggregate, preventing a future
runtime adapter from losing concurrent session or presentation updates.
Checkpoint `d3e31c6f` adds the pure operator-handoff projection. It accepts only
an active retained binding and a bounded path-safe Agent Browser handoff ID,
then returns exactly `handoffId` and `/remote-view/<handoff-id>`. Released
bindings and slash-bearing IDs fail closed. Route, desktop, provider, display,
and credential identity cannot enter the projection body.

Checkpoint `496c25f8` restores the presentation-neutral browser-launch
resource admission selected from P219. Immediately before the first local
Chrome effect, the Browser Session worker observes available memory, profile
filesystem capacity, host PID capacity, and Agent Browser root-process count.
Pressure returns a typed `browser_launch_resource_pressure` error without a
launch. The current
`remote_view_application_placement_contract_unavailable` pre-launch stop is
obsolete and must be replaced by exact assignment plus launch-environment
acquisition before the existing admission and Agent Browser-owned browser
launch.

Checkpoint `e36f7d42` exposes additive, read-only Browser Runtime operational
health through Service status, the generated client contract, and install
doctor. The projection reports redacted SQLite integrity and size, migration
archive state, verified-backup state, and current launch admission. Missing or
invalid storage collapses to a stable failure code and never exposes its path.
The status adapter opens only an existing regular database with SQLite
read-only flags; it performs no migration or backup effect.

Checkpoint `c1f5fe27` closes the last ambiguity in Agent Browser-owned
final-session cleanup. An explicit close of the only logical session invokes
the exact browser close once, does not issue a separate tab close, clears the
active browser, session, and tab records, and retains terminal session and tab
history. This is browser cleanup only. Remote View presentation release remains
an exact, separate transaction that requires an authenticated joined-release
outcome from runtime ingress.

Checkpoint `716ef3a5` replaces the obsolete application-placement launch stop
with a provider-neutral runtime-context contract. A ready allocated Remote
View lifecycle observation projects exact desktop UUID, generation, and
authoritative display name. The browser worker accepts that display only when
both UUID and generation match, rejects missing, stale, or invalid context
before resource admission or Chrome launch, and never derives display from a
route label. The default host supplies no fabricated context, so production
Remote View launch remains fail-closed until its public adapter is wired.

Checkpoint `cfaf3f12` adapts retained presentation identity to the accepted
shared-desktop model. Multiple browsers may retain the same exact registration,
pool, assignment, desktop generation, route, and viewer-session set while
keeping distinct Agent Browser browser, profile, session, tab, and target
identity. A browser detach is durable and does not release the Remote View
assignment. Joined release fails while any peer browser reference remains.
Conflicting or partially overlapping desktop identity still fails closed. The
legacy schema-v1 `placementId` string remains empty for reader compatibility
and no longer participates in validation or retention authority.

Agent Browser Plan 0219 is cancelled as superseded while incomplete after its
authorized final cold-install attempt failed. Its branch is 36 commits ahead of
the published topic ref at audit start and contains both reusable Agent Browser
domain work and retired presentation-specific work. Two later governance
commits bring the preserved branch to 38 commits ahead. The successor branch was admitted from
`origin/main` at `a3848e16`; P219 custody remains preserved and is not merged
wholesale.

## Product boundary

Agent Browser owns:

- profile, process, CDP, logical-session, tab, and target state;
- selection of the browser-to-desktop association;
- navigation, browser recovery, and addressed-session cleanup; and
- browser readiness and CDP diagnostics.

Remote View owns:

- fixed or bounded desktop selection and allocation;
- desktop generation, readiness, presentation, and OS-level input;
- stable opaque viewing routes and viewer connectivity;
- generic allowlisted application-launch execution when used; and
- exact desktop release, cleanup, and presentation diagnostics.

Neither product imports the other product's private state authority. The
initial operating model is one trusted privileged user in one WSL instance.

## Accepted interaction model

- Remote View configuration provides a desktop pool reserved from general-use
  allocation for Agent Browser. Agent Browser chooses desired capacity; Remote
  View performs and reports physical growth, draining, shrinkage, and cleanup.
- One Remote View assignment represents Agent Browser service use of one
  desktop. It is not a one-browser reservation. An open desktop with no
  observed applications is preferred, but a healthy occupied desktop remains
  eligible when Agent Browser resource admission and correlation are safe.
- Agent Browser obtains the exact generation-bound desktop launch environment
  from a local authenticated Remote View contract, then launches and stops its
  own browser. Launch-environment paths, sockets, tokens, and provider details
  never enter handoffs or ordinary diagnostics.
- Remote View automatically inventories applications and top-level windows.
  Agent Browser persists only minimal locators correlated to its own process
  families. A window ID is a generation-scoped hint; focus-sensitive actions
  refresh inventory and perform bounded reacquisition before failing on absence
  or ambiguity.
- A durable Agent Browser handoff identifies one logical session and tab, not a
  desktop. Resolution locates or boundedly recovers the browser, selects its
  current Remote View desktop, activates the exact CDP target, and raises the
  current top-level window. Restoration covers durable URL intent and logical
  metadata, never volatile DOM, form, scroll, or page-effect state.
- Agent Browser owns a heartbeat- and TTL-bound foreground lease. CDP target
  activation and Remote View window raise form one serialized per-desktop
  focus transaction fenced by the same lease epoch. The lease authorizes focus
  at handoff activation and immediately before addressed interaction; it does
  not continuously fight operator focus changes.
- Viewer presence, foreground control, and logical-session activity remain
  separate. Passive viewing does not refresh session activity. Authenticated
  input refreshes it only when the current foreground lease and target are
  confirmed.
- Deliberate tab close terminates only that tab handoff and requires explicit
  **Reopen tab**. An attributable operator browser stop creates durable,
  browser-generation-scoped recovery suppression; loading a handoff remains
  non-mutating until explicit reopen. An unattributed disappearance follows
  bounded demand-driven recovery.
- Browser recovery may relocate onto another reserved desktop while preserving
  session, tab, and handoff identities. The prior assignment remains
  quarantined until absence or exact cleanup is established. Remote View
  presentation failure does not invalidate a healthy browser or CDP path.
- Remote View durable events provide prompt changes and attribution; a periodic
  full reconciliation remains the correctness backstop. Remote View reports
  operator action and OS identities but does not interpret Agent Browser
  session or tab identity.
- Assignment release returns a clean desktop to the Agent Browser-reserved
  pool. Automatic scale-in removes only desktops with no live browser
  references, foreground lease, or unresolved cleanup or recovery. Routine
  scale-in never migrates or terminates a browser solely to reduce capacity.

## Consolidated batch

The first delivery batch contains four related outcomes:

1. Classify every unpublished P219 commit and changed surface as retain, adapt,
   retire, or evidence-only.
2. Freeze a provider-neutral Agent Browser presentation-client boundary against
   the current Remote View F0 identities and observations.
3. Prove one Agent Browser-reserved desktop can carry multiple distinct
   Agent Browser-owned browsers and that a browser can recover onto another
   reserved desktop without moving process, profile, CDP, or session authority
   into Remote View.
4. Produce an integration candidate that removes dependency on the retired
   Agent Browser-owned XRDP/Guacamole installation path without activating a
   live Remote View runtime.

Remote View internal capacity implementation, iframe embedding, generalized
IAM, cross-principal policy, multi-tenant isolation, and a formal release are
deferred. P220 consumes current pool growth and shrinkage contracts and keeps
presentation behind an adapter so later embedding does not change handoff or
browser semantics.

## Delivery sequence and budget

### Revision 26: delivery path to a working candidate

The operator approved this amendment on October 1 after the source checkpoint
at `1483b504`. This sequence governs remaining execution. Earlier S0 through S3
sections below preserve implementation history and supporting requirements;
they are not competing queues of new work. The original acceptance criteria,
P219 dispositions and ownership boundaries remain in force. At amendment time, implementation
was paused at the requested token checkpoint. Subsequent explicit resumption is
recorded in RUNBOOK. This amendment authorizes
planning changes only; it does not resume the goal or authorize installed effects.

The next explicit resumption follows four outcomes in order:

1. **Connect normal browser requests.** Wire consumer launch admission and intent
   retention into the ordinary Session Manager and atomic host publication.
   Connect assignment selection, fresh launch inputs and stable tab-specific
   handoff resolution through the existing public adapter. Exit with an
   executable development-candidate path for open, session creation and handoff
   resolution, plus provider-free composition tests that exercise the actual
   host, manager and adapter together. Simulated results are source evidence;
   working installed links are proved in milestone 3.
2. **Finish everyday browser behavior.** Complete tab and window focus,
   presentation renewal and reconnect, deliberate tab/browser close, independent
   activity clocks, bounded demand-driven recovery, desktop relocation and
   final-reference cleanup. Preserve multiple browsers per assigned desktop.
   Exit with the user behavior implemented on the normal path and focused
   regression coverage, including uncertain outcomes without duplicate effects.
   Reuse existing mechanisms; add a new abstraction only when a named remaining
   behavior cannot be implemented coherently through the current seams.
3. **Prove the installed experience.** After the existing installed-acceptance
   prerequisites and explicit effect authorization, run the Alice/Bob packet
   below against exact development binaries. Prove real opening and tab-specific
   viewing, multiple tabs and browsers, focus, reconnect, deliberate close,
   recovery, relocation and cleanup. Exit with source-bound results for every
   applicable axis and a fresh before/after resource census. Keep partial results
   and diagnose demonstrated defects; simulated substitutes do not close a live
   criterion. No additional permission is required merely for ordinary source
   repairs within already authorized scope.
4. **Prepare the release candidate.** Fix demonstrated acceptance defects,
   freeze the candidate source and binary identities, complete changed-surface
   checks and protected integration, and qualify the applicable installer,
   diagnostic and many-to-many remote-operation requirements in AGENTS.md.
   Exit with a reproducible candidate artifact and a compact release-readiness
   report identifying every passed gate and any remaining blocker. A checkpoint
   build or development publication alone is not a qualified release candidate.
   Formal release, production promotion and publication remain separately
   directed maintainer actions; candidate preparation does not authorize them.

At each milestone, checkpoint code custody, demonstrated behavior, validation
and remaining blockers before starting the next milestone. Use RUNBOOK as the
single current execution record. Derive only the next bounded implementation
packet; do not maintain another infrastructure backlog beside these outcomes.
Do not expand into generalized IAM, multi-tenancy, provider implementation,
unrelated architecture extraction or repairs to unrelated historical tests.

An additional one-million-token resumption is a ceiling, not a delivery estimate
or an instruction to consume it. On explicit resumption, record its actual goal
starting meter and stop with a restart checkpoint before the additional allowance
is exhausted. Plan versions and packet boundaries do not reset cumulative
accounting. Assess progress by
completed user behavior and remaining release gates. Reuse passed checks when
covered executable inputs have not changed; widen validation for an actual
changed surface or demonstrated regression. Stop earlier when the authorized
outcome is achieved or a genuine external dependency prevents further progress.

### Earlier delivery sequence and retained evidence

### S0 — Custody and semantic audit

Inventory the 36 unpublished commits, changed files, schemas, tests, docs, and
runtime assumptions. Record exact retain, adapt, retire, and evidence-only
groups. Select an integration base without rewriting or deleting P219 custody.

Exit: every changed surface has one disposition and conflicting mixed commits
have an explicit extraction strategy.

### S1 — Provider-free consumer contract

Add an Agent Browser-owned Remote View client boundary and fixtures for public
identity, observation, desktop selection, opaque handoff, status, and release
shapes. Use no Remote View internal modules and perform no runtime effects.

Exit: the F0-shaped fixture proves reserved-desktop selection, multiple
distinct browser associations on one desktop, and browser relocation across
desktops while Agent Browser retains its domain authority.

Status: F0 and J1 complete at source checkpoint `8f753da0`; strict J3 public
wire consumption is complete at `552c8162`. Public identity, foundation
observation, lifecycle observation, durable operation status, allocated
desktop selection, distinct browser association, and exact
UUID-plus-generation release targeting are covered. J2 and J3 are canonical.
The existing placement-shaped fixture must now be adapted to the accepted
consumer boundary: one service-level assignment may host several Agent Browser
browsers, application/window inventory is observed rather than registered by
Agent Browser, and assignment release is reference-safe. Operator handoff
materialization remains Agent Browser-owned and must preserve the route-ID
versus handoff-URL distinction.

### S2 — Retained-domain integration

Integrate only the P219 browser/session/runtime changes selected by S0. Adapt
presentation calls behind the new client boundary. Exclude or remove the
retired XRDP user, Guacamole provider-rebuild, route-pool, and
presentation-helper path from the successor candidate.

Exit: changed-surface tests pass; architecture checks reject reintroduction of
the retired presentation ownership and reject use of Remote View managed
application placement for Agent Browser browsers.

Status: partial. The initial session, durable store, and runtime spine plus
named-tab, reaping, focus, addressed-command, disposable-retention, and exact
URL compaction behaviors are extracted and locally qualified. The quota model
accepts protected session identities without importing the retired
viewer/controller tables. The J2/J3 join must now map exact public pool,
assignment, desktop, route, viewer-session, and application/window observations
into Agent Browser's own retention policy and fail closed on ambiguity.
Checkpoint `ca3709e1` replaces
the supported JSON persistence path with SQLite WAL authority for the session
and profile aggregates. Migration stages the database atomically, archives
legacy inputs read-only, makes an existing database authoritative even when it
is corrupt, and provides integrity-checked current-plus-previous online backup
rotation. Checkpoint `64f75821` adds pure recovery admission and transition
rules plus a restart-safe SQLite recovery registry. Replacement requires exact
client or authenticated-viewer demand and proof that the old browser is
unusable; retry timing is bounded, and observed-live, failure, and success
transitions reject stale generations. Runtime launch and observation wiring,
authenticated operator-handoff resolution, joined presentation release, and
protected integration remain. Checkpoint
`a2080250` adds the architecture guard
for this boundary. It rejects provider-private types in the pure model, local
display inference from Remote View identity, retired route-keeper authority in
the new CLI store, persistence dependencies in the service-model crate, J3
checkpoint drift, and loss of strict private-field or cleanup validation.
Checkpoint `d7c43915` completes operator backup status and creation across CLI,
HTTP, MCP, schema, and generated-client surfaces. Focused Rust backup and
contract tests, strict Clippy, formatting, client contract and type checks,
API/MCP parity, the no-launch contract smoke, P220 architecture guard,
documentation links, and the production documentation build pass. The broad
`test:service-client` umbrella remains blocked by a pre-existing stale P157
source-literal oracle that expects `ServiceRequestProvenance::capture` while
both the batch baseline and current source use
`capture_service_request_provenance`.
Checkpoint `b47f7a0c` completes the provider-neutral retention mapping for one
validated public presentation binding and its exact joined release. Runtime
ingress must adapt that browser-keyed prototype into a desktop-assignment
binding shared by all Agent Browser browsers currently associated with the
desktop. It then invokes the pure mutation from authenticated Remote View pool,
desktop, route, viewer, and observation records without using application
placement.
The store adapter is transaction-ready at `fd4264ec`; transport ingress remains
intentionally absent rather than accepting caller-forged public identities.
The durable-link output contract is ready at `d3e31c6f`, but readiness and
resolution still require authenticated runtime evidence and are not inferred
from link construction.
Checkpoint `496c25f8` completes the independently extractable browser-launch
resource gate. Focused pure and runtime-worker tests prove that zero capacity
fails before Chrome launch; the architecture guard requires admission to
precede `BrowserManager::launch` and rejects provider vocabulary in the
admission adapter. Installed capacity projection remains separate from this
effect fence.
Checkpoint `e36f7d42` completes the independently extractable read-only
Browser Runtime health projection. Focused Rust tests prove available and
missing-database results, path redaction, unchanged database digest, and no
backup creation. Schema, generated-client, cross-seam, API/MCP parity,
install-doctor provenance, documentation, architecture, formatting, strict
Clippy, and production docs-build checks pass. The browser-launching collection
smoke timed out before reaching status assertions; its exact `sc-24089`
process tree and empty disposable directory were removed, so it supplies no
acceptance evidence and leaves no matching residue.
Checkpoint `c1f5fe27` directly proves the already-implemented final-session
browser cleanup invariant for explicit close. Agent Browser removes its active
browser/session/tab records only after the browser close succeeds and preserves
terminal history. The test intentionally does not release Remote View state:
that mutation must decrement Agent Browser's exact desktop-assignment
references. Remote View assignment and presentation release occurs only after
the final browser reference and every cleanup, recovery, and foreground
obligation are gone, using the authenticated assignment, generation, route,
and complete viewer-session result supplied by runtime ingress.
Checkpoint `716ef3a5` completes the launch-side context seam. Focused
service-model and CLI tests prove projection from the source-bound lifecycle
fixture, exact generation matching, authoritative display use, and a
pre-launch failure when context is absent. The self-testing P220 architecture
guard now rejects restoration of the application-placement stop, loss of the
generation fence, or route-label display inference. Workspace formatting and
strict Clippy pass. The production adapter, dynamic refresh, viewing/window
join, and live launch remain pending.
Checkpoint `cfaf3f12` completes the pure shared-assignment retention and
transactional detach seam. Five focused retention tests prove exact sharing,
distinct logical identities, conflict rejection before and after detach,
reference-fenced release, handoff privacy, and exact joined cleanup. The full
228-test service-model unit suite and every service-model integration test,
the focused SQLite restart transaction, the self-testing architecture guard,
formatting, and strict workspace Clippy pass. Runtime ingress still must call
detach and final release from authenticated Remote View evidence.

### S3 — Checkpoint replay and final acceptance preparation

Replay the consumer fixture against Remote View J1, J2, and J3 contracts as
they become available. Extend it for generation-safe viewing, diagnostics,
many-to-many observation, and exact cleanup without importing Remote View
internals.

Exit: provider-free final acceptance is source-bound and the separately gated
installed acceptance packet has exact targets, stop rules, and rollback or
forward-recovery boundaries.

No installed acceptance begins merely because S0 through S3 source work passes.

Status: J1 replay complete against Remote View source checkpoint
`f674518e34fea346002c72c4adc3966b628d0b78`. The fixture rejects unversioned
shape drift and tampered operation payload evidence. The J3 Agent Browser wire
fixture passes at `552c8162`, including strict private-state exclusion and
exact joined cleanup. The earlier conclusion that the provider-free
application effect blocked P220 was incorrect because Agent Browser does not
delegate browser launch to `application place`. The source spike consumed the
installed desktop lifecycle and established that its exact live resource
observation is sufficient for Agent Browser-owned launch. It also demonstrated
the concrete identity discontinuity between the dynamic installed-control
desktop UUID and the separately derived gateway desktop UUID plus
slot-addressed window surface. Remote View issue #146 tracked that
generation-bound viewing and window-action join and is now closed; revision 24
requires consuming its integrated successor application interface. Remote
View's provider-free fixture remains compatibility evidence, not a substitute
for proving Agent Browser's browser launch, recovery, durable handoff,
retention mapping, focus transaction, or cleanup adapter.

### Separately gated installed-acceptance packet

This packet is prepared but not executable under the plan's current authority.
It becomes eligible only after all of the following are true:

- the source integration spike binds an exact Remote View reserved-pool
  assignment, generation-bound desktop launch environment, live
  application/window inventory, durable event cursor, viewing route, and
  window-raise operation without using `application place` or `application
  stop`;
- provider-free replay proves service-level assignment sharing, multiple
  browsers on one desktop, browser relocation, window-locator reacquisition,
  foreground-lease fencing, operator-stop suppression, and reference-safe
  release;
- the development runtime identifies exact Agent Browser and Remote View
  binaries, contract versions, reserved pool, desktop UUIDs and generations,
  listener owners, and an isolated disposable profile root; and
- the operator separately authorizes the installed Remote View and browser
  effects. Readiness or source completion alone does not grant that authority.

The execution target is one reviewed Agent Browser-reserved Remote View pool,
at least two desktops, and disposable Agent Browser profiles in the isolated
development runtime. Record the pool, desktop UUIDs and generations,
service-level assignments, browser IDs, process identities, CDP endpoint
identities, logical session and tab IDs, foreground-lease epochs, route IDs,
viewer-session IDs, handoff IDs, event cursors, and runtime operation IDs before
interpreting any result. No default or production profile, installed production
binary, shared operator browser, or unlisted desktop is in scope.

Run the acceptance axes in this order, stopping before the next effect whenever
the current axis is not proved:

1. **Baseline and identity.** Capture fresh process, listener, unit, container,
   filesystem-capacity, PID-capacity, and memory census evidence. Require clean
   development-runtime doctors, Browser Runtime integrity, a verified backup,
   and exact installed binary and contract provenance.
2. **Tab-bound handoff.** Open Alice and Bob on one exact-profile browser.
   Require distinct logical session, tab, target, handoff, activity, and expiry
   identities. Each handoff must resolve the addressed tab, locate the
   browser's current desktop, activate the exact CDP target, raise the current
   top-level window, and publish ready only after both layers agree.
3. **Desktop co-location.** Launch a second Agent Browser browser on the same
   service-assigned desktop. Prove distinct process, profile, CDP, browser, and
   handoff identities without acquiring a second assignment for that desktop.
   Pool exhaustion alone must not reject the launch when resource admission and
   observation remain unambiguous.
4. **Foreground arbitration.** Transfer focus between sessions and browsers.
   Serialize CDP target activation and Remote View window raise per desktop;
   fence both with one Agent Browser foreground-lease epoch. Passive viewer
   heartbeats do not extend logical-session activity. Attributable input extends
   it only when the current foreground target is confirmed.
5. **Close and recovery policy.** Prove a deliberate tab close affects only its
   handoff and requires explicit **Reopen tab**. Prove an attributable operator
   browser stop suppresses automatic recovery until explicit reopen. Prove an
   unattributed synthetic loss performs one bounded recovery under active
   demand and restores only durable URL intent, not volatile page state.
6. **Desktop relocation and presentation interruption.** Make the original
   desktop unavailable and recover the browser once onto another reserved
   desktop while preserving logical session, tab, and handoff identities.
   Separately interrupt Remote View presentation and prove healthy CDP
   automation continues while viewing and OS-input readiness fail truthfully.
7. **Pool elasticity and exact cleanup.** Grow the pool only when current
   desktops cannot safely absorb demand. Release an idle assignment back to the
   reserved pool without exposing it to general allocation. Shrink only a clean
   desktop with no browser references, foreground lease, or unresolved cleanup
   or recovery state. Close a non-final shared session without closing its peer
   and terminate the browser only after its final active obligation ends.
8. **Fresh residue census.** Repeat the baseline census from a fresh process.
   Accept only when no unexplained Agent Browser browser, runtime daemon,
   listener, unit, Remote View operation, assignment, viewer session, or owned
   desktop resource remains. Historical terminal records and the reviewed
   verified backup are evidence, not live occupancy.

Stop immediately on any raw provider URL or credential in operator output;
provider-private state crossing the consumer boundary; desktop, generation,
process, CDP, route, viewer-set, or owner ambiguity; a stale or unknown runtime
operation; launch-resource pressure; a mutation aimed at production identity;
or any effect not attributable to the exact development-runtime custody. A
partial axis is diagnostic evidence only and cannot validate later axes.

Recovery is forward and identity-bound. Reinspect the same durable operation
and handoff after an unknown outcome; do not submit a second open, assignment,
release, or cleanup request. A cached window ID is only a fast-path hint: stale
locators trigger bounded reacquisition from a fresh Remote View inventory and
Agent Browser process-family evidence. A quarantined result retains its exact
cleanup obligation. Release only from authenticated assignment and viewing
records after Agent Browser's reference count reaches zero, and close a browser
only through the addressed Agent Browser session path. Never delete by slot,
PID alone, display, window title, process-name sweep, or guessed ownership. If
the exact owner cannot be proved, stop with the resource intact and record the
unavailable-work impact plus the supported recovery action.

| Acceptance axis | Current evidence | Installed proof still required |
| --- | --- | --- |
| Contract and boundary | F0/J1/J3 provider-free replay and architecture guard pass | Exact pool, environment, observation, event, viewing, and window-action adapter provenance |
| Browser behavior | Session, tab, recovery, launch-admission, and final-close fixtures pass | Shared-browser tabs, co-located browsers, operator-stop policy, and relocation |
| Presentation | Retention, handoff projection, and exact release fixtures pass | Tab-bound ready handoff, fenced focus transfer, presentation interruption, and reference-safe release |
| Resources | Read-only launch admission and Browser Runtime health pass | Before-and-after fresh OS census around the separately authorized run |
| Integration | Branch checkpoints are pushed and issue #202 is current | Changed-surface batch qualification, protected review, and merge |

This packet completes acceptance preparation only. Every installed-proof cell
remains incomplete until the exact runtime evidence exists.

## Worker assignments

One primary P220 owner controls the integration base, shared client contract,
and final reconciliation. Read-only commit classification and provider-free
fixture design are parallelizable after branch admission because they write
separate audit and test surfaces. Client-contract implementation, extraction
of mixed P219 commits, schema generation, and shared documentation remain on
the serialized critical path. The launch half of the runtime integration spike
is complete at `716ef3a5`. The next critical-path packets are the Agent Browser
installed-control adapter and shared desktop-assignment retention; viewing and
window-action resolution now consumes the integrated revision 3 application
interface; issue #146 is no longer an external blocker.

Remote View retains authority over its repository. Issue #70 and RV-014 are
closed from Remote View's J3 fixture; Agent Browser consumes the published J2,
J3, and corrective production-adapter checkpoints and does not edit Remote
View source as part of this plan.

## Evidence and exit

| Axis | Required evidence | Invalidation |
| --- | --- | --- |
| Custody | Exact P219 commit and file disposition with preserved branch and artifacts | Missing or silently discarded unpublished work |
| Boundary | Tests prove Agent Browser and Remote View retain the ownership split above | Either product becomes authoritative for the other's private state |
| Contract | Provider-free fixtures consume only published Remote View contracts | Importing Remote View internals or inventing a second control plane |
| Browser behavior | Shared-browser tabs and co-located browsers retain distinct browser/process/profile/CDP/session identities | Duplicate association, profile substitution, fixed one-browser-per-desktop admission, or Remote View browser ownership |
| Presentation | Handoffs resolve one logical tab through its browser's current desktop; readiness and failures remain layered | Handoff bound permanently to a desktop, raw provider URLs, credentials, stale focus publication, or fabricated viewer readiness |
| Cleanup | Operator-stop intent, browser references, assignment release, and pool shrink remain distinct and ambiguity fails visibly | Automatic reversal of operator stop, cleanup by slot/PID/title, or assignment release while references remain |
| Resources | Fresh process/resource census after separately authorized installed acceptance | Unexplained browser, desktop, daemon, listener, unit, or container residue |

## Acceptance criteria

- The P219 audit assigns every unpublished commit and changed file one durable
  disposition.
- Agent Browser uses a versioned provider-neutral consumer boundary for Remote
  View operations and observations.
- One reserved Remote View desktop can support multiple Agent Browser-owned
  browsers without transferring profile, process, CDP, session, focus, or
  recovery authority; pool exhaustion alone is not a refusal reason.
- The same opaque Agent Browser handoff resolves its exact logical tab through
  the browser's current desktop and survives bounded relocation without
  exposing provider credentials or raw Guacamole routes.
- Agent Browser serializes tab activation plus Remote View window raise under
  one foreground-lease epoch and reacquires stale window locators before
  publishing readiness.
- Attributable operator stop suppresses automatic recovery until explicit
  reopen; deliberate tab close affects only that handoff; unattributed loss
  follows bounded demand-driven recovery.
- Viewer presence, logical-session activity, and foreground control remain
  separate clocks. Passive viewing does not indefinitely retain a session.
- Assignment release returns a clean desktop to the Agent Browser-reserved
  pool. Only explicit clean scale-in returns it to general Remote View use.
- Diagnostics independently report pool/desktop, browser/CDP, tab/focus,
  transport, viewer, and cleanup readiness.
- Provider-free probe replays pass at applicable Remote View F0/J1/J2/J3
  checkpoints; each result is compatibility evidence at that checkpoint only.
- Exact release and cleanup leave no unexplained owned resources, while
  ambiguous ownership fails without broad deletion.
- Required Rust, generated-client, schema, documentation, architecture,
  planning, and lane gates pass for every touched surface.

## Non-goals

- Repairing or accepting the P219 Agent Browser-owned XRDP/Guacamole cold
  installation.
- Moving browser lifecycle or browser-private state into Remote View.
- Implementing Remote View's internal desktop provider, application-placement
  effect, or embedding architecture. Agent Browser may consume current pool
  growth/shrink and viewing contracts behind adapters that can adopt embedding
  later.
- Adding generalized multi-user security to the present single-user proof of
  concept.
- Treating a provider-free test as installed, public, production, or release
  authority.

## Hard stops

- Keep the admitted successor worktree bound to the audited `origin/main`
  baseline and preserve the separate P219 custody branch.
- Do not merge the 36-commit P219 branch wholesale.
- Do not delete, rewrite, or clean the P219 branch, failed overlays, serial
  logs, or receipts without an explicit custody disposition.
- Do not repair the retired XRDP/Guacamole presentation path under this plan.
- Do not use Remote View `application place` or `application stop` for Agent
  Browser browsers, and do not add a second Agent Browser desktop-reservation
  ledger beside Remote View's pool assignment authority.
- Stop on ownership ambiguity, raw provider credential exposure, contract drift
  without an explicit checkpoint decision, or any required live effect without
  exact authority.

## Definition of done

Plan 0220 is complete when the selected presentation-neutral P219 work is
integrated through protected review; Agent Browser consumes Remote View public
pool, desktop-environment, observation, event, viewing, and window-action
contracts through one provider-neutral boundary; provider-free F0/J1/J2/J3
evidence and final installed acceptance are source-bound and truthful; the
Alice/Bob tab-bound handoffs, co-located browsers, fenced focus transfer,
operator-stop policy, bounded recovery, and desktop relocation survive the
presentation replacement; the retired XRDP/Guacamole ownership does not remain
on the supported path; exact reference-safe cleanup and a fresh resource census
pass; and roadmap, runbook, plan, work item, lane, Git, validation, and installed
identities agree.
