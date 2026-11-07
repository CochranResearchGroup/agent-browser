# Donor recovery after completed retirement

Product lane: PL-BUGFIX
Parent: P300, ticket 530
Branch: fix/p300-donor-retirement-readback
Base: e2785ee97a273d192bc9a8985c3f37795be3e6fe
State: ACTIVE

## Bounded outcome

Ordinary browser commands must acknowledge exact completed durable retirement
receipts before rejecting stale process-local custody. Unknown or unpublished
effects and unsaved navigation remain fenced. Acknowledgement reads owner
receipts and does not close, launch, focus or navigate a browser.

The selected donor later exited through an acknowledged idle retirement.
Its retained task metadata is intentional. Fresh ordinary owner commands reject
with browser_session_publication_conflict. Resident and durable public aggregates
agree; hidden operation fields prevent claiming a conclusive live root cause.
This later recovery blocker is distinct from the original no-input report.
Private identity, account and raw runtime evidence remain outside Git.

## Ownership and acceptance

Use the sole existing clean checkout. The source worker owns host/effect code and
regressions. Root owns this note, catalog, serial integration and runtime effects.
No additional checkout, state-file repair or raw shared-host restart is admitted.

Prove a completed peer receipt permits an ordinary command with zero repeated
browser-close effects. Keep existing unpublished-navigation and unknown-custody
refusals. Run focused regression tests, affected host/effect tests, workspace
format and strict Clippy, plus relevant documentation checks. Canonical source,
preserving installation and actual donor/input recovery are separate gates.

Ticket 530 still requires the fourth actual account/storage/cleanup row. Final
P300 broad review and post-review remediation counts remain zero. Source tests
here are bounded blocker repair, not the final whole-spec review/remediation.

## Reproduction checkpoint

The public owner regression
`completed_peer_retirement_does_not_poison_an_ordinary_host_command` failed on
the unmodified implementation. Its original close settled, an independent peer
completed the same SQLite custody record, and an ordinary open for another
profile returned browser_session_publication_conflict. Setup and peer completion
assertions passed. Test execution took 3.24 seconds; compilation and execution
together took 352.143 seconds with no compiler errors or warnings.

Command: `AGENT_BROWSER_CARGO_CACHE=off scripts/ci/rust-tests.sh --focused completed_peer_retirement_does_not_poison_an_ordinary_host_command`.
Cargo Signal retained the exact test patch, inputs and failed attempt privately.
The preceding cache-enabled attempt stalled before compilation and was cancelled
by exact owned process identity; it is not a failing test result.

This establishes the source hazard. It does not prove the original mouse failure
or conclusively classify the resident owner's private unpublished state.

## Source qualification

The repair acknowledges only the exact completed successor of a cached retirement.
Intent, preparation, target scopes, effect settlement and previously recorded
publication/completion identity must remain unchanged. This receipt read makes
no provider call, process observation or state publication. Unpublished local
state and pending launch custody still veto an ordinary refresh.

Local checks on the six-file formatted code patch
`22b7f8c94861706c90852a008af1634307ae78740d85c637f6a4e09c33a7775b`:

- Focused `peer_retirement`: 2 passed, including unknown-completion refusal.
- Focused `ordinary_refresh`: 1 passed, preserving unpublished navigation.
- Focused `automatic_retirement_`: 14 passed.
- Focused `failed_retirement_publication_does_not_poison_an_unrelated_task`: 1 passed.
- `scripts/ci/cargo-safe.sh fmt --all --manifest-path Cargo.toml -- --check`: passed.
- `scripts/ci/cargo-safe.sh clippy --workspace --manifest-path Cargo.toml -- -D warnings`: passed.
- `scripts/ci/cargo-safe.sh test --manifest-path Cargo.toml -p agent-browser-service-model -- --test-threads=1`: 416 passed, no ignores.
- Policy wiring: 11 passed. Changed documentation links and patch hygiene passed.

Cargo Signal CLI receipts retain every attempt outside Git. Compilation used the
existing shared Cargo target and safety wrapper with compiler caching disabled
after the initial cache stall. Rust formatting corrected three wrapping-only
differences; source semantics and earlier behavior evidence are unchanged.
Validation used Linux/WSL, Rust 1.94.1, synthetic providers and disposable SQLite
fixtures. Hosted CI was skipped by operator policy. Canonical integration,
sealed-artifact qualification, preserving installation and actual viewer input
remain separate gates. No account acceptance is inferred from these checks.
