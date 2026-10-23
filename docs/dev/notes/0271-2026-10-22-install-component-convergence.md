# Installed MCP component convergence

Plan: [P271](../plans/0271-2026-10-22-install-component-convergence.md)
Work item: CochranResearchGroup/agent-browser#367
Product lane: PL-BUGFIX
Branch: fix/install-component-convergence

## Result

The workstation installer already selects a complete sealed binary, support
assets and managed services. Persistent MCP servers were missing from its
convergence boundary, allowing old clients to reject the selected runtime as
stale. Installation now inventories and terminates older installed MCP servers
after selection and checks again before acceptance. Resume and forward-only
failure recovery use that same selected-candidate boundary. Browser/profile
transfer and forward-only transaction rules remain intact.

Ownership requires the current UID, an exact installed executable location,
an MCP command normalized with the CLI flag parser, and matching runtime-home
and installation scope. A different HOME is a separate namespace even when it
uses production executable bytes. Deleted executable inodes remain observable.
Signals bind the exact process birth to a kernel process handle. Retirement
allows five seconds for TERM and five seconds for forced-exit proof, with waits
outside physical transaction locks. Uncertain identity or stale survivors
prevent acceptance. Private transaction receipts retain planned targets,
attempt history, outcomes and the host reconnect obligation.

Doctor exposes installedMcpClients and independently fails with
installed_mcp_clients_not_current for stale or unproven inventory. Installation
reports expose the same inventory; status exposes the latest retirement receipt.
MCP hosts reconnect through the installed launcher to obtain a new stdio server.
The installer cannot recreate host-owned pipes.

## Validation

Final focused batch passed nine actual-process retirement tests, the adapted
post-commit payload test, the committed-handoff forward-recovery test, workspace
formatting and strict workspace Clippy. Cases cover current/development peer
survival, runtime-home ownership, deleted inodes, changed process birth,
receipt failure, bounded force termination, replay and retained forward recovery.

The public source-free installer fixture passed against debug source candidate
SHA-256 d6638d972d2d3c77f0e9e802c2ed3a06a1e20a9bf4f1db03cc305bdd903ec82b.
Only debug symbols were removed from a disposable copy to reduce repeated
fixture hashing. It is an unsealed test artifact, not a production candidate.
The fixture initializes a real old-generation MCP server, applies the changed
payload, proves that server exits, initializes the selected generation and
reads its workflow catalog. It then deliberately starts a pinned old server
and proves doctor identifies exactly one stale client and fails. Normal client
closure and the independently checked cleanup finish with two owned runtime
hosts retired and zero runtime-host residue.

Host provisioning, fresh-VM harness, packaged assets, PostgreSQL durability,
route synchronization and handoff guidance checks passed. Policy wiring,
documentation links and the docs build passed.

The earlier broad 219-test run recorded 217 passes and two fixture failures.
One new fixture attempted a rollback that the existing forward-only protocol
forbids; the other added a synthetic executable without sealing its directory.
The current focused regressions correct and pass both cases. The initial broad
receipt remains failed; this note does not relabel it as a comprehensive pass.
Its unaffected checks and the full public installer fixture are retained as
bounded evidence. Private command logs and machine-readable final batch receipts
remain outside Git.

## Boundary

This is source qualification for future workstation installations. No production
publication or formal release is included. Native Remote View remains a separate
owner, and no provider or account operation is part of this acceptance.
