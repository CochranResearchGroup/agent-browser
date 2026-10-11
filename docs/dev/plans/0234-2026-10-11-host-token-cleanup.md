# Remove owned host token at graceful exit

Product lane: PL-BUGFIX
Work item: #249
Related work: #183
Branch: fix/host-token-cleanup-249
State: COMPLETED
Baseline: 5a24570781fa81ac6085698709dab0a3134b4c21

After ordinary close and exact graceful standalone-host termination, only runtime-host.token remains. The next admitted CLI launch refuses incomplete endpoint metadata. This also blocks continuation of P233's preserved browser. Repair owner cleanup without weakening the duplicate-host guard or deleting a replacement host's token.

Use the minimized public close/stop/reopen failure and a one-variable task-token probe. Add a focused ownership regression, qualify both matching and replacement-token cases, then rerun original preserved-browser continuation. Preserve publication/custody and exact process cleanup. Match quality and tests to daemon/connection and unchanged managed consumers. Production publication, provider or credential effects, foreign cleanup and reboot remain excluded.

P234 owns daemon endpoint cleanup. P232 owns protected profile enrollment in its existing checkout; source paths are disjoint. Each owns its catalog and plan-index addition.

Scope refinement from the original continuation loop: owner token cleanup makes ordinary reopen succeed, but the first retained-task getter still fails with browser_session_runtime_browser_missing. An explicit open reattaches the same PID/CDP and makes continuation succeed. Complete this one restart workflow through the existing fenced live-browser attachment path; do not add a new recovery owner or bypass custody. P234 additionally owns ordinary command reattachment in browser_session_host. Add a cache-empty restart regression before changing that path.

The [acceptance receipt](../notes/0234-2026-10-11-host-restart-repair.md) qualifies matching-token owner cleanup and direct named-task continuation after graceful host restart. Original public failures, minimized token proof and attachment-unit red are retained. Both public workflows pass with exact cleanup; 87 browser-session tests, 12 daemon tests, 46 connection tests, 39 output tests and quality/docs gates pass. No installed publication or wider #183 completion is claimed.
