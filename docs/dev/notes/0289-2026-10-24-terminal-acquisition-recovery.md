# P289 terminal acquisition recovery

Product lane: PL-BUGFIX
Disposition: active-input
Owning plan or work item: P289 / CochranResearchGroup/agent-browser#412
Related lanes: P285, P288, P283

The production synthetic open fails before task/browser identity with `remote_view_pool_acquisition_readback_required`. A supported same-task resume fails identically. Read-only custody retains one pending pool acquisition. Resolve its original key using the provider's scoped replay algorithm and current consumer registration: it maps to an assignment now recorded as released. This disproves the earlier assumption that no replay existed when searching only the unscoped key. The pending record has no creation time; no origin is inferred.

The real adapter reconciliation replays the original key, parses the assignment, reads current inventory, then calls active-acquisition validation. That rejects an exact released response before it can retain the observed terminal outcome. Pool preparation therefore repeatedly treats the request as unresolved and cannot proceed. Missing provider identity and store-claim mismatch remain alternate failure modes; negative regressions must preserve their refusal.

Fix only pending reconciliation: validate all acquisition identity fields and exact current inventory membership while allowing the released state to be retained as terminal history. Ordinary acquire remains active-only; no released route becomes placement capacity. Current provider state and metadata stay user-scoped; no Native provider or ingress mutation is part of this repair.

The public adapter regression fails first with Response(InvalidTarget), then all387 model cases pass, including exact released outcome, seven missing/mismatched/intermediate/persistence refusals, and ordinary acquire rejection. A deeper current-provider check found a second boundary: Native currently rejects Released replay before returning any assignment. The fake released response is therefore consumer qualification only, not installed recovery. Remote View#657 / plan0198 must merge and adopt the fingerprint-preserving terminal replay producer before the actual same-task check. Retain this correction; do not describe the initial model pass as end-to-end proof.

The actual host/pool/SQLite lost-then-released restart regression passes: zero launch calls before recovery, old envelope retained with exact Released outcome, one new acquisition key, one browser launch on active capacity, and no pending acquisition afterward. The actual module filter runs95 passed/1 existing ignored in577.71s; the earlier filename-only filter ran zero cases and is not counted. This packet does not claim full CLI coverage. Native producer0198's actual HTTP close/replay regression captures assignment_not_acquirable red, then passes the minimal fingerprint-preserving terminal branch. Producer full qualification/adoption and combined installed acceptance remain open.
