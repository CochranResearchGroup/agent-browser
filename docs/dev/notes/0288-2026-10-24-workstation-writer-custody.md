# P288 writer custody TDD evidence

Product lane: PL-BUGFIX
Disposition: implementation-evidence
Owning plan or work item: P288 / CochranResearchGroup/agent-browser#414
Related lanes: P284, P283, P285

## Seams and bounded repair

The public recover-host fixture seeds retained workstation metadata naming an unrelated live process. It must remain ready and complete its selected-host transfer. A separate public installer/preview slice will prove actual executing writer exclusion. These provider-free fixtures do not qualify production or browser continuity.

Slice 1 red: `AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD=<qualified-417d-binary> TMPDIR=/tmp node scripts/smoke-runtime-host-supervisor-takeover-no-launch.js` exits1 because stale metadata produces `workstation_transaction_busy`. The archived binary was qualified for the unchanged affected source path; this is red evidence, not final-candidate qualification. The existing fixture tears down only its isolated hosts. Raw receipts stay outside Git.

The implementation binds a logical writer claim to exact process-start identity and operation ID. Short file locks cover claim read/commit only, never process convergence. A persistent inode avoids unlink/replacement races; diagnostic PID text from an unrelated process is not custody. Slice 1 green: `AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD=<owned-cli/target/ci/agent-browser> TMPDIR=/tmp node scripts/smoke-runtime-host-supervisor-takeover-no-launch.js` exits0 and completes takeover plus steady-state verification with the retained PID file. The first owned optimized CI build passes in4m33s; two unused-import warnings remain for final lint cleanup. Final validation remains pending.
