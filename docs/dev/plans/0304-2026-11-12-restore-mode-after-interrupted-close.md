# Restore previous mode after an interrupted close

Status: OPEN
Product lane: PL-BUGFIX
Consolidation: required
Owner: root
Source baseline: 4f0b61959b97f0739374831a24cb0a0342998a02
Branch: fix/mode-restoration-published-profile
Original integrated branch: fix/restore-mode-after-original-exit

## Current State

The operator resumed completion of the custody fix and installed LinkedIn retest. The branch includes current main 32d3e315b26818c9ad099548e46ccfcb9be7762e, preserving the newer Native close repair. The actual host-path regression reproduces mode_operation_launch_custody_readback_required with zero launches. The scoped published-profile repair now passes it: previous CDP mode selected, one coordinator launch entry and deliberate refusal by the synthetic build selector before any browser process. All 25 focused tests and formatting pass; strict workspace Clippy passes. A new production artifact and installed recovery/content acceptance remain pending. Earlier installed outcomes below remain historical, not current completion proof.

## Consolidated batch

Provide explicit previous-mode restoration after an interrupted close only when the exact original process is positively exited. Preserve immutable requested mode, process/profile/task fences, replacement reconciliation and absence checks, confirmation and single-launch semantics. No global control release, forced close, profile reset or unrelated peer effects. The operator subsequently authorized PR review, merge, preserving deployment and a LinkedIn product retest.

## Delivery sequence and budget

One failing durable-coordinator regression, one targeted implementation, affected local checks. Source qualification does not establish installed recovery or successful consumer acquisition. The authorized delivery continuation builds one production-shaped sealed candidate, preserves shared runtime continuity through the supported installer, restores the original scoped operation and retests the consumer product path.

## Worker assignments

Root owns the serialized diagnosis, source change and local evidence. No delegation or new worktree.

## Evidence and exit

Require RED before implementation and GREEN afterward, including a live-original refusal and no obsolete-mode launch. Run affected coordinator coverage and required source gates before claiming merge readiness. The consumer acquisition probe remains the installed acceptance oracle and must be rerun after separately governed installation. No claim that source tests fix the current installed profile.

## Qualified source outcome

The fresh exact-original exit check now runs before restoration phase admission.
Live originals remain refused. No requested-mode history is rewritten, no close
is repeated and no obsolete requested-mode replacement is launched.

| Requirement | Evidence | Remaining gate |
| --- | --- | --- |
| Actual acquisition failure | Same private stable-task probe fails with browser_profile_operation_pending under product config20.068s and default config8.699s | Installed acquisition still blocked |
| Actual recovery failure | Public restore_previous returns mode_operation_phase_conflict with Closing and 0 launches | Installed recovery still blocked |
| Regression RED | cargo-safe.sh test --manifest-path Cargo.toml -p agent-browser --test browser_mode_operation restore_previous_after_interrupted_close_requires_proven_original_exit -- --exact --nocapture:1failed on phase conflict | None for RED |
| Coordinator GREEN | cargo-safe.sh test --manifest-path Cargo.toml -p agent-browser --test browser_mode_operation -- --test-threads=1:17passed | Public host/runtime acceptance remains separate |
| Strict source checks | cargo-safe.sh clippy --workspace --manifest-path Cargo.toml -- -D warnings:passed; fmt --all --manifest-path Cargo.toml -- --check:passed | No merge readiness claim |
| Documentation | pnpm test:documentation-links and git diff --check:passed | None for these checks |

Commands ran with AGENT_BROWSER_CARGO_CACHE=off after the first cache client
stalled before compilation; that owned attempt was terminated with143 and is
preserved separately. RED setup/build took121.508s; GREEN target took114.850s;
strict Clippy took200.236s including admission waiting. Test execution itself is
provider-free; no hosted CI, browser or native datastore is required.

Base commit is4f0b61959b97f0739374831a24cb0a0342998a02 plus the uncommitted
branch-local patch. Private Cargo Signal receipts bind the argv and results;
source/test/diff digests are retained in the consumer's private evidence.
No temporary debug instrumentation was added. The named acquisition probe is
retained as an explicit diagnostic artifact. The installed original-consumer
reproducer remains RED because this source packet does not replace the shared
runtime. The next gate is a preserving qualified runtime build/adoption and
original task restoration followed by the actual product acquisition probe.
This is a source-qualified repair, not installed or end-to-end completion.

## Delivery review | 2026-11-12

Review baseline:4f0b61959b97f0739374831a24cb0a0342998a02. Standards and Spec axes were reviewed serially in the primary context against the source packet, domain contract, testing discipline and shared-runtime effect policy. Standards:0accepted blocking findings. Spec:0accepted blocking findings. The fresh exact-original observation propagates uncertainty, leaves live originals held, and preserves the existing confirmed restoration path and immutable requested mode. The regression demonstrably failed before the fix and passes after it. This dependency review does not consume the Contacts program's reserved final review.

