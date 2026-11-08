# CDP-free idle closure and live handoff

Product lane: PL-BUGFIX
Work item: 585
Related validation defect: 589
Owner: ecochran76
Branch: fix/cdp-free-idle-handoff
Base: a779f3fe9
State: ACTIVE

Close a managed CDP-free original through a retained kernel process handle and
confirm OS exit without requiring desktop window-close success. Keep PID/start/
executable identity verification and never escalate automatic close to force kill.

An explicit human handoff may supersede an unfinished automatic close only after
an exclusive existing retirement-effect guard proves no cleanup worker is active,
and fresh immutable-original process readback proves that browser is still alive.
Retain the entire attempted-close history and its unsettled physical outcome.
Atomically record the explicit live-retention decision, publish unchanged task
state and complete the original Native reservation. Future automatic effects for
that retired operation are forbidden. A concurrent worker or unavailable/changed
identity must defer. This is not a no-effect or browser-closed assertion.

The earlier generic-helper diagnosis was incomplete: the runtime has an earlier
manual-browser branch that already uses window-close and process verification.
That path can fail or leave a close prompt while the browser is alive. Missing
CDP does not prove the original attempt was undispatched. Discard that assumption.

Reproduce the existing durable handoff and test its actual host/SQLite/provider
route with a live original and pending close. Add process-close, worker exclusion,
unknown observation and retained-journal controls. Run applicable local Rust,
contract and documentation checks. Hosted CI remains disabled.

Reuse the canonical checkout, preserving unrelated CONTEXT.md changes. No new
worktree; other checkouts and candidates keep their existing custody. This lane
owns the runtime close helper, retirement model, store and explicit handoff
recovery. No installed replacement, original browser termination, state clearing
or private input in this source packet.

## Validation and review

The actual host/SQLite/Native handoff fixture reproduced the exact idle-readback
error before repair and passes after it, including worker exclusion, unavailable
identity, unchanged attempted-close history, same-link reconnect and rejection
of a delayed original writer. OS termination and positive exit plus original
identity controls also pass. Native completion now receives the validated owner
resolution, which can be explicit live retention without claiming physical close.

An optional broad browser-session run was deliberately interrupted during slow
profile-copy disposal cases. It also exposed a metadata race fixture failure.
The same failure reproduces against exact baseline Rust source a779f3fe9: both
overlapping operations can acknowledge a serializable update-then-delete. The
fixture now permits those acknowledgements while preserving its terminal deletion,
restart, unchanged browser/tab/session and no-provider-effect assertions. The
stronger final-state check exposed an actual pre-existing lost deletion: generic
current-writer preference can discard an acknowledged local deletion. Issue589
owns a narrow handoff-map deletion rule in the existing reconciler. This lane is
its primary writer; no other collection or publication behavior is expanded.

An additional outer-adapter control proves automatic recorded retirement does
not require Native window context; ordinary explicit window close keeps that
requirement. Its initial fixture omitted pool configuration and therefore tested
ordinary close. Those setup failures are preserved and excluded from product
reproduction evidence. The corrected managed-pool fixture passes.

Source validation is complete at 14ddb083c8a92fa813862843e212d83f204472ff.
PR: https://github.com/CochranResearchGroup/agent-browser/pull/588

- All eleven focused CLI commands passed: cdp_free_idle_retirement (4),
  browser_idle_retirement_store (12), automatic_retirement (14),
  runtime_original_retirement (1 pass and 1 existing ignored real-Chrome test),
  native_handoff_resolution (1), mcp::workflow (5), the concurrent delete/policy
  fixture (1), retirement_effect_allows_recovery_snapshot (1),
  browser_session_host::reconciliation (6), expired-link deletion fencing (1),
  and deletion preserving its target after restart (1). Total: 47 passes.
  Each used `scripts/ci/rust-tests.sh` with its focused filter. The Rust/script
  input closure remained 4e72887bf812596ea5647e85142255ac14124633da4e8b19181616d9830e1069
  throughout these commands.
- `scripts/ci/cargo-safe.sh test --manifest-path Cargo.toml -p agent-browser-service-model -- --test-threads=1`
  passed all 418 tests, including documentation tests, on 7bf2cecf8. The only
  subsequent Rust change enables the CLI test's pool; model sources are identical.
- Required cargo-safe workspace format checking and strict workspace Clippy
  passed. Clippy used `--locked --offline` and `-D warnings`.
- Policy wiring, changed documentation links against a779f3fe9, remote-view
  handoff documentation contracts and the 44-page docs build passed.
- Optional workstation install, host provisioning, fresh VM harness, Guacamole
  assets, PostgreSQL durability and route-user sync fixtures passed. These are
  ancillary fixture evidence; their existing CLI binary is not installed proof
  of this repair.

Commands ran locally on Linux/WSL with the ordinary Cargo admission wrapper,
eight jobs, host reserve and cgroup limits. No hosted CI or other-platform proof
is claimed. Full broad browser-session testing was interrupted and is not a
passing comprehensive gate. Raw failures and local receipts remain in the private
user diagnostics directory; no private browser state is copied into this repo.

The original live browser, journals and durable handoff were preserved. No repair
binary or shared skill was installed by this task. Source review and isolated
installed acceptance remain separate. Retain the canonical checkout for PR review;
next review 2026-11-09. Graphiti disposition: queued, with a correction to the
incomplete earlier helper-only diagnosis; queue acceptance is not retrieval proof.
