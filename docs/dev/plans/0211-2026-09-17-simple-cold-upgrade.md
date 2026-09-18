# Plan 0211 | Simple Install, Upgrade, And Remote View

Date: 2026-09-17

Plan version: 20

State: OPEN

Consolidation: required

Product lane: PL-PLATFORM

Lane: P211

Work items: `CochranResearchGroup/agent-browser#181`, `CochranResearchGroup/agent-browser#183`, `CochranResearchGroup/agent-browser#195`

Branch: `platform/p211-simple-cold-upgrade`

Target: `main`

Integration: merge through the protected pull-request workflow after provider-free shutdown, cold-install, restart, remote-view, and documentation validation

Source baseline: `fb616aeed2233aca06b4379d61af5d34609a804d`

## Objective

Replace the default production hot-upgrade path with one bounded cold workflow:
stop all Agent Browser-owned machinery, install the new payload, and start a
clean runtime. The ordinary operator interface must not require transaction
revisions, census digests, replacement-plan hashes, rollback decisions,
draining, runtime adoption, or semantic recovery orchestration.

Provide one idempotent `agent-browser shutdown` command that promptly closes
Agent Browser-owned browsers, stops Agent Browser units and timers, stops owned
presentation containers, releases Agent Browser leases and runtime ownership,
and leaves profile data present but unowned. Coordination metadata may be
reported as residue; it cannot veto an operator-requested shutdown.

Make a clean first install and the restarted runtime useful, not merely
nominally healthy. In the default trusted single-user mode, an ordinary client
must be able to name a profile, self-identify, and obtain a ready durable
`/remote-view/<handoff-id>` without generating identity hashes, capabilities,
repair plans, route identifiers, display identifiers, or recovery tokens.
Multi-user concurrency and adversarial identity are deferred until this
single-user journey is operational.

The ordinary trusted single-user path is recovery-first. A valid configured
profile must not be denied merely because retained session, browser, owner, or
prior-boot identity records collide. Preserve and report those records for
diagnosis, select the requested valid profile explicitly, and continue through
a fresh or requalified connection. Reject only an invalid profile request or a
case where the product truly cannot choose a concrete profile. Collision
telemetry must help improve reconciliation without turning Agent Browser into
a denial-only machine.

Replace the ordinary-path lease-management design rather than continuing to
repair each failure signature independently. The SoyLei Website failure is one
reproducer in a larger issue corpus, not the design target. A new Browser
Session Manager owns the simple profile, browser, session, tab-attribution,
heartbeat, and cleanup lifecycle inside the extracted Service Model crate.
Legacy Lease Authority, principal, owner-generation, sealed-recovery, and
boot-bound coordination modules remain available for historical readback and
deferred hardened modes, but they are not consulted as authority or fallback
by the default trusted single-user path.

## Current State

The installed recovery on 2026-09-17 required assisted transaction resume,
manual quarantine of a token-only socket, manual repair of ingress fallback
metadata, direct viewer authentication repair, finalize, and generation GC.
The accepted runtime eventually converged, but the process reproduced issues
#181 and #183 and did not meet unattended-install expectations. Subsequent
browser recovery proved that the named profile and Chrome CDP endpoint could be
healthy while remote view still failed. Prior-boot presentation inventory,
orphaned route and display ownership, and profile-identity proof rejected the
ordinary request. The supported browser-reattach path then terminated the
runtime host and timed out. This is the open #195 acceptance boundary.

Current `origin/main` defaults workstation installation to the preserve-mode
hot transaction. Its full-shutdown alternative still requires a dry-run plan,
a caller-supplied SHA-256 digest, supervisor-takeover census, exact selected
process identity, and the absence of active drains and transactions. Those
preconditions make the terminal operator action subordinate to stale
coordination state.

P211 was admitted from `origin/main@fb616aee`. Issues #181, #183, and #195 are
the primary outcome items. Issues #189 and #190 are required remote-view
regression subcases under #195, not additional implementation lanes. P207
remains the primary writer for overlapping CLI help,
README, agent skill, service docs, and generated service contracts until it
integrates; P211 will rebase before editing those shared documentation
surfaces. P205 completed as Plan 0216 and merged its service-model extraction
into `main`; the extracted Service Model and Lease Authority crates now own the
pure cold-shutdown state transitions, while the CLI retains repository and
effect adapters.

Graphiti discovery was healthy but returned no source-backed prior decision
for a simple cold upgrade. Plan 0116 and the current hot-upgrade implementation
are advisory history, not constraints on this replacement contract.

Checkpoint `b0ea9cc8` establishes the provider-free cold-shutdown controller.
Its external interface has six fixed phases and accepts no transaction,
admission, census, rollback, hash, or token input. It always executes ownership
release and final verification even when an earlier effect fails. Four focused
tests prove fixed ordering, continuation after failure, foreign-process
preservation, and owned-residue failure. Focused tests, workspace Clippy with
warnings denied, and formatting pass. Platform effect adapters and the public
command remain the next slice.

Checkpoint `b36bf40b` removes the prior daemon-shutdown exception that detached
a live browser when owner-authority metadata was stale. Daemon termination now
always selects browser close. Its focused stale-owner regression, the four
controller tests, strict workspace Clippy, and formatting pass.

