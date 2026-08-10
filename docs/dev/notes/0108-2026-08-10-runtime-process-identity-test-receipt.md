# Plan 0108 Runtime Process Identity Final Test Receipt

Date: 2026-08-10

Role: distinct final tester after Cycle 2

Verdict: **FAIL**

Commit disposition: **HEAD is not commit-ready.**

Exact tested HEAD:
`2883dd0642ecf7ddfa4c484f79fda146733bdfcc`

## Scope and authority

This receipt independently tested Plan 0108 after the terminal Cycle 2 PASS.
It used the committed plan, current source, the untracked terminal work-audit
note, and executable local gates. It did not reopen work audit or edit source,
plan, or audit files.

The tester edited exactly this receipt. The pre-existing untracked work-audit
note and generated actions-inventory target were read or excluded as
appropriate and were not edited:

- `docs/dev/notes/0108-2026-08-10-runtime-process-identity-work-audit.md`
- `scripts/architecture/actions-inventory/target/`

No browser, installer, doctor, installed-runtime read, live-runtime read,
route, display, X session, real profile, `last30days-facebook`, or PID 63205
operation was performed. Every compiling Cargo command used
`scripts/ci/cargo-safe.sh` directly or through `scripts/ci/rust-tests.sh`.

The repo's Graphiti discovery procedure was considered but not invoked because
its required runtime doctor and live memory query conflict with this retest's
explicit no-doctor and no-live-runtime boundary. Committed repo authority
controlled.

## Blocking finding

### `P0108-T1-01` — canonical control-plane status integration exposes a dead synthetic browser as modeled

Status: **FAIL, blocking**

Criterion: Plan 0108 acceptance requires the focused and canonical no-launch
Rust gates to pass. A process-identity repair must preserve the complete
Service Status and Browser Session Authority contract, not only its focused
ownership fixtures.

Evidence: the one authorized `scripts/ci/rust-tests.sh` run passed its complete
parallel-safe lane and the first 43 serialized filters. Serialized filter 44,
`native::control_plane::tests`, then failed exactly one test:

`native::control_plane::tests::service_status_response_combines_worker_and_service_state`

Exact partition result:

- 31 passed;
- 1 failed;
- 0 ignored;
- 1,869 filtered.

The failed assertion is at `cli/src/native/control_plane.rs:2403`. The fixture
supplies a synthetic service-browser PID of `2147483647`, which is not a live
browser owner. It expects
`/data/browserSessionAuthority/summary/modeledBrowserCount` to be `0`; the
current response returns `1`.

A single focused no-launch reproduction through `cargo-safe.sh` failed the
same assertion:

- left: `Some(1)`;
- right: `Some(0)`;
- 0 passed, 1 failed, 1,900 filtered.

Consequence: the focused process-identity, runtime-profile, service-health, and
handoff fixtures are green, but the production control-plane integration does
not preserve its dead-browser projection contract. The canonical driver is
red and stopped before its final five serialized filters.

Reproducer:

```bash
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml \
  native::control_plane::tests::service_status_response_combines_worker_and_service_state \
  -- --test-threads=1
```

Confidence: high. The exact canonical partition and focused reproduction agree.

Disposition: `blocking`. Reconcile process-identity-aware service-browser
reconciliation with Browser Session Authority projection, then rerun the
focused test and one complete canonical driver. Do not weaken the assertion or
reintroduce PID-only ownership.

## Canonical Rust ledger

The tester ran the canonical driver once. No canonical retry was started.

Parallel-safe lane:

- 1,085 passed;
- 0 failed;
- 57 ignored;
- 759 filtered;
- 1,142 scheduled.

Serialized execution before the stop:

- filters 1 through 43 passed, totaling 635 tests;
- filter 44 passed 31 and failed 1;
- filters 45 through 49 did not run after the fail-closed stop;
- the 92 unrun tests are parity 18, policy 11, providers 4,
  service-health 44, and runtime-profile 15.

Whole canonical invocation at stop:

- 1,751 passed;
- 1 failed;
- 57 ignored;
- 92 not run because the driver stopped on the failure.

The final successful pre-failure partition was
`native::cdp::chrome::tests`, 76 passed. The control-plane failure is the only
observed canonical failure.

## Focused Plan 0108 validation

All 135 prescribed focused Rust tests passed through the WSL-safe wrapper:

