# LinkedIn command-state latency repair

Product lane: PL-BUGFIX
Status: SOURCE_VALIDATED (not installed)
Branch: fix/linkedin-command-state
Base: 48174c3148d1c64e0a91d283e4ee7018aa9dda06

## Outcome and custody

Reduce repeated history deserialization in ordinary retained-task mode inspection, while preserving fresh task/browser/process comparisons. Reuse the clean Contacts recovery worktree; no additional checkout. The canonical authentication lane remains separate. Primary writer for this repair is the current Contacts latency session; surfaces are browser_session_store.rs and its mode-inspection tests. No schema, history retention, public API, runtime installation or source publication change. Governance packet lookup returned POLICY_MANIFEST_UNAVAILABLE; canonical policies are the fallback.

## Feedback and acceptance

Three installed trivial reads took14.67–18.89seconds versus direct exact-target read0.37–0.47seconds. The runtime session JSON is approximately8MB, mostly historical operations and navigation. Current mode inspection reloads full state before and after physical observation, and once inside the concrete inspector. Use the real store regression to require zero aggregate-history deserializations while rejecting browser replacement and allowing peer membership changes. RED before production edit, then existing mode inspection guards and focused store tests. Preserve complete history in storage; use a current SQL projection rather than a stale cache. No measured installed speedup until candidate qualification.

## Validation economics

Use existing target artifacts, cargo-safe and Cargo Signal CLI fallback, one compiler job with enforced6GiB maximum because the host is under CPU/swap pressure,6GiB claim and16GiB reserve. No hosted CI. Retain red/green artifacts in private owner runtime. Source tests are not runtime latency acceptance.

## Implementation and verification

Mode inspection now uses a fresh SQL projection that excludes navigation history, task operations, handoff records and terminal session/tab history from this read only. It retains all other fields and the same initial observation and fresh post-observation task/browser comparisons. Source storage, history retention, original-operation reconciliation and process validation are unchanged. SQLite still parses the stored JSON; this patch reduces Rust deserialization, not every database read or the whole command delay.

RED: the real process-replacement regression rejected the replacement but then failed the new assertion, aggregate reads2 expected0. GREEN: `scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml --bin agent-browser browser_session_store::tests -- --nocapture` passes20tests. Required local `scripts/ci/cargo-safe.sh clippy --workspace --manifest-path Cargo.toml -- -D warnings` passes with no diagnostics; `cargo fmt --all --manifest-path Cargo.toml -- --check` and `git diff --check` pass. No hosted CI. Each compilation used Cargo Signal CLI fallback and the existing target directory. The original8GiB-claim attempt was cancelled and its exact scope stopped; the tracked bounded replacement completed RED, then GREEN. No active build is left behind.

A read-only current-state rehearsal returned approximately8.0MB for the full document versus223KB for the mode projection, about97%fewer bytes returned. Every non-excluded field matched. This is a payload-size result; service-time uncertainty does not qualify a precise subsecond SQL speedup. The deployed MCP command baseline remains14.7–18.9seconds versus direct exact-target read0.37–0.47seconds. No installed candidate timing or full semantic-agent benchmark is claimed.

Custody: local source and plan changes prepared for authorized PR publication on fix/linkedin-command-state in the reused Contacts recovery checkout; prior branch is retained. The source-validation packet made no installation or additional runtime restart. The operator subsequently authorized PR review, merge, deployment and testing. Review/integration and installed performance comparison remain next. Private receipts and recoverable diff are retained under the owner runtime three-additional-pr102 directory. Broader command-stage attribution and normalized task-scoped storage remain open rather than inferred from this narrow change.
