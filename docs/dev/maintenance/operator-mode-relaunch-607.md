# Operator-confirmed mode relaunch during human handoff

Product lane: PL-BUGFIX
Work item: 607
Branch: fix/607-operator-mode-relaunch
State: OPEN

## Current state

The mode-lookup and explicit owned-close regressions failed with their exact
control refusals before repair, then passed. Source `db48401b6` was sealed as
candidate-98240c7127440729-7a208ba9dacee065. Its exact installer fixture passed
(two owned hosts, zero residue), and isolated fg607 publication passed three
open/read/close cycles with production unchanged. Development doctor passed all
checks except optional unconfigured local ingress; no public dev viewer is claimed.
Strict workspace Clippy, 3 control tests and 7 host-policy tests passed. The extra
161-test consumer sweep timed out after 79 passing cases, including both repair
regressions; it is not a complete-suite claim. No synthetic children remain.

The operator's original process was explicitly closed through an identity-bound
pidfd/SIGTERM. Exact exit, retained profile and no matching Chrome process were
confirmed. Stale lock entries remain for lifecycle recovery. Public reopen still
rejects the historical human hold. Account continuity remains unverified.

A pre-install readback found newer installed source `8edb25b3` from #596. The first
candidate is retained, not installed. Integrate both native-login commits from the
common `ceac15364` base and preserve the installed native saved-login tooling.
The only textual conflict is the lane catalog; both records are retained. Native
login code matches `8edb25b3`, and close/relaunch code matches `db48401b6`.
Combined qualification passes: 17 native-login tests, the explicit owned-close
regression, 6 mode-lookup tests, strict workspace Clippy, Rustfmt, SDK/types,
API/MCP parity, policy wiring, handoff docs, links and documentation build. The
merge is ready for a clean combined production build.

Reuse the canonical checkout; no additional checkout is created. This lane owns
operation lookup and explicit-close admission plus required guidance. Retirement,
manual recovery and native-login implementations retain their existing ownership.
Next: qualify one combined production-shaped candidate and preserving runtime
transition, then close/archive the ended task and reopen its same existing profile
with CDP under current operator authority. Keep installation, process replacement,
account readback and scientific acceptance separate.

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
