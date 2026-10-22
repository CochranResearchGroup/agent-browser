# P268: attach and explicitly share an app browser

Status: CLOSED
Product lane: PL-PLATFORM
Work item: CochranResearchGroup/agent-browser#349
Branch: platform/hosted-attach-349
Owner: ecochran76

## Outcome

Attach higher-level workflow operations to a directly observed app browser without
replacing its process, profile or existing tabs. Explicit sharing keeps independent
app/task identities; unavailable capability and profile-placement alternatives are
concrete. Attachment grants no process termination or relocation authority.
Approved spec #347 governs; P265/#348 is integrated at 8d99a0c78.

## Parallel custody

Root reuses dashboard primary for #349. User explicitly requests #349/#350 in
parallel. P267/#350 uses one necessary auxiliary /tmp/agent-browser-takeover350,
branch platform/hosted-takeover-350 from the same canonical base; its subagent
owns control coordination and gates, dedicated tests and plan267. Root owns
attachment/model ownership and integration. Shared workflow/host-context/model
exports and feature docs are root-arbitrated: separate modules where possible,
merge contracts before dependent consumers. Preserve root stale-host and enrollment
resource-bounds worktrees and all retained candidates. Auxiliary returns reviewed
commits to root and is removed after clean integration; no independent live effects.

## Acceptance and effects

Use public workflow fixtures plus synthetic app browser process/profile/tab/cookie
continuity, required exact binding, unsupported CDP/capabilities, placement conflicts,
independent shared tasks and peer preservation. Validate scoped task closure/detach
without app process termination. Rust via cargo-safe, meaningful focused/broad
model/CLI tests, strict format/Clippy and all feature docs/parity. No production
installation, private accounts, held browser use, skill sharing or uncertain replay.

## Integration checkpoint

The placement/attachment regressions and joined Native run 04 qualify the source
workflow. Root integrated P267's separate reviewed commits into this branch for
one combined CLI candidate and PR closing both independent tickets. This avoids replacing a running shared test executable
or rebuilding identical provider inputs in parallel; subagent source custody
remains explicit. No additional product scope or deployment authority.

## Consumer auxiliary

Bots consumer work uses /tmp/grok-bots-hosted-349350, branch
codex-bots/agent-browser-hosted-349350 from canonical 6b4d17ce6eb4ae23ef6b40f0c5cbe8a7886445da.
It is necessary to implement both app surfaces without touching 41 registered
foreign/primary trees (22 dirty at admission). The consumer subagent owns only
additive hosted adapter/service/UI/tests/docs; root owns provider contract and
actual synthetic acceptance. No independent merge/runtime authority. Return
reviewed commits to root; remove the clean auxiliary after integration and unlink
its test-only dependency symlink without touching the installed target.

Plan identifier reconciled to P268 after canonical main allocated P266 to #358.
Earlier private validation logs retain their original p266 prefix as evidence.

## Source closeout

Integrated with the paired attachment/control outcome through provider PR361 and
consumer PR175, in provider-first order. The [source acceptance note](../notes/0268-2026-10-22-hosted-attachment-control.md)
records the model/session/control/MCP/action, strict Rust and client/docs gates,
actual Native run 04 with healthy chat, 63 marker samples and 26 WebSocket frames,
and positive absence of 14 observed app process generations before release.
Native run 03 retains its invalid cleanup history and separate correction evidence.
Native run 05 also passes synthetic cookie continuity through both task detachments,
healthy chat, real Native SDK control, 63 markers/47 WebSocket frames and all 14
observed process generations absent before exact release, with natural harness exit.
Accepted source only; no production deployment or private-account acceptance.
The wider #347 specification and unrelated P266/P269 custody remain separate.
