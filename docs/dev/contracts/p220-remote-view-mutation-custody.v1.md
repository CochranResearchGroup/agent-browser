# P220 Remote View Mutation Custody v1

This Agent Browser contract coordinates public revision 3 acquire, activate,
issue-view, revoke-view and release requests. It is request custody, not a
desktop reservation ledger or installed execution authority. The
[P220 plan](../../history/2026-10-05-legacy-planning/plans/0220-2026-09-28-remote-view-consumer-integration.md)
owns runtime adoption and acceptance.

The service-model boundary carries the exact application envelope and a stable
client mutation key. The application selector is resource context. A client
key is not a provider operation ID. Revoke-view uses its opaque route identity;
the other mutations preserve their caller-supplied idempotency key. Reusing a
key with different application operation fields or generations is a conflict.

The CLI stores one schema-versioned record per hashed client mutation identity
inside the existing Browser Runtime SQLite `state_documents` table. A
`BEGIN IMMEDIATE` transaction commits a pending record before transport may
send the request. Competing connections receive that existing record. A
pending record contains the exact public request and no qualified outcome.
Completion compares the entire request before committing a typed public
outcome. A different completed outcome cannot overwrite terminal evidence.
Neither launch environments nor arbitrary provider response bodies enter these
records. The existing online SQLite backup boundary includes them.

Completed local replays are validated again against current consumer context.
Cached acquisition must agree with current inventory; old active results cannot
restore a now-released assignment. Window activation uses both independent
generations and exact window identity. View validity uses a clock read at the
response boundary; grant issuance is not visible-pixel or input readiness.

Any unknown transport outcome, malformed or mismatched response, or failed
completion commit retains the pending request. A repeated call returns
`MutationReadbackRequired` without sending another mutation. Provider inventory
and assignment events alone omit the client replay-key correlation needed to
prove an unknown acquisition or action. Do not infer completion from timing,
titles or target similarity. Authoritative reconciliation and any explicitly
qualified exact-key owner replay remain separate integration work; this
checkpoint adds no automatic replay or abandoned-operation deletion.

Release requires schema version 1 cleanup acknowledgement for the exact
assignment and lifecycle generation, with all four readiness assertions true.
The adapter accepts a non-clonable, non-deserializable cleanup permit from
the pure Agent Browser evidence gate rather than raw wire booleans. The permit
binds the entire release target, including route and viewer sets. The gate checks
current browser affinities, retained assignment joins, sessions and tabs. Detached
presentation does not clear live references. Independent recovery, foreground
and cleanup inventories must be explicitly complete and empty; unknown inventories
fail closed. Conflicting identity and dangling current references also fail.
The runtime collector must still establish completeness under transactional
release admission and fence new associations. The pure gate does not establish
OS process absence or eliminate a snapshot-to-send race. The response must release that exact assignment and
retire the complete expected route and viewer sets. Duplicates, partial sets,
wrong identities or generations fail. Empty replay retirement is not silently
accepted as proof of a previously unknown nonempty retirement.

Provider-free tests cover all five mutation transcripts, custody-before-send,
local completed replay, unknown and partial release, failed custody/completion,
simulated restart, and cached-acquisition drift. Disposable real SQLite tests
prove restart persistence, conflicting payload rejection, immutable completion,
and one claim winner across two connections. These prove source/storage
behavior only. Installed consumer acceptance, complete runtime cleanup collection and physical
release qualification remain pending. Native transport and demand-driven
consumer composition are described below.


## Runtime release admission integration seam

`BrowserSessionSqliteStore::mutate_session_state` already loads and saves retained
presentation changes under `BEGIN IMMEDIATE`. Ordinary
`BrowserSessionPersistence::compare_and_save_session_state` now compares the
host's last persisted aggregate with current SQLite state under `BEGIN IMMEDIATE`
before publishing its entire in-memory snapshot. A stale host receives
`browser_session_publication_conflict` and cannot overwrite newer session or
retention state. This comparison does not inspect a separate release claim, so
adding a release claim only to the presentation path would still leave ordinary
browser association publication able to race it. Launch effects also precede publication
of their resulting browser records, so a save-time check alone is insufficient.