Passing local checks are the CI evidence for this packet. Hosted GitHub Actions are skipped under operator cost policy. The local target is Linux/WSL with the repository Cargo resource wrapper and cache disabled; cross-platform hosted execution is not claimed. Product code and test bytes are unchanged from the17-test/strict-Clippy qualification. Only delivery status documentation changed afterward.

## Installed recovery finding and bounded remediation

PR651 merged at984f092fe6a0a7e6e37ea6fcd782dbde4372ec28. Its production-shaped artifact8615a01d94e4cf7891f8c2ca06531c733ec570470111f5d7e10744b21bd576bc passed the source-free workstation fixture and was adopted through an accepted preserve transaction. All 10 pre-apply browser roots survived unchanged. An optional older no-launch session smoke failed its unregistered synthetic-session premise; it is retained as failed evidence, not counted as passing validation. The monitor's first startup lock timeout recovered naturally; no backoff override or unrelated maintenance was performed.

Actual confirmed restoration advanced Closing to OriginalExited without a replacement launch, proving the first repair's effect. It then exposed browser_login_mode_conflict. The exact retained journal has restoring_previous=true, effective_mode=None and 0 launch attempts. The selected profile binding is confirmed by its private owner-wait evidence; no alternate-profile fallback is selected.

Accepted blocking finding RECONCILE_SCOPE: the read-only manual replacement reconciliation path consults a guard that admits only post-launch phases. Restoration must inspect possible replacements before a launch. Keep launch admission unchanged and add a separately scoped reconciliation admission for exact retained OriginalExited restoration. The regression must exercise the real BrowserManagerRuntime worker branch and prove that read-only reconciliation does not authorize manual launch.

The bounded follow-up branch fix/mode-restore-reconcile-scope starts at current main df359a635, including the independently merged mode/viewer continuity repair652. One demonstrated installed finding drives this remediation; no additional broad review or Contacts final-review counter is consumed. Installation and product acceptance remain OPEN until the combined source is qualified and the actual original operation and product canary pass.

The follow-up local selector at df359a635 selects focused Rust coverage, formatting, strict workspace Clippy, patch hygiene, policy wiring and changed documentation links. Policy wiring passed 11 tests and documentation links passed. The unchanged-input regression retry has a 7200-second setup allowance and retains the same resource-admission limits; no regression verdict is claimed while admission is pending.


## Follow-up source qualification

Baseline: df359a635308ceb58dca457fdae6bc0d224efdde. The real BrowserManagerRuntime regression failed before the fix with browser_login_mode_conflict and passed afterward; test execution was 0.09 seconds in both cases. The RED job took 2346.637 seconds including resource admission and compilation. The earlier 1801.236-second timeout produced no verdict and remains separate infrastructure evidence.

The fix splits read-only reconciliation admission from manual replacement launch admission. Only the exact published profile/browser operation in OriginalExited with restoring_previous=true gains pre-launch inspection. Existing post-launch admission and all profile, browser and requested/effective-mode fences remain intact. Ordinary opens still refuse the conflict, and launch admission remains false for the newly admitted inspection state.

The GREEN job ran the new real-worker regression successfully. Its integration filter matched zero coordinator cases; this is explicitly excluded from coverage. The resulting exact-source integration executable then ran all 17 coordinator cases successfully. Three completed-mode handoff tests and two native-mode-close tests also passed with an isolated synthetic home, which was removed. Total focused coverage: 23 passing tests. No temporary debug logging or live browser input was added.


Strict workspace Clippy passed with zero warnings in 130.472 seconds. Cargo-wrapped formatting, policy wiring (11 tests), documentation links and patch hygiene passed. Hosted CI was skipped under the operator cost policy; Linux/WSL local validation is the evidence, with no cross-platform execution claim.

### Follow-up review

Standards and Spec were reviewed serially in the primary context against the frozen diff from df359a635308ceb58dca457fdae6bc0d224efdde, AGENTS.md, the domain contract, policies 0042 and 0051, and the RECONCILE_SCOPE requirement above. Standards: 0 accepted blocking findings. Spec: 0 accepted blocking findings. The shared private helper retains the existing fences; only the read-only call site uses the additional restoration scope. The actual launch call site keeps its prior guard. This is a bounded remediation review, not the Contacts program's reserved final review.

The follow-up is source-qualified. Its preserving production deployment, original-operation recovery and real product canary remain pending; earlier-binary acceptance is not evidence for this source.


## PR654 installed retest and custody finding

PR654 merged at 110788ccbbee5f18173955122159648535a4517c, identical in tree to reviewed source ad8cd0820cd40ce7500bd7e36b7e271826eb62f2. A clean production-shaped build sealed binary 91c01efcaad69cfebcef6b50fbd7cf27bf87aae01ffbc64d171c012e3b56ac04 in 1437.188 seconds with zero errors/warnings. Its source-free installer fixture passed with two owned hosts and zero residue. Preserving installation was accepted, the installed hash matches and doctor passes with the existing operator-journey advisory; the monitor is healthy.

