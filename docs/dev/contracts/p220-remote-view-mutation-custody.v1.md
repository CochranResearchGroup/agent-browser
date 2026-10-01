# P220 Remote View Mutation Custody v1

This Agent Browser contract coordinates public revision 3 acquire, activate,
issue-view, revoke-view and release requests. It is request custody, not a
desktop reservation ledger or installed execution authority. The
[P220 plan](../plans/0220-2026-09-28-remote-view-consumer-integration.md)
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
behavior only. Network transport, actual consumer integration, complete
runtime cleanup collection, transactional release admission and installed
acceptance remain pending.


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