The integration must durably admit a generation-bound launch intent before the
browser effect, include those intents in cleanup evidence, and compare release
admission in every association writer. Final release must claim custody in the
same transaction that loads current session and obligation state. It must then
retain that fence across provider transport and ambiguous outcomes. A confirmed
exact retirement can terminalize the claim; an interrupted send cannot silently
reopen the assignment. The host now compares current durable session state with its baseline before
constructing each ordinary Session Manager operation. A detected competing writer
blocks that operation. This read does not hold a launch reservation and therefore
does not close the read-to-effect race. The host must still admit durable intent
and check release claims as well as the session aggregate. Aggregate comparison now rejects
stale whole-state publication, but does not yet fence pre-publication effects
or separate release claims. These are pending requirements, not behavior provided by the
pure permit or existing mutation store.


Deterministic session admission validates expiry arithmetic before profile
resolution and validates a new session sequence before launching a browser.
Exhausted fields cannot create a process that lacks a publishable session.
Healthy existing-session reuse does not require another sequence value. These
checks do not replace durable launch intent or ambiguous-process reconciliation.


## Launch intent record

`BrowserLaunchCustodyRecord` retains an active assignment and exact profile under
an opaque intent UUID. Before observation it retains an unknown process outcome.
Observation binds one nonzero PID and browser ID to the assignment desktop and
lifecycle generation; a conflicting observation cannot replace it. Publication
requires that exact browser ID, PID, profile and desktop generation in current
Browser Session state. The record contains no launch environment or CDP endpoint.

The pure model validates source joins only. Its `published` field is qualified
by an adapter's atomic session-and-record commit, not by deserialization or a
standalone model call. The SQLite adapter commits admission under `BEGIN IMMEDIATE`, compares current
session state, excludes another unresolved intent or current browser for the same
profile, and returns Existing for exact replay. Different profiles may share an
exact assignment; contradictory unresolved assignment identity or generation is
a conflict. Stored null or malformed ledger evidence is rejected. Observation is immutable. Session
and custody publication commit together after a fresh aggregate comparison.
Release-claim fencing, process readback and runtime invocation remain pending. Unobserved and observed but
unpublished records must remain cleanup obligations across restart; absence
cannot be inferred from an empty observation. No failure-to-absent transition or
automatic retry is introduced by this checkpoint.


## Assignment release fence

`BrowserReleaseCustodyStore` admits an opaque stable release UUID against a full
retirement target. The SQLite transaction loads current session state and the
launch ledger, rejects any unpublished launch on that assignment or desktop,
validates scoped cleanup evidence, and persists the assignment fence before
returning a cleanup permit. Exact replay must preserve the entire target. A
second release identity for the same assignment or desktop conflicts.

The additive `release_fences` field defaults to empty when reading earlier v1
launch ledgers; present malformed values fail. Launch admission, launch
observation, launch publication and ordinary aggregate publication reject a
retained fence. The fence survives restart and is not cleared by timeout or
unknown provider outcome. Completion validates and retains the entire exact joined retirement outcome;
a conflicting result cannot replace it. The historical fence remains. A fresh
assignment may use the same desktop and lifecycle generation after exact
completion, because assignment return is separate from physical scale-in. The
retired assignment and lower-generation contexts remain fenced. No timeout,
fence deletion or automatic provider retry is introduced.

Recovery, foreground and cleanup owner admission are not yet coordinated with
this fence. Caller-supplied complete inventories do not prove current external
owner absence or stop a concurrent new owner. Actual runtime wiring and a
fresh physical process census remain required before release acceptance.

Completed-fence publication requires a published launch-custody record for the
new assignment with exact browser ID, PID, profile, desktop and generation.
Atomic launch publication applies the same check to the entire proposed session
aggregate after marking its own intent published in the transaction. It cannot
smuggle a browser onto an unrelated fenced desktop. These are durable identity
checks; physical process identity and complete runtime ownership remain separate
acceptance requirements.


## Fresh launch coordinator

`launch_remote_view_browser` verifies profile intent and durably admits launch
custody before any provider read or process effect. Existing intent returns
ReadbackRequired. Fresh inputs use observe, environment, observe and one final
observation immediately before process invocation. Independent lifecycle and
viewing joins must still match. The injectable process receives the consumed
private environment wrapper and exact assignment intent; it must consume that
wrapper against the supplied observation. No display is inferred from a label.

The coordinator validates returned browser identity and persists observation
before returning the launch. A provider error, viewing drift, unknown process
outcome, wrong returned identity or failed observation commit retains intent and
never authorizes an automatic retry. It does not publish the session aggregate.
The production process driver and Session Manager atomic publication wiring
remain pending; tests use synthetic transport and process effects only.


## Private process ingress

