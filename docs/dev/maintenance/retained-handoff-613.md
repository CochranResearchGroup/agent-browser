# Retained handoff after task closure

Product lane: PL-BUGFIX
Work item: 613
Owner: ecochran76
Branch: fix/retained-handoff-613
State: SOURCE_QUALIFIED_AWAITING_RUNTIME_ADOPTION

The original authenticated URL reports a closed task and requests a replacement
link while owner inspect reports active, unexpired and perpetual. Repeated
read-only public probes reproduce this exact mismatch. The retained target's
history says explicit_close; that classification does not identify its caller.

A valid link must remain actionable after its task closes. Navigation remains
read-only. An explicit reopen on the same URL reaches the existing owner. Missing
saved profile storage requires a disclosed, separate choice before allocating a
fresh disposable profile; named profiles are never silently substituted. Deleted
or expired links remain terminal. Pending reconstruction retains its original
request and choice across retries. Explicit assistance coordinates a new human
invitation on the resulting exact target before native viewing.

This primary uses the sixth registered checkout after the completed reconciliation
in PR614. Primary writer613 owns closed-link presentation, model resume admission,
and explicit reconstruction;607 owns mode relaunch,611 terminal reconciliation,
and596 saved login. Preserve their physical checkouts, candidates and runtime.
Integrate shared host changes serially; no production mutation or original task
recreation is authorized by source validation alone.

Validation: original public RED; focused owner and model regressions first, actual
HTML/control browser fixture, affected provider-free suites, required Rust format
and strict workspace Clippy, contracts/docs and patch checks. Keep source and
installed acceptance separate. One serial Standards/Spec review with bounded
remediation follows the implementation. No hosted CI, broad cleanup or effect
replay. Record private evidence outside Git and close this checkout through the
governed helper after integration and candidate disposition.

The bounded review found two additional reconstruction obligations: prepare the
Native observation context before clearing the pending human invitation, and
admit only the frozen request's initial navigation while the same invitation
still holds a surviving shared browser. The latter uses the existing private
cooperative attestation key with a distinct domain and exact command scope.
Another current invitation or unresolved flight refuses before tab creation.
Private-key unavailability fails closed on the held-browser path; normal
agent-owned reconstruction uses ordinary admission. Cross-platform hosted
runtime acceptance is not claimed by the Linux fixtures.

Source qualification for PR616 covers executable source commit
20b8a39cf640a10f41a93f7b69a324807c48f221. Subsequent changes only record this
validation and integration custody. The final focused replay passed; the draft
can advance to source integration. The issue remains open for runtime adoption.

| Check | Result and scope |
| --- | --- |
| `scripts/ci/rust-tests.sh --focused closed_handoff` | 3 passed: default resolve has no effects, same-link explicit choices survive restart, named profile loss refuses, shared browser and human intent survive, other invitation refuses, forged scope rejects |
| `scripts/ci/rust-tests.sh --focused handoff` | Final source: 207 passed, one pre-existing ignored; 356 seconds of test execution |
| `scripts/ci/rust-tests.sh --focused browser_host_control` | 27 passed, including idempotent return and no reassertion of human ownership on a late retry |
| `scripts/ci/rust-tests.sh --focused remote_view_presentation` | 7 passed, including actionable closed-link HTML and terminal expiry/deletion |
| `scripts/ci/cargo-safe.sh test --manifest-path Cargo.toml -p agent-browser-service-model -- --test-threads=1` | 419 passed across binaries and doctests; retained for unchanged model inputs after the CLI-only review corrections |
| `scripts/ci/cargo-safe.sh fmt --all --manifest-path Cargo.toml -- --check` | Passed |
| `scripts/ci/cargo-safe.sh clippy --workspace --manifest-path Cargo.toml -- -D warnings` | Passed on final executable source |
| `pnpm exec node scripts/test-handoff-reopen-ui.js` | Three actual-template scenarios passed; same-origin synthetic backend, narrow viewport, exact error qualification, no native dialogs, fresh owned Chrome exit readback |
| `pnpm run test:service-client` and `pnpm run test:service-api-mcp-parity` | Passed |
| `pnpm run test:policy-wiring`, `pnpm run test:remote-view-handoff-docs`, `pnpm run test:repository-tooling` | Passed |
| `node scripts/check-documentation-links.js --base origin/main --head HEAD`, `pnpm --dir docs build`, `git diff --check` | Passed |

One exact debug binary was built through `cargo-safe.sh build` and copied to a
private fixture location. Its SHA-256 is
efe5e329907f2d326ec76e2aaafd9927ce58c0a902d37f68846fa4c0b889576a.
Pinned through `AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD`, it passed all six scripts:
`smoke-service-contracts-no-launch.js`, `smoke-mcp-read-no-launch.js`,
`smoke-service-profile-sources.js`, `smoke-service-site-policy-sources.js`,
`smoke-service-incident-summary-http.js` and `smoke-service-incident-summary-mcp.js`.
Each used a disposable home. The session cleanup left six fixture daemons;
private executable, disposable-home and PID-start identity were reverified,
SIGTERM was sent through kernel PID descriptors, all six kernel exits were
observed, and a fresh exact-binary census found zero residual processes.

Preserved failed samples record the initial Native-context/control gaps and
review-remediation errors: binding before lazy tab admission, requiring an
optional hosted-task record, and unnecessarily validating an unused exception
for ordinary reconstruction. The final candidate fixes these ordering and scope
errors. Earlier passing samples never substituted for the final candidate.
A separate cache version-query invocation was stopped before tests began;
remaining tests used the documented `AGENT_BROWSER_CARGO_CACHE=off` setting.
Resource admission, memory reserves and concurrency limits stayed enabled.

This is focused provider-free Linux/WSL evidence with a debug binary, not the
comprehensive Rust suite, supported-platform CI, a production-shaped candidate,
or provider-backed/original-consumer acceptance. Hosted CI is disabled. The
selector reports broad fallback for the new standalone UI test; that test runs
directly. Installer, VM, PostgreSQL and RDP surfaces did not change and their
unrelated fixture recommendations were not executed. Installed skill sync is
reserved for a runtime that actually supports the new behavior.

Installation must preserve already-installed source d26086895fe18871e946fcf6fdfffac3a020e847,
which is not an ancestor of this checkout's main baseline 48e87e5ee3575c5100511e853c6b3f3256235816.
Coordinate that active source lane before runtime adoption. No production
replacement or original handoff recreation has occurred in this implementation.

## Authorized preserving deployment

Product lane: PL-BUGFIX. The operator requested "pr merge deploy" after source
integration. The new deployment primary is fix/handoff613-deploy at
/tmp/agent-browser-deploy613. Six existing checkouts were reconciled and retained;
this necessary primary owns composition, validation, one sealed production build,
one preserving installation and original-link readback. Other lane checkouts and
candidates remain untouched. The existing installed source d26086895 is preserved
through parent2ba47635; their difference is accepted runtime evidence only.
The only merge conflict is the active-lane catalog; preserve both intents.

The deployment window has a 120-minute overall ceiling, including build/waits,
one serial integration review and one bounded repair if a new defect appears.
Reassess after30 minutes without outcome progress. Reuse unchanged source gates;
run combined owner/control, lifecycle, model and client contracts, strict Rust
quality and docs checks. Build one production-shaped candidate through the
candidate executor, qualify its exact installer/no-launch fixtures, then use the
canonical preserve installer with fresh transaction/census readbacks. Never
replace missing original profile storage implicitly. Validate the original URL
read-only and use a separately owned disposable target for any reopen exercise.
