# Reliable local check results

Product lane: PL-PLATFORM
Work item: #295
Branch: platform/check-results-295
State: COMPLETED
Version: 1

Prevent failed browser-workflow acceptance checks from being hidden by a later successful shell command. Existing presubmit verification consumes results and the Rust runner already fails closed; neither executes arbitrary independent local checks with per-command receipts.

Deliver one small local validation runner under scripts/dev. [The local validation guide](../../agents/local-validation.md) documents the command. Input is an explicit JSON list of independent direct commands and arguments. It executes serially, preserves each terminal result and private log, and returns failure if any check fails, cannot start, times out or is signalled. No shell evaluation. A new output directory prevents overwriting prior evidence. Record Git HEAD and dirty status, command, timing and log locators. Interrupted/incomplete receipts never count as passing. No secrets belong in command arguments or manifests.

The public seam is the actual runner CLI: first failure followed by success must fail overall and retain both results; all-pass must pass; malformed input must start no child; launch failure and timeout must fail closed. Logs go directly to files rather than buffered model output. This seam does not establish that caller-selected checks cover every changed surface or authorize their effects.

Reuse the existing package-script and repository-tooling lane, with a focused developer guide and one execution pointer in RUNBOOK. Do not change browser/runtime code, presubmit semantics, candidate publication, foreign work or disabled CI. Shared package/docs source is owned only for this bounded command and its test wiring; P245 login and P240 resource work remain preserved. Reuse the clean former dashboard primary after inventory, retaining its existing candidate files and original branch ref. Integrate through a PR linked to #295.

Validation: actual CLI red/green, repository-tooling tests, validation-selection contracts if package wiring changes, patch hygiene, policy wiring and changed documentation links. No Rust gates or runtime installation are required for this Node-only tooling change.

## Result

The direct CLI preserves an earlier exit7 followed by exit0 as batch failure, with separate private logs and a source-bound receipt. Seven CLI tests pass: mixed failure/success, all-pass with literal arguments and two-million-byte file output, invalid later input with no earlier sentinel effect, missing executable, timeout, signal and output preservation. The initial RED lacked the new runner/receipt; the first GREEN provided it. A separate selector RED classified the new paths as broad; GREEN routes them to repository tooling. These are new-tool and selector regressions, not claims of an existing command's behavioral RED.

Existing repository-tooling checks pass, including candidate and governed worktree fixtures; validation-selection contracts pass. Policy wiring initially failed when a temporary guidance pointer changed AGENTS.md without matching its canonical hash. That unnecessary pointer was removed; no policy hashes were refreshed. The batch correctly remained failed despite later passing tooling/patch checks. Independent corrected policy wiring and a fresh policy/patch batch pass. Final patch hygiene and changed documentation links pass. No Rust, dashboard application, runtime, production or CI changes occurred. Commands and per-check private logs are retained under /tmp/agent-browser-check-results; logs contain synthetic fixtures only. The local guide and RUNBOOK point agents to the execution command. #295 and its PR own canonical integration; the reused clean primary retains existing candidate artifacts and foreign checkouts remain preserved.
