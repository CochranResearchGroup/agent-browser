# Plan 0108 Runtime Process Identity Work Audit

Date: 2026-08-10  
Cycle: 1 of at most 2  
Role: independent implementation evaluator  
Reviewed base and HEAD: `58e5f19cba8f78b50cfae8b92970075992551f80`  
Verdict: **REWORK REQUIRED**

## Scope and identity

This review evaluated the current uncommitted Plan 0108 repair against the frozen contract in `docs/dev/plans/0108-runtime-process-identity-and-pid-reuse-hardening.md` and the originating handoff in `docs/dev/notes/2026-08-10-last30days-stale-runtime-pid-lock-handoff.md`. The review covered deletion safety, no-signal safety, legacy compatibility, identity capture before observable ownership, every indexed browser-ownership consumer, Linux, macOS, and Windows adapter behavior, tests, and critical regressions.

The reviewed worktree had 13 modified Rust files, one untracked source file at `cli/src/process_identity.rs`, and the pre-existing untracked generated cache at `scripts/architecture/actions-inventory/target/`. `git diff --check` passed. CodeGraph was healthy with 535 indexed files, 18,313 nodes, and 63,308 edges. `cli/src/runtime_profile.rs` was pending during the review and was therefore read directly; its reviewed SHA-256 was `8f6632aace165ce17da1505f43acc7f32777be68d950e8a11f156ab040392722`.

No Cargo command, browser, installer, doctor, live runtime read, signal, or operation against `last30days-facebook` or PID 63205 was performed.

## Finding ledger

### P0108-W1-01: No-signal observation and signal authorization are not PID-reuse safe

Status: **FAIL, blocking**  
Criterion: identity observation must not signal or mutate a process, platform observation failures must classify conservatively, and an authorized signal must not be deliverable to a different process after PID reuse.  
Evidence: `cli/src/process_identity.rs:117` uses `libc::kill(pid, 0)` in Linux and macOS `process_exists`. Signal zero normally carries no payload, but it is still the signal API forbidden by the frozen no-signal adapter contract. On Windows, failure to open the process is returned as absence and becomes `Missing`, rather than a conservative ambiguous observation failure. `cli/src/native/action_runtime/runtime/launch.rs:282` assesses identity and then later invokes Unix `kill` or Windows `taskkill` by PID; process exit and PID reuse can occur between those operations.  
Consequence: a permission or metadata failure can be reported as missing, and the final termination operation can target a replacement process despite an earlier identity match. The central safety claim is therefore not established.  
Reproducer: inject an observer access failure and assert an ambiguous, non-destructive result; inject process replacement between final assessment and the signaling primitive and assert that the replacement process receives no signal.  
Confidence: high.  
Required disposition: replace signal-based observation with metadata-only platform APIs. Introduce a termination authorization primitive that remains bound to the observed process instance through the operation, such as a Linux pidfd and a retained Windows process handle with creation-time verification, plus a macOS equivalent or a fail-closed platform strategy. Map permission and metadata failures to an explicit ambiguous result. Add deterministic fault-injection tests for observation failure and replacement at the final signal boundary.

### P0108-W1-02: Exact-token matching omits executable and browser-family identity

Status: **FAIL, blocking**  
Criterion: an exact recorded start token must also remain consistent with the recorded executable and browser family; caller expectations must not manufacture observed identity.  
Evidence: `cli/src/process_identity.rs:68` returns `MatchingBrowser` immediately when start tokens match, without validating executable identity or browser family. `cli/src/process_identity.rs:31` prioritizes the caller-supplied expected family and may fall back to the expected executable when observation is incomplete. Existing tests then capture the current nonbrowser test process with expected family `chrome` and treat it as a matching browser.  
Consequence: the same PID and start token can remain authorized after an executable transition, and a caller can label an unrelated process as a managed browser. This weakens every status, lock, attach, and termination consumer.  
Reproducer: assess equal PID and start token with a recorded Chrome executable and an observed nonbrowser executable or incompatible family; current behavior returns `MatchingBrowser`.  
Confidence: high.  
Required disposition: derive family only from observed executable metadata. Store the requested family separately if it is useful context. Require exact start token plus executable and family consistency for `MatchingBrowser`; classify a same-instance nonbrowser or incompatible family as reused or unrelated. Add tests for equal-token executable mismatch, equal-token family mismatch, and incomplete observed executable metadata.

### P0108-W1-03: Legacy compatibility is based on an unrelated endpoint, not PID and profile consistency

