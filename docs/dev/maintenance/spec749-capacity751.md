# Task operation capacity and archival

Product lane: PL-PLATFORM
Work item: #751 under #749
Status: IMPLEMENTED, pending combined Rust validation and integration
Base: b7bbc5692

## Behavior

The existing task-operation interface accepts params.operation=capacity. Omit
policy to inspect; supply all of initialCapacity, extensionIncrement and
maximumCapacity to update the addressed task. scope=defaults updates system
defaults instead. The default scope is task. Updates apply immediately without
daemon startup or service restart. Per-task overrides survive defaults changes.
The response reports effective policy/source, current capacity, active/archive
and unresolved counts. Existing tasks, including full tasks, use this path.

Defaults are initial256, extension256 and maximum4096. A synthetic ordinary
record serialized to326bytes, giving approximately83KB for256 or1.34MB for4096,
before variable evidence and process-identity sizes. This bounds the initial
working set and grants configurable headroom; it is not a latency guarantee or
a measured universal memory ceiling.

When admission reaches capacity, acknowledged completed originals move into an
indexed SQLite archive in the same immediate writer transaction before any
extension. Pending and uncertain originals stay active. All original fingerprints,
outcomes and evidence references remain stored. Original lookup and request
conflict protection include the archive. Native observation, attempt and episode
checks read complete task-scoped history, so archival does not renew their
budgets. No archive deletion or retention policy is introduced.

A capacity ceiling returns counts, no-effect status and the supported capacity
inspection/update operation. Increasing policy can continue the same retained
task. An archive publication failure aborts the transaction and dispatch; it does
not evict history. Generic host publications strip archived originals from stale
snapshots so background metadata writes cannot restore unbounded active history.

## Schema and custody boundary

The first archival transaction creates the archive table and promotes runtime
schema to6. Preexisting runtime schemas remain accepted before that operation.
A prior binary supporting only schema5 must reject a schema6 runtime instead of
silently ignoring archived duplicate-execution evidence. Preserving upgrade,
rollback and old-owner replacement must respect this boundary: rollback to an
older binary cannot continue against schema6 by changing a version marker or
removing the archive. Use an archive-aware executable or the supported complete
runtime-backup recovery process with explicit effect reconciliation. No installed
runtime was changed by this source packet.

## Evidence and remaining gate

Actual isolated installed MCP RED: task_operation with params.operation=capacity
returns Invalid params, browser_command params.operation is not a page argument.
No daemon/socket directory was created. This establishes missing public capacity
management before the new implementation. The coordinator owns the matching
public schema change.

Two focused tests were added in the existing MCP task-operation suite:

- public_task_capacity_can_change_without_transport_or_losing_pending_history
  covers live policy adjustment, reopening, defaults versus task overrides and
  pending-original preservation without transport.
- public_task_history_archives_and_originals_still_replay_after_restart covers
 260successful operations, archive lookup, original identical/conflicting
 requests, uncertain-capacity exhaustion and continued work after policy growth.

Commands queued through cargo-safe for focused Rust and format checks waited
for memory admission and were stopped before compiler dispatch. The16GiB host
reserve was preserved. No RED/GREEN Rust result is claimed. Source rustfmt and
git diff --check pass. The coordinator will run combined source validation,
including strict Clippy and the two focused tests, after integration. Archive
failure/concurrency behavior is transactionally implemented but not yet qualified
by executed Rust tests in this packet.
