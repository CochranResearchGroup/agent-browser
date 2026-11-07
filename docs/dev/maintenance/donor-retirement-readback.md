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

## Installed recovery checkpoint

PR 555 integrated the first repair at canonical commit
`2fab4b793ab835fed169911f1b591d4c32489f5b`. The qualified sealed production
artifact passed its installer fixture and normal preserving installation.
Installed doctor passed. The publication conflict cleared, and the ended donor
task was normally retired while preserving its profile data.

A fresh open then exposed a separate allocator blocker. Read-only inventory,
window counts and durable custody show that selection chooses the first empty
desktop despite its pending idle-retirement reservation. Admission correctly
refuses it before creating a launch claim. Another empty desktop has no such
reservation. Pool eligibility must exclude retirement reservations before
selection, retaining the final admission fence. No reservation was settled or
removed to work around this failure, and no blind launch retry was issued.

The original viewer-input report remains unqualified until a live donor can be
tested through dev-browser and independently observed desktop input. Ticket 530
remains open; three copy rows are accepted and disposed, with the fourth unrun.

## Allocator repair qualification

The real SQLite pool regression failed before the patch because both desktops
were offered instead of only the unreserved desktop. The repair composes the
existing launch eligibility check with a validated read of nonterminal retirement
desktop reservations. It preserves reservation history and leaves transactional
launch admission unchanged. A concurrent reservation still fails at admission.

The source patch digest is
`81852f57aaf5aa093c6eb86945047df0ce13b45731c89d31dd03f1296cc860ee`.
Local checks passed: nine `pool_demand_` tests, fourteen
`automatic_retirement_` tests, workspace format and strict workspace Clippy.
The initial format check found two wrapping differences in the new regression;
formatting was corrected and the final check passed. Policy wiring passed all
eleven tests. No hosted CI ran. Sealed build, preserving installation and actual
viewer-input evidence remain pending for this second repair.
