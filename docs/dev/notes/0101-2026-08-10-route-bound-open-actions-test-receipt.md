# Plan 0101 Route-Bound Open and Actions Deepening Final Test Receipt

Date: 2026-08-10

Role: distinct final tester

Verdict: **FAIL**

Commit disposition: **Candidate 4 is not commit-ready.**

Exact tested HEAD:
`991f4ca914baca3ae7865feabdea70e7b85e3e9d`

## Scope and authority

This receipt independently tested Candidate 4 against Plan 0101, Plan 0102,
Plan 0106, Plan 0107, the terminal two-cycle work-audit ledger, the execution
receipt, `CONTEXT.md`, the service roadmap, and current source. It did not
reopen planning or perform a third audit.

The tester edited only this receipt. No implementation, plan, audit, commit,
installed binary, service, tenant, route, or remote was changed intentionally.
The required canonical Rust driver did expose a test-boundary defect that
launched Chrome against the default runtime profile. The tester stopped that
process tree immediately after detection and made no attempt to repair or
clean profile contents.

The following pre-existing untracked paths are excluded from Candidate 4
identity and were not edited:

- `docs/dev/notes/2026-08-10-last30days-stale-runtime-pid-lock-handoff.md`
- `scripts/architecture/actions-inventory/target/`

Graphiti discovery was healthy but returned only older route context. Current
repo authority controlled. CodeGraph was healthy at 534 files, 18,267 nodes,
and 63,156 edges; current source and executable gates supplied final evidence.

## Blocking findings

### `P0101-T1-01` — canonical parallel-safe lane is nondeterministic

The first exact `scripts/ci/rust-tests.sh` invocation failed in the
parallel-safe lane:

- 1,077 passed
- 2 failed
- 57 ignored
- 736 filtered
- 1,136 scheduled in the lane

The failures were:

- `native::service_resources::tests::gc_apply_requires_matching_review_token`
- `native::service_resources::tests::gc_apply_refuses_changed_candidate_identity`

The first assertion observed `response["applied"] == false` rather than
`true`. The second observed a null `counts.terminated` rather than `0`.

Both failed tests passed individually, 1 of 1 each. The complete
`native::service_resources::tests` module passed twice, 11 of 11 each time.
The one authorized exact rerun did not reproduce either failure: its
parallel-safe lane passed with 1,079 passed, 0 failed, 57 ignored, and 736
filtered, then advanced through the serialized partitions. This isolates the
finding to cross-module process-global or runtime-state interference rather
than either assertion's local behavior.

An exact canonical gate that fails under its declared parallel-safe schedule
and passes only when isolated is not reliable release evidence. The two green
isolated repetitions do not supersede the recorded exact-driver failure.

Disposition: `blocking`. Reconcile the shared-state ownership or move the
affected module into the justified serialized inventory, then obtain stable
canonical runs.

### `P0101-T1-02` — the declared no-launch parity partition launches Chrome and mutates the default profile

The exact rerun advanced through the first 43 serialized filters and reached
filter 44, `native::parity_tests`. A focused readback showed
`test_all_documented_actions_are_handled` iterating every documented action
through production `execute_command`, beginning with `launch`.

A bounded focused observation of that same partition proved the effect:

- the partition scheduled 18 tests;
- 2 tests completed successfully before the dispatch-coverage test;
- `test_all_documented_actions_are_handled` started Chrome
  `151.0.7922.77`;
- Chrome used
  `/home/ecochran76/.agent-browser/runtime-profiles/default/user-data`;
- the process opened a loopback CDP connection and spawned Chrome renderer,
  GPU, utility, broker, and crashpad children;
- default-profile files including `DevToolsActivePort`, `Last Version`,
  `Local State`, `Variations`, and first-party-set state received timestamps
  during the run.

The tester terminated the test and its exact Chrome process group at the hard
no-browser boundary. A final process readback found no remaining Candidate 4
Cargo, rustc, parity-test, or Chrome process.

This contradicts both the assigned test boundary and the fast-gate
documentation that describes parity coverage as no-launch. It also prevents a
clean success result for the canonical driver and invalidates behavioral-gate
truthfulness even though the structural gates are green.

Disposition: `blocking`. Make documented-action coverage structurally prove
dispatch without executing effectful actions, or supply an explicit inert
runtime fixture and temporary profile. The ordinary fast Rust gate must not
launch a browser or address the operator's default profile.

