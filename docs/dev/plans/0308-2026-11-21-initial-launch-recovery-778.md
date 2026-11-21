# Initial launch recovery execution ledger

State: IN_PROGRESS
Product lane: PL-BUGFIX
Specification: [#778](https://github.com/CochranResearchGroup/agent-browser/issues/778)
Integration branch: fix/initial-launch-recovery778
Base and review fixed point: da36d2f1a1b6aab118d3ab3ddc418027d0755fb7

| Ticket | Blocked by | Ownership | State |
| --- | --- | --- | --- |
| #779 Initial launch status and recourse | None | Coordinator. Launch custody readback, Open refusal, MCP diagnostics and shared documentation | Initial source validated at 826b65f0e. Reopened for task-free MCP dispatcher verification |
| #780 External close recovery | #779 | external_close. Worktree /var/tmp/agent-browser-780-recovery on fix/initial-launch-external-close780 from 826b65f0e | Guarded settlement integrated at 7bdd94a48. Public Open replay and native verification pending |
| #781 Prior boot recovery and combined candidate | #780 | Worker upon validation of #780 | Pending |

Use existing model and SQLite recovery tests, then one candidate for installed acceptance. Physical reboot requires an isolated environment. Production deployment is outside this spec.


## Evidence

- #779 RED: `scripts/ci/cargo-safe.sh test -p agent-browser-service-model --test browser_handoff_recovery unresolved_initial_launch_readback_never_authorizes_implicit_second_launch` failed because the refusal lacked the original launch identifier.
- #779 GREEN: `scripts/ci/cargo-safe.sh test -p agent-browser-service-model --test browser_handoff_recovery --test remote_view_browser_launch` passed all 13. Strict workspace Clippy and Remote View handoff documentation checks passed locally.
- #780 checkpoint: worker commit `11911635d` integrated as `7bdd94a48`. Thirteen launch and handoff model tests pass. Native SQLite verification and durable public Open replay are pending.
- Installed gate: the isolated development presentation provider reports unconfigured and development readiness is false. A new release build cannot qualify that environment.
- #779 dispatcher: the pinned CLI response reached `launchId` validation without a task. The regression initially asserted the outer JSON-RPC message; the detail-field correction is `c6e5eff20`. Rerun is pending.
- #780 native SQLite: `scripts/ci/rust-tests.sh --focused exact_absent_unpublished_launch_settlement_preserves_history_and_checks_state` passed with the guarded settlement checkpoint.
- #780 MCP replay: worker `a5aebaa8d` integrated as `e6f4e9481`. Cargo check passed on the worker source. Combined regression and Native request parity remain pending.