| Partition | Passed | Failed | Coverage highlights |
| --- | ---: | ---: | --- |
| `process_identity::tests` | 16 | 0 | exact identity, executable and family mismatch, observation failure, pidfd binding, final-signal replacement, Windows parser, macOS fail-closed policy |
| `runtime_profile::tests` | 15 | 0 | reused PID status, cleanup and close refusal, manual no-CDP ownership, unrelated endpoint rejection, ephemeral-port legacy compatibility |
| `native::service_health::tests` | 44 | 0 | reused PID rejection and live default, named, and custom service-browser compatibility |
| `native::action_runtime::runtime::route_host_tests` | 60 | 0 | matching and mismatching non-runtime handoff identity, attach and route-host ownership contracts |

Specific required fixtures passed:

- `test_runtime_status_rejects_reused_unrelated_pid`;
- `pidfd_signal_is_bound_to_the_captured_test_process`;
- `replacement_at_final_signal_boundary_is_refused_before_effect`;
- `test_profile_consistent_legacy_browser_with_ephemeral_port_retains_compatibility`;
- `refresh_accepts_live_default_named_and_custom_service_browsers`;
- `refresh_rejects_live_unrelated_reused_pid_as_browser_owner`;
- `test_no_runtime_profile_handoff_identity_matches_at_resume_boundary`;
- `test_no_runtime_profile_handoff_identity_mismatch_is_rejected_before_resume`.

These focused passes establish the local decision and consumer fixtures but do
not supersede `P0108-T1-01` at the production control-plane seam.

## Structural and quality gates

Passed:

- CLI `cargo fmt` check through `cargo-safe.sh`;
- CLI strict Clippy with `-D warnings` through `cargo-safe.sh`;
- actions architecture gate:
  `definitions=6 tests=0 lines=902 wrappers=0 final=true`;
- actions remediation structural regression gate;
- WSL Cargo entrypoint safety gate;
- all eight route-confusion no-launch gates;
- `pnpm validation:select -- --base 578f5d15`;
- `git diff --check 578f5d15..HEAD`;
- worktree `git diff --check` before writing this receipt.

The remediation gate recursively inventories process-identity and runtime
assessment consumers, direct `libc::kill`, `TerminateProcess`, and `taskkill`
paths, Windows synchronization rights, macOS signal policy, platform command
line adapters, service identity schema, and handoff identity schema. It passed
at the exact tested HEAD.

The selector reported 23 paths because it included the untracked Cycle 2 audit
alongside 22 tracked Plan 0108 paths. Its workstation install and provisioning
recommendations were intentionally withheld by the explicit no-install and
no-live boundary. Other selector recommendations do not override the already
proven canonical blocker.

## Ownership-contract readback

Current source provides the four frozen ownership outcomes:

- `MatchingBrowser`;
- `Missing`;
- `ReusedUnrelated`;
- `AmbiguousLegacyBrowser`.

Observation failure maps conservatively to ambiguous. Exact ownership requires
the recorded PID and start token plus observed executable and browser-family
consistency. Linux termination binds authorization to a pidfd. Windows retains
one verified process handle. macOS attached-runtime signaling fails closed.
CodeGraph reported seven direct callers of `assess_process_ownership` and six
production callers of `runtime_process_assessment`, all within the declared
consumer surfaces.

The structural and focused evidence supports the local ownership model. The
control-plane failure shows that downstream status projection remains
incompletely reconciled.

## Exact source identity

Campaign base: `578f5d15`

Exact HEAD: `2883dd0642ecf7ddfa4c484f79fda146733bdfcc`

Source commit chain:

- `58e5f19cba8f78b50cfae8b92970075992551f80` — red PID-reuse fixture;
- `c6bf83949ead85878d68612d3cd588cc6af3b8c4` — shared process identity;
- `7ae3ae972d113f958741c67382029a93e573f75f` — ephemeral DevTools proof;
- `2883dd0642ecf7ddfa4c484f79fda146733bdfcc` — cross-platform ownership closure.

The sorted current-content source manifest contains exactly 20 paths:

```text
cli/Cargo.toml
cli/src/main.rs
cli/src/native/action_runtime/runtime/cdp_free_plan.rs
cli/src/native/action_runtime/runtime/daemon.rs
cli/src/native/action_runtime/runtime/launch.rs
cli/src/native/action_runtime/runtime/navigation.rs
cli/src/native/action_runtime/runtime/recovery.rs
cli/src/native/action_runtime/runtime/remote_headed.rs
cli/src/native/action_runtime/runtime/route_host_tests.rs
cli/src/native/cdp/chrome.rs
cli/src/native/remote_view.rs
cli/src/native/remote_view/open/shared.rs
cli/src/native/service_config.rs
cli/src/native/service_health.rs
cli/src/native/service_health/action_helper_tests.rs
cli/src/native/service_model.rs
cli/src/native/service_resources.rs
cli/src/process_identity.rs
cli/src/runtime_profile.rs
scripts/test-actions-remediation-architecture.js
```

The SHA-256 of the sorted newline-delimited `sha256sum` stream for those 20
paths is:

`14d72ff5c5f440775bf200d8099d42b2d085267dd293270ca02940c7e9721d48`

The complete tracked base-to-HEAD binary patch SHA-256, including the tracked
plan and originating handoff, is:

`219bafcbbe001f7675c019ca360aa6608d64c2cc82374e781ed59a9a8f169002`

Authority hashes:

- Plan 0108:
  `b2545918bf68fcae845051db314dbadf1b330093f87156ccffee28274deafda5`
- terminal Cycle 2 work audit:
  `fcb1124b87edf5025686640c310111ef93687f87df0ec65138b17204f2c03780`

The untracked audit, generated target, and this receipt are excluded from the
source identity.

## Cross-platform and effect residuals

Native Windows and macOS binaries were not executed by this Linux/WSL final
tester. The passing Windows command-line parser and macOS fail-closed policy
tests are platform-neutral unit evidence, not native-platform proof. The Cycle
2 audit's Windows cross-target attempt stopped in the `ring` dependency because
the host lacked `lib.exe`; it is neither a project failure nor a Windows pass.
Native Windows and macOS CI remain residual.

No claim is made about installed runtime state, a live browser, PID 63205,
Last30Days acceptance, real profile contents, route state, display state, or X
state. Those surfaces were outside authority and were not read.

## Process and memory closeout

Every compiler invocation reported the WSL aggregate guard:
`jobs=4`, `MemoryHigh=20G`, `MemoryMax=24G`, and `MemorySwapMax=4G`.

Available host memory remained approximately 38 to 39 GiB. At closeout, swap
use was approximately 6.3 of 32 GiB. No Cargo, rustc, or `rust-tests.sh`
process from this validation remained.

## Final disposition

Plan 0108 at exact HEAD
`2883dd0642ecf7ddfa4c484f79fda146733bdfcc` is **FAIL** and **not
commit-ready** because `P0108-T1-01` deterministically fails the canonical
control-plane contract. Repair that one integration seam, preserve the green
ownership and no-signal invariants, then perform a bounded focused reproduction
and one canonical retest. No additional work-audit cycle is authorized or
required by this receipt.

## Superseding bounded T1-01 repair retest

Date: 2026-08-10

Exact retest HEAD:
`c54150f25aad9a0d1e62f693ab45eb86a2549a1a`

Parent HEAD:
`2883dd0642ecf7ddfa4c484f79fda146733bdfcc`

Retest verdict: **FAIL**

Commit disposition: **not commit-ready**

This section supersedes the preceding final disposition for the repair HEAD.
The original receipt remains intact as the historical record of the failure at
its exact source identity.

### T1-01 closure

The bounded repair commit changes only
`cli/src/native/service_health.rs`. Legacy service-browser rows without a
persisted process identity now retain endpoint-health compatibility only while
the recorded PID is observed live. Missing or unobservable legacy PIDs receive
the typed process assessment and can no longer remain modeled as live solely
through a stale endpoint projection.

The two prescribed focused commands passed under `cargo-safe.sh`:

- exact former control-plane failure: 1 passed, 0 failed, 1,900 filtered;
- `native::service_health::tests`: 44 passed, 0 failed, 1,857 filtered.

This closes `P0108-T1-01` at the repair HEAD.

### Complete canonical retest

Exactly one new complete `scripts/ci/rust-tests.sh` invocation was started from
the beginning after the source change. It failed in the parallel-safe lane:

- passed: 1,084;
- failed: 1;
- ignored: 57;
- filtered out: 759;
- total scheduled in that lane: 1,142.

The failing test was:

`native::remote_view_handoff::tests::rollback_failure_restores_lease_and_summarizes_cleanup`

Because the parallel-safe lane exited nonzero, the driver did not reach its 44
serialized filters. No second or third canonical run was started.