Fresh OS census confirmed 10 of 11 pre-apply browser roots survived. One system Chrome root exited during the interval; causality is unproven. This does not establish whole-runtime preservation or authorize unrelated recovery. The prior deployment's separate 10-of-10 result remains historical evidence only.

The original public restoration passed the repaired reconciliation branch, then stopped at mode_operation_launch_custody_readback_required, still OriginalExited with zero launches. Read-only source/state comparison proves the retained and published profiles match, while the transient requested-mode copy differs in manualLoginUrl. The custody store requires exact published-catalog equality. Do not weaken that equality or edit live journals/catalog state to clear the error.

Accepted finding PUBLISHED_PROFILE_CUSTODY: absence settlement must use the exact currently published profile, while retaining all bound profile identity and state checks. Before another deployment, exercise the real host dispatch through restoration preparation, SQLite custody settlement and the next synthetic launch boundary. A regression is added on fix/mode-restoration-published-profile from main ed6dcf37cd17a03657e4f0b75d4b5a0c3a91031d, which includes independently merged ordinary-handoff repair653. The proposed implementation is private and unapplied pending a trustworthy RED/GREEN verdict. This is a changed local verification tactic, not a reset of cumulative repair/replay accounting.

Cumulative delivery evidence: two production-shaped candidates, two preserving replacements, 3507.563 seconds of production builds, plus retained compilation/admission waits. No useful LinkedIn source observations were collected. No social write, credential replay, global input release, forced closure or profile reset was performed by this delivery. Final Contacts program review/TDD remain unused. The finite continuation cutoff is 2026-11-12T07:10:00Z; another production cycle must fit its remaining qualification, installation and consumer proof, rather than spending the last window on another intermediate artifact.

The host-path regression attempt timed out after 600.004 seconds in memory admission, with no compilation or test verdict. Its exact wrapper is inactive. The proposed fix remains unapplied. Another complete source-qualification, production-build, install and consumer-proof cycle cannot fit the remaining continuation window, so no third candidate or replacement is started. The completed requested PR delivery and failed installed retest are recorded separately from the unfinished consumer outcome.


## Resumed custody qualification | 2026-11-12

The user explicitly requested completion of the build and another try. The regression fixture was qualified locally before another production cycle. Retained setup failures exposed a missing sessionName, an incomplete manually authored operation table, noncanonical operation serialization and a missing synthetic profile directory. The final fixture uses the actual OperationStore schema and Operation type, the real host dispatch and exact SQLite settlement, and an existing synthetic directory. It is Linux-only because process-absence proof is Linux-only. A further fixture expectation was corrected to the intentional unsupported build-selector stop; no failure is erased or counted as a product regression.

The trustworthy RED at baseline 32d3e315b26818c9ad099548e46ccfcb9be7762e reports the exact installed custody error, zero launches and no effective mode in 2.28 seconds. The fix passes the same host path in 4.33 seconds. The passing regression verifies effective CDP mode, one durable coordinator launch entry, the synthetic selector's refusal and zero browser process launches. This proves restoration preparation through custody and the next transition; it does not stand in for installed Native/browser acceptance.

The implementation obtains the currently published named/disposable profile for absence settlement and verifies it against the retained bound identity, allowing only the mode-marker difference. The custody store's exact full-catalog and state checks remain unchanged. Launch still commits the explicitly selected mode afterward. No live state edits, weakened store checks, profile resets or global control changes are introduced.

Focused local qualification: 1 real-host regression, 17 durable mode-operation cases, 1 read-only restore-scope regression, 1 exact custody-settlement case, 3 completed-mode handoff cases and 2 Native close cases pass (25 total). The filtered Cargo integration execution matched zero cases and is excluded; its exact compiled integration artifact separately ran all 17 cases. The remaining focused executions used an isolated home/runtime tree, removed afterward. Formatting, policy wiring (11 cases), documentation links and patch hygiene pass. Strict Clippy and clean source-bound production delivery are next; no hosted CI or cross-platform runtime claim.


### Custody repair review and final source gates

At baseline 32d3e315b26818c9ad099548e46ccfcb9be7762e, Standards and Spec were reviewed serially in the primary context against the closed-world PUBLISHED_PROFILE_CUSTODY finding, AGENTS.md, the domain contract and policies 0042/0051. Standards: 0 accepted blocking findings. Spec: 0 accepted blocking findings. The published profile is verified against retained identity, and the storage transaction still compares the full published profile and runtime state before settlement. The regression reaches actual host dispatch and custody storage, stops at the synthetic selector, and leaves process launch unimplemented. No independent-agent approval or final Contacts program review is claimed.

Strict workspace Clippy passed with zero warnings in 150.421 seconds. All 25 focused cases, formatting, policy wiring (11 cases), documentation links and patch hygiene pass. Local Linux/WSL checks satisfy the operator CI cost policy; hosted CI is skipped. This closes source qualification only. One clean source-bound production-shaped build, exact artifact fixture, preserving installation and actual original-operation/content retest follow under the operator's resumed authority.
