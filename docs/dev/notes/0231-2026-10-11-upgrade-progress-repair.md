# Upgrade status timing and bounded doctor commands

Issue: #181
Product lane: PL-BUGFIX
Baseline: 624c22e8df7f7f79a18b8d4b09d859f20473d817
Outcome: bounded source repair; issue remains open for wider qualification.

A disposable public CLI transaction fixture reproduced missing phase timing and convergence-deadline fields. The real status-function regression failed with a null phase start. Status now projects recorded phase start, elapsed milliseconds, last durable checkpoint, convergence deadline and observation time. Future installer transitions persist phase starts in additive successor bookkeeping. Unknown legacy starts remain null. Status reads do not manufacture heartbeat or progress evidence.

A controlled final-doctor subprocess reproduced an unbounded wait despite the four-attempt retry limit. The adapter now reuses the existing bounded command runner with a 45-second timeout. Timeout returns immediately as `post_commit_validation_command_timeout`, identifying the doctor command; it does not consume another retry. The runner terminates only the child probe it launched.

All 188 provider-free workstation tests passed, including four added regressions for public status, fixed-clock reads, persisted phase starts and the stalled doctor. The original public CLI loop changed from missing four fields to none missing. Strict workspace Clippy, Rust formatting, docs build, handoff docs and changed Markdown links passed. Tested unsealed debug CLI SHA256: `ad342e40ada38d3c6ca4ac02b6fd901d26347393ee4d3d34ef7dc3a9044f5851`. It was not published to any installed runtime.

The recommended JavaScript installer fixture remains failed independently of this repair. Both the prior development binary and candidate fail its unseeded-profile admission assertion. A private corrected probe with an explicit disposable profile reaches `/bin/false` launch attempts instead of the expected admission-drain error on both binaries. The original tracked fixture is preserved, and the corrected probe and patch are retained privately for the next diagnosis. This is not a passing install gate or proof that the admission contract is obsolete.

Those four fixture runs left eight temporary runtime hosts after deleting their own directories. Exact fixture-home, admission-claim, executable-digest and process-start checks qualified cleanup. The system Python supplied pidfd signaling, and exact SIGTERM cleanup left no process residue. The initial private Python lacked pidfd support and failed before signaling; that failed cleanup attempt is retained. Launch probes configured `/bin/false` and no browser became ready. Production hosts, foreign runtimes, providers and credentials were untouched; no reboot occurred.

Remaining #181 evidence: incremental install progress or heartbeat, separate build/apply duration receipts, and a production-shaped current upgrade or deliberately nonconverging verification. The exposed deadline is specifically the runtime-host convergence deadline, not a claimed whole-install deadline. The newly observed admission behavior and fixture cleanup defect are separate from this repair.

Private evidence is retained under the serial-repair execution checkpoint: public red/green logs, unit and timeout red logs, the corrected-chrono compilation attempt, 188-test log, source build and quality gates, four failed installer fixture logs, corrected fixture patch and exact cleanup receipt. No failed attempt is relabeled as a clean first run.