## Architecture and inventory ledger

| Criterion | Result | Independent readback |
| --- | --- | --- |
| `actions.rs` line budget | PASS | 902 physical lines |
| `actions.rs` definition budget | PASS | exactly 6 production definitions |
| `actions.rs` test budget | PASS | 0 in-file tests |
| compatibility wrapper budget | PASS | 0 wrappers |
| frozen responsibility inventory | PASS | exactly 615 records: 609 `moved`, 6 `baseline` |
| dispatcher allowlist | PASS | exactly `action_skips_browser_launch`, `active_target_binding`, `handle_dependent_batch`, `execute_command`, `success_response`, and `error_response` |
| predecessor reconciliation | PASS | four P0100 baseline rows removed and moved `handle_service_status_with_dependencies` added |
| target-depth ledger | PASS | 57 entries |
| production definition reconciliation | PASS | current source accounts for all 615 frozen definitions |
| test reconciliation | PASS | 261 tests across 34 physical modules: 82 dispatcher/helper and 179 owner tests |
| reverse dependency guard | PASS | no owner module imports the dispatcher; four reviewed external `actions::execute_command` consumers remain |
| facade and god-module guard | PASS | no compatibility facade, `common.rs`, central `tests.rs`, page-PDF facade, browser-evaluation facade, stream-screencast facade, or service-configuration-inventory facade |

`pnpm test:actions-architecture` passed its structural, remediation, and WSL
entrypoint checks. It reported
`definitions=6 tests=0 lines=902 wrappers=0 final=true`.
`pnpm test:actions-architecture-self` accepted the classified fixture and
rejected the unclassified fixture with its stable definition ID.

The inventory file SHA-256 is
`85cab84cc401d357f02be825fff2d323fd036915b348461df6b8173802b038a4`.
Its frozen source SHA-256 remains
`a868a2e9fa81e6debd7e4e676f51752b5a98ddaaf0e2db0236a3f687dc38d111`.

## Route-bound open semantic ledger

| Criterion | Result | Independent readback |
| --- | --- | --- |
| typed route seam | PASS | exactly 13 runtime operations with concrete request and result records |
| outcome algebra | PASS | exactly 7 outcomes: `Planned`, `NotFound`, `ExplicitlyClosed`, `Reopened`, `Opened`, `RolledBack`, and `ProviderFallback` |
| authenticated ingress | PASS | attribution is constructed at the authenticated dispatcher boundary; private construction rejects unauthorized attribution before runtime effects |
| fallback semantics | PASS | exact nine Plan 0102 predicates are modeled and exercised |
| two-file atomic handoff | PASS | one matrix test covers all 4 injected boundaries: handoff write, state write, handoff rename, and state rename |
| precleanup quarantine | PASS | failed cleanup preserves `rollback_incomplete` and route-pool quarantine |
| promotion | PASS | confirmed cleanup promotes to `rollback_complete`, records rolled-back state, and removes quarantine |
| deadline ownership | PASS | bounded forward and compensation phases reserve lock time and drop unfinished futures at the deadline |
| no detached route task | PASS | route-bound open contains no `tokio::spawn`; the coordinator owns the future |
| kernel I/O residual | PASS, documented boundary | uninterruptible kernel I/O may outlive cooperative cancellation and remains an explicit residual rather than an atomicity claim |

The exact 13 runtime operations are `observe_browser`, `launch_browser`,
`refresh_targets`, `switch_target`, `navigate_target`, `open_target`,
`focus_target`, `close_created_target`, `close_created_browser`,
`checkout_route`, `ensure_display_access`, `observe_visible_window`, and
`observe_operator_access`.

The exact nine fallback predicates are `immutable_snapshot_exists`,
`explicit_close_allows_resolution`, `exact_opaque_rdp_identity`,
`typed_retained_owner_conflict`, `current_bounded_route`,
`operator_access_succeeded`, `best_effort_result`, `no_new_ownership`, and
`retained_browser_and_unrelated_tabs_unchanged`.

Focused Rust results, all run through `scripts/ci/cargo-safe.sh`:

