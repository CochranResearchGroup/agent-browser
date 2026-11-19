# LinkedIn command-state latency repair

Product lane: PL-BUGFIX
Status: CLOSED (installed and measured)
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

## Authorized release review

Operator requested PR, review, merge, deployment and testing. Fixed code review base48174c3148d1c64e0a91d283e4ee7018aa9dda06; source checkpoint91b5a28076ec54b64ebb068d4ac1a5162b77043b. Serial independent Standards and Spec reviews of `git diff48174c314...91b5a2807` found0accepted findings each. Standards sources: AGENTS, CONTEXT, product contract, scoped state ownership, local validation, custody policy and the code-review smell baseline. Spec source: this bounded maintenance plan. The helper is a fresh read, all current owner checks remain, and omitted fields are not consumed by mode inspection. No stale cache, history mutation or schema migration. Duplication of the small private read/error shape is proportionate to keeping the general full-state reader unchanged; no blocking abstraction change is warranted. Standards0, Spec0; no accepted severity.

Passing code validation remains current because this follow-up is prose only. Production-shaped candidate assembly, preserving install and installed before/after command timings remain required; broader latency improvement is not inferred from source tests.

## Installed acceptance

PR747 merged as4613ba89c38ef46651151ec65a07f6ab55987dd5, tree-identical to reviewed source71c769c0c52ccbda406f73e77654965bbd6ecea4. Source review has zero accepted Standards findings and zero accepted Spec findings. Incoming authentication main was preserved; the publication-gate tests passed4cases after the catalog merge. The store module remained byte-identical to the20-test and strict workspace lint validation.

The maintained production-shaped builder sealed binary72d2b4d4c2985435a5c5137f8f016bfdb8d649bc4d735b742075106d4ea06af0 with opt3, fat LTO and one codegen unit. The first candidate was intentionally superseded when main advanced. A capacity-waiting replacement was stopped before compiler dispatch and retried by its exact failed operation ID with an enforced8GiB cap and16GiB host reserve. No foreign build was stopped. The final sealed build passed with no compiler diagnostics. This is an installed roadmap checkpoint, not a formal version release.

Preserving installation reached accepted at transaction upgrade-03cde78b-907f-4d4c-8adf-427862b18ede revision13, with success, ready and complete true. Standalone install doctor exits0. The actual running process digest matches the sealed binary. Production host PID1171839 became1615525. All24 browser parent PID/start-token/executable/profile identities survived on the same boot, including the original LinkedIn browser. The temporary comparison task was closed after testing. No browser reset, reboot, profile replacement, enrichment job or graph write occurred.

### Observed command timings

StableTime service UTC observations and uncertainty bound the same-page reads before and after activation. A fresh pre-install baseline used the accepted authentication generation36e5d93138b0, after compilation was finished. Both phases asserted the exact target URL, readable profile top card and absence of login/challenge. The accessibility snapshot returned useful, byte-identical text in both phases.

| Read | Before seconds | After seconds |
| --- | ---: | ---: |
| Trivial evaluation median, n=3 |12.5861|6.9553|
| Production profile extractor, n=1 |16.3327|6.5973|
| Accessibility snapshot, n=1 |15.1776|6.2513|

The observed trivial-read median fell about45percent in this paired run. Activation also restarts the host and retires older installed MCP clients, so this does not isolate a causal share for the projection alone. It is not a distribution study, full enrichment pass, UI-button test or semantic-agent benchmark. Several seconds of command overhead remain. SQLite still scans the stored JSON; further command-stage attribution remains separate work.

The initial ad-hoc census parser missed Chrome processes whose command lines were rewritten. It was corrected before installation, and the corrected24-process census is the preservation baseline. Scoped conflict validation preserves both lane records; the unrelated full-catalog YAML parse defect was reproduced in the upstream parent and not changed here.

Custody: retain the reused checkout and sealed candidate as deployment provenance, with next review2026-11-19. Source and acceptance receipts are in owner-scoped runtime storage; personal page bodies and URLs remain outside Git and project memory. Code validation is reused for this documentation-only closeout.
