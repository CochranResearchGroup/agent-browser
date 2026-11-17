# Selected-browser command maintenance

Status: CLOSED (source correction)
Owner: Contacts LinkedIn latency diagnosis
Branch: fix/contacts-browser-command-scope
Base: bbbcc97e35edf48e5864c75b3dc49725e7c58485

## Outcome and scope

Ordinary page commands for a retained browser should initialize maintenance for the selected profile, without reconciling unrelated activity or retirement obligations. Preserve exact current task, process, tab, control and publication checks. Existing broader lifecycle work remains global. No runtime adoption, source publication or browser reset in this slice. Reuse canonical clean checkout under a feature branch; no new worktree or auxiliary.

## Current evidence

Contacts installed connector measured with qualified NTS-verified Stable Time host-counter pairs: target readable upper32.127seconds; full acquisition upper76.833seconds. Trivial page evaluation5.593seconds versus real extractor5.7097seconds in one persistent transport. Metadata briefing upper0.272seconds. Private observations remain under the Contacts tenant runtime, not Git. A >60second initial-page failure was not reproduced. Numerical WSL-clock probes from the preceding packet are provisional and superseded by these qualified measurements.

Source inspection finds ordinary command recovery, tab activity, navigation recording and command dispatch initialize global operation scopes. RemoteViewSessionEffects implements begin_browser_operation, but RuntimeSessionEffects inherits the global default instead of forwarding it. The proposed correction removes repeated peer-profile observations; its share of installed latency remains unproven until an installed candidate comparison.

## Regression and validation

Use existing host, SQLite and Remote View fixtures. The real execute_managed_command path must perform zero original-process observations for a peer profile and still observe the selected profile when its own activity exists. The concrete runtime wrapper must preserve selected-target rejection. Require RED before the production change, then affected consumer/wrapper guards. Local checks only; hosted CI disabled by operator cost policy.

## Definition of done

Source regression passes, existing selected-resource/recovery guards pass, and exact remaining installed performance acceptance is explicit. Serial critical path; no delegated tracks. No claim that fewer source sections or suppressed connections solve latency.

## Source correction and validation

The real host regression failed before the fix: one selected-profile page read made3peer original-process observations, expected0. The concrete-wrapper regression separately failed because unknown selected-browser admission returnedOk instead of the existing target-changed error. After the patch both pass. The corrected host scopes ordinary command recovery, tab activity, navigation recording and page dispatch to the selected browser's profile; RuntimeSessionEffects explicitly forwards begin_browser_operation. The existing global manager remains for broader lifecycle operations. No original journal or resource checks are removed.

The exact built binary selected27existing consumer/wrapper tests, including the new regression, ordinary physical recovery, unknown outcome retention, pending retirement refusal, PID reuse, unchanged peer obligations and exact profile rebinding. All27pass. rustfmt and git diff --check pass. Strict production-binary Clippy passes with -D warnings. Validation uses repository cargo-safe.sh with2jobs,8GiB declared claim and16GiB reserve, cache disabled after its -vV discovery stalled, and the existing mold linker. No dependency, profile or cache cleanup was performed. Hosted CI skipped per operator cost policy.

Source qualification is separate from runtime acceptance: installed binaryabdbd60b73db remains unchanged. Do not claim that this patch alone reduces the observed5.59second command cost or76.8second acquisition until a candidate/installed comparison proves it. Initial profile readiness already passed the60second threshold on the installed build; the reproduced source defect is repeated unrelated maintenance.

The broader clippy --bin agent-browser --tests attempt reached unrelated profile_duplicate_operation integration-target dead-code errors: hold_profile_operations and OperationSnapshot in unchanged profile_duplicate_copy.rs. That overbroad attempt was stopped after the failure; the scoped production-binary strict lint is the applicable lint gate. No unrelated warning suppression or test changes.

Final commands: cargo-safe.sh test --manifest-path cli/Cargo.toml --bin agent-browser ordinary_page_read_does_not_observe_peer_profile_activity -- --nocapture (RED then GREEN); exact built binary selected27consumer/wrapper checks (27passed); cargo-safe.sh clippy --manifest-path cli/Cargo.toml --bin agent-browser -- -D warnings (passed); rustfmt --check on three modified Rust files and git diff --check (passed). Test build uses8GiB declared claim; production lint4GiB claim; both keep16GiB reserve and2jobs. Source-only closed packet; installed performance comparison remains the next release gate.

