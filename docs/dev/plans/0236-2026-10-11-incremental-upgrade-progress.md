# Incremental workstation install progress

Product lane: PL-BUGFIX
Work item: #181
Branch: fix/upgrade-progress-stream-181
State: COMPLETED
Version: 2
Lane: P236

## Current State

P231 integrated truthful status timing and a bounded doctor subprocess. A current-source public isolated installer completed in 50.8 seconds with one terminal JSON output chunk and no earlier output. The exact pinned unsealed debug candidate was built from clean canonical main. The probe's progress assertion failed; installer apply succeeded and exact cleanup found no owned hosts or residue. Its planned unit-shim delay did not execute, so this is real staging evidence, not injected-delay or production qualification.

## Bounded outcome

Expose optional incremental machine-readable install observations while preserving the existing terminal JSON report. Report durable phase transitions separately from observer liveness; a periodic observation must never manufacture work progress, extend a validation deadline, or change a transaction revision. Cover the public command and failure path, not only a formatting helper. Preserve separate build and apply timing boundaries and report their qualification honestly.

The operator can distinguish an active installer in a recorded phase from a silent command. The observer must terminate with the invocation, omit private runtime identities and paths, and leave no worker or process residue.

## Ownership and dependencies

This session owns the installer progress adapter, parser integration and regressions. P232 retains protected enrollment, browser manager and recovery ownership. Its branch-local catalog explicitly assigned P231 upgrade and shared guidance ownership; this successor changes upgrade-only guidance sections. Preserve concurrent enrollment guidance and record any concrete conflict before editing the overlapping section. Each lane owns its own plan/catalog/index entries. The canonical main checkout stays clean; this primary repair checkout starts at freshly fetched origin/main.

## Acceptance and closeout

First minimize the public red loop and test ranked hypotheses: terminal-only dispatch, JSON suppression, and staging before durable progress recording. Add meaningful deterministic tests with injected time and output capture before the smallest repair. Rerun the original public installer with output timestamps and exact cleanup. Update CLI help, README, agent skill, docs-site guidance and source comments for the final public behavior. Run changed-surface Rust tests, cargo-safe format and strict workspace Clippy, relevant repository fixtures and documentation checks. Integrate only through an issue-linked PR; inspect governed worktree closeout and assign every pinned candidate a disposition.

Production publication, provider restoration, private authentication, reboot and CI dispatch are excluded. This slice does not close #181 unless its wider installed/build-and-apply gates are independently qualified. The reset goal window ends before 2026-10-11T20:43:17Z or before two million additional tokens from baseline 56881, whichever comes first. Preserve the prior accounting.

## Candidate entry-point correction

The original issue uses candidate install. Its current public adapter rejects progress-json before reading any artifact; a zero-effect public invocation reproduces the missing route. Ranked hypotheses are parser rejection, loss during reviewed-payload adaptation and preflight verification outside the observer. The parser rejection is established; canonical source also shows the reviewed adapter builds new workstation arguments after verification.

Route the same opt-in observer through the reviewed candidate adapter before expensive binary verification, then transfer it to workstation apply. Preserve global JSON restrictions, artifact seals, source verification and coordination custody. Add a parser regression before repair and qualify the actual candidate entry point with synthetic sealed evidence or a typed early failure. Reuse passed workstation gates only for unchanged inputs; rerun affected adapter and public checks after this correction. No production installation authority is added.

## Outcome

The bounded source outcome is complete. The final debug CLI built from frozen executable source 8cd1921891f6ed113d9a6acde2e97c4d6a74c10d passes both public progress success/failure fixtures, with first output within 100 milliseconds and zero cleanup residue. Final candidate, workstation and output suites pass with 11, 192 and 39 tests; strict workspace Clippy and format pass. The [accepted source note](../notes/0236-2026-10-11-incremental-upgrade-progress.md) binds identities, measurements, retained gates and limitations.

PR256 owns integration. This completion does not close wider issue 181 or qualify a production-shaped build, installed runtime, provider or separate build/apply receipts. The user waived the previous host reserve for validation; the original resource window and cumulative evidence remain preserved.
