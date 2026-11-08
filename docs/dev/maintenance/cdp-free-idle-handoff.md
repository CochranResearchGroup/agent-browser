# CDP-free idle closure and live handoff

Product lane: PL-BUGFIX
Work item: 585
Owner: ecochran76
Branch: fix/cdp-free-idle-handoff
Base: a779f3fe9
State: ACTIVE

Close a managed CDP-free original through a retained kernel process handle and
confirm OS exit without requiring desktop window-close success. Keep PID/start/
executable identity verification and never escalate automatic close to force kill.

An explicit human handoff may supersede an unfinished automatic close only after
an exclusive existing retirement-effect guard proves no cleanup worker is active,
and fresh immutable-original process readback proves that browser is still alive.
Retain the entire attempted-close history and its unsettled physical outcome.
Atomically record the explicit live-retention decision, publish unchanged task
state and complete the original Native reservation. Future automatic effects for
that retired operation are forbidden. A concurrent worker or unavailable/changed
identity must defer. This is not a no-effect or browser-closed assertion.

The earlier generic-helper diagnosis was incomplete: the runtime has an earlier
manual-browser branch that already uses window-close and process verification.
That path can fail or leave a close prompt while the browser is alive. Missing
CDP does not prove the original attempt was undispatched. Discard that assumption.

Reproduce the existing durable handoff and test its actual host/SQLite/provider
route with a live original and pending close. Add process-close, worker exclusion,
unknown observation and retained-journal controls. Run applicable local Rust,
contract and documentation checks. Hosted CI remains disabled.

Reuse the canonical checkout, preserving unrelated CONTEXT.md changes. No new
worktree; other checkouts and candidates keep their existing custody. This lane
owns the runtime close helper, retirement model, store and explicit handoff
recovery. No installed replacement, original browser termination, state clearing
or private input in this source packet.

## Validation and review

The actual host/SQLite/Native handoff fixture reproduced the exact idle-readback
error before repair and passes after it, including worker exclusion, unavailable
identity, unchanged attempted-close history, same-link reconnect and rejection
of a delayed original writer. OS termination and positive exit plus original
identity controls also pass. Native completion now receives the validated owner
resolution, which can be explicit live retention without claiming physical close.

An optional broad browser-session run was deliberately interrupted during slow
profile-copy disposal cases. It also exposed a metadata race fixture failure.
The same failure reproduces against exact baseline Rust source a779f3fe9: both
overlapping operations can acknowledge a serializable update-then-delete. The
fixture now permits those acknowledgements while preserving its terminal deletion,
restart, unchanged browser/tab/session and no-provider-effect assertions. No
production publication-reconciliation change was made for that finding.

Final focused checks and source-bound receipts are pending. Local qualification
is Linux/WSL; hosted CI, other platforms and production installation are separate.
