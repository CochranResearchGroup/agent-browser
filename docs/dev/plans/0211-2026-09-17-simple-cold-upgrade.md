# Plan 0211 | Simple Install, Upgrade, And Remote View

Date: 2026-09-17

Plan version: 36

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

Checkpoint `a9d3887e` activates the Browser Session Manager's existing bounded
cleanup policy instead of requiring an operator command to trigger it. Each
Unix and Windows runtime-host server starts one lazy reaper on the existing
Service reconciliation cadence, defaulting to 30 seconds. The task does not
initialize an unused Browser Session host, skips missed ticks rather than
bursting, and is aborted and joined when the server exits. Once the host has
been used, each tick expires idle sessions, closes newly sessionless browsers,
and applies the manager's exact disposable-profile eligibility rules. The
focused interval test, all 17 executed CLI browser-session tests, formatting,
strict workspace Clippy, and diff hygiene pass; the disposable real-Chrome
restart test remains intentionally ignored in this provider-free lane.

Checkpoint `f5a0c287` connects manager-owned browser tiles to the established
dashboard focus request without changing the public action contract. The
runtime host claims `view_focus` only when its stable browser ID exists in
independent Browser Session State; foreign and legacy IDs continue through the
legacy handler. The manager proves current PID and CDP liveness, selects the
attributed target when supplied, refreshes that session heartbeat, and uses
BrowserManager's existing bring-to-front and native maximize operation. A
missing target is rejected rather than redirected to another tab. All 20
manager lifecycle tests, the host restart/focus fixture, the manager-ownership
routing test, all 17 executed CLI browser-session tests, formatting, strict
workspace Clippy, and diff hygiene pass. Static viewer lookup and durable
handoff issuance remain open.

Checkpoint `4177d983` completes static viewer selection for manager-owned
browser tiles. The dashboard treats the persisted desktop `routeId` as the
configured route-pool entry identity, resolves that entry's concrete route,
and attaches the already-published Service view stream to the browser read
model. The join is read-only, deterministic, and preserves the route-pool
entry identity for diagnostics; it neither checks out a legacy lease nor
constructs a provider URL. Workspace nodes, view projection, selected-context,
and navigator tests plus the optimized dashboard build pass. The tile can now
select its desktop viewer and route focus through the manager; creation of a
ready durable opaque handoff for a newly opened manager browser remains open.

Checkpoint `95b5758f` routes both ordinary workstation apply and reviewed
candidate apply through the concrete cold controller before any prior upgrade
transaction convergence. Stop executes the public owned shutdown adapter;
replace stages and atomically selects one immutable generation; start performs
transaction-free workstation reconciliation; readiness proves the selected
generation and binary digest, then runs the final doctors on a real host.
Reviewed-candidate artifact identity is stored in the immutable generation
manifest and contributes to its generation identity without creating a hot
upgrade transaction. A black-box isolated apply succeeds with contradictory
retained transaction and admission-drain files present, emits only stop,
replace, start, and readiness, and leaves those diagnostic files untouched. A
second injected selector-commit failure executes one rollback and restores the
exact prior selector. The four controller fixtures, both reviewed-candidate
tests, both shutdown process fixtures, formatting, strict workspace Clippy,
and diff hygiene pass. A broad 181-test installer filter completed before the
final success-return refactor; the two directly affected reviewed-candidate
tests were rerun after that refactor. Filesystem replacement is not yet an
interruptible operation, so hard deadline enforcement and installed readiness
acceptance remain open.

Checkpoint `c754180f` routes ordinary tab-bound commands through the Browser
Session Manager after a simple named-session open. Each managed browser now
retains one full command state beside its BrowserManager, preserving snapshot
references and other per-session command context across separate CLI calls.
The runtime selects the session's attributed current target before each
command, refreshes the session heartbeat, and bypasses only the legacy
runtime-owner, admission-drain, manual-seeding, and profile-mismatch gates that
the independent manager replaces. A manager-owned state cannot fall back to
legacy auto-launch when its browser is unavailable. Browser and tab lifecycle
operations remain manager-owned rather than entering the generic dispatcher.
The host fixture proves exact Alice routing and Bob noncapture. A disposable
real-Chrome restart fixture proves PID/CDP reattachment, navigation, ordinary
title and snapshot calls, and exact browser close. All 19 executed focused
browser-session tests, the one opt-in real-Chrome test, formatting, strict
workspace Clippy, and diff hygiene pass. Durable opaque handoff publication
and a full daemon-process client journey remain open.

