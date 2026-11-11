# Committed dashboard forward recovery

Product lane: PL-BUGFIX. Work item:635. Branch: `fix/committed-dashboard-recovery635`.
State: OPEN. Owner: ecochran76.

Restore forward recovery when an upgrade has committed its sealed generation and
runtime ownership, but its healthy dashboard remains staged after a failed
post-commit check. Recovery must use that exact candidate, preserve runtime and
installation custody, promote only its observed healthy managed backend, and
retain the existing physical selected-generation proof before acceptance.
An absent authenticated presentation receipt stays absent.

Bound this source packet to the existing recovery, rehydration and dashboard
promotion seams and focused regressions. At most three meaningful repair
attempts within 45 minutes. No installed mutation, foreign-process cleanup,
authentication, new browser lifecycle authority, global cache restart, commit or
push is part of this delegated task. Reuse the canonical checkout after its
owned build stopped; preserve all peer checkouts and builds.

The minimized regression creates a sealed generation in a disposable root,
commits it, records operator recovery, and leaves the old dashboard selected
with the exact candidate staged. A bounded local HTTP fixture supplies the
managed backend's manifest. The real recovery path must promote that candidate
before the selected-generation validator can accept it. Supervisor and host
physical checks are injectable only at the test seam; production uses the
existing concrete adapters. This is source qualification, not installed proof.

Governance MCP returned POLICY_MANIFEST_UNAVAILABLE for this client. Canonical
product, runtime-proof, domain, collaborative-development, runtime-custody and
worktree policies were read directly. Hosted CI is excluded under operator
policy. Final Contacts review and findings TDD passes remain unused.

## Regression and repair

The exact regression ran RED on the unmodified recovery behavior after adding
only its injectable concrete-runtime adapter seam. Command:

```bash
AGENT_BROWSER_CARGO_CACHE=off AGENT_BROWSER_CARGO_BUILD_JOBS=2 \
AGENT_BROWSER_CARGO_MEMORY_CLAIM_KIB=6291456 CARGO_PROFILE_TEST_DEBUG=0 \
scripts/ci/cargo-safe.sh test --bin agent-browser --manifest-path cli/Cargo.toml \
workstation_install::tests::forward_recovery_promotes_healthy_staged_dashboard_before_physical_proof \
-- --exact --nocapture
```

Base `90e8c01f` plus the neutral seam and regression produced zero passes and one
failure: `current_selection_dashboard_identity_unproven`. Compilation took
6m04s; the sealed fixture and test took30.25s. Cargo Signal retained
`job-20261111T181758Z-13e8bcf15927`. An earlier selector waiter was canceled
before compilation; it is not RED evidence. A formatting admission waiter was
also stopped before compilation and rerun with its explicit2GiB claim.

Forward recovery now requires the exact staged generation, matching original
candidate binary/support digests and an active installation custody record.
Rehydration recognizes an operator-recovery transaction with committed effects
and verifies its candidate selection without refreshing deadlines or rewriting
history. Existing managed-backend promotion then validates the observed manifest
under the original fence, preserves only genuine presentation evidence, and
keeps the existing physical selected-generation proof as the acceptance gate.
Changed targets, seals or custody remain errors.

This repair improves continuation of the same committed transaction. It is not
a prerequisite for every later installation: README's existing new-apply
contract says a new operator-requested apply creates its own coordinated
operation and historical unrelated failures do not force prior recovery.
Selecting a fresh preserving apply remains distinct from retrying this exact
transaction. Neither source qualification nor that documented alternative
claims an installed outcome.

The first repaired snapshot passes six forward-recovery tests (five new cases
plus an existing runtime-adoption case). Compilation took 4m14s and tests 182.57s;
Cargo Signal job20261111T182634Z-22a7142e983b is retained. Its frozen executable also
passes five physical-selection tests, two existing promotion/fencing tests,
three live-manifest tests, failed-backend rejection, preserved same-generation
recovery and committed cooperative-handoff resume:19distinct tests total.
The normal docs build and policy/link/handoff checks pass.

Before final source freeze, root identified a race in existing forward recovery:
the final Accepted write preceded the completion-time custody check. An added
same-fixture regression cancels active custody inside a successful physical
validation callback and requires byte-identical unaccepted transaction state.
This remains ordinary635 implementation qualification, not the designated
Contacts review or findings-remediation pass. The repair will guard only the
bounded final persistence under existing transaction custody; external proof
and subsequent completion keep their separate lock lifetimes.


## Qualified source snapshot

The cancellation-during-proof test ran RED before the acceptance guard:
`job-20261111T183456Z-269ee777b99b`, zero passes and one failure, exact assertion
`failed or fenced recovery must not persist Accepted`. Compilation took2m11s
and the fixture44.33s. The bounded Accepted persistence now runs inside
`with_transaction_candidate_install_custody`; physical validation and later
completion remain outside that lock.

Final `forward_recovery_` group passes seven cases, including six635regressions
and one existing runtime-adoption case. Job
`job-20261111T183820Z-b1a53647e11f` took2m17s to compile and209.73s to execute.
The exact frozen executable SHA-256 is
`2942e421855bbb1d0da40dcc7a3a915fbccefb75b9a99cf84ce968f5d25d28df`.
All13support cases were replayed against those same bytes: five physical
selection checks, two existing promotion/fencing cases, three live-manifest
checks, failed-backend rejection, preserved same-generation recovery, and
committed cooperative-handoff resume. Total:20distinct tests passing.

Final strict workspace Clippy passes with warnings denied
(`job-20261111T184409Z-99bab34944e1`,53.064s). The exact command is
`scripts/ci/cargo-safe.sh clippy --workspace --manifest-path Cargo.toml -- -D warnings`.
The final wrapped whole-workspace format check passes; policy wiring passes11
cases, changed documentation links and remote-view handoff checks pass, and
`pnpm --dir docs build` passes with the normal Turbopack builder. All Cargo
commands used the safety wrapper with cache disabled. Tests used two jobs and a
6GiB claim, final lint two jobs and4GiB, formatting2GiB. Full command/receipt
and source-hash manifest stays outside Git in the bounded qualification packet.

The selector's monolithic installer recommendation exceeds this Rust recovery
slice for unchanged host provisioning, VM, legacy RDP/Guacamole and PostgreSQL
scripts/assets; these were not rerun. No whole-installer fixture or production
candidate acceptance is claimed here. The later exact sealed production
candidate remains responsible for its applicable installation qualification.
The installed shared skill is unchanged because this packet changes only repo
source and guidance. Hosted CI was skipped under the operator cost policy.

Source qualification is complete and uncommitted for root review. Root owns
commit/push/PR integration, sealed production build and real exact-transaction
recovery readback. Keep635OPEN until that installed readback succeeds. The
original browser, peer processes, generations and transaction receipts were
never mutated by this source packet. Contacts' designated review and findings
TDD remain unused. Memory disposition is forbidden for this delegated packet:
root retains the substantive closeout and no memory effects were delegated.