Memory disposition: unavailable. The project memory service rejected the Stable Time nanosecond reference timestamp before queuing (Invalid isoformat string). Exact rejected-before-queue receipt is retained in the private tenant artifact linkedin-scope-source-memory.json; no queued job or successful persistence is claimed and no write was retried. This does not block source validation.

## Authorized review, merge and deployment

Operator accepted review, merge, deployment and an installed Enrich-button retest. Fixed review basebbbcc97e35edf48e5864c75b3dc49725e7c58485, corresponding to the recorded clean source base. Preserve authenticated profiles and current browser processes; no whole-runtime reset. Retain source checks above because executable source is unchanged. Build one production-shaped sealed candidate and use supported preserving installation, then compare installed LinkedIn acquisition using qualified Stable Time and prove a native Enrich click executes the source workflow.

## Standards

Serial primary review against AGENTS, existing CONTEXT, runtime/product boundary, shared runtime custody, testing and validation policies plus the skill smell baseline:0accepted findings. Existing selected-browser trait and fixture boundaries reused; global lifecycle manager retained, and wrapper delegation preserves concrete resource checks. No new public contract or unrelated refactor.

## Spec

Independent serial review against this bounded artifact and Contacts Plan0369:0accepted findings. Real command regression preserves peer journal while selected-profile checks remain active; wrapper rejection guard and retained physical recovery pass. Initial page readiness already met60seconds, so neither root-cause share nor installed speedup is asserted by source tests.

Review summary: Standards0, Spec0; no blocking findings. Release activation and live Enrich acceptance remain open.

## Installed source workflow acceptance (2026-11-17)

PR727 merged as ed62bb3a30803b5ebf0c50427d99c308f04cdb0e with a tree identical to tested sourceb05d4f85908586cbe0a568274322d2b02cab1e3f. The preserving installation exposed a separate final-doctor home-isolation defect, corrected and installed through [issue734](install-doctor-home-isolation.md). Current production binary408821d0c76f contains both corrections. Standalone doctor passes, one supervisor host is selected and all14 original managed browser identities survive.

A native Contacts Enrich click, with exactly LinkedIn selected, created and enqueued one pass. Acquisition, interpretation, reconciliation and application completed. Five source-backed additions were applied and one conflicting detail remains pending review. The upper rail showed an active job and spinner, briefly entered reconnecting, recovered and cleared at completion. The contact card displays Update finished. The native click and create/enqueue responses, stage/status records, source result and aggregate verdict are retained privately. Temporary page request tracing was removed. No second enrichment job was submitted for final installation verification.

The source receipt observes profile URL, current role, employment history and education, plus10retained connections. Network coverage remains incomplete with page_budget_exhausted; neither complete-network acquisition nor ten added graph relationships is claimed. The UI pass was observed terminal within a Stable Time upper337.197seconds from the native click boundary. Final post-install card inspection retains the finished result. No raw contact identities, URLs or source facts are copied into this project artifact.

Performance remains unresolved. Earlier baseline profile upper32.1262seconds / full acquisition upper76.8327seconds; intermediate two-host observation40.3106 /98.0416seconds; final quiet installed single-host observation58.2493 /129.2882seconds. All preserve four profile categories and10connections under the same bounded acquisition configuration. Initial profile readiness still passes60seconds, but no speedup is demonstrated. These are single observations, not a distribution or a causal performance attribution. The final trivial page evaluation upper9.9391seconds versus extractor11.9125seconds keeps ordinary browser-command overhead as the next diagnostic boundary. Do not reduce source features to claim improvement.

Private acceptance verdicts: release727-ui-verdict.json, release734-preservation-verdict.json, linkedin-after734-verdict.json. Source correction and this review/merge/deploy/retest packet are complete; broader latency optimization and the intermittent Jobs polling gap remain separate PL-BUGFIX follow-up. Historical source-only qualifications and memory delivery failure above are preserved as history, superseded for current installed identity by this section.
