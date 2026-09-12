# Plan 0171 Timestamp Integration And Install Receipt

Date: 2026-09-12

- Integration: PR 43 merged as
  `40da27f75f7ff1116d7e92e463182c8ab644783b`.
- Final CI: run `34707619741` passed Version Sync, Rust Quality, Dashboard,
  Service Client, Workstation Fixtures, comprehensive Rust tests, and no-launch
  service smokes.
- CI repair: parallel comprehensive lanes now use separate Cargo target
  directories. The first run's eleven missing-executable failures were caused
  by one lane rebuilding and unlinking the other lane's active test binary.
- Fixture repair: the frozen no-launch MCP allowlist now includes the existing
  profile repair/reset tools and profile-diagnosis resource template.
- Exact candidate: 43,910,400 bytes, SHA-256
  `2c185ec7ccd691deaf0ee59412d3d31485ab0d8ad464cc02775785fb81be621d`.
- Install: transaction `upgrade-1fcb7d57-671f-4cf9-afc6-1bcb45293e1a`, accepted
  revision 13, selected generation `0.28.0-2c185ec7ccd6-f318ad66074f`.
- Runtime: admission drain off; `steady_current`; one executable generation,
  one runtime host, one dashboard, no legacy daemon, no multiplicity issue.
- Resources: zero candidates, zero unknown cleanup obligations, zero
  transferring obligations.
- Nonblocking doctor warnings: dashboard operator journey is converging; one
  default-profile lease has unproven session authority; another worktree's
  candidate differs from the installed command.

## Tooling/security finding

The first exact integrated build failed before product compilation when
sccache's compiler probe emitted its inherited environment. The current
sanitizer removes credential-shaped names through a blacklist, but several
sensitive variable shapes do not match that list. Do not reproduce the raw
diagnostic. The supported `AGENT_BROWSER_CARGO_CACHE=off` fallback completed
the exact build in 4m25s. Replace blacklist sanitization with a minimal compiler
environment allowlist in a separately reviewed security repair.

No browser launch, navigation, profile mutation, provider call, route switch,
cleanup, or upstream pull request occurred.

The retained Graphiti closeout job
`084f242a-977a-4017-a113-6f9a5e62b1a3` exhausted its one bounded retry with a
second transport `TimeoutError` before episode creation. No duplicate memory
job was queued.