P211 merged `origin/main@a3848e16` at checkpoint `0e28e44c` after Plan 0216
landed. The only source conflict was the stale-owner daemon-shutdown regression;
the resolution retains P211's terminal close behavior while using the extracted
Service Model types. Focused validation for
`shutdown_closes_browser_when_owner_authority_is_stale` passes. P207 remains
open in pull request #184, so P211 still excludes its help and documentation
surfaces.

Checkpoint `6b04975b` adds the first post-extraction shutdown adapter seam.
Lease Authority can now atomically fence and release every active resource
claim while retaining events and fencing high-water marks. Its runtime-owner
kernel can remove all current owners and principal bindings while terminalizing
retained lifecycle history. Service Model joins those operations with session,
viewer-lease, and pending-acquisition release without deleting profile records
or profile paths. The CLI repository adapter commits that pure transition under
the canonical Service State lock. Two focused Lease Authority tests, one
Service Model test, one CLI repository test, formatting, and diff hygiene pass.
Process, user-unit, container, transient-metadata, verification, and public
command adapters remain in Slice 2.

Checkpoint `766cde6b` completes the source-only Slice 2 platform adapter and
public routing boundary. `agent-browser shutdown` now enters the fixed
six-phase controller without accepting transaction, admission, census, digest,
rollback, or target-selection input. The live adapter uses only exact
Service-State browser identities, authenticated daemon lanes, the fixed
workstation user-unit set, and the three fixed Agent Browser Guacamole
container names. Every daemon and subprocess wait consumes the controller's
phase deadline; terminal verification reloads Service State and independently
checks recorded browsers, installed units, running containers, runtime owners,
and active resource claims. Stale close-command errors do not veto a shutdown
whose exact process postcondition is already terminal. Two provider-free
adapter tests prove phase mapping, escalation receipt propagation, and
continuation through ownership release and verification after an effect
failure. All 12 focused shutdown tests, all 118 Lease Authority tests, the
Lease Authority architecture guard, formatting, strict workspace Clippy, and
diff hygiene pass. This checkpoint was pushed without running the shutdown
command or mutating an installed runtime. P207-controlled help and
documentation, cold-install routing, joined restart acceptance, and the
remote-view slices remain open.

Checkpoint `34c6a0e3` adds a black-box CLI integration fixture for the public
`agent-browser shutdown --json` route. The fixture runs the real candidate
binary twice against one disposable home, workstation root, runtime directory,
and socket directory. Its `PATH` contains only fake `docker` and `systemctl`
commands, so it cannot reach host services or containers. Both runs return the
same successful six-phase receipt with zero owned residue and no upgrade
transaction fields. The only subprocess observations are the three fixed
Agent Browser Guacamole container names in the effect and verification phases;
no user unit is inspected when no owned unit is installed. The focused process
fixture, formatting, and diff hygiene pass. Shutdown fixtures for populated,
stale, failed-upgrade, and interrupted states remain open with cold install,
restart, remote view, documentation, and separately authorized installed
acceptance.

Checkpoint `839f8cf8` extends the same black-box boundary with a populated
retained-profile case. The public command changes an exclusive retained
session to `released`, reports zero runtime-owner and active-lease residue,
preserves the named profile record and a physical marker inside its profile
directory, then succeeds without changes on replay. Both disposable process
fixtures pass with formatting and diff hygiene. Active protected authority
claims, interrupted phases, and stale upgrade-sidecar cases remain to be added
before the shutdown acceptance row is complete.

Checkpoint `83e23eb2` addresses the broader valid-profile refusal represented
by `existing_session_profile_identity_inconsistent`, observed most recently by
the SoyLei Website workflow. In trusted single-user shared-local mode, a valid
explicitly requested profile now wins over contradictory retained session and
browser history. The selector returns `ExplicitProfile`, which prevents the
wrong retained browser from qualifying for reuse and sends the request through
the fresh or requalified profile path. Conflicting records remain available
for diagnosis. Registered-capability callers and requests without the
shared-local self-declared identity contract retain their stricter behavior.
The exact regression was red before the change and green afterward; six
existing-session tests, nine shared-local tests, formatting, strict workspace
Clippy, and diff hygiene pass. End-to-end connection proof and a typed
collision-observation surface remain open.

Checkpoint `0bebb42c` establishes the independent provider-free Browser Session
Manager and Browser Profile Catalog seams. The manager admits exact-profile
requests without consulting Lease Authority, principals, owner generations,
boot identity, or legacy Service State. Thirteen focused tests prove one
browser per exact profile, Alice and Bob sharing that browser through separate
named sessions, repeated-command heartbeat refresh, the same session name
opening a different exact profile without an identity-conflict denial,
expiration before a replacement epoch, last-session browser closure, bounded
unresponsive-browser replacement without command replay, bootstrap-tab
adoption, navigation without tab growth, explicit-only tab creation,
most-recent remaining-tab selection, and session-owned tab cleanup on a shared
browser. Five catalog tests prove tolerant field-level profile import despite
malformed unrelated legacy state and malformed sibling profiles. Focused tests,
package Clippy with warnings denied, workspace formatting, and diff hygiene
pass. Filesystem persistence, Service hosting, concrete PID and CDP adapters,
disposable allocation and cleanup, terminal tab history, and CLI routing remain
open.

