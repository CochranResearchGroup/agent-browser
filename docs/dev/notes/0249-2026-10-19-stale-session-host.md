# Ordinary task recovery after a peer session update

Product lane: PL-BUGFIX
Disposition: accepted-evidence
Owning plan or work item: P249, CochranResearchGroup/agent-browser#320
Related lanes: PL-AUTH, PL-PLATFORM

## Problem and boundary

Auth-receipts synthetic disposable task acquisition repeatedly returned
`browser_session_publication_conflict`. A read-only session-document hash was
unchanged across one rejected open. The shared host kept a snapshot from before
a peer persistence update and ordinary dispatch did not refresh it.

The stale-write fence is correct. The repair adopts current durable session state
only before a new ordinary command when local state matches its last published
snapshot and no unpublished launch or retirement operation is pending. It retains
the effect adapter and its journals. Identity checks, effect reconciliation and
publication CAS still run; no failed command is automatically replayed. Status
reads remain available when dirty local state prevents refresh. Direct low-level
stale publication and handoff-policy mutation retain their original fences.

Source changes cover ordinary managed command dispatch, explicit session
commands, Remote View task opening and the new reaper invocation boundary.
No installed binary, runtime database, credentials, peers or profiles were reset.
Existing pending Native acquisition (#316) and observed-launch custody (#286)
remain separate boundaries, not assumed repaired by this work.

## Test seams and evidence

The new regression uses actual disposable SQLite persistence and two host owners.
A peer creates a task/tab; the first host opens, reopens, navigates and closes its
own task while preserving the peer and using one browser. The companion failure
case injects unknown navigation after initial-tab acquisition, advances a peer,
and requires the first host to retain its unpublished state without another
navigation. Status diagnostics must remain available.

Red command:

```sh
AGENT_BROWSER_CARGO_CACHE=off scripts/ci/cargo-safe.sh test --profile ci \
  --config 'profile.ci.package.agent-browser.opt-level=0' \
  -p agent-browser --bin agent-browser ordinary_task_open_refreshes_peer_changes -- --nocapture
```

It failed at the valid open assertion with
`browser_session_publication_conflict` (0 passed, 1 failed). Earlier iterations
failed on incomplete initial-tab fixture behavior (`unused`, then invalid source
attribution); those were fixture failures, not defect reproductions. A prior
compiler-cache startup stalled and was stopped; cache-free runs use the admitted
Cargo wrapper. The package optimization override shortens development feedback;
this is not a production release build.

Green: 9 host regressions pass, including the new SQLite peer-update and
unknown-navigation cases. Persistence qualification passes 12/12. The final
consumer composition (including nested pool cases) passes 107, with 1 explicit
ignored test requiring the task-owned real Remote View native viewer fixture.
Total selected Rust qualification: 128 passed, 0 failed, 1 ignored. The queued
executor case that aborted with the omitted stack setting now passes.
The first broader batch retained an obsolete expectation that leave-open must fail
after an unrelated sequence increment; that assertion is updated to require both
peer sequence and retention to persist. The same batch then aborted at a queued
executor test because direct Cargo omitted the repository test harness's 16 MiB
thread-stack setting. Final consumer qualification uses `RUST_MIN_STACK=16777216`,
matching `scripts/ci/rust-tests.sh`; the failed batch is not a passing receipt.

Policy wiring passed 11 tests and its repository assertion script. Changed-file
secret scanning and whitespace checks passed before final evidence updates.
Hosted CI is skipped at operator direction. Production installation and actual
receiving acceptance are not claimed by these provider-free regressions.

Reproduce the selected qualification with the same development profile and the
repository harness's thread-stack setting:

```sh
for filter in native::browser_session_host::tests native::browser_session_store::tests native::browser_session_consumer_tests; do
  RUST_MIN_STACK=16777216 AGENT_BROWSER_CARGO_CACHE=off scripts/ci/cargo-safe.sh test \
    --profile ci --config 'profile.ci.package.agent-browser.opt-level=0' \
    -p agent-browser --bin agent-browser "$filter" -- --test-threads=1
done
```

PR #321 owns source integration. The source checkpoint is `2505abecd`; subsequent
evidence-only updates do not change the tested Rust input. Installed Agent Browser
and auth-receipts runtime receiving acceptance remain unchanged and open.
