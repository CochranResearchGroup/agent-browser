# Restore previous mode after an interrupted close

Status: OPEN
Product lane: PL-BUGFIX
Consolidation: required
Owner: root
Source baseline: 4f0b61959b97f0739374831a24cb0a0342998a02
Branch: fix/restore-mode-after-original-exit

## Current State

A consumer cannot acquire its existing profile because an obsolete CDP-off conversion retains profile custody in Closing after reboot. The public restore_previous choice returns mode_operation_phase_conflict. The live acquisition probe reproduces browser_profile_operation_pending with both the consumer configuration and the default configuration. Private profile and operation identifiers stay in consumer user storage.

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
| Actual recovery failure | Public restore_previous returns mode_operation_phase_conflict with Closing and0launches | Installed recovery still blocked |
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