A single no-live focused reproduction of that exact test passed: 1 passed, 0
failed, 1,900 filtered. The test uses a process-and-nanosecond-qualified
temporary JSON path and passed in isolation, so the current evidence identifies
parallel-state nondeterminism rather than a deterministic local assertion
failure. It does not convert the red canonical run into a pass.

This is the new blocking finding `P0108-T2-01`: the required complete canonical
driver is not green at the exact repair HEAD. The relevant handoff test or its
shared state must be made parallel-safe or moved into the driver's serialized
lane, followed by one fresh complete canonical run.

### Repair-head quality checks

The source change was also checked with:

- strict Rust formatting: passed;
- strict Rust clippy with warnings denied: passed;
- actions remediation architecture gate: passed;
- tracked base-to-HEAD diff check: passed;
- worktree diff check: passed.

The full actions architecture, WSL guard, route-confusion, and validation
selector results recorded earlier were not repeated because the bounded repair
changed only `service_health.rs`; no result from the predecessor HEAD is used to
override the failed canonical result.

### Corrected repair-head source identity

Campaign base: `578f5d15`

Exact HEAD: `c54150f25aad9a0d1e62f693ab45eb86a2549a1a`

The source commit chain now additionally contains:

- `c54150f25aad9a0d1e62f693ab45eb86a2549a1a` — reconcile legacy service
  browser liveness.

The sorted current-content source manifest still contains exactly 20 paths,
the same paths listed in the preceding identity section. Its corrected SHA-256
is:

`923c867774aebfbfabdb96e8e2bb9328f30c940a5f700ff57f8cab46bc6f3262`

The complete tracked base-to-repair-HEAD binary patch SHA-256 is:

`03b77f8c36a973982134edfbab9b688b653f5ed4296fcb37931e1729991d5a18`

The bounded parent-to-repair binary patch SHA-256 is:

`aceacd675239be3bbb837ccf8bb88149d4c5d82540240f5376ef97e5ba4c2891`

Authority hashes remain:

- Plan 0108:
  `b2545918bf68fcae845051db314dbadf1b330093f87156ccffee28274deafda5`;
- terminal Cycle 2 work audit:
  `fcb1124b87edf5025686640c310111ef93687f87df0ec65138b17204f2c03780`.

The untracked audit, generated inventory target, and this receipt remain
excluded from candidate identity.

### Effect, process, and platform closeout

All Rust compilation used the serialized WSL wrapper with four Cargo jobs and
aggregate `MemoryHigh=20G`, `MemoryMax=24G`, and `MemorySwapMax=4G` controls.
No browser, install, doctor, live runtime, profile, Last30Days, PID 63205, route,
display, or X effect or readback was performed.

At final closeout, approximately 38 GiB host memory was available, swap use was
approximately 6.3 of 32 GiB, and no Cargo, rustc, or `rust-tests.sh` process
remained. Native Windows and macOS execution remains an explicit residual; no
native-platform pass is inferred from Linux/WSL evidence.

### Superseding final disposition

Plan 0108 at exact HEAD
`c54150f25aad9a0d1e62f693ab45eb86a2549a1a` is **FAIL** and **not
commit-ready**. The intended `P0108-T1-01` repair is verified, but the sole
complete canonical retest is red on `P0108-T2-01`. A fresh canonical pass is
required after that parallel-state failure is repaired or correctly serialized.

## Superseding T2-01 harness retest

Date: 2026-08-10

Exact retest HEAD:
`3f0c03dce2e314489617db634641b057e38ac8c4`

Parent HEAD:
`c54150f25aad9a0d1e62f693ab45eb86a2549a1a`

Final verdict: **PASS**

Commit disposition: **ready from the Plan 0108 final-test boundary**

This section supersedes the preceding final disposition for the harness-repair
HEAD. The earlier sections remain the immutable history of their exact failed
candidate identities.

### T2-01 closure and complete canonical result

The harness repair changes only `scripts/ci/rust-tests.sh`, adding
`native::remote_view_handoff::tests` to the driver's serialized filters.
`bash -n` accepted the exact driver, whose filter inventory now contains 50
serialized partitions.

Exactly one fresh complete canonical `scripts/ci/rust-tests.sh` invocation ran
at this HEAD. It finished with exit code 0. The parallel-safe lane reported:

- scheduled: 1,098;
- passed: 1,041;
- failed: 0;
- ignored: 57;
- filtered out: 803.