- route coordinator tests: 18 passed, 0 failed, 0 ignored;
- route action helper tests: 6 passed, 0 failed, 0 ignored;
- atomic two-file fault matrix: 1 passed, 0 failed;
- coordinated handoff tests: 2 passed, 0 failed;
- rollback quarantine and promotion test: 1 passed, 0 failed;
- service-health partition: 66 passed, 0 failed;
- no-launch CDP screencast filter: 3 passed, 0 failed.

These 97 focused Rust test executions passed. They do not override the two
canonical-gate findings.

## WSL build-safety ledger

PASS:

- one repository `flock` serializes Agent Browser Cargo invocations;
- compiler parallelism defaults to four jobs;
- each compiling invocation enters one aggregate user-systemd scope with
  `MemoryHigh=20G`, `MemoryMax=24G`, `MemorySwapMax=4G`, and `TasksMax=512`;
- the wrapper fails closed when the WSL user-systemd manager is unavailable;
- every Cargo command in `scripts/ci/rust-tests.sh` uses the wrapper;
- package, JavaScript, and shell raw-Cargo entrypoint scans passed;
- route-confusion Cargo invocations use the wrapper;
- CLI and actions-inventory harness format and strict clippy checks passed
  through the wrapper.

All compiling commands in this test receipt used
`scripts/ci/cargo-safe.sh`. Available memory remained between approximately
36 and 38 GiB, swap usage remained approximately 3.4 of 32 GiB, and no rustc
memory blowout occurred. An accidental duplicate driver queued behind the
same repository lock and was terminated before it could compile; the wrapper
therefore demonstrated serialization rather than concurrent compiler load.

The build cap applies to the aggregate Cargo compiler process tree. It does
not make runtime tests no-launch, which is the distinct `P0101-T1-02` defect.

## Other required gates

Passed:

- `pnpm test:route-confusion-gates`: all 8 route-confusion gates;
- `pnpm test:route-handoff-audit`;
- `pnpm test:service-client`;
- `pnpm test:dashboard-inspector-actions`;
- CLI strict format and clippy;
- actions-inventory harness strict format and clippy;
- validation selector against the Candidate 4 base and selector JSON against
  `HEAD`;
- `git diff --check 290ded00..HEAD`;
- worktree `git diff --check` before this receipt.

The selector included the unrelated untracked Last30Days note when run against
the slice base. That path is not Candidate 4 evidence. Live CDP was recommended
by path selection but intentionally withheld by the no-browser and no-live
boundary.

## Structural versus behavioral gate truthfulness

The structural gates truthfully prove file shape, inventory reconciliation,
ownership direction, wrapper absence, route type shape, raw Cargo entrypoint
closure, and the exact allowlists. The focused behavioral fixtures truthfully
prove the typed route outcomes, fallback predicates, fault boundaries,
quarantine, promotion, and deadline contracts under their fixtures.

They do not prove that the full canonical schedule is isolated or that the
parity partition is no-launch. `P0101-T1-01` and `P0101-T1-02` are direct
counterexamples to those broader claims. Documentation, roadmap, audit, and
execution-receipt coverage is complete, but their final green and no-launch
statements are not accepted as current behavioral truth.

## Exact identities

- tested HEAD:
  `991f4ca914baca3ae7865feabdea70e7b85e3e9d`
- fixed integration base: `4f052ba3`
- tracked base-to-HEAD paths: 165
- base-to-HEAD binary patch SHA-256:
  `cf465aa525b0d9d06c31df1c0ecac86ea369fcee8fb9bb808886149139efdeb6`
- responsibility inventory SHA-256:
  `85cab84cc401d357f02be825fff2d323fd036915b348461df6b8173802b038a4`
- Plan 0101 SHA-256:
  `c2e1b3cb415f99a73de4ddb2a7990ebc16bddc969e1ffc457cde709cb359aca1`
- Plan 0101 audit SHA-256:
  `4c31ddd622b5a356ca8785db0a6009637e37ac705b14b57d6716c76847439063`
- Plan 0102 SHA-256:
  `65e9f3a464b5532b292efd74851586adcd8db2ae533ab41c1953413f9e5caa45`
- Plan 0106 SHA-256:
  `2e7575e20188242c4f184afa63e96f4217b63fdd6d5a4ea34543d9e7e36a9f1d`
- Plan 0107 SHA-256:
  `f858dd87e21541459629f56583e7571cc1905d2c52213d8e72f58ea8f7288dc5`