The checkpoint used three bounded workers after the primary froze the model
boundary. `/root/session_seam_inventory` identified reusable browser process,
CDP, and atomic-store leaf mechanisms while confirming that legacy session and
action-runtime records remain lease-shaped. `/root/display_routing_inventory`
located static route, display-owner, focus, and dashboard projection seams and
confirmed that no least-crowded selector or third route exists yet.
`/root/profile_catalog_inventory` implemented only the catalog importer and its
five tests after the manager gate passed. The primary owned the manager
interface, integrated the disjoint catalog patch, and ran the combined gates.

Checkpoint `d510a054` completes the provider-free persistence and disposable
lifecycle portion of Slice 4. Browser Session State and the Browser Profile
Catalog now use private atomic JSON files that are separate from legacy
`state.json`; the first catalog load imports legacy profile fields once, after
which the independent catalog is authoritative. Repeated replacement uses the
repository's Unix rename and Windows replace-existing patterns. Live tab
removal appends terminal tab metadata, and completed
navigation records retain profile, session, browser, tab, target, URL, and
timestamp attribution. Serialized state resumes the same healthy session and
browser after a simulated Service restart. Disposable intent allocates one
managed profile per named session and policy, reuses it for that session,
isolates another named session, closes its final browser, and deletes only the
recorded managed directory through the reaper after the policy-configured
delay. Exact-profile intent rejects a disposable definition. Seventeen manager
tests, five catalog tests, two CLI store tests, workspace formatting, strict
workspace Clippy, and diff hygiene pass. Service hosting, concrete browser and
filesystem effects, CLI routing, and process-restart fixtures remain open.

Checkpoint `833da2a5` adds the concrete BrowserManager effect adapter without
switching any public route or launching Chrome. One serialized worker owns all
manager instances and maps launch, recorded-PID plus CDP liveness, exact close,
bootstrap adoption, session-initial tab creation, explicit tab creation, and
target close to existing BrowserManager primitives. Disposable filesystem
effects create a private direct child of the configured absolute root and
delete only that recorded child. The multi-session tab contract now handles
the real Chrome bootstrap constraint: Alice adopts the unowned bootstrap; Bob
gets exactly one session-initial tab when no unattributed target remains; and
closing Alice's only tab first provisions Bob's required survivor tab before
physical close. Repeated commands still reuse the session-current tab. Eighteen
manager tests, the focused adapter filesystem test, workspace formatting,
strict workspace Clippy, and diff hygiene pass. The adapter is not yet hosted
by the Service and restart-time CDP reattachment remains open.

Checkpoint `578abe9a` hosts the manager lazily inside the existing runtime-host
daemon and routes the first ordinary public operation through it. An explicit
`--session <name> open <url>` now selects the independent manager when runtime
host admission is enabled. An explicitly selected named profile is exact;
omitting the profile selects the configured disposable policy. The host loads
the independent catalog and state, injects configurable five-minute session
and disposable-cleanup defaults, persists every successful transition, and
serializes browser effects through one BrowserManager worker. Navigation
acquires or reuses the session-current tab, switches the exact Chrome target,
performs the navigation, and records immutable attribution history. An
explicitly named `close` ends only that manager session and leaves a shared
browser alive while another session remains. Name-only close succeeds only
when it identifies one active session; otherwise it returns a typed ambiguity
instead of guessing. Provider-free restart and close, three public-routing,
thirteen CLI host/store/runtime, eighteen manager, and five catalog tests pass
with workspace formatting, diff hygiene, and strict Clippy. No live browser or
installed runtime was used. Concrete process-restart CDP reattachment remains
open because the new BrowserManager worker does not yet adopt a recorded live
endpoint after its own process restarts.

Checkpoint `c3ce1b97` closes the concrete restart gap. A graceful runtime-worker
shutdown now relinquishes its launched Chrome process instead of treating
Service lifetime as browser lifetime. The replacement worker first verifies
the persisted PID, reconnects to the recorded browser-wide CDP endpoint,
rediscovers current targets, and requires a responsive CDP command before
reusing the browser. Final session close sends `Browser.close` through the
reattached manager and requires the recorded PID to disappear within five
seconds. If a live recorded process cannot be requalified through its recorded
endpoint, the worker reports it as nonresponsive but will not claim it was
closed or launch a competing profile browser after an unproven close. A real
ignored acceptance test launched Chrome against a disposable profile,
relinquished it with the first worker, reattached from persisted identity in a
second worker, and closed the exact process. The focused restart test, fourteen
CLI browser-session tests, formatting, strict workspace Clippy, diff hygiene,
and a fresh residue readback pass; the readback found no remaining fixture
browser process.