Status: **FAIL, blocking**  
Criterion: a legacy browser record may be upgraded only when a reachable DevTools endpoint is demonstrably owned by the observed automated browser and is consistent with the recorded runtime profile.  
Evidence: `cli/src/runtime_profile.rs:535` proves only that a localhost port answers `/json/list`. That Boolean is passed to `runtime_process_ownership` without binding the endpoint to the PID, executable, command line, user-data directory, or runtime profile. The legacy compatibility test copies `/bin/sleep` to a browser-looking filename and combines it with an unrelated test HTTP server; this is accepted as matching. `cli/src/native/service_health.rs:1678` can make the same upgrade for a service browser record.  
Consequence: an unrelated browser-looking process plus any responsive local DevTools server can acquire matching ownership, leading to false live status, attach, lock attribution, or termination authority.  
Reproducer: run a browser-looking sleeper and a separate HTTP server that returns a valid `/json/list`; current legacy assessment upgrades the sleeper to `MatchingBrowser`.  
Confidence: high.  
Required disposition: implement one shared legacy proof that binds observed PID metadata, browser family, command-line or user-data evidence, DevTools endpoint ownership, and the requested runtime profile. A merely responsive endpoint must remain ambiguous. Replace the false-positive test with negative unrelated-endpoint coverage and positive profile-consistent legacy automated-browser coverage.

### P0108-W1-04: Ownership consumers diverge and close can discard live ownership evidence

Status: **FAIL, blocking**  
Criterion: every ownership-sensitive consumer must use the shared decision consistently; ambiguous ownership may not be adopted, signaled, deleted, or silently discarded.  
Evidence: `cli/src/native/action_runtime/runtime/navigation.rs:203` permits `AmbiguousLegacyBrowser` during handoff resume, then connects and persists the browser without first proving PID and profile consistency. `cli/src/native/action_runtime/runtime/recovery.rs:1072` and `cli/src/native/remote_view.rs:484` pass endpoint reachability as false, so a legitimate reachable legacy record cannot satisfy compatibility in those paths. Service health uses the weaker arbitrary-endpoint proof. At `cli/src/native/action_runtime/runtime/navigation.rs:267`, close calls termination and then clears runtime state even when termination refuses or fails, removing evidence for a potentially live ambiguous browser.  
Consequence: the same record is adoptable in one path, rejected in another, and can lose its durable ownership record after a refused close. This creates regressions in handoff, remote-view reuse, recovery attach, diagnostics, and later safe cleanup.  
Reproducer: exercise an ambiguous legacy record through handoff resume, remote-view PID reuse, recovery attach, service health, and close; observe conflicting outcomes and runtime-state removal after termination refusal.  
Confidence: high.  
Required disposition: route all ownership-sensitive consumers through one typed assessment that carries its evidence and reason. Require a proven match before handoff adoption or attach. Preserve runtime state when close cannot prove and complete termination, returning a typed refusal or partial-close outcome. Add a table-driven consumer matrix covering matching, reused, missing, ambiguous legacy, and observation-failure outcomes across status, list, lock, diagnostics, service health, handoff, recovery, remote view, and close.

### P0108-W1-05: Required test and structural closure can produce a false green

Status: **FAIL, blocking**  
Criterion: Plan 0108 requires decision-table, platform-failure, no-signal, all-consumer, legacy-compatibility, capture-order, and deletion-boundary tests, plus a structural guard that prevents ownership logic from bypassing the shared helper.  
Evidence: current unit coverage includes token match and mismatch, basic legacy classifications, Linux stat parsing, selected lock cases, capture-order source checks, and a termination refusal test. It does not cover equal-token executable or family mismatch, metadata or permission failure, Windows and macOS adapter behavior, endpoint-to-PID/profile binding, the handoff ambiguity path, close-state preservation, or the final assessment-to-signal reuse race. The structural guard at `cli/src/process_identity.rs:402` searches eight files only for the literal `pid_is_running`; it cannot detect `kill(pid, 0)`, alternate liveness helpers, divergent endpoint upgrades, direct PID signaling, or a new ownership consumer outside its allowlist.  
Consequence: the suite can pass while the safety invariants in P0108-W1-01 through P0108-W1-04 remain broken.  
Reproducer: add any differently named liveness helper or direct PID signal to an ownership consumer; the current source-text guard remains green.  
Confidence: high.  
Required disposition: replace the substring guard with an exhaustive, deterministic consumer inventory or structural checker. Add the missing decision, adapter-failure, cross-platform, endpoint-binding, consumer-matrix, close-preservation, and final-signal-boundary tests. Require those tests to fail against the current implementation before accepting the repaired implementation.

## Verified positive evidence