Checkpoint `ca74617e` publishes and resolves a durable opaque handoff for a
manager-owned browser without creating a route lease or consulting legacy
browser ownership. A successful managed navigation on a ready assigned static
desktop persists one stable UUID handoff, returns only the authenticated
dashboard `/remote-view/<handoff-id>` URL, and updates the desired URL while
retaining that handoff across later same-session navigation. Resolution
revalidates the live session record, heartbeat, exact PID, responsive CDP
endpoint, attributed target, static display binding, route-pool readiness, and
route readiness before raising the browser. A stale proof returns a terminal
typed unavailable response and never enters the dashboard's automatic retry
loop or falls through to legacy retained-browser adoption. The dashboard
accepts the manager-specific receipt from its
authenticated Service response while preserving the stricter legacy receipt
checks for legacy handoffs. Both manager handoff fixtures, all 19 other
executed browser-session tests, all 14 durable-handoff regressions, the route
confusion gates, dashboard durable-handoff, workspace-navigator, and inspector
action smokes, the optimized dashboard build, formatting, strict workspace
Clippy, and diff hygiene pass. A real login redirect and full
daemon-to-dashboard acceptance remain open.

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

## Second-Window Review And Remediation Freeze

The second execution window started from clean local and remote candidate
`6f8dccd112be3c10d72e75455d53fdf3c371d8f1` against
`origin/main@a3848e16f75769f31e6855a7e77f9ce9bdb8967f`. Pull request #191 remained
open and draft. Pull request #184 remained open and draft at `838b771a`, so
P207 still owns overlapping help, README, skill, generated-client, and service
documentation surfaces. CodeGraph was not initialized in this worktree and
was not initialized without operator authority. Graphiti was healthy but had
no Plan 0211-specific source-backed recall.

The operator authorized one additional execution window capped at 2,000,000
tokens. Goal-control thread `01a0b5a3-edfc-7a91-94de-e6bf2f1804e3` did not
expose a writable token-budget field, so this plan records the cap as the
manual hard ceiling. The prior window consumed 1,988,143 tokens and 11,388
seconds. The new allowance does not reset the one-review limit, retry counts,
evidence history, or the 360-active-minute provider-free ceiling. This window
can finish the provider-free source objective if the accepted remediation and
P207 documentation join fit the remaining bounds. It cannot establish the
separately authorized installed clean-install and replacement-upgrade result.

The fresh review was one parallel two-axis pass. The primary adjudicated every
candidate as follows:

| Finding | Disposition | Adjudication |
| --- | --- | --- |
| `P211-S1` | `blocking` | `presentation_requalification.rs` has no production caller and duplicates the presentation design. Delete the unused module and its isolated tests. |
| `P211-S2` | `blocking` | Public shutdown, manager behavior, and configuration are not documented on all mandatory surfaces. Perform the parity update only after P207 overlap custody is clear. |
| `P211-S3` | `needs_evidence` | RUNBOOK and the active-lane entry are stale, but both are shared surfaces currently overlapping P207 and P214. Reconcile one writer before refreshing them. |
| `P211-S4` | `nonblocking_backlog` | Duplicate controller cases and test-harness setup raise maintenance cost but do not falsify current behavior. Consolidation is outside this remediation cycle. |
| `P211-R201` | `blocking` | Shutdown and residue verification omit independent manager state, while runtime-host teardown relinquishes manager browsers. Add exact manager-process custody, terminal state transition, and replay proof. |
| `P211-R202` | `blocking` | One `DaemonState` per browser shares refs, frame state, confirmations, and request state across named sessions. Separate session command context from browser lifecycle custody. |
| `P211-R203` | `blocking` | Managed navigation preserves `success: true` when required handoff publication fails. The ordinary ready-remote-view operation must fail as a whole when its handoff is unavailable. |
| `P211-R204` | `blocking` | Managed handoff readiness currently treats retained dynamic pool and allocation fields as authority. Bind publication and resolution to the configured static viewer plus current browser, display, route, and control observations without importing legacy leases. |
| `P211-R205` | `blocking` | Handoff resolution bypasses manager serialization, does not persist heartbeat activity, and can race close or reap. Route it through the live host and its existing persistence lock. |
| `P211-R206` | `blocking` in part | Hard-coded ready/active projection from an unreconciled persisted manager record is invalid. Preserve legacy browser history as a separately classified compatibility source; reject the candidate suggestion to remove all legacy rows from the dashboard. |
| `P211-R207` | `rejected` | The exact catalog ID is the profile selector; `name` is a display label. Adding display-name alias selection would weaken the frozen exact-profile rule and create new ambiguity behavior. |
| `P211-R208` | `blocking` | Several shutdown, replacement, start, readiness, rollback, filesystem, and lock effects only report a deadline without enforcing it. Add injected bounded execution at the effect seam. |
| `P211-R209` | `rejected` | Indexed close and arbitrary tab switching were explicitly deferred. The prototype owns explicit new-tab and current-tab close only; unsupported indexed operations must not be reinterpreted. |
| `P211-R210` | `needs_evidence` | The final daemon, dashboard, shutdown, redirect, and joined install fixtures are still absent. This is the acceptance-evidence packet after source repair, not a separate defect. |

The internal `Converging` variant carrying terminal `unavailable` data and the
constant managed presentation generation are accepted as nonblocking cleanup
inside the handoff repair only if removing them simplifies the joined
interface. They do not justify a new generation or recovery subsystem.

The single remediation batch is frozen in this order:

1. Add one red public shutdown fixture for a manager-only browser, then persist
   exact process identity, close or safely preserve it, terminalize manager
   sessions, verify independent-state residue, and prove idempotent replay.
2. Add one two-session command-context regression, then split session-owned
   command state from the shared browser transport and lifecycle owner.
3. Add red handoff publication, close-race, heartbeat, and current-readiness
   fixtures, then route publication and resolution through the serialized host
   and one current configured presentation observation.
4. Add injected non-completing effect checks, then enforce the existing phase
   deadlines without creating a second transaction or rollback design.
5. Delete the unused presentation-requalification module and retain only tests
   that cross a production interface.
6. Run the missing daemon-process, dashboard handoff/control, same-site login
   redirect, shutdown-state, and joined fresh-install/restart fixtures against
   one frozen provider-free candidate.
7. After P207 custody clears, synchronize CLI help, README, Agent Browser skill,
   docs site, inline docs, RUNBOOK, and the P211 active-lane record. Run the
   changed-surface selector, required format and strict Clippy gates, focused
   contracts, and one broad provider-free lane once at the final batch.

Budget reservation for this window is 12 percent review and adjudication, 43
percent implementation and red-green fixtures, 25 percent selective and broad
validation, 12 percent documentation and P207 reconciliation, and 8 percent
publication and closeout. If accepted remediation consumes the documentation
or final-validation reserve, stop lower-value cleanup and report the exact
provider-free gate reached. No broad review reopens after this freeze;
verification is closed-world against the accepted findings and regressions
introduced by their remediation.

The version 28 remediation checkpoint implements the frozen source repairs.
Shutdown now treats independently persisted Browser Session Manager processes
as owned only when their exact process identity verifies, terminalizes the
manager state after exit proof, preserves named profile data, survives replay,
and leaves an unreferenced foreign process running in the public command
fixture. Browser command state is owned per named session while lifecycle and
transport custody remain shared per browser. Managed handoff publication now
requires one configured static viewer plus current display, route, control,
browser, session, and tab evidence; publication failure fails navigation.
Resolution runs under the manager host lock, focuses the exact target, and
persists the session heartbeat. Status reconciles process and CDP liveness
before projection, and the dashboard treats current manager state as the
active inventory when that state is available. Shutdown and cold-install
controllers detect injected deadline overruns after every effect boundary;
the measured filesystem replacement allowance is 60 seconds and the other
phase allowances remain 30 seconds or less. The unused presentation
requalification module is removed.

