# P300 ticket 530: Profile-copy disposal census repair

Product lane: PL-BUGFIX
Parent: [P300](../plans/0300-2026-11-01-workspace-siteguides-523.md)
Work item: [530](https://github.com/CochranResearchGroup/agent-browser/issues/530)

## Observed workflow

The first real same-build CDP-free copy passed independent account usability,
fresh read-only navigation and synthetic localStorage persistence and isolation.
The synthetic key was removed and each owned probe browser closed with process
and profile-lock readback. These are one row of the four-row matrix, not full
portability acceptance.

Original-operation disposal prepared a durable cleanup receipt, then stopped
with `process_observer_identity_changed_or_unproven`. One reconciliation reached
the quarantine checkpoint and stopped at the same process census guard before
deletion. No manual removal, force closure or new copy was used for recovery.

## Diagnosis boundary

The disposal guard obtains a host-wide process census. The existing validator
rejects a process observed alive by the helper if it exits before the final
start-token check. A read-only probe of one owned short-lived process reproduced
that input pair. A current full census had no stable token mismatches. This is
a demonstrated failure pattern; the particular PID responsible for the real
disposal failure was not identified.

The repair must distinguish positively established absence from unreadable or
changed process identity. It must preserve original command-line profile
references as conservative blockers and continue rejecting PID reuse, changed
identity, private argument transport and malformed evidence.

## Custody and acceptance

Reuse the existing P300 acceptance auxiliary at canonical base
`517e4b7f09cd4182d3eef746c19ae86aa9f565e1`. The diagnostic worker is the sole
source writer for census validation and its regression tests; root owns serial
integration, documentation and runtime qualification. No additional checkout is
created. Foreign dashboard changes and the pinned P300/P302 candidates are
preserved. The original cleanup operation remains the only recovery authority.

Default was reopened on its original Google Account start page after the
synthetic probe. The expected account remained visible, the start-page setting
was restored through the public open operation, and guarded normal closure
again passed process and profile-lock checks.

The three remaining matrix rows and actual teaching qualification remain open.
Final broad review count: 0. Final post-review TDD count: 0. The focused diagnosis
regression does not consume either final pass.

## Local validation

At source base `517e4b7f09cd4182d3eef746c19ae86aa9f565e1`, with the
census repair applied, the following local Linux/WSL checks were observed.
Cargo checks use `AGENT_BROWSER_CARGO_CACHE=off` and the bounded wrapper.

- `scripts/ci/rust-tests.sh --focused cleanup_census_accepts_confirmed_exit_after_exact_observation`:
  failed before the repair with the same identity error, 1 failed and 3,772 filtered.
- `scripts/ci/rust-tests.sh --focused process_identity::namespace_observer::census`:
  6 passed and 3,769 filtered after the repair.
- `scripts/ci/rust-tests.sh --focused process_identity::namespace_observer`:
  8 passed and 3,767 filtered.
- `scripts/ci/rust-tests.sh --focused profile_cleanup_`:
  4 passed and 3,771 filtered, preserving profile-reference blockers.
- `scripts/ci/cargo-safe.sh fmt --all --manifest-path Cargo.toml -- --check`: passed.
- `pnpm test:policy-wiring`: 11 tests passed and current policy hashes verified.
- `pnpm test:documentation-links -- --base 517e4b7f09cd4182d3eef746c19ae86aa9f565e1`:
  passed.
- `git diff --check`: passed.

The initial cache-client attempt was cancelled after stalling; its exit143 is
retained as a cancellation, not a test result. The cache-disabled retry compiled
normally. Strict workspace Clippy passed with
`scripts/ci/cargo-safe.sh clippy --workspace --manifest-path Cargo.toml -- -D warnings`.
Hosted CI remains disabled by operator
policy. These provider-free checks do not prove installed cleanup, account
portability in other modes or browsers, or target-platform parity.
Private account evidence and browser artifacts remain outside the repository.

## Installed qualification and bounded portability checkpoint

PR543 is merged at `ba029e5a6c3cde86adcdc3f46e35bf47e0feb21b`.
The sealed production binary passed its source-free workstation installer
fixture, including cleanup of both owned fixture hosts with zero residue.
Preserving installation and installed doctor passed. The original quarantined
copy then completed through its original operation: delete acknowledged,
original and quarantine paths absent, destination catalog entry removed,
completed operation history retained, and donor directory identity preserved.
Private process, transaction and installation receipts remain outside Git.

Three independently created copies now have positive, time-bounded evidence:

| Target build | Mode | Fresh selected-account page | Storage divergence and key removal | Exact copy cleanup |
| --- | --- | --- | --- | --- |
| Chrome for Testing 156.0.8078.80 | CDP-free | PASS | PASS | PASS |
| Chrome for Testing 156.0.8078.80 | CDP | PASS | PASS | PASS |
| Chromium 156.0.8078.47 | CDP-free | PASS | PASS | PASS |
| Chromium 156.0.8078.47 | CDP | NOT_RUN: donor readiness requires a human step | NOT_RUN | No copy created |

The donor uses Chrome for Testing 156.0.8078.80 in CDP-free mode. Trials use
one Linux/WSL host and one explicitly selected real account. Each claimed copy
loaded its exact pinned executable and mode, then displayed the expected
account on a fresh read-only Google Account page. Synthetic localStorage was
written only to the independent copy, survived reopening, remained absent
from the donor, and was removed before exact disposal. The donor was never
opened under the alternate build. These observations do not prove arbitrary
build compatibility, other accounts, other platforms, cross-machine copying
or indefinite authentication continuity. A later donor-readiness check
required human verification; its cause is unverified and the fourth copy was
not prepared or created.

The tested sequence warms the donor through ordinary `browser_open`, prepares
with `launch:false`, closes through the original duplication owner, then opens
the published destination separately. The earlier cold-source
`profile_duplicate_source_tab_missing` outcome remains a cancelled attempt with
zero copies. It is not represented as a qualified automatic cold-source path.
Native input policy remains unchanged; initial-URL navigation and existing
guarded Remote View or public CDP close operations supplied the bounded probes.
Wrapper attribution and nested-result interpretation failures were retained;
known completed closes were verified by readback rather than replayed.

All three experimental profiles were disposed through their immutable original
operations. The synthetic server was stopped after retaining its protocol state;
the fourth row never issued a probe phase. The original donor task alone is
intentionally preserved for the human readiness step. Final post-matrix donor
usability and start-page restoration remain pending that step and the fourth row.

Ticket530 remains OPEN with a concrete human readiness gate. Ticket529 still
needs the actual narrated teaching context and principal mapping. Final broad
review count: 0. Final post-review TDD count: 0.
