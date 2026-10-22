# MCP handoff origin diagnosis

Issue: CochranResearchGroup/agent-browser#358. Product lane: PL-BUGFIX.

## Installed failure and controlled differential

Installed generation `0.28.0-9c8edd62a16f-3ef0c5e95c2e` creates a disposable task
successfully. Its existing MCP connection rejects handoff with
`remote_view_public_origin_missing`. A fresh stdio MCP invocation against the
same durable task, removing only `AGENT_BROWSER_PUBLIC_ORIGIN`, reproduces that
exact rejection. With the current reviewed Codex MCP origin configuration, the
same executable and task return a durable opaque link and
`operatorVisible.state=ready`. The old connection retains startup environment;
this is not evidence of missing Native ownership or container boot failure.
No public origin was guessed or borrowed from the native provider.

## Bootstrap defect

The production `agent_env::load_env_file` parser filters all keys through
`KNOWN_VARS`. It accepts the separate native viewer origin but omits the MCP
service origin. A standard dotenv setting therefore cannot supply the service
origin. Add the missing key, retaining process-environment precedence and the
handoff adapter's existing HTTPS validation. A loader regression covers an
unset process origin and an explicitly supplied process override.

## Live workflow acceptance

The returned opaque link passed existing production Authelia authentication,
connected Native Desktop16 over WebSocket at1472x960, and delivered a paced
keyboard marker to the synthetic input. Independent addressed-task evaluation
read the exact value. Reopening the identical opaque link reconnected and
retained task content. This is a newly issued post-boot link, not a claim about
an unavailable pre-boot locator. Combined evidence closes Remote View316:
https://github.com/CochranResearchGroup/remote-view/issues/316#issuecomment-6344218833.

Supported task closure removed its browser. An OS birth-qualified census proves
the recorded managed browser root exited and all eight observer process
identities exited/nonrunning. The shared observer daemon remains borrowed.
Private synthetic receipts and harness stay outside Git. No production binary
replacement, shared service restart, profile reset or foreign cleanup occurred.

## Validation

The first Cargo invocation stalled in its own sccache compiler-version probe;
only that owned cgroup was stopped, then caching was disabled. The first test
setup referenced a dependency unavailable to the CLI crate and failed compile;
it is not behavioral red evidence. The corrected fixture uses existing standard
library and UUID utilities. The corrected loader regression then failed with `Err(NotPresent)` versus the configured HTTPS origin. The allowlist repair is one added recognized variable; all four dotenv tests pass, including explicit environment precedence. Strict workspace Clippy, formatting, policy wiring, links and handoff guidance pass.


An isolated real MCP startup check uses disposable HOME/runtime directories and
an origin supplied only through AGENT_BROWSER_ENV_FILE. The installed baseline
rejects remote_view_public_origin_missing; the candidate advances through origin
validation to the expected missing task database error. No browser or provider
is used by this startup check. Candidate executable SHA256:
`7685bdb8fd54d732cd1eb369216d1aec63499ef99730c3a78ce14a4d8f8e94d2`.
The first harness assertion expected a differently named database error; the
assertion now matches the actual missing-store boundary. This is bootstrap proof,
not a positive candidate native-viewer workflow.

An attempted candidate workflow against the shared live host refused a stale
daemon session before browser creation. The same stable task key was used for
readback; no recovery, replacement or ownership bypass followed. No candidate
process remains. The installed configured-origin workflow above is the positive
native acceptance evidence; production retains its installed executable.
Full Rust regression, docs-site build, production deployment and in-place reload
of the existing Codex MCP connection were not performed. Reconnecting a client
is required to replace its cached startup environment.

## Review

Standards: the existing configuration bootstrap owns this variable; no parallel
configuration source, dependency or runtime owner was introduced. Spec: missing
process origin loads from the selected dotenv file; explicit process origin wins;
HTTPS origin validation and task ownership remain at their existing seams.
No blocking findings in this bounded change.


## Production deployment | 2026-10-22

Operator explicitly requested merge/deploy. PR360 merged at
`fa9a03253fe200f923ccafbd363a3107ff879e9a` after preserving P265 hosted workflow
help and lane records during integration. The production-shaped release built
once in25m24s and sealed as
`candidate-acef7f51e9e9c3e5-76cec5b73183681c`.
Binary SHA256:
`76cec5b73183681c53cbf2a64ff5c73f71d1fc6131121379940f9d61e6499b9d`.

Isolated `org360` services all executed the sealed bytes. Three disposable
browser open/read/close/residue cycles passed with production unchanged.
Development doctor passed every check except the unpublished local hostname
route. Both namespace front/backend listeners independently returned the exact
sealed executable and development environment. No shared ingress route was
changed for that headless qualification. The isolated real MCP startup check
also read a dotenv-only origin and advanced through origin validation.

The candidate's preserve-mode workstation preview and sealed-artifact validation
passed. Supported transaction `upgrade-a50a580c-1f99-421b-8c24-810167f89a9b`
reached accepted/ready with admission reopened. Selected production generation:
`0.28.0-76cec5b73183-79b66b79b467`. All three production services execute the
sealed binary; production install doctor passes. This is a local deployment,
not a formal release.

The reviewed service origin from existing Codex configuration was added to the
standard dotenv file, preserving its prior settings and a private backup.
A fresh installed MCP connection with no process origin and no explicit dotenv
file opened a disposable task and returned a ready opaque HTTPS handoff.
Production Authelia authentication, Native Desktop16 WebSocket1472x960 pixels,
paced keyboard marker with independent task readback and identical opaque-link
reconnect all passed. The marker remained after reconnect.

Both original browser roots and all68native desktop process identities survived
adoption. All17browser,143profile,38session,40tab and243prior handoff IDs remained
present. Protected hashes were unchanged across apply. The test's additional
closed handoff remains attributable. Supported fresh-client task closure and
birth-qualified observer cleanup left zero of23recorded test processes running;
final root count returned to two. Qualification services were stopped/disabled.
All14canonical workflow skill files match their installed shared copies; three
safe updates retained private backups and additional installed files.

The existing conversation still holds its old MCP process. Its close operation
refused the stale default lane; a fresh installed MCP closed the exact same task.
Codex Wake self-refresh configuration was unavailable because shared scope lacks
host caller approval. No shared reload was attempted. Fresh-client installed
behavior is qualified; in-place refresh of this cached conversation is not.
Private receipts: `public-origin-deploy-20261022` under user-scoped Remote View
state. Foreign dirty worktrees and unintegrated topic fixes were preserved.