`BrowserManagerRuntime` implements the coordinator's process effect. It checks
intent/profile identity and consumes the private environment against the supplied
fresh observation before sending a worker command. The complete map reaches
Chrome process construction; DISPLAY comes from that map, while the returned
desktop affinity uses assignment desktop and lifecycle generation. Cached
display contexts are used only by the older ordinary launch path. Neither a
friendly label nor viewing generation becomes assignment identity.

Presence of private inputs limits Chrome to one launch attempt. The process
launcher suppresses persistent stderr capture for this path, and BrowserManager
redacts errors before writing its failure journal. Worker/transport errors map
to a static unknown outcome at the process boundary. Custody remains retained
for reconciliation; no automatic second launch is authorized. Sequence capacity
is checked before admission and process construction, avoiding another
deterministic post-effect publication failure.

This is source ingress, not installed adoption. The ordinary Session Manager
must still invoke the custody coordinator. Its host publication path now joins
an adapter-provided observed intent and the session aggregate atomically. Real headed success, CDP attach,
owner recovery and installed acceptance remain unqualified.


## Session host publication bridge

Before constructing an ordinary Session Manager operation, the host checks its
durable baseline and supplies that baseline to the effect adapter. This is the
expected aggregate for launch custody admission, even if the manager later
removes stale browser references in memory. An adapter may expose one observed
launch intent awaiting publication. The host asks persistence to publish that
intent and the resulting aggregate together, then advances its local baseline
and acknowledges the adapter only after transaction success.

SQLite uses the existing custody publication transaction for this path. An
unsupported persistence implementation rejects a pending intent rather than
falling back to the ordinary save. Ordinary whole-state publication also rejects
browser records matching an unpublished intent's profile or observed browser
identity. The atomic path marks its own qualified intent published within the
transaction before validating the aggregate, preventing a fallback writer from
creating published browser references without terminal custody. Other profiles
may still share the desktop. Failed publication preserves durable baseline and
intent, and does not acknowledge the adapter.

This bridge is source-qualified independently of effect admission. The default
runtime still supplies no intent; a consumer driver must implement durable
launch admission, retain ambiguous outcomes, expose its observed intent and
prevent further effects until publication or exact reconciliation. Default
methods preserve the ordinary local path and do not qualify external mode.


## Ordinary consumer effect composition

`RemoteViewSessionEffects` composes the existing browser effect owner, public
application adapter and durable launch store. It accepts transient exact active
assignment context by desktop UUID, validates generation at selection, and uses
the host's durable baseline for coordinated launch admission. It preserves the
selected descriptive label for the ordinary manager's full-value comparison
after custody has qualified actual UUID and lifecycle generation. Labels do not
provide identity or placement authority.

The store exposes unresolved launch readback. Constructor and operation entry
reject unresolved records, including unknown process outcomes after restart.
Within an operation, the adapter retains its exact intent before invoking the
coordinator. Only an observed intent is exposed for host publication; unknown
outcomes cannot become a fabricated browser record. Failure retains the intent
and blocks later effects. Host acknowledgement clears observed intent only
after atomic publication. A new intent cannot bypass an unresolved durable
claim. Other ordinary browser and profile effects remain owned by the underlying
Agent Browser implementation.

Three provider-free composition tests run actual host, Session Manager,
coordinator and real SQLite with injectable transport/process effects. They
prove healthy reuse, two profiles on one assignment, restart reuse, unknown
process outcome, competing publication and stale admission baseline. In every
failed-effect case, retry and restart do not start another process. The tests
do not launch Chrome or exercise live Remote View transport.

Default runtime construction now selects the composed owner when explicit
Remote View origin and pool settings are present. Stable tab-specific handoff
materialization remains in milestone one; automatic capacity shrink remains in
the everyday lifecycle work. Replacement
of a dead browser must reconcile and durably detach its old profile association
before fresh admission; this composition does not implement that milestone-two
recovery transition or authorize clearing an unresolved claim.


## Loopback application HTTP transport

The native transport sends the application envelope to `POST /v1/consumer`
and decodes the direct JSON response, matching Remote View source revision
`da540f22a6ff851272c5e9b91d7e3117a28bf6cd`. Construction requires an explicit
HTTP origin with a literal loopback address and a nonzero timeout. Requests
use no proxy, redirect, credential header or application retry. A joined worker
runs asynchronous HTTP so ordinary synchronous effects can call it inside an
existing Tokio runtime. Responses are bounded to four MiB. Non-200, malformed,
oversized or interrupted responses return a static unknown-outcome error;
provider response text is never surfaced as an error.