The preceding failed candidate scheduled 1,142 tests in that lane. The exact
44-test reduction, together with the driver's explicit skip filter, verifies
that `native::remote_view_handoff::tests` was absent from parallel execution.
The driver then ran that module as a serialized partition: 44 passed and 0
failed, including
`rollback_failure_restores_lease_and_summarizes_cleanup`. All 50 serialized
partitions finished successfully.

The repaired Plan 0108 integration remained green inside the same canonical
run: `native::control_plane::tests` passed 32 of 32,
`native::service_health::tests` passed 44 of 44, and
`runtime_profile::tests` passed 15 of 15. No focused rerun or second canonical
run was started.

This closes `P0108-T2-01`. The canonical driver is green at the exact harness
repair HEAD.

### Proportionate final checks

Both the campaign base-to-HEAD and worktree diff checks passed. The canonical
driver itself exercised and proved the edited shell path, and `bash -n` passed.
Rust formatting and strict clippy were not repeated because this commit changes
no Rust source; their passing result at the direct parent remains applicable to
the unchanged Rust tree.

### Exact final source identity

Campaign base: `578f5d15`

Exact HEAD: `3f0c03dce2e314489617db634641b057e38ac8c4`

Harness repair commit:

- `3f0c03dce2e314489617db634641b057e38ac8c4` — serialize remote-view
  handoff fixtures.

The sorted current-content source manifest contains exactly 21 paths:

```text
cli/Cargo.toml
cli/src/main.rs
cli/src/native/action_runtime/runtime/cdp_free_plan.rs
cli/src/native/action_runtime/runtime/daemon.rs
cli/src/native/action_runtime/runtime/launch.rs
cli/src/native/action_runtime/runtime/navigation.rs
cli/src/native/action_runtime/runtime/recovery.rs
cli/src/native/action_runtime/runtime/remote_headed.rs
cli/src/native/action_runtime/runtime/route_host_tests.rs
cli/src/native/cdp/chrome.rs
cli/src/native/remote_view.rs
cli/src/native/remote_view/open/shared.rs
cli/src/native/service_config.rs
cli/src/native/service_health.rs
cli/src/native/service_health/action_helper_tests.rs
cli/src/native/service_model.rs
cli/src/native/service_resources.rs
cli/src/process_identity.rs
cli/src/runtime_profile.rs
scripts/ci/rust-tests.sh
scripts/test-actions-remediation-architecture.js
```

The SHA-256 of the sorted newline-delimited `sha256sum` stream for those 21
paths is:

`4ca91d7b09263c62ab5ce16c931cd7cee6a3409381e37c731cfb9403fbcd184a`

The complete tracked base-to-final-HEAD binary patch SHA-256 is:

`781e3b0f95056ea7c1affda8f2686fc72998ec3be578f942d054f5e8578a6666`

The bounded parent-to-harness-repair binary patch SHA-256 is:

`52cc83d732640a0e0aee520b2625e512cc73bea22246cfcd2ed3e818e20160c9`

Authority hashes remain:

- Plan 0108:
  `b2545918bf68fcae845051db314dbadf1b330093f87156ccffee28274deafda5`;
- terminal Cycle 2 work audit:
  `fcb1124b87edf5025686640c310111ef93687f87df0ec65138b17204f2c03780`.

The untracked audit, generated inventory target, and this receipt remain
excluded from candidate identity.

### Final effect, process, and platform closeout

Every Cargo invocation in the canonical driver used the serialized WSL wrapper
with four Cargo jobs and aggregate `MemoryHigh=20G`, `MemoryMax=24G`, and
`MemorySwapMax=4G` controls. No browser, install, doctor, live runtime, profile,
Last30Days, PID 63205, route, display, or X effect or readback was performed.

At closeout, approximately 38 GiB host memory was available, swap use was
approximately 6.3 of 32 GiB, and no Cargo, rustc, or `rust-tests.sh` process
remained. Native Windows and macOS execution remains a residual and is not
claimed by this Linux/WSL receipt.

### Superseding final disposition

Plan 0108 at exact HEAD
`3f0c03dce2e314489617db634641b057e38ac8c4` is **PASS** and **commit-ready
from the final-test boundary**. Both prior blockers are closed, the complete
canonical driver passes with the remote-view handoff fixtures serialized, and
no authorized validation remains outstanding. Native Windows and macOS
execution remains the documented residual rather than an invented pass.
