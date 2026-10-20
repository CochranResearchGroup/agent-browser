# P256 absent-browser workflow recovery

Product lane: PL-BUGFIX
Disposition: accepted-evidence
Owning plan or work item: [P256](../plans/0256-2026-10-20-absent-browser-workflow.md), CochranResearchGroup/agent-browser#327
Related lanes: P249; #307 retirement contention remains separate

## Result

A stale executable refusal previously directed every caller to a handoff candidate, while preparation with no attached browser returned only `prepared=false` and `browserPresent=false`. Text output hid that result behind Done. Repeating the same workflow could only repeat the refusal. The no-manager branch also removed retained handoff evidence without proving that its custody was terminal.

Preparation now reports `reason=no_browser_to_transfer`, `browserObservation=not_attached` and recovery metadata. On Linux/WSL its executable next step is the existing read-only preserve transaction:

```sh
agent-browser install workstation --dry-run --runtime-replacement-policy preserve --json
```

The stale transport refusal explains both prepared and no-candidate outcomes, so an older daemon's unextended reply remains actionable. CLI text explicitly renders that older no-op reply. Other platforms report that automatic replacement is unavailable instead of suggesting the Linux-only command. The no-manager branch retains handoff bytes. Missing local attachment does not establish global browser absence, qualify automatic retry, or bypass the existing transaction's peer/unknown-effect checks.

This is the supported recovery-path option in #327's acceptance. It does not automatically restart an old daemon or promise that a particular workstation plan will be admissible. The returned plan owns any further effects and their exact blockers.

## Evidence

The provider-free regression called the actual native preparation handler and failed because `reason` was absent. After repair, three regressions pass: actual stale transport startup still refuses an old version and exposes browserless recourse; actual no-browser preparation preserves retained uncertain handoff bytes; and the CLI formatter renders legacy no-op replies without inventing a candidate. The final three-test run takes 0.35 seconds after compilation. The Linux transport fixture pins executable inode identity to avoid repeatedly hashing a large debug executable.

The broader handoff selection passes 190 tests with one explicit disposable-Chrome test ignored. This gate preceded the final platform-aware guidance adjustment; the final three regressions cover that adjustment. The final connection selection passes 47 tests, the preparation selection passes 10, and strict workspace Clippy passes. Formatting, docs build, policy wiring (11 tests), service API/MCP and compact skill parity, Remote View handoff documentation and changed-link checks pass.

An additional existing workstation-install fixture was interrupted by the agent after entering slow unrelated transaction scenarios. Its transaction assertion after interruption is not diagnosed as a product defect and the suite is not claimed passed. The exact isolated child was interrupted through a PID-bound handle; the fixture's own cleanup retired two owned hosts and reported zero residue. The earlier broad Rust run completed before its attempted interruption and returned success. Raw logs and the interruption disposition remain outside Git.

## Boundary

Source base is canonical `3ca65ae38` in the existing dashboard primary. No production executable, runtime, account, cookie, profile or shared skill was changed. The repaired source skill and docs describe the new response; installed guidance remains bound to its installed generation. No GitHub CI was enabled or dispatched. Production installation and receiving-account continuation are not claimed by these provider-free checks. Foreign worktrees and pinned candidates remain preserved.