Checkpoint `8c9b181c` implements the first simple presentation calculation and
joins it to browser launch. The provider-free selector accepts only configured
desktop routes and current live-browser display placements. It excludes `:0`,
excludes routes whose startup health probe failed, chooses the smallest live
browser count, breaks ties by configured route order, and rejects duplicate
route or display identities. Retained route leases, prior allocations, and
display-owner history are not inputs. The Service host derives candidates from
the existing RDP route inventory and display-socket probe. The manager stores
the chosen route, display, and selection count on the independent browser
record; the BrowserManager adapter launches headed on that exact display.
Four selector tests and a nineteenth manager lifecycle test prove `:10` then
`:11` assignment for distinct profile browsers, deterministic ties, local and
unhealthy exclusion, and duplicate rejection. Fourteen CLI browser-session
tests, formatting, strict workspace Clippy, and diff hygiene also pass.
Dashboard projection, focus-on-selection, and durable remote-view handoff are
still open.

Checkpoint `26743c65` makes independent Browser Session State visible without
making legacy state authoritative. Service Status now adds a top-level
`browserSessionState` snapshot from the live host or independent atomic store.
An unreadable snapshot adds `browserSessionStateError` while preserving the
successful legacy status response. The dashboard converts those browser,
session, tab, desktop, and navigation records into its existing read model and
merges by stable ID. One concrete browser process produces one live parent
tile; named sessions and tabs contribute attribution and counts rather than
extra browser tiles. Latest navigation history supplies the tab URL. Two Rust
status-join tests, the dashboard workspace-node smoke, selected-context,
workspace-view, and navigator tests, the production dashboard build,
formatting, strict workspace Clippy, and diff hygiene pass. The tile has its
configured route and display identity, but focus-on-selection and a ready
durable handoff remain open.

Checkpoint `a4c08010` routes the explicit tab-growth operations through the
same public named-session boundary. `--session <name> tab new [url]` creates
exactly one explicitly attributed tab, optionally navigates that tab, and
records its history. `--session <name> tab close` closes only the session's
current tab and selects its most-recent remaining tab through the existing
manager rule. Indexed tab close stays on the legacy path because the simple
prototype does not reinterpret a legacy numeric index as attributed identity.
The host restart fixture now proves open, explicit new tab, navigation,
current-tab close, and final session close in sequence. The focused host and
public-routing tests, formatting, strict workspace Clippy, and diff hygiene
pass.

The 2026-09-18 design interview generalized that repair into the first Browser
Session Manager prototype. One Service process owns browser-session decisions;
`--session` is the caller-visible attribution identity rather than a daemon
routing alias or authenticated principal. Multiple named sessions may share
one browser for one exact named profile. Exact-profile requests compare
profile identity, while disposable requests compare allocation policy and
reuse only the current compatible allocation for that named session. Activity
refreshes a configurable five-minute idle heartbeat. A session is live only
while that heartbeat is fresh, its browser PID exists, and its CDP endpoint
responds. The last ended session closes its browser, and a proactive reaper
removes eligible disposable profile data. Browser Session State and the Browser
Profile Catalog are independent of legacy Service State so retained lease or
owner records cannot veto current work.

The same checkpoint simplified presentation. Configured Guacamole routes map
directly to virtual desktops: Guac A to `:10`, Guac B to `:11`, and a future
third route to `:12`. A remotely viewable browser launches on the healthy
virtual desktop with the fewest live browsers; `:0` is reserved for explicitly
local on-screen browsers. Dashboard active tiles represent concrete browser
processes. Selecting a tile chooses the viewer for that browser's desktop and
raises its primary browser window. Tree expansion from browser to sessions to
tabs remains a later UX improvement.

The tab follow-up closes the accidental-growth loophole. Each session has at
most one current tab. Its first tab-requiring command adopts an unattributed
Chrome bootstrap tab when one is available; otherwise it creates exactly one
session-initial tab. Ordinary `open` and navigation commands then reuse the
current tab; only an explicit new-tab operation increases the tab count within
that session. Closing the current tab selects the session's most recently used
remaining tab, or leaves the session tabless until another command needs one.
Session termination closes its live tabs, while historical tab metadata
remains queryable. The prototype records tab-count and cleanup telemetry but
does not add a tab cap or silent least-recently-used eviction.

The 2026-09-18 execution readback synchronized the P211 branch and its draft
pull request #191 at `81699174` before implementation. P207 remains open in
pull request #184 and retains help and documentation custody. P211 therefore
keeps this packet in new lane-owned modules until those dependencies integrate.
Graphiti was healthy but returned only older remote-view history. Current
source, focused tests, and Git evidence remain authoritative.

## Frozen Interface Packet

The shutdown module exposes one operation that receives one effects adapter.
The controller owns the fixed phase order and short deadline for each phase;
the adapter receives only the selected phase and its deadline. The operation
accepts no transaction, drain, census, revision, rollback, hash, capability,
or repair-token input. Every phase returns a typed step receipt. Verification
returns independently attributable Agent Browser-owned residue plus separately
reported foreign-process observations. A failed phase does not suppress later
ownership release, transient cleanup, or final verification.

The cold-install module exposes one operation with the fixed sequence `stop`,
`replace`, `start`, and `readiness`. Stop must return a successful shutdown
receipt before replacement begins. Any replace, start, or readiness failure
executes one bounded rollback phase and reports both the original failure and
rollback integrity. A successful result requires all four phases and a ready
probe from the newly selected generation. Legacy hot-upgrade transaction state
is not an input to this interface.