The external-process Chrome fixture then exposed one additional routing defect
inside the already frozen `P211-R210` evidence packet. A mutating command such
as `click` for an existing manager session entered the legacy prestart launch
path and was rejected as duplicate profile pressure before the daemon could
route it to the manager. The CLI now detects a persisted live named manager
session before prestart, sends its ordinary command directly to the shared
runtime host, and fails rather than falling back to a legacy launch if that
host is unreachable. The ignored Linux fixture crosses the compiled CLI and
runtime-host daemon with disposable Xvfb and Chrome. Alice and Bob share one
named-profile browser while retaining distinct sessions, tabs, targets, and
independent `e1` snapshot references; Alice's close preserves Bob and Bob's
final close terminates the exact browser process. The same fixture follows a
real loopback HTTP redirect from `/protected` to `/login`, then reaches
`/account` on Alice's existing tab while retaining the same opaque handoff ID
and leaving Bob unaffected. A fresh process census found no fixture browser or
daemon residue.

Focused receipts at this checkpoint are 25 executed browser-session tests with
two explicit Chrome tests excluded from the default filter, the explicit
external-process Chrome/Xvfb fixture, three public shutdown process fixtures,
231 stream tests, three CDP transport tests with compiler caching disabled,
the dashboard durable-handoff contract, and diff hygiene. The stream rerun also
replaced a stale exported-dashboard assertion: current `main` server-renders
the stable dashboard shell and `Restoring session` before client
authentication, so the route test now checks the shell plus injected section
rather than later client-only login copy. Full authenticated dashboard render
and control, joined cold-install restart use, documentation parity, broad final
validation, and separately authorized installed acceptance remain open.

The version 29 validation checkpoint freezes the remediated source candidate.
The first comprehensive runner invocation lost its controlling session during
the serial workstation compartment after recording green results for the
native action, native service, native other, and CLI core compartments. The
unfinished and supporting lanes were then rerun against the unchanged Rust
source: all 214 workstation tests, all CLI integration tests, 99 native browser
tests with two explicit Chrome tests ignored, 118 Lease Authority tests, the
complete 221-test Service Model unit lane and its integration binaries, and
the candidate, challenge-control, desktop-services, and CDP transport crates
passed. The 231-test native stream lane had already passed against the same
source. Strict workspace Clippy, formatting, the dashboard production build,
dashboard action and durable-handoff contracts, route-confusion gates,
workstation host and VM harness contracts, Guacamole and PostgreSQL fixtures,
service API and MCP parity, generated-client checks, the cold-install fixture,
and the Service collection no-launch smoke also passed. This is final
provider-free validation for the frozen remediation batch, not proof of the
still-open authenticated dashboard-control, joined install-to-use,
documentation, or installed-runtime acceptance gates.

The version 30 continuation preserves the bounded authenticated-dashboard
attempt without promoting it to acceptance. Manager handoff resolution now
returns the dashboard's required compatibility presentation generation and an
exact ready receipt binding the logical browser, target, required and observed
stream provider, and Browser Session Manager source. Generation `1` is a
compatibility sentinel, not lease, owner-generation, or monotonic authority.
The focused handoff-resolution test, dashboard workspace-node projection
smoke, formatting, strict workspace Clippy, all four nonignored workstation
shutdown integration tests, and diff hygiene pass.

Three bounded read-only workers supported this continuation without source,
Git, runtime, or acceptance authority. `/root/p211_efficiency_audit` used the
requested `gpt-5.6-luna` low route to assess the prior attempt economics;
`/root/p211_plan_audit` used `gpt-5.6-terra` medium to reconcile the plan and
lane catalog; and `/root/p211_validation_audit` used `gpt-5.6-luna` medium to
select the cheapest discriminating checks. The runtime did not expose their
effective model identities. The primary accepted the bounded state and test
evidence, retained integration and acceptance, and rejected the plan worker's
broader joined-install next step because the fresh handoff made this preserved
dashboard fixture the immediate packet.

