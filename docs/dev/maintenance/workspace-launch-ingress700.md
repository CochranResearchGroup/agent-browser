# Workspace launch first-response deadline

Product lane: PL-BUGFIX
Work item: https://github.com/CochranResearchGroup/agent-browser/issues/700
Owner: ecochran76
Branch: fix/workspace-launch-ingress-budget
Base: 0edab79d21996d018c9a0fdbbf50b450392204e1
State: SOURCE_VALIDATED; installed acceptance pending

## Observed failure and bounded diagnosis

An authenticated live external workspace Chromium launch returned HTTP 502
`mutation_outcome_unknown` after approximately 2.1 seconds. Its response explicitly
identified the selected generation's first-response timeout. The owner retained
the launch intent but no published task or completed response. Preserve that
original operation; another submission is not reconciliation.

The launcher allows 60 seconds for a response. Stable ingress instead selects its
generic two-second first-response deadline for the launch POST. A delayed real TCP
proxy regression must reproduce the premature502 before the repair. The exact
launch route should retain its bounded owner-response opportunity without
changing mutation fallback or replay rules. Existing generic POST, handoff,
Native callback and service-job budgets remain separately qualified.

The earlier external attempt returned an unconfirmed launch and later
`workspace_launch_unknown`; authentication was lost between probes and no owner
record was found. Its original network response was not captured. The later
instrumented502 proves a deadline defect, not the first attempt's cause.

## Custody and validation

Reuse the clean canonical primary from freshly fetched main. No new worktree.
This lane is sole writer for the exact launch deadline and regression in
`cli/src/dashboard_ingress.rs`. Issue 677 owns residual handoff latency; this
repair does not modify its maintenance or lifecycle adapters. Preserve foreign
mode/control worktrees, original runtime requests and their private evidence.

Run the delayed real TCP regression RED then GREEN, focused ingress regressions,
Rustfmt and strict workspace Clippy through cargo-safe. Record source-bound local
results before review. Hosted CI remains disabled. No production installation
or successful live toggle acceptance is claimed by source validation.

## Remaining acceptance

Reconcile the retained launch outcome through its owner. Actual Chromium and
supported Chrome CDP/CDP-free round trips must be driven through the external
workspace, with profile/page continuity, Native visibility, continued automation
and exact test cleanup. Chrome for Testing is excluded from supported-browser
acceptance. Do not replace missing live evidence with the old copy matrix.

## Regression checkpoint

At base `0edab79d21996d018c9a0fdbbf50b450392204e1` plus the new TCP regression,
`AGENT_BROWSER_CARGO_CACHE=off scripts/ci/rust-tests.sh --focused workspace_launch_waits_for_owner_without_replaying_to_fallback`
failed in 2.20 seconds with HTTP 502 `mutation_outcome_unknown`, explicitly
reporting the selected backend first-response timeout. Compilation took 7m23s;
the regression itself takes seconds and requires no browser or provider.

The repair gives only the exact launch POST, with an optional query, 65 seconds:
the existing UI's 60-second deadline plus delivery margin. Negative route/method
cases preserve the generic deadline. The delayed TCP test also rejects fallback
replay. The GREEN ingress suite and strict Rust quality checks remain pending.
Policy wiring (11 cases) and changed documentation links have passed locally.

## Qualified source checkpoint

Executable source `43577bef2f884d488d921be8282f76ee69d1a4a4` passes:

- `AGENT_BROWSER_CARGO_CACHE=off scripts/ci/rust-tests.sh --focused dashboard_ingress::tests`:
  43 passed, none failed or ignored, 48.26 seconds after 3m13s compilation.
- `AGENT_BROWSER_CARGO_CACHE=off scripts/ci/cargo-safe.sh clippy --workspace --manifest-path Cargo.toml -- -D warnings`:
  passed, 2m31s after resource admission.
- `scripts/ci/cargo-safe.sh fmt --all --manifest-path Cargo.toml -- --check`:
  passed.
- `pnpm test:policy-wiring`: 11 passed and current policy hashes verified.
- `pnpm test:documentation-links -- --base origin/main`: passed.
- `git diff --check`: passed.

Checks ran locally on Linux/WSL through the required admission wrapper. The TCP
regression uses synthetic loopback backend responses without authentication,
Chrome or a Native provider. It qualifies proxy response timing and mutation
custody, not target-platform parity, installed publication or live mode switching.
Hosted GitHub CI was skipped under operator policy. Cargo admission waits and
compilation are separate from the seconds-long failing regression.

The final attempted external read was refused with
`browser_session_activity_admission_conflict`; it is a failed prerequisite, not
a clean workflow result. Both original request IDs, the observer and private
sanitized receipts remain retained. No new creation replay, process cleanup,
profile reset, production installation or mode conversion occurred. The temporary
browser-local fetch wrapper was removed. Private diagnosis scripts are clearly
marked under the user's QA debug directory, outside Git.

PR702 is the reviewable source repair. Issue700 stays open. The next installed
acceptance must verify the original unminimized external workflow; the repair is
not described as a completed CDP/CDP-free toggle qualification.