- terminal work-audit SHA-256:
  `3d3b78dcc3ae7fb66ffad45a159f9ba62d93470f634e24bb83dbc12741ced856`
- execution receipt SHA-256:
  `703acc69265da1489b38e27708ebfda977e6d20503c9f7354f3f43703fd8adbd`
- `CONTEXT.md` SHA-256:
  `9c1ba91f7a19af0a746335354a3926b5d7a60771ca89ce03a0204720e2dc25f5`

This receipt is intentionally excluded from the Candidate 4 patch identity.
The untracked inventory target and Last30Days handoff are also excluded.

## Warnings and residual boundaries

- The two service-resource failures did not recur in isolation or in the one
  exact rerun, but the original canonical failure remains valid evidence of a
  scheduling defect.
- The canonical parity surface addressed the operator's default runtime
  profile before the tester could enforce the hard stop. Its contents may now
  contain test-created Chrome state. No cleanup was attempted because that
  would be an additional runtime/profile mutation outside test authority.
- Live browser, ignored E2E, install, doctor, publication, tenant, route,
  profile-repair, and remote checks were not run.
- Cooperative deadline cancellation cannot guarantee immediate kernel-level
  interruption during uninterruptible file I/O. That residual remains
  correctly documented.
- WSL aggregate Cargo caps protect compilation. They do not cap or sanitize
  separately launched runtime browsers.

## Final disposition

Candidate 4 is **FAIL** and **not commit-ready**. Preserve the structurally
green architecture and route semantics, repair the canonical shared-state
isolation and the effectful parity gate, then rerun the canonical Rust driver
under an inert temporary runtime boundary. No third work audit is authorized
or required by this receipt.

## Bounded blocker retest

Retest date: 2026-08-10

Exact retested HEAD:
`2480f11f54232e7c22e5b0edef1cdfbe5ea8dc4c`

Superseding verdict: **PASS**

Commit disposition: **Candidate 4 is commit-ready.**

This closed-world retest supersedes the earlier FAIL disposition for
`P0101-T1-01` and `P0101-T1-02`. It tested only the two accepted blockers and
the resulting final disposition. It did not reopen architecture discovery,
planning, or work audit, and it did not run a second canonical retry.

### Repair readback

`P0101-T1-01` is closed. `scripts/ci/rust-tests.sh` now includes
`native::service_resources::tests` in its exact serialized-filter inventory.
The parallel-safe command therefore excludes the module, and its dedicated
serial partition runs with `--test-threads=1`.

`P0101-T1-02` is closed. The parity module no longer imports or calls
`execute_command`. Documented-action coverage now:

- parses the production dispatcher registry structurally;
- reconciles every documented action against that registry;
- rejects source references to `execute_command`, `auto_launch`,
  `handle_launch`, and `launch_chrome_detached`;
- uses a temporary `HOME` guard for filesystem-dependent parity cases;
- removes temporary homes on guard drop;
- asserts that its temporary home did not acquire the default runtime-profile
  path.

The repair commit changes exactly two tracked paths. Its predecessor-to-HEAD
binary patch SHA-256 is
`5986c7f0abb3d006bb96c0206786eb8a42f215d5b38c14467019dc2b0b2ac72b`.
The repaired file hashes are:

- `cli/src/native/parity_tests.rs`:
  `fe41e5c35cb84a6016262ed06a8790bd04c2e7ce23f549de1c9e265c89e9c60c`
- `scripts/ci/rust-tests.sh`:
  `849d9c98c785af22ec54d32a8a1b3a1ba303bf02f73a624bb0f23f77d713e850`

### One canonical run

The one authorized retest invocation of `scripts/ci/rust-tests.sh` completed
every test partition successfully through `scripts/ci/cargo-safe.sh`:

- parallel-safe lane: 1,068 passed, 0 failed, 57 ignored, 747 filtered;
- serialized filters: all 49 partitions passed;
- serialized test executions: 747 passed, 0 failed, 0 ignored;
- aggregate: 1,815 passed, 0 failed, 57 ignored;
- `native::service_resources::tests`: 11 passed, 0 failed, serialized;
- `native::parity_tests`: 18 passed, 0 failed, serialized, completed in 0.02
  seconds.