Disposable loopback-server tests exercise the actual transport and application
adapter, including wire format and redirect refusal. This is source transport
coverage. Runtime setup selects this transport for an explicitly configured application
pool. These tests do not contact an installed Remote View service.


## Runtime construction and managed commands

The ordinary host uses native HTTP when origin and pool are configured together.
Application defaults to `agent-browser`; desired desktop count defaults to one.
Construction validates settings without contacting the provider. Local runtime
construction remains the unconfigured path.

The runtime effect owner dispatches local or composed Remote View effects. Both
share the provider-neutral managed-command trait. Before executing a command on
the current manager-owned tab, the host supplies its published baseline again
because tab acquisition may have committed and acknowledged a launch. The
composed owner rejects unknown pending launches before command forwarding.

## Demand-driven pool acquisition

The ordinary manager asks its effect owner for fresh desktop candidates only
when a new browser needs placement. Healthy browser reuse and status do not
prepare capacity. The configured target count must be positive and within
Remote View's maximum reserved pool policy. Missing active assignments are
acquired through the existing custody adapter, then verified in fresh public
inventory before launch. Each successful acquisition must increase observed
capacity; a stale cached result stops the operation instead of spinning.

The SQLite owner retains one request-head pointer per application and pool,
with a UUID client key. This tracks request continuity, not desktop allocation.
Unsubmitted and pending keys are shared across connections. Pending acquisition
records block another capacity operation before provider reads, including after
restart when an assignment is visible but its client request remains unknown.
Completed request history stays immutable. Another acquisition key can follow
an exact currently active outcome, or the existing launch-custody ledger's
confirmed exact assignment retirement. Missing provider inventory is insufficient
retirement evidence. The request head and all mutation outcomes share the
existing transactional runtime database and backup boundary.

Fresh window reads validate independent lifecycle and viewing generations.
Unavailable assignments are omitted from candidates while healthy peers remain
eligible. Window-free desktops precede occupied peers when ordinary browser
counts tie. Titles and window inventories are not persisted by pool preparation.
Actual launch still refreshes the full private environment under durable process
admission. Lowering desired capacity does not release assignments; final cleanup
collection and automatic shrink remain separate lifecycle work.


## Ordinary request admission

Explicit Remote View settings select the managed owner even before a session
exists. Partial settings enter validation instead of falling back to local
launch. The configured owner admits ordinary browser commands through Session
Manager, using an exact catalog profile or the default disposable policy.
Catalog identity, name or recorded directory resolves an explicit selector;
conflicting, malformed and unknown selectors fail before admission.

Sockets and the control-plane queue borrow the same registered host. Metadata-only
selection precedes legacy scheduler profile acquisition, so selected managed
requests retain their original profile selector. The action executor keeps global
runtime admission, action policy and confirmation checks. Selected requests use
Session Manager admission instead of the legacy default-profile owner claim;
a changed route returns an explicit error without falling back to local launch.
The internal browser worker is manager-owned and does not recursively select
this host. Explicit tab/window creation and switching, and no-launch actions
such as close, still require their everyday-lifecycle join.

The focused native queue regression rejects an unknown profile through the
Session Manager, preserves the aggregate and proves policy and confirmation
precede dispatch. Existing actual-host composition tests prove first launch
with injected process effects. The actual HTTP and MCP normalizers now feed this same queue regression,
preserving profile, session, task and request identity. The positive composition
fixture uses HTTP-normalized first admission and MCP-normalized healthy reuse
with injected process effects and real SQLite. Normalizer route hints do not
rewrite ordinary navigate or snapshot commands; legacy shared-profile hints
apply to tab creation and the historical remote-view open action. Those
lifecycle actions still need the managed join. A successful first browser
request through the installed HTTP/MCP service remains unproved.

The native action executor still receives the complete command, preserving
navigation headers, wait policy and request identity. Responses add a separate
`browserSession` identity object without rewriting the action's data. Successful
ordinary navigation appends exact URL history through host publication; failed
navigation does not fabricate a successful history row. Repeated requests and
restart reuse preserve the current managed tab. Explicit tab lifecycle commands
remain separate remaining work; retained handoff resolution is described below.


## Logical tab handoff custody

Browser Runtime retains `remoteViewTabHandoffs`, keyed by opaque URL-safe IDs
and bound to logical session and tab IDs. Resolution joins the retained target,
profile and active-session membership, rather than the mutable current tab.
A deliberately closed target cannot become a replacement. The shared owner
recovers an idle-cleaned browser and focuses the addressed owned window before
returning native presentation readiness. Logical identities stay stable during
recovery; physical targets and CDP endpoints may change.