The existing compiled-CLI, runtime-host, Xvfb, and Chrome fixture was extended
to authenticate through the real dashboard endpoint, reopen Alice's same
opaque handoff, and inspect the rendered workspace control viewport against a
static fake viewer. Its static Service State now includes the required
route-pool-entry to concrete-route mapping. The one permitted replay produced
no terminal verdict: after more than three minutes it still owned its isolated
daemon, two Chrome profiles, and Xvfb process, so the run was cancelled and
only those exact task-owned processes were stopped. No second browser replay
was started. The failed fixture directory remains at
`/tmp/agent-browser-workstation-shutdown-71248-1789766339538183572` as
ephemeral diagnostic evidence.

A new focused regression then proved the fixture's local auth server could
block teardown indefinitely after accepting an idle browser connection. That
test failed before the repair and passed after accepted connections received a
100-millisecond read timeout. This repairs evidence visibility but does not
reconstruct the hidden dashboard assertion or prove authenticated rendering
and control. Those acceptance claims remain open. One initial focused Cargo
invocation also failed before compilation in the optional compiler-cache
wrapper; the same manager test and subsequent Rust gates passed with compiler
caching disabled.

The version 31 continuation closes the remaining provider-free input variants
for the public shutdown fixture without changing production logic. A compiled
CLI case now persists an active protected profile claim, invokes
`agent-browser shutdown --json`, proves zero active claims in both the receipt
and reloaded Service State, and proves an unchanged successful replay. A second
case retains contradictory failed-upgrade transaction and admission-drain
sidecars while proving that they neither veto shutdown nor enter its receipt.
A third case represents a prior shutdown interrupted after ownership release:
the session is already released, profile data remains present, and stale
session-stream and dashboard PID metadata remain. The next public shutdown
removes only that transient metadata, preserves the profile marker, and then
replays without changes. All seven nonignored workstation shutdown integration
tests pass; the disposable Chrome/Xvfb browser journey remains intentionally
ignored in this provider-free batch. Formatting passes. Installed shutdown
acceptance remains open and no installed or shared runtime was mutated.

The version 32 continuation closes the remaining injected-clock platform
timeout evidence. The real platform-effects adapter is composed with the
controller's injected clock while its browser phase consumes the exact
controller-owned allowance. The resulting receipt attributes the browser
deadline overrun, retains the platform's changed and escalation evidence, and
still records ownership release, transient cleanup, final verification, and
zero residue. This is a deterministic no-sleep fixture; concrete daemon,
systemd, and container subprocess effects remain independently bounded by
their existing adapters.

The version 33 continuation joins a disposable fresh cold install to first
Service use without launching Chrome or mutating an installed runtime. The
fixture executes the selected installed binary, starts one runtime host and
its embedded dashboard, receives HTTP 200 from the dashboard authentication
status endpoint, completes two fresh Service status requests, and proves that
the repeated dashboard and Service calls retain the same runtime-host process
identity and stream port. The runtime host's recorded executable resolves to
the selected installed generation. The installed binary then performs a
successful cold shutdown, removes its process metadata, and terminates that
exact process. Both cold-install integration tests pass together, and a fresh
process and filesystem readback finds no fixture residue. The first draft of
the fixture correctly unwound through its shutdown guard but failed because it
expected separate dashboard PID files; the dashboard on this path is embedded
in the runtime host. The repaired assertion uses the reachable dashboard
endpoint as the logical dashboard identity. The fixture also makes the
cold-install harness remove read-only generation directories during cleanup.

