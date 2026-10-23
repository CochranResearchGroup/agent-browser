# Exact MCP task routing: installed qualification

Product lane: PL-BUGFIX
Work item: [#365](https://github.com/CochranResearchGroup/agent-browser/issues/365)
Plan: [P272](../plans/0272-2026-10-22-exact-mcp-task-routing.md)
Disposition: installed runtime accepted; configured-client reconnection pending

## Source and candidate

[PR370](https://github.com/CochranResearchGroup/agent-browser/pull/370) merged as
`16450473a3d117035e0601ec38df3bec751ed9a3`. Documentation conflicts preserved the
installer convergence work and renamed this plan from P271 to P272. Reconciled
MCP138, connection50 and the exact-task regression pass, with strict workspace
Clippy, format, policy/docs checks and the public no-launch smoke.

The single production-shaped build uses that clean source, release optimization,
fat LTO and one codegen unit. Candidate
`candidate-0cedc188f61c7e63-ff0b22c3bb9399e7` has executable SHA256
`ff0b22c3bb9399e7aa43bfe8f6cb1a609ddc2dffa061001f9c551c9353bcde75`.
Independent checks bind the binary, support manifest, executable-input closure,
candidate manifest and seal. The exact-task public stdio fixture passes against
these exact sealed bytes; the preceding installed generation reproduced the
duplicate-name failure in the same isolated fixture.

## Isolated qualification

The separate `issue365` development namespace uses seven pinned ports and the
same sealed binary. Its own CLI `install doctor` passes. Three supported
launch/current-URL/close cycles pass with production unchanged and independent
profile-process absence checks. The aggregate development helper doctor retains
one failure: the temporary namespace has no local Traefik hostname route. Binary,
unit, port, authority and other checks pass. Tests use pinned local ports; no
shared ingress configuration was changed to satisfy this unrelated route check.

All three temporary units were disabled and stopped. A fresh OS readback finds
no process for that namespace or its smoke profiles. The candidate remains
retained for provenance; default development and peer installations are preserved.

## Production transaction

Another authorized lane upgraded production while this candidate waited for
build capacity. The fresh accepted baseline was
`0.28.0-217c9ef34bd3-8383e0b3a792`; no competing committing transaction was active.
The preserve preview used existing native Remote View and protected authority,
with no host provisioning and no mutation. Its generic provisioning summary is
not an infrastructure-readiness diagnosis.

Preserve transaction `upgrade-1f7e320b-c56d-431a-b096-0c5f92edc14a` reached
`accepted`, `ready=true` and `complete=true`. Production selects
`0.28.0-ff0b22c3bb93-e5a1b9f945e7`. The installed executable and live host PID
`1660641` both match the sealed SHA256. Both pre-existing live browser roots
retained their exact PID/start identities. Installed doctor passes; MCP inventory
is ready with zero stale clients. Public Agent Browser ingress returns HTTP 200;
that alone is not authenticated application or viewer proof.

## Original workflow and remaining gates

A fresh installed stdio MCP connection opens the original persistent AuraCall
inspection task, snapshots its exact returned task ID, continues in the same
browser, and closes only that task. All four operations pass. The task, browser
record and client process are absent after close, no process matches its managed
browser profile path, and persistent profile data is retained.

The current page redirects to `https://auth.ecochran.dyndns.org/`. Authenticated
AuraCall desktop content and physical viewer input are not established. No login
or credentials were submitted. AuraCall #240 retains its separate acceptance gate.

The older configured Codex MCP connection reports `Transport closed` following
client retirement during the earlier upgrade and still reports that after this
install. Fresh stdio acceptance proves the installed server workflow, not recovery
of that host-owned connection. No supported client-reconnect control is exposed
in this session. Reconnect the Agent Browser MCP client in Codex, then verify its
configured workflow before closing #365. No additional server reset, profile
deletion, provider replacement or broad process termination was performed.

Raw RPC, census, process and transaction evidence stays in private user-scoped
artifacts. This note contains curated non-secret identities and bounded results.