- `RuntimeState.process_identity` is optional, preserving serialization compatibility for legacy state files.
- Managed and manual launch paths attempt identity capture before writing the new runtime state or returning managed ownership, and their failure paths terminate the newly spawned child.
- Profile-lock cleanup deletes disposable artifacts only for `Missing` or `ReusedUnrelatedProcess`; matching and ambiguous records are preserved and rejected.
- Authorization metadata is not used by the shared decision function to upgrade a start-token mismatch.
- The Linux start token includes boot identity and process start ticks; macOS and Windows adapters use process creation metadata rather than PID alone.

These positives are insufficient for acceptance because the shared decision, legacy proof, consumer closure, and final signaling boundary remain unsafe.

## Consolidated remediation and Cycle 1 disposition

Implement one cohesive repair that: (1) observes process identity without any signal API and represents observation failure explicitly; (2) derives executable and family from observation and requires them alongside the start token; (3) binds legacy DevTools proof to PID and runtime profile; (4) makes every ownership consumer use the same typed evidence and preserves state on refused close; (5) binds termination to the verified process instance; and (6) replaces the narrow text guard with exhaustive structural and fault-matrix coverage.

Acceptance requires every stable ID P0108-W1-01 through P0108-W1-05 to pass in the independent Cycle 2 review. This evaluator stops after Cycle 1. No compilation or runtime validation was authorized or performed, so the work is not ready for final tests.

## Cycle 2 closed-world verification

Date: 2026-08-10  
Mode: closed-world verification of P0108-W1-01 through P0108-W1-05  
Reviewed source chain: `c6bf83949ead85878d68612d3cd588cc6af3b8c4`, `7ae3ae972d113f958741c67382029a93e573f75f`, and final HEAD `2883dd0642ecf7ddfa4c484f79fda146733bdfcc`  
Verdict: **PASS, FINAL-TEST READY**

Cycle 1 named the frozen plan with an incorrect filename. The actual authority reviewed in both cycles is `docs/dev/plans/0108-2026-08-10-runtime-process-identity-pid-reuse-repair-plan.md`.

At final review, CodeGraph was current with 535 indexed files, 18,388 nodes, and 63,613 edges. The source worktree matched final HEAD except for this audit note and the excluded generated `scripts/architecture/actions-inventory/target/` cache. `git diff --check` passed.

### P0108-W1-01: PASS

Criterion: observation is metadata-only, failures are conservative, and termination cannot signal a PID replacement.  
Evidence: Linux observation reads `/proc`; termination opens a pidfd, verifies recorded identity, polls that descriptor, and signals through `pidfd_send_signal`. Windows opens one retained process handle with `PROCESS_QUERY_LIMITED_INFORMATION`, `PROCESS_TERMINATE`, and the correct windows-sys 0.52 constant `PROCESS_SYNCHRONIZE`; creation time and executable are read from that handle, waiting and termination use the same handle, and `Drop` closes it. macOS observes through `proc_pidinfo`, `proc_pidpath`, and `KERN_PROCARGS2`; attached-runtime PID signaling now returns a typed fail-closed error and never calls `kill`. Observation and access failures become `ProcessObservation::Failed`, which assesses as ambiguous.  
Consequence: the original wrong-process signal and false-missing paths are closed. macOS may preserve state and report a refusal instead of terminating by PID, which is the required safe outcome.  
Reproducer: `process_identity::tests` includes observation failure, replacement at the final signal boundary, Linux pidfd signaling, and the macOS no-effect policy.  
Confidence: high for Linux behavior and static interfaces; medium-high for Windows and macOS native behavior because those targets were not executed by this evaluator.  
Disposition: accepted.

### P0108-W1-02: PASS

Criterion: exact ownership requires start token, observed executable, and observed browser family; expected arguments cannot manufacture identity.  
Evidence: `capture_process_identity` requires observed start token, executable, and family, then rejects disagreement with expected executable or family. `assess_process_ownership` checks token and `recorded_executable_matches` before returning `MatchingBrowser`; missing executable evidence is ambiguous and disagreement is reused unrelated. Tests cover equal-token executable mismatch, family mismatch, missing executable evidence, and attempted relabeling of the current nonbrowser process.  
Consequence: a caller cannot bless an unrelated process merely by supplying Chrome expectations.  
Reproducer: the independently run `process_identity::tests` suite passed 16 of 16 tests.  
Confidence: high.  
Disposition: accepted.

### P0108-W1-03: PASS