The Browser Session Manager is a deep module inside the extracted Service
Model crate. Its interface admits a named session against an exact-profile or
disposable-profile intent, records activity and tab attribution, closes a
session, reconciles current PID and CDP observations, and runs one reaper tick.
The existing user-scoped Service process hosts it. CLI commands automatically
start that Service once when needed and never fall back to legacy lease
authority. Existing browser launch, termination, PID and CDP observation,
atomic persistence, and event adapters remain mechanisms behind the new seam.

The manager persists `browser-session-state.v1`, containing only browser
instances, named sessions, tab attribution, heartbeat timestamps, and terminal
history. A separate `browser-profile-catalog.v1` contains named profile
definitions and disposable allocation policy. On first startup, an absent
catalog is populated by a tolerant field-level import of legacy profile
definitions; unrelated legacy fields are neither validated nor migrated. The
new catalog is authoritative after that import. Legacy Service State receives
best-effort diagnostic projections only. Failure to mirror a projection emits
a warning and backlog item but cannot reject a browser command.

An active browser is one concrete process attached to one profile. Multiple
named sessions may share it and route requests through its one CDP endpoint.
Each session owns attribution, not permanent tab control. Every live tab has at
least one active session reference and one session-current pointer is retained
when that session has tabs. A session adopts an available unattributed Chrome
bootstrap tab, or receives one session-initial tab when none remains. Ordinary
navigation reuses the current tab; explicit new-tab is the only additional
tab-creation command within a session. Closing the current tab selects the most
recently used remaining tab without eagerly creating a replacement. Session
termination closes its live tabs and preserves only historical metadata. Tab
sharing, tab handoff, authenticated principals, identity-restricted profiles,
desktop-exclusive leases, per-session timeout overrides, tab caps, and silent
tab eviction are deferred. `session_idle_timeout_ms` defaults to `300000`;
activity is a heartbeat. Explicit close, heartbeat expiry, browser termination,
or failed bounded liveness recovery ends a session. The browser closes when its
last session ends. Interrupted commands are never replayed automatically.

Presentation uses a configured display-to-viewer map rather than dynamic route
and display leases. Remote-view browsers are assigned to the healthy configured
virtual desktop with the fewest live browsers, using deterministic route order
for ties. Local-screen browsers use `:0`, which is excluded from remote-view
selection until a viewer is deliberately configured for it. Each browser has
one primary window with many tabs. Dashboard active tiles derive only from
manager-owned browser instances; tile selection opens the viewer for the
browser's desktop and serializes a raise-and-maximize request. A normal
same-site authentication redirect retains the durable handoff. Raw provider,
route-binding, embed, dashboard, or health URLs are never the operator result.

This packet declares the following disjoint write scopes before fan-out:

- Primary: `cli/src/workstation_shutdown.rs`,
  `cli/src/workstation_cold_install.rs`, and the smallest command adapter in
  `cli/src/main.rs`; `cli/src/workstation_install.rs` remains deferred until
  P205 reconciliation.
- Provider-free fixture worker:
  `cli/src/workstation_shutdown_contract_tests.rs` and
  `cli/src/workstation_cold_install_contract_tests.rs` only. The primary owns
  module declarations and production logic.
- Remote-view implementation worker:
  `cli/src/native/presentation_requalification.rs` only, including its local
  unit tests. The primary owns module registration and later integration with
  presentation inventory and route selection.
- The former P205 Service State exclusion ended when Plan 0216 merged. P211 may
  now add narrowly scoped cold-shutdown transitions to the extracted crates,
  while P207-controlled help and documentation files remain excluded.

The first packet completed its bounded fan-out. Fixture worker
`/root/p211_fixture_contracts` added only the two declared contract-test files.
Remote-view worker `/root/p211_presentation_requalification` added only the
declared requalification module and its local tests. The primary reconciled the
boot-epoch type with the repository's string identity, registered the modules,
and implemented the shutdown-deadline and cold-install controllers. The
shutdown deadline tracer failed first because `deadline_ms` was absent, then
passed after the controller owned and normalized every deadline. The
cold-install tracer failed first because its module was absent, then all four
contract cases passed after the fixed sequence and rollback result landed.

Focused primary validation is green for five shutdown-controller tests, four
shutdown-contract tests, four cold-install contract tests, and eight
presentation-requalification tests. Strict workspace Clippy with warnings
denied, workspace formatting, and `git diff --check` pass. The validation
selector also named broad workstation and Guacamole lanes, but those remain
deferred because this checkpoint changes no platform adapter, installer route,
embedded asset, or live runtime. Production shutdown routing, platform effects,
cold installer integration, profile release, and remote-view integration remain
open.

## Consolidated Batch

1. Add a deep cold-shutdown module with one external operation and injected
   process, unit, container, and state adapters for provider-free tests.
2. Route `agent-browser shutdown` through that module. Use fixed short phase
   deadlines, polite termination followed by exact owned-process escalation,
   and idempotent postcondition checks.
3. On shutdown, close every Agent Browser-owned browser, release every runtime
   owner and lease, mark retained profile definitions unowned, preserve profile
   directories, and remove only Agent Browser-owned transient coordination
   artifacts.