The version 34 continuation closes two manager-host lifecycle gaps. The hosted
reaper now has an end-to-end provider-free fixture across the host,
persistence, manager, and concrete filesystem adapter: an ended disposable
session loses only its recorded direct-child profile directory, a foreign
sibling and marker remain, and the persisted disposable allocation is gone.
The external unresponsive-CDP path now falls back from a typed CDP attach
failure to bounded exact-process termination only when the retained process
identity opens a kernel-backed termination capability and its PID matches the
browser record. The red fixture first failed with
`browser_session_reattach_failed`; after repair, a live disposable external
process with a deterministic HTTP 503 CDP endpoint is terminated and the
adapter can complete manager retirement. A paired negative fixture proves the
same CDP failure preserves a live process when exact identity is absent. An
initial loopback-refusal draft exposed the adapter's long connection timeout
and was cancelled; its process left no residue, and the deterministic local
503 fixture replaced that tactic.

The version 35 continuation closes hosted multi-display persistence and the
remaining legacy-containment display path. A host-level fixture imports only
two valid named profile definitions from a legacy Service State whose session,
runtime-owner, and display-allocation fields are deliberately contradictory.
Two independent browser opens select the least-crowded healthy routes in
configured order, assigning `:10` and then `:11`. A restarted host reloads the
same independent browser and desktop assignments, while the legacy diagnostic
file remains semantically unchanged and never participates in selection.

