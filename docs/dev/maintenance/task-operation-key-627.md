# Concurrent task-operation key publication

Product lane: PL-BUGFIX. Work item: #627. Owner: ecochran76.
Branch: `fix/task-operation-key-publication`.
Baseline: `ae639b178efb6c5ef3164c8d9a895b89ec45c56c`.

## Outcome and bounds

Two independent clients making their first browser writes must use the same
complete private MAC key. Preserve existing keys, permissions, original operation
history, and fail-closed handling of missing, malformed or substituted keys.
No production install, restart, allocator tuning or real website input.

Use at most three repair attempts, one self-review and two hardening checkpoints
within a 45-minute implementation packet. The source gates passed at the
validation checkpoint; one bounded 15-minute continuation covers exact-candidate
MCP acceptance and PR closeout. A material new failure triggers bounded
replanning rather than silently widening this repair. Current state: source
repair and disposable candidate acceptance complete; PR integration pending.

## Reproduction and diagnosis

The public MCP loop uses two independent clients, two synthetic tabs on one
disposable Chrome browser, and concurrent first form writes without serial
initialization. A fixture-only preload pauses the real exclusive key creation
before its first write. It reproduces `browser_task_operation_key_invalid`
twice and leaves zero fixture process residue.

Ranked explanations are a visible incomplete key, incorrect file permissions,
and unrelated task/process identity refusal. A cross-process regression must
exercise the actual key initializer and paused writer before a repair is applied.
The fixture must establish valid process identity through ordinary existing-browser
admission and must never fabricate identity or retry uncertain website effects.

The native regression then reproduced the same refusal in 0.05 seconds while
observing a zero-byte file with mode 0600. The independent reader failed while the
initial creator completed. The cause is create-before-write publication, rather
than bad permissions or unverified browser identity.

The repair takes a stable private cross-process sidecar lock before checking or
loading the key, and holds it through first creation, write, file sync and
directory sync. Contention waits up to five seconds. The sidecar is not removed
while other processes may use it. Existing-key validation and no-regeneration
rules remain in force. Linux subprocess tests cover initialization, an abandoned
partial write, malformed or missing keys, and substituted or public files.

## Custody and overlap

Reuse the clean canonical checkout from the integrated 607 lane. Preserve its
branch, immutable artifacts and all other worktrees, including missing/prunable
registrations. No new checkout or pruning. PR626 owns Native invocation changes
in the same source file; 627 owns only the key helpers and regression tests.
This lane owns its new catalog entry and this bounded receipt.

## Validation

Require deterministic cross-process RED before repair, then GREEN with identical
keys and all existing malformed/missing-key refusals. Run the focused existing
task-operation tests, Cargo-safe Rustfmt and strict workspace Clippy. Build an
isolated candidate and rerun the public cold first-write loop without serial
warmup. Preserve the first failed receipts. Hosted CI is excluded by operator
policy. Source validation is separate from installed or production acceptance.

Observed local gates: four key-publication regressions pass with one ignored
subprocess helper; the broader focused task-operation lane passes 20 tests with
the same helper ignored. Cargo-safe Rustfmt and strict workspace Clippy pass.
Policy wiring passes 11 cases and changed documentation links pass. Tests used
four Cargo jobs, a 10 GiB memory maximum and test debug information disabled;
the 16 GiB host reserve was preserved. The first native fixture-path failure and
subsequent exact-error RED remain retained outside Git with command receipts.

The frozen debug CLI passes the public cold first-write workflow without serial
warmup: two clients with the original after-create pause, plus four clients on
two borrowed browsers with default and two-arena allocator settings. All 34 task
lifecycles verify form values and peer preservation, then finish with zero
fixture process residue. A fresh OS readback verifies the positively identified
fixture process identities independently of Service State. No production binary,
configuration, process or allocator setting is changed.

The single source self-review checked guard lifetime through durable sync,
cross-process exclusion, bounded contention, unchanged key/history refusals and
absence of key material in diagnostics. Linux-only subprocess fixtures avoid
depending on another platform's home-directory override behavior. No platform
matrix or installed-runtime acceptance is claimed.

Local commands used for source qualification:

```bash
CARGO_PROFILE_TEST_DEBUG=0 scripts/ci/rust-tests.sh --focused key_publication
CARGO_PROFILE_TEST_DEBUG=0 scripts/ci/rust-tests.sh --focused task_operation
scripts/ci/cargo-safe.sh fmt --all --manifest-path Cargo.toml -- --check
CARGO_PROFILE_DEV_DEBUG=0 scripts/ci/cargo-safe.sh clippy --workspace --manifest-path Cargo.toml -- -D warnings
pnpm test:policy-wiring
pnpm test:documentation-links -- --base origin/main
```

The isolated CLI uses the development profile with debug information disabled;
it is a test executable, not a production-shaped workstation installation.
Compilation, test setup, RED and GREEN attempt receipts remain private and
append-only outside Git. The inherited memory-reserve gate was not lowered.

No public command, flag, environment setting or service schema is added by this
repair; it restores the existing private-key initialization contract.