Ordinary configured commands publish their handoff through the existing SQLite
aggregate transaction and return `browserSession.handoffId` and `handoffUrl`.
A live binding reuses its opaque ID. After link expiry, a subsequent normal
request can retain a new opaque ID for the same target while preserving the old
expired record. Local unconfigured commands do not acquire these identities.

The aggregate records optional `createdAtMs` and `expiresAtMs` for backward
compatibility. The upgraded owner initializes legacy links once with the
24-hour default and compare-and-saves that migration. New links accept positive
`handoffTtlMs`; responses expose `handoffCreatedAtMs` and `handoffExpiresAtMs`.
Opening a link or restarting the owner does not renew retention.

`service_remote_view_handoff_resolve` supports `handoffOperation=inspect` to
report link status without browser effects, and `handoffOperation=extend` with
positive `ttlMs` to ensure at least that much remaining lifetime. Extension
preserves any longer expiry and commits only link metadata. Expired links and
closed bindings cannot be extended. Link expiry does not close a browser,
change native authentication, issue a viewer grant or disconnect a viewer.

Historical grant-backed presentation APIs and their ledger tests remain separate
from the native operator contract; their grants are not handoff prerequisites.

## Authenticated top-level presentation

Opening `/remote-view/<handoff-id>` automatically uses the authenticated
`/api/remote-view/<handoff-id>/presentation` endpoint. It checks link expiry
before recovery, then asks the shared browser owner to recover and focus the
retained browser. It requires owned-window readiness and joins the exact current
owned desktop generation to Remote View's native route. There is no dashboard
workspace, extra Open desktop click or Agent Browser viewer-grant ceremony.

The endpoint uses the configured public HTTPS Remote View origin and returns a
noncached 303 to its native desktop path. Remote View's existing authentication,
including production Authelia, governs that viewer. Request parameters cannot
supply another origin or desktop. Reopening the original link follows recovery
without substituting a peer desktop. Expired links return HTTP 410 with
`remote_view_handoff_expired`; other unavailable joins remain explicit failures.
Keep the opaque Agent Browser handoff as the operator bookmark while it is live.

## Managed ordinary tab creation

Configured `tab_new` joins socket and queued dispatch through the same host.
Normal service normalization leaves its catalog profile selectors for Session
Manager instead of applying the historical profile-route acquisition. Command
policy, confirmation and runtime admission remain in the action executor.
Unconfigured local tab creation retains its legacy route.

Browser Runtime retains `managedTabRequests` in the existing aggregate before
session admission or CDP creation. A record binds request key, session name,
profile and a SHA-256 command digest; it stores no raw command inputs. The optional
`params.tabRequestId` is the stable HTTP/MCP retry key; native requests without
it use their command ID. Transport-generated identity fields do not change the
digest. Completed results return with the current response ID and require the
retained logical handoff target to remain present and its link unexpired. Changed input conflicts;
closed targets do not reopen. A missing result retains a creation/readback
obligation across restart, blocking another creation or new session admission
on that profile. No current owner readback clears that obligation yet.

First admission uses the session's initial tab rather than creating a second.
Existing sessions invoke the manager's explicit tab creation once, publish the
new current tab, then dispatch optional navigation against that exact tab and
retain the result. Every tab receives its own durable handoff. A later
publication or navigation failure keeps the request pending instead of another
creation attempt. Old aggregate documents default to an empty request map.
Completed records remain retained; request-history compaction is not qualified.
These pending requests must enter final cleanup evidence before installed
release acceptance. Explicit legacy service tab handles currently return a
join-unavailable error; they are not silently bypassed or fabricated.

Provider-free SQLite composition covers first admission, distinct second tab,
exact replay with changed transport ID, conflicting input and unknown creation
across restart, including a peer session selecting the same profile. This
source join does not establish installed tab focus, native Service State tab
handle interoperability or final cleanup.

Explicit native build choices are retained in `profileExecutables` in the shared
aggregate before recovery. Launch and recovery select the addressed profile's
reviewed executable instead of inheriting the previous caller's global choice.
Observed executable proof must still match the selected build. Legacy choices
are initialized by explicit existing-client build requests, not profile inspection.

## Absent retained recovery claims

An unobserved recovery claim is retired from occupancy only after exact retained-profile process absence, unchanged published browser state and matching prior assignment are proved. The original claim remains in the durable ledger with its terminal reconciliation marker. Observed claims and release fences retain their existing checks. Recovery validates current provider environment and assignment before creating a new process claim.
