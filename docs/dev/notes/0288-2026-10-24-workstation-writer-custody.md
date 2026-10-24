# P288 writer custody TDD evidence

Product lane: PL-BUGFIX
Disposition: accepted-evidence
Owning plan or work item: P288 / CochranResearchGroup/agent-browser#414
Related lanes: P284, P283, P285

## Seams and bounded repair

The public recover-host fixture seeds retained workstation metadata naming an unrelated live process. It must remain ready and complete its selected-host transfer. A separate public installer/preview slice will prove actual executing writer exclusion. These provider-free fixtures do not qualify production or browser continuity.

Slice 1 red: `AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD=<qualified-417d-binary> TMPDIR=/tmp node scripts/smoke-runtime-host-supervisor-takeover-no-launch.js` exits1 because stale metadata produces `workstation_transaction_busy`. The archived binary was qualified for the unchanged affected source path; this is red evidence, not final-candidate qualification. The existing fixture tears down only its isolated hosts. Raw receipts stay outside Git.

The implementation binds a logical writer claim to exact process-start identity and operation ID. Short file locks cover claim read/commit only, never process convergence. A persistent inode avoids unlink/replacement races; diagnostic PID text from an unrelated process is not custody. Slice 1 green: `AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD=<owned-cli/target/ci/agent-browser> TMPDIR=/tmp node scripts/smoke-runtime-host-supervisor-takeover-no-launch.js` exits0 and completes takeover plus steady-state verification with the retained PID file. The first owned optimized CI build passes in4m33s; two unused-import warnings remain for final lint cleanup. Final validation remains pending.

Slice 2 red: the same public fixture against slice1's owned CI binary exits1 because takeover sees `workstation_transaction_busy` while full-shutdown preview returns `ready_for_full_shutdown`. After adding that code to the inherited contention set, the same command against the rebuilt binary exits0. The fixture now also proves a competing isolated apply stops before candidate staging under a real exclusive filesystem writer. An initial systemctl-barrier harness attempt exited before its intended boundary; it is retained as a harness failure and is not red product evidence.

Before final qualification, a third compatibility slice covers older PID-only writers running mutation commands without an apply flag. This remains the same custody seam and preserves serialization across the format transition.

Slice 3 red: the public CLI fixture pauses `install transactions close` at its transaction-file read, then presents its older PID-only wire record. The command has no apply flag. The fixture exits1 because takeover fails to report its executing custody. The initial fixture ID lacked the required upgrade prefix and failed before the boundary; retain it separately as a harness failure. The final compatibility change recognizes existing mutating workstation/transaction verbs. Slice 3 green: the same public fixture against the final rebuilt CI binary exits0. It observes the actual guarded transaction under both new process-instance and older PID-only records, blocks a competing apply before staging under a held filesystem writer, reports shutdown contention, then completes takeover despite unrelated retained PID diagnostics.
