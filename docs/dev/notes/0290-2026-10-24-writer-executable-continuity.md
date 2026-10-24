# P290 writer executable continuity evidence

Product lane: PL-BUGFIX
Disposition: active-input
Owning plan or work item: P290 / CochranResearchGroup/agent-browser#417
Related lanes: P288, P289

## Public regression

The existing takeover fixture pauses an actual public transaction close at its OS input read. It copies the exact validation binary into disposable fixture storage and unlinks only that copy after proving initial custody. Takeover must still observe an executing writer; the existing competing apply and full-shutdown preview must preserve exclusion. Its PID-only compatibility case now also observes an unlinked executable. Releasing the deliberately invalid transaction ends that writer, after which unrelated retained PID metadata must permit takeover. This seam qualifies source coordination, not production or browser continuity.

Red command: `AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD=<qualified-PR415-binary> TMPDIR=/tmp node scripts/smoke-runtime-host-supervisor-takeover-no-launch.js`. Exact binary SHA256 is194e223198400749f465e0ba11921adc87296da82295187bf8115e511a25cfd8. It exits1 with `an executing writer must retain custody after its executable is unlinked`. The prior binary's affected module is identical at the fresh e609afd2d baseline. Raw receipts remain user-scoped outside Git.

The minimal implementation checks the claimed PID-file identity and exact process boot/start token. Executable path is diagnostic evidence, not a lifetime boundary for an already claimed process. Missing or changed process-start identity is not promoted into a matching writer. Linux's deleted-executable suffix is normalized only for the existing legacy executing-installer classifier; its mutation command checks remain.

Green command: `AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD=<owned-cli/target/ci/agent-browser> P290_VALIDATION_RUN=<isolated-run> TMPDIR=/tmp node scripts/smoke-runtime-host-supervisor-takeover-no-launch.js`. It exits0. The paused writer retains exclusion after unlink, competing apply stops before staging, full-shutdown preview reports contention, PID-only compatibility passes, and normal takeover plus zero-effect steady state succeeds after writer exit. One owned CI build completes in4m22s; exact binary SHA256 isaf198f49c0b068d5b197a404323603b76d126b7c677033e572aa15f8f5786e21.

Formatting, strict workspace Clippy, docs build and policy wiring pass. The selected planning audit and15-file local-link check pass after correcting the new plan status and note disposition to the auditor's existing vocabulary. Preserve those initial metadata audit failures separately. Workstation tests and full installer fixture remain pending. No production publication, provider effect or foreign cleanup.