The version 36 continuation closes Service startup joining for the independent
Browser Session State and Profile Catalog. `service status` remains a local
no-launch read when no daemon exists. When the selected runtime host is already
ready, the same command now joins that host instead of bypassing it, so status
is serialized with the live Browser Session Manager. The joined disposable
cold-install fixture first failed with a missing `browserSessionState`; after
the routing repair it returns `browser-session-state.v1`, persists
`browser-profile-catalog.v1` with the default disposable policy and no invented
named profiles, retains the same host identity and dashboard endpoint on a
second read, and shuts down without process or filesystem residue.

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
| One-command shutdown | `agent-browser shutdown` fixture returns success from healthy, drained, failed-upgrade, and partial-prior-run inputs | public route, empty-workstation idempotence, retained-profile release, protected-claim release, failed-upgrade independence, partial-prior-run completion, manager-only state, exact process termination, foreign-process preservation, and replay fixtures green; installed acceptance pending |
| Bounded completion | injected-clock tests prove fixed phase deadlines and exact escalation without production-scale sleeps | fixed phase deadlines, injected-clock controller and platform-adapter overruns, bounded daemon and command waits, and provider-free escalation receipt mapping are green |
| Complete owned shutdown | receipt proves owned units, timers, browsers, runtime hosts, dashboard, MCP, and owned containers are stopped | exact browser/daemon, fixed user-unit, fixed container, state-release, metadata, and verification adapters implemented; installed residue proof pending |
| Profiles become unowned | fixture proves profile data remains while runtime owners and leases are released | authority kernels and repository fixture green; public process fixtures release retained sessions and an active protected claim while preserving profile records and physical data |
| Metadata cannot veto | the controller interface accepts no coordination inputs and the fixed-sequence test passes | controller and platform adapter green; installed stale-metadata acceptance pending |
| Cold replacement | workstation and reviewed-candidate apply execute stop, replace, start, and readiness in that order | ordinary and reviewed-candidate source routes plus isolated success and rollback fixtures green; installed acceptance pending |
| Clean restart | post-start fixture proves one selected generation, one runtime host, one dashboard, and clients can make a fresh service request | joined disposable cold-install-to-first-use proves the selected generation, one reused runtime-host identity, one reachable reused dashboard endpoint, repeated fresh Service requests, and exact installed shutdown; independent host serialization, provider-free restart reuse, concrete disposable-Chrome worker reattachment, and the compiled-CLI shared-daemon title, snapshot, click, redirect, and close journey are also green |
| Independent profile catalog | first startup imports only legacy profile definitions into `browser-profile-catalog.v1`; malformed or contradictory legacy lease state cannot block lookup | tolerant field-level import, independent atomic first-startup persistence, and public status joining through an already-ready Service host are green; no-daemon status remains local and no-launch |
| Shared browser sessions | Alice and Bob use one named-profile browser through independent named sessions; activity refreshes each heartbeat and ending either session preserves the other | manager, independent persistence, concrete adapter, lazy Service host, public named-session lifecycle routing, generic current-tab command routing, and the two-session compiled-CLI/runtime-host Chrome fixture are green |
| Disposable lifecycle | one named session reuses its compatible disposable allocation; another session receives another allocation; the final session closes the browser and the reaper removes only an exactly proven managed disposable directory | provider-free allocation, reuse, isolation, final close, configurable-delay reaping, hosted exact recorded filesystem deletion with foreign-sibling preservation, default host policy, and exact hosted-Chrome final-process termination are green |
| Bounded tab lifecycle | ordinary navigation reuses one session-current tab; first use adopts an unattributed bootstrap or creates one session-initial tab; explicit new-tab is the only further growth path within that session; close selects the most recently used remainder; session end removes live tabs | provider-free lifecycle, concrete adapter, real-Chrome restart and ordinary-command fixture, public named-session new/current-close routing, and the two-session daemon-process tab and close fixture are green |
| Current liveness | active requires a fresh heartbeat, existing recorded PID, and responsive CDP; bounded recovery ends dead sessions without replaying the interrupted command | heartbeat, bounded-recovery model, recorded-PID plus CDP checks, Service hosting, concrete restart reattachment, full daemon-process command routing, and bounded exact-process recovery for a verified external unresponsive-CDP browser are green; unverified processes remain untouched |
| Legacy containment | ordinary session, browser, profile, tab, and display decisions remain unchanged when legacy lease, principal, owner, generation, and recovery records are contradictory | catalog import ignores unrelated malformed legacy state, the manager has no legacy-authority input, and hosted persistence plus multi-display selection remain independent of contradictory legacy session, owner, and display records |
| Trusted single-user profile | `--session` alone supplies attribution; a named profile reuses one healthy matching browser and requires no principal, hash, capability, sealed plan, or repair token | selector collision regressions, independent manager proof, named-session lifecycle routing, persistent generic command state, real-Chrome title and snapshot calls, and the full daemon-process command journey are green |
| Simple display selection | remote-view browsers use the least-crowded healthy configured virtual desktop; `:0` remains explicit local-screen only; retained route allocations do not participate | provider-free selection, hosted two-display least-crowded assignment, persisted browser assignment across host restart, existing-inventory adapter, exact-display launch wiring, and a hosted static-route Xvfb handoff are green |
| Dashboard browser identity | each active tile represents one concrete browser and selects its desktop viewer while raising its primary window | independent status projection, browser-parent tile identity, static desktop-viewer join, and manager-owned focus and maximize routing green |
| Login handoff | #190 regression proves a normal same-site authentication redirect leaves a usable durable handoff or typed authentication-required state | the compiled-CLI Chrome fixture follows a same-site `/protected` to `/login` redirect and retains the same ready opaque handoff through `/account`; the dashboard same-origin post-auth return contract is green; one authenticated dashboard replay ended without a terminal verdict and its teardown-hang regression is repaired, so authenticated rendering remains pending |
| Ready remote view | an ordinary route-free open returns `operatorVisible.state=ready` and an opaque `/remote-view/<handoff-id>`; doctor, status, capacity, preflight, and checkout agree | compiled ordinary opens now return ready opaque handoffs from exact static route readiness without leasing, and host resolution focuses and heartbeats the target; the manager presentation receipt is focused-test green, but the bounded dashboard replay produced no terminal verdict, so authenticated rendering, control, and doctor agreement remain pending |
| Simple interface | default operator path requires no preflight digest, transaction ID, revision, census code, rollback choice, or manual recovery command | the full compiled CLI and runtime-host journey shares one browser, drives independent commands, returns the ready handoff, and closes cleanly with none of those inputs; documentation remains pending |
| Legacy hot-upgrade containment | hot transaction mutation is not reachable from the default install or upgrade path | default apply bypasses prior transaction convergence and creates no transaction; explicit legacy inspection and recovery commands remain |
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