The outer output-filter command attempted to assign to zsh's read-only
`status` parameter after the Rust driver had completed all 49 serial filters.
That observer shell consequently returned 1 without printing its intended
summary variable. This is not a Rust-driver failure: every test result was
captured as `ok`, including the last `runtime_profile::tests` partition, which
passed 9 of 9.

No further canonical run was started.

### Focused zero-launch proof

Because unrelated live runtime activity overlapped the canonical observation,
the tester ran the permitted focused parity partition once after the external
activity had quiesced.

Result: 18 passed, 0 failed, 0 ignored in 0.02 seconds.

The real default profile was fingerprinted immediately before and after that
focused run. Both hashes remained byte-for-byte identical:

- complete path, type, size, and modification-time tree before and after:
  `79474f8b8537d4792ae8e5923e0da7b7240725e01ff625a7d3a99b8c09fc3c05`
- regular-file path, size, and modification-time tree before and after:
  `a4d47c18aa664d0879c4d3db2e7bd05cadb4f937d7376d726c1976b7edfc77e4`

A three-second pretest stability interval produced the same two hashes. No
Chrome or Chromium process existed before the focused test or in its immediate
posttest readback. The focused result therefore proves that the repaired parity
partition neither launches a browser nor touches the real lowercase `default`
profile.

No extra focused `service_resources` run was needed because its exact
canonical serial partition passed 11 of 11 and its exclusion from the
parallel lane was visible in both the script and the canonical command.

### Concurrent live-state contamination

The complete real-profile fingerprint did change during the broader canonical
window:

- initial path-metadata tree:
  `3014447ed1eefa3318a0aa8b7cd8c2bf560c774514d81fe44fdb959a9783066b`
- initial regular-file tree:
  `fa51faf132afb536884e04a1fec82aada0aa68f48f654f976bb1aefb4c5e9963`
- post-window path-metadata tree:
  `79474f8b8537d4792ae8e5923e0da7b7240725e01ff625a7d3a99b8c09fc3c05`
- post-window regular-file tree:
  `a4d47c18aa664d0879c4d3db2e7bd05cadb4f937d7376d726c1976b7edfc77e4`

The user journal attributes that mutation to a separate overlapping live
runtime-convergence workflow rather than the repaired parity partition:

- the separate workflow began before the canonical run, stopping the installed
  dashboard at 12:29:28 and restarting it at 12:29:45;
- it invoked installed-runtime, sudo helper, RDP-route, and install-verification
  operations outside this tester's process tree;
- systemd recorded Chromium application scopes at 12:31:48.701,
  12:31:49.085, and 12:31:50.601;
- the default profile's `Local State` and `DevToolsActivePort` were modified in
  the same interval;
- the canonical Cargo scope began later, at 12:31:11, and the repaired parity
  partition did not run until after the Chromium scopes had exited.

This overlap prevents treating the whole canonical window as isolated
profile-side-effect evidence. It does not invalidate the canonical test
results or the subsequent isolated parity proof. The tester did not initiate,
stop, repair, or clean the external installed runtime or its profile.

A later closeout readback found no Cargo, rustc, or `rust-tests.sh` process from
this retest. It did find an external Chromium tree rooted at PID 28406. That
browser started at 12:35:30, after the focused parity command had finished, as
a child of pre-existing daemon PID 95551 whose working directory is
`workspace.local/last30days-skill`. It uses the distinct uppercase profile
`runtime-profiles/Default/user-data`. The external runtime-convergence workflow
also restarted independently at 12:34:18 and systemd registered PID 28406's
Chromium scope at 12:35:39. The tester did not stop or modify that out-of-scope
browser. Approximately 38 GiB RAM remained available, swap use remained
approximately 4.3 of 32 GiB, and no compiler memory excursion occurred.

The pre-existing untracked Last30Days handoff, Plan 0108, and actions-inventory
target remain excluded from Candidate 4 identity. This receipt remains the
only file edited by the final tester.

### Final bounded retest disposition

Both accepted blockers are closed at exact HEAD
`2480f11f54232e7c22e5b0edef1cdfbe5ea8dc4c`. Candidate 4 is **PASS** and
**commit-ready**. The concurrent installed-runtime activity is an operator
environment warning, not a Candidate 4 code blocker. No third audit and no
additional canonical rerun are authorized or required.
