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