4. Make ordinary workstation apply and reviewed-candidate install use cold
   shutdown, payload replacement, unit activation, and a bounded readiness
   probe. Do not create a new hot-upgrade transaction.
5. Park hot-upgrade mutation commands as legacy recovery/readback surfaces.
   They may inspect or close old transactions but are not selected by the
   default install or upgrade command.
6. Add the Browser Session Manager, independent Browser Session State, and
   independent Browser Profile Catalog. The ordinary path must not call Lease
   Authority, principal, owner-generation, sealed-recovery, or boot-identity
   modules as authority or fallback.
7. Import only valid legacy profile definitions when the new catalog is absent.
   Keep all other legacy state as diagnostic history. Make Service State
   projection best effort and nonblocking.
8. Route all ordinary CLI requests through the one user-scoped Service. Share
   one live browser per exact named profile across named sessions, use a
   session-scoped allocation for disposable profile intent, refresh heartbeats
   from activity, and proactively reap expired sessions, sessionless browsers,
   and exactly proven disposable profile directories.
9. Give each session one current tab. Adopt an unattributed Chrome bootstrap
   tab or create one session-initial tab, reuse the current tab for ordinary
   navigation, create further tabs only on explicit request, select the most
   recently used remaining tab after close, and close live tabs when their
   session ends without deleting historical metadata.
10. Replace dynamic presentation leasing on the ordinary path with configured
   display-to-viewer lookup. Assign remotely viewable browsers to the least
   crowded healthy virtual desktop, reserve `:0` for explicit local-screen
   work, and make a dashboard tile select the desktop viewer and raise the
   browser's primary window.
11. Return success only when `operatorVisible.state=ready` and the durable
   opaque handoff resolves. Doctor, service status, capacity, preflight, and
   checkout must agree on the same effective readiness result.
12. Update CLI help, README, agent skill, documentation site, and inline docs to
   present the one-command workflow and remove hot-upgrade ceremony from the
   ordinary path.

## Scope And Effect Boundary

Expected implementation writes are limited to the CLI command router and help,
a focused cold-shutdown/cold-install module, the smallest required adapters in
the workstation installer, a new Browser Session Manager and Browser Profile
Catalog in the extracted Service Model crate, their CLI persistence and effect
adapters, native runtime, profile acquisition, display selection, dashboard
workspace projection, and remote-view handoff paths, provider-free
fixtures, README, the Agent Browser skill, installation and remote-view
documentation, this plan, and P211's active-lane projection.

The command may affect only Agent Browser-owned user units, timers, browsers,
runtime hosts, dashboard processes, MCP processes, presentation processes and
containers, leases, ownership records, and transient runtime metadata. It must
not kill unrelated browsers or containers. Unknown foreign processes are
reported and left alone; stale Agent Browser bookkeeping does not block the
owned shutdown set.

This plan does not authorize a production install, production shutdown,
credential or provider use, release, broad process cleanup, deletion of profile
data, or mutation of unrelated worktrees. Installed validation remains a
separate user-directed effect after source qualification. The product contract
is intentionally single trusted user; this plan does not weaken identity or
ownership behavior for a future multi-user mode.

The first prototype explicitly defers authenticated principals,
identity-restricted profiles, adversarial or distributed fencing, tab sharing
and handoff, desktop-exclusive leases, per-session idle-timeout overrides,
tab caps and silent eviction, multi-window management,
browser-to-session-to-tab tree expansion, a viewer for `:0`, and implementation
of the third Guacamole route. Extension points may be retained, but no deferred
concept may appear in the prototype's ordinary interface or live authority
calculation.

## Delivery Sequence And Budget

- Optimization target: balanced wall-clock and token efficiency.
- Active-agent concurrency: at most 3 total, including the primary. Delegation
  is one level deep; workers cannot spawn workers.
- Critical path: frozen lifecycle and trusted-single-user contracts, public
  shutdown, profile release, Browser Session Manager prototype, profile-catalog
  import, cold installer routing, simple display selection, ordinary
  remote-view open, installed acceptance, documentation.
- Slice 1: freeze the shutdown and cold-upgrade result contracts and add red
  provider-free fixtures for broken transaction state, active drain, stale
  metadata, partial retry, and a clean machine.
- Slice 2: implement `agent-browser shutdown` and the bounded owned-target
  adapters.
- Slice 3: route workstation and reviewed-candidate apply through
  stop-replace-start while leaving legacy hot transaction inspection intact.
- Slice 4: implement the provider-free Browser Session Manager and Profile
  Catalog interfaces first. Prove two named sessions sharing one named-profile
  browser, session-scoped disposable allocation, heartbeat expiration,
  last-session browser closure, service restart recovery, nonresponsive-browser
  recovery without command replay, legacy-state noninterference, and exact
  disposable-profile garbage collection.
- Slice 5: host the manager in the existing user-scoped Service, route ordinary
  CLI commands through it, project browser-only dashboard tiles, implement
  least-crowded virtual-desktop selection and static display-to-viewer lookup,
  and add durable remote-view fixtures for #195, including #189 and #190.
- Slice 6: join the cold-install and Browser Session Manager paths in one provider-free
  fresh-install and reboot acceptance fixture.
