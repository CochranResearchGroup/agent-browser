# Restore previous mode after an interrupted close

Status: OPEN
Product lane: PL-BUGFIX
Consolidation: required
Owner: root
Source baseline: 4f0b61959b97f0739374831a24cb0a0342998a02
Branch: fix/mode-restore-reconcile-scope
Original integrated branch: fix/restore-mode-after-original-exit

## Current State

PR651 is merged and its exact artifact is installed through an accepted preserving transaction; all 10 pre-apply browser roots survived. Installed doctor passes after natural monitor recovery. The original operation advanced to OriginalExited with 0 replacement launches, then exposed browser_login_mode_conflict during pre-launch reconciliation. The selected profile is confirmed privately. One bounded real-runtime regression/remediation addresses RECONCILE_SCOPE; product LinkedIn acceptance remains unproven. The first regression build exhausted its setup/admission time budget without a test verdict and is preserved as timed out; its exact scope is inactive before the unchanged-input retry.

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
