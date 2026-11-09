# Operator-confirmed mode relaunch during human handoff

Product lane: PL-BUGFIX
Work item: 607
Branch: fix/607-operator-mode-relaunch
State: OPEN

## Current state

Combined source `dfcb6c105` is installed as generation
`0.28.0-6b17b58a38f0-10036adf76ec`, SHA256
`6b17b58a38f0bd155f191cb256614a70994747ebc281d102e8eb5be4b81445ec`.
The exact installer fixture and three isolated browser cycles pass; public
installed doctor passes. The source preserves installed native-login source
`8edb25b3`. No shared global skill was published.

The corrected unfiltered metadata probe now passes. The original operator-owned
process was closed through an identity-bound pidfd; public browser-scope close
then archived its ended task. The same profile reopened with CDP, and the
application's signed-in account was independently confirmed. Private account and
profile evidence remains in the user-scoped debug directory.

A separate empty live Chrome qualification exposed a remaining defect: public
browser-scope close reached the owner but selected normal manual window close.
It returned `manual_login_close_readback_required` with the original process
still live. No close was replayed. A profile-creation database-lock failure was
first reconciled as zero task/process effects before one serialized retry.

The refined owner regression reproduces the same manual-window refusal (1 failed,
8.50s). The new explicit browser-scope path revalidates the exact refreshed
browser and exclusive task scope, then uses the existing original-kernel shutdown
primitive with durable activity and positive exit observation. Ordinary task
closure keeps its prior behavior; earlier uncertain window-close history remains.
Focused GREEN: both owned-process lifecycle cases pass (2 tests, 29.74s).
The complete service-model package passes (418 tests across 25 suites). Strict
workspace Clippy, formatting, policy wiring, MCP/skill parity, handoff docs,
documentation links and the docs build pass. A new sealed candidate is pending.

Retain this canonical checkout and both sealed artifacts. No new checkout is
created. Next: finish source qualification, build/qualify the corrected candidate,
perform a preserving update and complete the retained empty live-browser cycle.
The actual FigureLabs profile is separately usable for read-only research.

## Scope and acceptance

Explicit attributable whole-browser close must preserve profile storage and
independent peers while ending its owned process during a human hold.

Keep inert mode-operation lookup available during human handoff. Its lifecycle
owner must retain explicit confirmation and exact process/profile/task fencing.
Regression at public owner handle_command must fail before the fix, then pass
without external effects or release of human control. Preserve ordinary page
read restrictions, copy-effect guards, uncertainty and peers. Run focused tests
and required local quality gates; no hosted CI. Record source qualification and
installed/private-account continuation separately. No new auth system, broad
runtime reset, profile deletion, paid site action or experimental production
publication. One bounded repair; no final FigureLabs review counter consumed.

## Diagnosis loop

The original hosted-control probe fails with
`browser_control_hosted_task_required`; it exercises an unsupported hosted-only
adapter and is not the feedback loop for the requested mode relaunch. The
refined public MCP loop addresses the original task and runs `mode_convert`
with `choice: inspect`. It fails in 0.68 seconds with
`browser_control_observation_paused: return control or explicitly allow observation`.
No Google content, credentials or physical browser effects are needed to trigger
this failure. The private runnable command and observations remain user-scoped.

Ranked predictions were: an early hosted guard prevents standalone return;
wrong-session routing selects another lane; or an existing operator route
reaches the lifecycle owner. Source joins establish that the correct task is
selected. The intended mode path has its own earlier failure:
`operation_request_readback` checks `url` permission before metadata lookup and
before the mode owner evaluates `ownerConfirmed`. Ordinary human handoff pauses
URL reads. This is sufficient to reproduce the exact relaunch admission failure;
returning hosted control is not its remedy.

The regression uses public owner `handle_command`, real SQLite and the existing
controlled external-process/provider fixture. It asserts mode metadata is
available with zero effect-count changes, unchanged task state and retained
human epoch; URL reads remain refused. It does not certify a real browser
replacement or account authentication.

The first Cargo selection also compiled unrelated integration targets and hit
unchanged `profile_duplicate_operation` E0433 errors resolving `crate::native`.
That failure is retained. The maintained focused lane uses `--bin agent-browser`
and isolated runtime directories; no unrelated integration repair is included.


## Validation receipt

Local Linux, Rust stable, retained Cargo Signal CLI attempts. No hosted Actions.

- RED: `scripts/ci/rust-tests.sh --focused human_handoff_keeps_inert_mode_operation_lookup_available`: one failing test with the same observation-paused error; actual test runtime 4.79 seconds after compilation.
- GREEN: same focused command: 1 passed.
- `scripts/ci/rust-tests.sh --focused operation_lookup_tests`: 6 passed.
- `scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml --test browser_mode_operation`: 16 passed.
- `scripts/ci/cargo-safe.sh clippy --workspace --manifest-path Cargo.toml -- -D warnings`: passed, zero warnings.
- `scripts/ci/cargo-safe.sh fmt --all --manifest-path Cargo.toml -- --check`: passed.
- `pnpm test:policy-wiring`: 11 passed.
- `pnpm test:remote-view-handoff-docs`, `pnpm test:documentation-links -- --base ceac15364`, `pnpm --dir docs build`, and `git diff --check`: passed.

The initial broad Cargo selection failed before executing tests because the
unchanged profile-duplicate integration target cannot resolve `crate::native`.
That is not a clean full-suite claim. A subsequent zero-match CLI mode filter
was also retained and is not counted as coverage; the explicit integration
target above ran all 16 actual mode-state tests.

The selector's workstation fixture suggestions are caused by the help-text-only
edit in output.rs; no installer, provisioning, VM, Guacamole or PostgreSQL logic
changed, so those lanes were not rerun for this source repair. Installed-skill
parity is intentionally pending publication; no shared guidance was overwritten.
The candidate installer fixture and installed workflow must be qualified against
the exact sealed artifact before any installed acceptance claim.

Retain this clean primary checkout for candidate and open installed-acceptance
custody; review 2026-11-10. No auxiliary checkout was created. The temporary operator
browser closed after an ambiguous close reply; subsequent exact task lookup and
OS root absence confirm closure. No input retry was made. The signed-in original
profile and private artifacts remain user-scoped.
