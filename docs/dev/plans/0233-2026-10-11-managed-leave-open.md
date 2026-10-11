# Preserve managed browsers on leave-open close

Product lane: PL-BUGFIX
Work item: #247
Branch: fix/managed-leave-open-247
State: COMPLETED

Three isolated public runs show that close with leave-open kills the final managed browser. Repair that concrete promised workflow through the existing managed-session owner. Leave-open retains the logical task for continuation and preserves physical profile occupancy; it must not pretend to release an externally occupied profile. Ordinary close continues to end the task and close its final browser.

Begin with a failing real managed-consumer regression. Preserve peers and enforce current publication custody before acknowledging preservation. Cover the explicit flag, profile default, repeat preservation and subsequent ordinary close. Rerun the original isolated same-PID/CDP workflow, then exact cleanup. Update help, README, agent skill, docs site and inline contract. Match Rust and documentation gates to the touched surfaces. Production publication, account/provider effects and reboot remain excluded.

P233 owns browser_session_host and leave-open documentation. P232 owns protected profile enrollment in its existing active checkout. Source changes are disjoint. Each lane owns its own plan and catalog/index entry; preserve both additions during integration. Do not prune P232's checkout.

The bounded close repair is qualified by the [receipt](../notes/0233-2026-10-11-managed-leave-open-repair.md): actual close red/green, same-PID/CDP survival, live-host continuation and explicit false shutdown, 50 consumers, 329 Service Model tests, six replacement tests, 1,114 CLI core tests, and quality/docs gates. The public probe caught and corrected CLI flag loss after the host tests passed. Automatic continuation after direct host termination remains failed under #183; this completion does not claim that broader recovery workflow or installed publication.
