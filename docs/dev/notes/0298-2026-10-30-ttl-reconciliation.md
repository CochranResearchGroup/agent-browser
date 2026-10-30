# P298 retirement progress and tab accounting

Product lane: PL-BUGFIX
Disposition: accepted-evidence
Owning plan or work item: P298, #498 under #488
Related lanes: PL-AUTH (#497 source overlap only)

## Changed behavior

New retirement scopes omit already idle-closed browser history. A historical
peer from another allocation sharing a desktop ID/generation is omitted only
after exact original-process readback proves it ended. Live, uncertain and
identity-less nonretired peers remain protected. Current assignment peers stay
in the frozen scope. Original launch evidence is never backfilled.

Legacy reconciliation marks orphan nonterminal tabs Closed without deleting
records or dispatching CDP effects. A concurrently registered browser owner
invalidates an earlier orphan observation before publication.

The shared host forwards its complete reap result to a bounded private latest-pass
receipt at `~/.agent-browser/service/browser-session-reaper.json`. Running,
completed, deferred and failed states survive discarded stderr. It includes
counts and up to 64 hashed browser IDs with bounded error codes, plus an omitted
count. Receipt failure never prevents or repeats the owner operation. Failed
passes may follow earlier committed cleanup; counts are not invented on error.
No TTL defaults, human holds, explicit leave-open policy or production state
changed. AuraCall/IM lifecycle ownership and unregistered popup/restored-page
provenance remain outside this bounded repair.

## Red and green evidence

Baseline: `c20cbb24e79e8d9b0de56d2c03e7d27a5e1981ea`.

- Public effects test `idle_closed_history_without_identity_does_not_block_current_retirement`
  failed because no guard was returned, then passed after historical exclusion.
- Public effects test `ended_peer_from_previous_assignment_does_not_block_current_retirement`
  failed with `browser_session_idle_readback_required`, then passed after
  assignment-bound original-exit qualification. Live/unknown peers still defer.
- Real SQLite custody test `current_retirement_excludes_historical_peers_through_sqlite_custody`
  passes both history cases through actual durable admission and reopen. No
  physical effect occurs during this admission check.
- `reconciliation_retires_orphan_tab_records_without_erasing_history` failed
  with Ready instead of Closed, then passes. Its initial changed-row count was
  corrected because handle refresh also updates the terminal peer; the test
  checks lifecycle, retained history and idempotence instead.
- `orphan_reconciliation_preserves_concurrently_registered_owner` failed with
  Closed instead of Ready, then passes after guarding newer owner registration.
- `background_pass_persists_failure_and_next_success_without_replaying_effects`
  failed because no receipt existed, then passes with private readback.
  Additional tests prove bounded redaction and exactly-once maintenance despite
  unavailable receipt storage.

Model red/green command:

```sh
AGENT_BROWSER_CARGO_CACHE=off AGENT_BROWSER_CARGO_BUILD_JOBS=2 AGENT_BROWSER_CARGO_MEMORY_CLAIM_KIB=4194304 scripts/ci/cargo-safe.sh test -p agent-browser-service-model --test browser_handoff_recovery <test-name> -- --exact
```

CLI source checks use the repository's isolated-home runner:

```sh
CARGO_PROFILE_TEST_DEBUG=0 AGENT_BROWSER_CARGO_CACHE=off AGENT_BROWSER_CARGO_BUILD_JOBS=2 AGENT_BROWSER_CARGO_MEMORY_CLAIM_KIB=6291456 scripts/ci/rust-tests.sh --focused <filter>
```

After the first compiled red CLI run, the existing pinned test executable was
also invoked directly with a disposable HOME/XDG tree, 16 MiB Rust test stack and
serial test execution. This avoided repeated compilation while preserving the
same test seam. Compiler errors for a missing generic trait bound were repaired
before the subsequent model run. Build admission initially waited for host
memory; shared sccache stalled on compiler identity, so this lane disabled its
cache without stopping the shared service. Model-only one/two-job and CLI
two-job builds retained the 16 GiB host reserve. Debug symbols were disabled for
the larger CLI lane to reduce memory and disk use.

## Validation and limits

Passing local checks recorded so far:

- All service-model unit, integration and documentation tests.
- 93 service-health tests, 40 output tests, 3 reaper receipt tests, the real
  SQLite retirement regression and 2 startup reaper checks.
- Cargo format and workspace clippy with warnings denied.
- Docs production build, Remote View handoff docs, policy wiring and lease
  authority architecture checks.
- Host provisioning, fresh workstation VM harness, Guacamole assets,
  PostgreSQL durability and route-specific user-sync contracts.

The original installer-fixture run selected the old bundled September binary
because no ordinary debug CLI existed. Its live-observation assertion failed.
The source-pinned rerun passes, including source-free installation and exact
cleanup of two owned fixture hosts with zero residue.
The historical planning-archive audit also fails its 200-line RUNBOOK limit;
the untouched baseline already has 416 lines. This pre-existing audit is not
represented as passing and no unrelated archive/runbook rewrite is included.

Hosted CI is skipped under operator direction. Local Linux/WSL provider-free
checks do not prove other operating systems, production deployment, private
account continuity or actual installed tab disappearance. The separate dirty
Auth-retention patch was preserved; its future integration must compose the
small host reaper-result hunk with its retention-command additions.

## Review

Frozen range: `c20cbb24e...e78db857f` (one source commit).

### Standards

Independent review reported no hard documented violations or actionable smells.
Primary adjudication accepted no findings; highest accepted severity: none.

### Spec

Independent review reported no missing, incorrect or out-of-scope requirement.
Primary adjudication accepted no findings; highest accepted severity: none.
Both axes preserve the distinction between source validation and live acceptance.
No review finding required a runtime change. The broader consumer gate exposed
one existing silent-deferral expectation; it was updated to assert the explained
`browser_session_idle_original_identity_missing` deferral while preserving all
no-claim, no-native-preparation and unchanged-state assertions. Its focused
rerun passes. Runtime source did not change after the broad gate or review.

## Additional final checks

The full browser-consumer run reported 127 pass, 1 ignored and that single stale
expectation. Reusing its 127 unchanged passing cases and the focused corrected
case gives 128 passing cases with 1 existing ignored case. This is composite
local evidence, not a claim of one wholly green execution. All 13 daemon tests,
3 receipt tests and 2 startup reaper tests pass. All 124 lease-authority tests pass.

The source CLI was built locally with Cargo dev debug symbols disabled; its
SHA256 is `24c9b549256fc4e49cb79ef18bf907025e52107349ae687da58572b9427e91f2`.
It is a fixture binary, not an installed production candidate. The installer
fixture pins this exact executable through `AGENT_BROWSER_FIXTURE_BIN`.
Pinned installer command: `AGENT_BROWSER_FIXTURE_BIN=/home/ecochran76/workspace.local/agent-browser/cli/target/debug/agent-browser pnpm test:workstation-install-fixture`. It passes with zero fixture residue.

PR #499 carries canonical integration; #498 owns this bounded source outcome.
#488 remains open for separately authorized installed acceptance and wider page
provenance. The passing gates above qualify the runtime source in `e78db857f`;
the follow-up changes only the expected deferral assertion and this receipt.
Git diff hygiene and Cargo format pass after that assertion change. Reuse of
unchanged runtime clippy and test evidence follows policy 0042. No hosted CI,
installation or production cleanup was performed by the repair.
