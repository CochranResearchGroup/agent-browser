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
The caller must prove those assertions from current Agent Browser references,
recovery, foreground leases and cleanup obligations. The wire validator cannot
establish actual absence. The response must release that exact assignment and
retire the complete expected route and viewer sets. Duplicates, partial sets,
wrong identities or generations fail. Empty replay retirement is not silently
accepted as proof of a previously unknown nonempty retirement.

Provider-free tests cover all five mutation transcripts, custody-before-send,
local completed replay, unknown and partial release, failed custody/completion,
simulated restart, and cached-acquisition drift. Disposable real SQLite tests
prove restart persistence, conflicting payload rejection, immutable completion,
and one claim winner across two connections. These prove source/storage
behavior only. Network transport, actual consumer integration, complete
cleanup evidence and installed acceptance remain pending.
