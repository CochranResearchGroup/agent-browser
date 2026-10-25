# P290 writer executable continuity evidence

Product lane: PL-BUGFIX
Disposition: accepted-evidence
Owning plan or work item: P290 / CochranResearchGroup/agent-browser#417
Related lanes: P288, P289

## Public regression

The existing takeover fixture pauses an actual public transaction close at its OS input read. It copies the exact validation binary into disposable fixture storage and unlinks only that copy after proving initial custody. Takeover must still observe an executing writer; the existing competing apply and full-shutdown preview must preserve exclusion. Its PID-only compatibility case now also observes an unlinked executable. Releasing the deliberately invalid transaction ends that writer, after which unrelated retained PID metadata must permit takeover. This seam qualifies source coordination, not production or browser continuity.

Red command: `AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD=<qualified-PR415-binary> TMPDIR=/tmp node scripts/smoke-runtime-host-supervisor-takeover-no-launch.js`. Exact binary SHA256 is194e223198400749f465e0ba11921adc87296da82295187bf8115e511a25cfd8. It exits1 with `an executing writer must retain custody after its executable is unlinked`. The prior binary's affected module is identical at the fresh e609afd2d baseline. Raw receipts remain user-scoped outside Git.

The minimal implementation checks the claimed PID-file identity and exact process boot/start token. Executable path is diagnostic evidence, not a lifetime boundary for an already claimed process. Missing or changed process-start identity is not promoted into a matching writer. Linux's deleted-executable suffix is normalized only for the existing legacy executing-installer classifier; its mutation command checks remain.

Green command: `AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD=<owned-cli/target/ci/agent-browser> P290_VALIDATION_RUN=<isolated-run> TMPDIR=/tmp node scripts/smoke-runtime-host-supervisor-takeover-no-launch.js`. It exits0. The paused writer retains exclusion after unlink, competing apply stops before staging, full-shutdown preview reports contention, PID-only compatibility passes, and normal takeover plus zero-effect steady state succeeds after writer exit. One owned CI build completes in4m22s; exact binary SHA256 isaf198f49c0b068d5b197a404323603b76d126b7c677033e572aa15f8f5786e21.

Formatting, strict workspace Clippy, docs build and policy wiring pass. The selected planning audit and15-file local-link check pass after correcting the new plan status and note disposition to the auditor's existing vocabulary. Initial metadata audit findings remain in the session record. All221 workstation tests pass with zero failures or ignores and3420 filtered cases; this is not a comprehensive Rust claim. The full installer fixture passes, including exact cleanup of two owned runtime hosts with zero residue. No production publication, provider effect or foreign cleanup.


## Final integration qualification

PR419 carries the repair for issue417. The validated pre-merge executable source is frozen at99db70da3; later P290 changes qualify documentation only. Current-main reconciliation below separately records imported executable inputs. Independent post-run OS environment and executable readback finds no matching task fixture processes. Foreign unreadable or raced entries remain unclaimed. The validation binary and raw commands/receipts are retained privately outside Git; the ordinary CI artifact is not production-promotable. No candidate seal or installed-runtime claim is made.

The changed-surface selector selects a broad lane because installer help shares output source; no service schema, model, generated client, action contract, provisioning, browser, Native/Guacamole route or VM behavior changed. Qualification covers writer custody through the public CLI, workstation cases, strict Rust quality and documentation checks. CI remains disabled and was not dispatched.

Self-review confirms claimed PID-file identity and boot/start token preserve executing custody across executable unlink; absent or different process instances remain recoverable. Legacy PID-only classification retains the executing installer/mutation requirements. No generic browser identity rule changed. Scope remains one red-green slice and one owned CI iteration build.

Memory disposition: forbidden. New repair-outcome memory writes lack explicit user authorization; preserve a user-scoped machine-readable non-write receipt without a memory cloud effect.


## Current-main reconciliation

Main advanced to PR418 / d058f8df8 during qualification. Merge conflicts are additive entries in the lane catalog and two planning indexes; preserve both P289 and P290 entries. Imported P289 code exactly matches its qualified main source, including its387 model and95 CLI pool results (one existing ignored case). P290's writer source, process identity, installer, takeover, replacement, CLI parsing and dependency files are unchanged by that merge. Reuse P290's public fixture, full installer and221-test receipts for those unchanged behaviors under policy0042; do not call the pre-merge executable a current-main or installed artifact. Rerun format, strict workspace Clippy, docs build, policy wiring, selected planning and changed links on the combined source and help text. No additional runtime candidate or production effect is required for source integration.
