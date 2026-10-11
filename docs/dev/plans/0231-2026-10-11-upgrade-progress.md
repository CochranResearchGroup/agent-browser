# Expose truthful upgrade progress

Product lane: PL-BUGFIX
Work item: #181
Branch: fix/upgrade-progress-181
State: COMPLETED

The current public status drops checkpoint timing and the recorded convergence deadline. A disposable public CLI fixture reproduces this missing evidence. Repair the status projection using recorded observations and persist accurate phase starts on subsequent transitions; never manufacture a heartbeat from a status read. Test fixed-clock projections and transition persistence. Inspect current waits for actionable bounded exits before claiming the wider issue accepted. Production installation and reboot remain excluded.

Validation: public red/green fixture, focused provider-free status/transition tests, Rust format and strict workspace Clippy, and all user-facing documentation surfaces. Build time remains distinct from apply time. A passing projection fixture cannot qualify a production-shaped upgrade or an untested wait.

Accepted bounded source evidence: [repair receipt](../notes/0231-2026-10-11-upgrade-progress-repair.md). Issue #181 remains open for the wider qualification explicitly listed there. The pre-existing JavaScript installer fixture failure is retained, not counted as a passing gate.
