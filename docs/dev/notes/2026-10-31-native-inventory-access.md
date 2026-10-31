# Native browser inventory access guidance

Product lane: PL-BUGFIX
Disposition: source-qualified repair; installation remains separate
Work item: #506
Source checkpoint: `0d91ab569e2a20adc2e4bebf16a9bc2e4f38d4fd`
Baseline: `41b71202dce08b48bafdcaedca7d8ceb12d63c6b`

## Outcome

A missing Remote View session was rendered as generic unavailable inventory and
an instruction to refresh. The native inventory hook now distinguishes HTTP 401
sign-in, HTTP 403 denial, an unexpected HTML page, stale observations, service
errors and transport failures. A transport failure suggests checking sign-in or
the connection without claiming an expired session. Dashboard and Remote View
sign-in remain separate. The specific guidance appears above the workspace even
with the rail collapsed and suppresses conflicting empty-rail refresh advice.
An old observation timer cannot overwrite the newer failure. Requested selection
is preserved and successful inventory reads clear the failure.

## Local evidence

The mounted regression first failed on the missing 401 sign-in message, then
passed after repair. `pnpm test:native-browser-windows` covers 401, 403, HTML,
network failure, 503, collapsed-rail visibility, preserved selection and recovery.
`pnpm test:native-window-inventory` passes. `pnpm build:dashboard`,
`pnpm --dir docs build`, `pnpm test:policy-wiring`,
`pnpm test:remote-view-handoff-docs`, `pnpm test:documentation-links -- --base origin/main`
and patch hygiene pass. Both builds use the local Linux/WSL toolchain; dashboard
static-export rewrite warnings remain. No hosted CI was dispatched.

Rust format passes through `scripts/ci/cargo-safe.sh fmt --all --manifest-path Cargo.toml -- --check`.
Formatting used a 1 GiB admission claim because it does not compile. Strict
workspace Clippy runs through the same wrapper, four jobs and an 8 GiB admission
claim to preserve the default 16 GiB reserve under current host pressure. The
shared target directory is reused. The first cache-enabled Clippy attempt stalled
in sccache startup; its exact task-owned systemd scope was stopped, then Clippy
was rerun with the documented cache opt-out. Strict workspace Clippy passed with `-D warnings` in 2m 21s. The full command was:

```bash
AGENT_BROWSER_CARGO_CACHE=off AGENT_BROWSER_CARGO_BUILD_JOBS=4 AGENT_BROWSER_CARGO_MEMORY_CLAIM_KIB=8388608 CARGO_TARGET_DIR=/home/ecochran76/workspace.local/agent-browser/cli/target scripts/ci/cargo-safe.sh clippy --workspace --manifest-path Cargo.toml -- -D warnings
```

Review custody: [draft PR508](https://github.com/CochranResearchGroup/agent-browser/pull/508).

## Retained baseline failure and boundaries

`pnpm test:compact-native-workspace` fails at line 170 with
`No dead mutation controls before their implementation` (one existing menu item,
expected zero). The same failure reproduces in the canonical checkout with the
unchanged dashboard source at the baseline. It is not repaired by this scope.
This is passing targeted validation, not a claim that every dashboard test passes.

No production installation, authentication-policy change, browser launch recovery,
profile reset, ingress change or shared skill publication occurred. The running
dashboard does not yet contain this repair. Work moved to its own checkout after
a separate local-validation session took custody of the canonical checkout;
that session's files and the Auth-retention checkout were preserved.