- Slice 7: synchronize all required documentation and run changed-surface
  validation once against the consolidated candidate.
- Slice 8, separately authorized after source qualification: run one installed
  clean-state journey and one replacement-upgrade journey through a ready
  remote-view handoff.
- Maximum work-unit attempts: 3 per slice.
- Maximum review and rework cycles: 1.
- Maximum consecutive hardening checkpoints: 2.
- Reassess after two checkpoints or 30 active minutes without outcome progress.
- Overall effort ceiling: 360 active minutes through a provider-free qualified
  candidate. Installed-runtime validation is excluded until separately
  directed.
- Next outcome artifact: a concrete Browser Session Manager effect adapter and
  Service host fixture proving PID and CDP liveness, launch and close, tab
  operations, disposable directory custody, and atomic state commit without
  consulting legacy Lease Authority state.

## Worker Assignments

The P211 lane owner retains architecture, source custody, Git transitions,
shared-contract decisions, runtime effects, finding disposition, integration,
and final acceptance. The primary uses the strongest available tier for
consequential architecture and integration, currently `gpt-6-astra` at high
reasoning. Deterministic repository and test tools remain the first choice.

The Browser Session Manager prototype began as a serialized primary-owned
packet. After its first five provider-free interface tests passed, the primary
delegated only the disjoint tolerant profile-catalog importer. Read-only seam
inventories ran in parallel. None of the workers received Git authority,
runtime effects, or overlapping source custody.

The earlier shutdown packet used these completed disjoint assignments:

| Worker | Initial route | Exact scope | Return and stop condition |
| --- | --- | --- | --- |
| Provider-free fixture worker | `gpt-5.6-luna`, medium | Only primary-declared test modules for shutdown, clean install, interruption/replay, profile release, restart, and the joined end-to-end fixture; no production logic or module declarations | Return a patch, commands, and exact failing invariant within 30 active minutes or after one failed implementation attempt; stop if production edits or a contract change are required |
| Remote-view implementation worker | `gpt-5.6-terra`, medium | One primary-declared post-boot requalification and route-admission module plus its unit tests; no installer, shutdown, docs, Git, or live-runtime writes | Return a patch and focused validation within 45 active minutes; stop after two cross-lane clarifications, one failed approach, or any need to change the frozen interface |

The primary implements the shutdown and cold-install critical path while those
workers run. At the source join, the primary inspects and integrates returned
diffs without repeating accepted investigation, runs focused tests, and allows
at most one repair handback per worker. If interface churn makes either lane
coordination-heavy, cancel that worker and absorb the work into the critical
path.

After P207 integrates, all workers pause while the primary refreshes the
worktree inventory, rebases, and reconciles shared documentation surfaces. One
freed slot may then run a documentation-parity
worker on `gpt-5.6-luna` at low reasoning, limited to `cli/src/output.rs`,
`README.md`, `skills/agent-browser/SKILL.md`, and the relevant installation and
remote-view MDX pages. It gets 25 active minutes and one correction pass.

After the candidate is frozen and deterministic validation finishes, one freed
slot runs a fresh read-only review on `gpt-5.6-sol` at high reasoning for at
most 20 active minutes. The packet contains the exact candidate and base,
acceptance table, changed files, validation receipts, effect exclusions, and
stable finding IDs. The reviewer may return no findings and cannot broaden
scope or claim acceptance. No worker receives an auxiliary worktree,
independent branch, commit authority, production effect, or nested delegation.

P205 completed the Service Model extraction and its integrated result is the
starting seam for this packet. P207 remains primary writer for its current CLI
help, README, skill, service documentation, and generated-client changes. P211
owns the new Browser Session Manager, Browser Profile Catalog, independent
state codec, CLI and process adapters, cold-shutdown module,
workstation-install routing, display selection, dashboard projection,
remote-view joining logic, and their tests; it will rebase after P207 before
touching overlapping documentation surfaces.

## Evidence And Exit

