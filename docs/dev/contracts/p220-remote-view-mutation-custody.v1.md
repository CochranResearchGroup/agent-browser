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
unknown provider outcome. Qualified retirement completion is still pending, so
this checkpoint deliberately has no fence deletion or automatic reopening.

Recovery, foreground and cleanup owner admission are not yet coordinated with
this fence. Caller-supplied complete inventories do not prove current external
owner absence or stop a concurrent new owner. Actual runtime wiring and a
fresh physical process census remain required before release acceptance.