Criterion: legacy automated compatibility must bind browser PID metadata, browser command line, user-data directory, and the reachable DevTools endpoint, including Chrome's ephemeral-port launch shape.  
Evidence: legacy probing is enabled only for a browser observation whose command line names the same user-data directory and requests either the assigned port or port zero. The assigned port still comes from the matching profile state or `DevToolsActivePort`, and `/json/list` must respond before compatibility is upgraded. Linux reads `/proc` command-line metadata, macOS parses `KERN_PROCARGS2`, and Windows queries `ProcessCommandLineInformation`. The unrelated-endpoint fixture remains ambiguous, while the profile-consistent fixture launches with `remote-debugging-port=0`, stores the assigned nonzero port, and passes.  
Consequence: a random local endpoint cannot upgrade an unrelated browser-looking process, while real Chrome ephemeral-port legacy state remains compatible.  
Reproducer: the independently run profile-consistent ephemeral-port test passed 1 of 1; the platform command-line parsers also passed inside the 16-test identity suite.  
Confidence: high on Linux and static parser logic; medium-high on native Windows and macOS metadata calls.  
Disposition: accepted.

### P0108-W1-04: PASS

Criterion: ownership consumers use typed evidence consistently, non-runtime handoff remains supported, ordinary service browsers do not reinterpret service profile IDs as runtime profiles, and refused close preserves durable state.  
Evidence: CodeGraph reports seven callers of `runtime_process_assessment`, all within the declared runtime consumers. Handoff preparation captures optional recorded process identity before relinquishing ownership; non-runtime preparation fails if identity is unavailable, and resume assesses the descriptor identity before CDP adoption. Service state now keeps an optional `browser_process_identities` map keyed by browser record ID. Service health assesses that recorded identity directly; legacy records without it retain endpoint-based health behavior instead of treating `profile_id` as a runtime-profile name. The identity is captured at the indexed ownership-establishment callers, including launch, verified attach, and verified handoff resume. Close clears runtime state only after `browser_shutdown_confirmed`; refusal restores attached ownership fields and preserves the state file.  
Consequence: default, named, custom-path, runtime-profile, and non-runtime handoff lanes no longer receive contradictory ownership outcomes. Ambiguous or mismatched records are neither adopted nor silently discarded.  
Reproducer: independently run checks passed the default, named, and custom service-browser fixture 1 of 1 and the non-runtime handoff match and mismatch fixtures 2 of 2. The existing refused-close fixture preserves durable state.  
Confidence: high.  
Disposition: accepted.

### P0108-W1-05: PASS

Criterion: tests and structural enforcement must cover the repaired decision table, platform failures, final signaling, legacy proof, service and handoff consumers, close preservation, and exhaustive ownership/direct-process inventories.  
Evidence: `scripts/test-actions-remediation-architecture.js` recursively inventories all Rust files that reference process identity or runtime assessment and compares them with an exact declared consumer map. It separately inventories direct `libc::kill`, `TerminateProcess`, and `taskkill` paths; rejects wrong Windows synchronization rights, PID-only taskkill, and a naked macOS signal in the identity module; requires both platform command-line adapters; forbids service profile-to-runtime-profile reinterpretation; and requires the service and handoff identity schemas. The focused suites cover the original missing decision, pidfd, parser, port-zero, service-browser, and handoff cases.  
Consequence: the Cycle 1 false-green gap is closed for current source, and a new undeclared consumer or direct signal path fails the deterministic repository gate.  
Reproducer: the structural gate passed independently. Guarded focused tests passed 16 identity tests, 1 service-browser compatibility test, 2 handoff tests, and 1 profile-consistent ephemeral-port test.  
Confidence: high for current-source inventory and Linux execution; medium-high for native cross-platform execution.  
Disposition: accepted.

## Validation and residuals

Every evaluator Cargo invocation used `scripts/ci/cargo-safe.sh` serially. The wrapper reported `MemoryHigh=20G`, `MemoryMax=24G`, and `MemorySwapMax=4G`. Available host memory was about 39 GiB before focused compilation and 38 GiB afterward; about 25 GiB of swap remained free. No Cargo or Rust compiler process remained after validation.

No browser, installer, doctor, ignored end-to-end test, live runtime read, signal to an operator process, real-profile mutation, or operation against `last30days-facebook` or PID 63205 occurred.

Nonblocking residual: Windows and macOS native execution was not available in this evaluator environment. The Windows cross-target attempt reported by the implementation lane stopped in the `ring` dependency before project source because the host lacked `lib.exe`; it is not a project-source failure or a native-platform pass. Static dependency and API review confirmed windows-sys 0.52 exports the selected constants and WDK command-line function, and platform-neutral parser/policy fixtures passed. Native Windows and macOS CI remain appropriate final validation, but this residual does not reopen any accepted Cycle 1 finding.

Cycle 2 is complete. P0108-W1-01 through P0108-W1-05 all pass. No Cycle 3 is authorized or required.
