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