| Requirement | Acceptance evidence | Current state |
| --- | --- | --- |
| One-command shutdown | `agent-browser shutdown` fixture returns success from healthy, drained, failed-upgrade, and partial-prior-run inputs | public route and empty-workstation idempotence fixture green; populated, failed-upgrade, partial-prior-run, and installed acceptance pending |
| Bounded completion | injected-clock tests prove fixed phase deadlines and exact escalation without production-scale sleeps | fixed phase deadlines, bounded daemon/command waits, and provider-free escalation receipt mapping are green; injected-clock platform timeout fixture pending |
| Complete owned shutdown | receipt proves owned units, timers, browsers, runtime hosts, dashboard, MCP, and owned containers are stopped | exact browser/daemon, fixed user-unit, fixed container, state-release, metadata, and verification adapters implemented; installed residue proof pending |
| Profiles become unowned | fixture proves profile data remains while runtime owners and leases are released | authority kernels and repository fixture green; public process fixture releases the retained session and preserves the profile record and physical data; active protected-claim process coverage remains |
| Metadata cannot veto | the controller interface accepts no coordination inputs and the fixed-sequence test passes | controller and platform adapter green; installed stale-metadata acceptance pending |
| Cold replacement | workstation and reviewed-candidate apply execute stop, replace, start, and readiness in that order | not implemented |
| Clean restart | post-start fixture proves one selected generation, one runtime host, one dashboard, and clients can make a fresh service request | independent host serialization, provider-free restart reuse, and concrete disposable-Chrome worker reattachment green; full daemon-process and client fixture pending |
| Independent profile catalog | first startup imports only legacy profile definitions into `browser-profile-catalog.v1`; malformed or contradictory legacy lease state cannot block lookup | tolerant field-level import and independent atomic first-startup persistence green; Service startup joining pending |
| Shared browser sessions | Alice and Bob use one named-profile browser through independent named sessions; activity refreshes each heartbeat and ending either session preserves the other | manager, independent persistence, concrete adapter, lazy Service host, and public named-session navigation and close routing green; hosted Chrome fixture pending |
| Disposable lifecycle | one named session reuses its compatible disposable allocation; another session receives another allocation; the final session closes the browser and the reaper removes only an exactly proven managed disposable directory | provider-free allocation, reuse, isolation, final close, configurable-delay reaping, exact recorded deletion, filesystem adapter, and default host policy green; hosted Chrome fixture pending |
| Bounded tab lifecycle | ordinary navigation reuses one session-current tab; first use adopts an unattributed bootstrap or creates one session-initial tab; explicit new-tab is the only further growth path within that session; close selects the most recently used remainder; session end removes live tabs | provider-free lifecycle, concrete adapter, hosted restart fixture, and public named-session new/current-close routing green; hosted Chrome tab fixture pending |
| Current liveness | active requires a fresh heartbeat, existing recorded PID, and responsive CDP; bounded recovery ends dead sessions without replaying the interrupted command | heartbeat, bounded-recovery model, recorded-PID plus CDP checks, Service hosting, and concrete restart reattachment green; full daemon-process fixture pending |
| Legacy containment | ordinary session, browser, profile, tab, and display decisions remain unchanged when legacy lease, principal, owner, generation, and recovery records are contradictory | catalog import ignores unrelated malformed legacy state and manager has no legacy-authority input; persistence and display paths pending |
| Trusted single-user profile | `--session` alone supplies attribution; a named profile reuses one healthy matching browser and requires no principal, hash, capability, sealed plan, or repair token | selector collision regressions, independent manager proof, and ordinary named-session navigation and close routing green; hosted Chrome fixture and remaining ordinary commands pending |
| Simple display selection | remote-view browsers use the least-crowded healthy configured virtual desktop; `:0` remains explicit local-screen only; retained route allocations do not participate | provider-free selection, persisted browser assignment, existing-inventory adapter, and exact-display launch wiring green; hosted remote-view fixture pending |
| Dashboard browser identity | each active tile represents one concrete browser and selects its desktop viewer while raising its primary window | independent status projection and browser-parent dashboard tile identity green; viewer selection and focus effect pending |
| Login handoff | #190 regression proves a normal same-site authentication redirect leaves a usable durable handoff or typed authentication-required state | not implemented |
| Ready remote view | an ordinary route-free open returns `operatorVisible.state=ready` and an opaque `/remote-view/<handoff-id>`; doctor, status, capacity, preflight, and checkout agree | not implemented |
| Simple interface | default operator path requires no preflight digest, transaction ID, revision, census code, rollback choice, or manual recovery command | not implemented |
| Legacy hot-upgrade containment | hot transaction mutation is not reachable from the default install or upgrade path | routing change pending |
| Documentation parity | CLI help, README, Agent Browser skill, docs site, and inline comments describe the same workflow | not implemented |

Exit requires all rows green against one frozen source candidate. Provider-free
tests must include idempotent replay, a shutdown interrupted after each phase,
stale PID metadata, exact foreign-process preservation, owned container
cleanup, browser close escalation, ownership release, and restart readiness.
It must also include two named sessions sharing one browser, exact and
disposable profile intent, bootstrap or session-initial tab acquisition,
repeated navigation without tab growth, explicit tab creation, current-tab
close selection, session tab
cleanup, heartbeat expiry, join-versus-final-close serialization, Service
restart recovery, unresponsive PID and CDP recovery, nonblocking legacy
projection failure, exact disposable cleanup, deterministic least-crowded
display assignment, a protected URL redirecting to login, and runtime-host
survival during reattach.

The final installed acceptance is one ordinary user journey against one frozen
candidate: clean install; bounded start and passing doctor; named-profile open
using `--session alice`; a second `--session bob` request sharing the same
browser; ready durable remote view; one-command shutdown with profiles
unowned; replacement install; bounded restart; and a second ready remote view
from the same named profile. It fails if the operator must choose a
route, desktop, or display; generate or copy a hash, capability, token, code,
or sealed plan; edit Service State; run a repair command; or interpret raw
Guacamole state.

## Stop Condition

Stop before production installation or shutdown, profile-data deletion,
unscoped process or container termination, live provider or credential use,
release, or mutation of another lane's checkout. Stop and reconcile if P205 or
P207 publishes an overlapping interface change before P211's corresponding
adapter or documentation work begins. Stop the affected worker after its stated
bound and return partial evidence; do not silently increase concurrency,
reasoning tier, retry count, or the cumulative plan budget.
